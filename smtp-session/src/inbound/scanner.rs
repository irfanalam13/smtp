use std::{io, net::SocketAddr, time::Duration};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

const CLAMD_CHUNK_BYTES: usize = 64 * 1024;
const CLAMD_REPLY_LIMIT: usize = 4096;
const SPAMD_HEADER_LIMIT: usize = 8192;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("message was classified as malware")]
    Infected,
    #[error("message was classified as spam")]
    Spam,
    #[error("malware scanner is unavailable or returned an invalid response")]
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct SpamAssassinScanner {
    address: SocketAddr,
    timeout: Duration,
}

impl SpamAssassinScanner {
    pub fn new(address: SocketAddr, timeout: Duration) -> io::Result<Self> {
        if timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SpamAssassin scanner timeout must be greater than zero",
            ));
        }
        Ok(Self { address, timeout })
    }

    pub async fn scan(&self, message: &[u8]) -> Result<(), ScanError> {
        timeout(self.timeout, async {
            let mut stream = TcpStream::connect(self.address)
                .await
                .map_err(|_| ScanError::Unavailable)?;
            Self::check_stream(&mut stream, message).await
        })
        .await
        .map_err(|_| ScanError::Unavailable)?
    }

    async fn check_stream<S>(stream: &mut S, message: &[u8]) -> Result<(), ScanError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let request = format!(
            "CHECK SPAMC/1.5\r\nUser: smtp-platform\r\nContent-length: {}\r\n\r\n",
            message.len()
        );
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|_| ScanError::Unavailable)?;
        stream
            .write_all(message)
            .await
            .map_err(|_| ScanError::Unavailable)?;
        let mut response = Vec::with_capacity(512);
        while response.len() < SPAMD_HEADER_LIMIT {
            let byte = stream.read_u8().await.map_err(|_| ScanError::Unavailable)?;
            response.push(byte);
            if response.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        if !response.ends_with(b"\r\n\r\n") {
            return Err(ScanError::Unavailable);
        }
        let headers = std::str::from_utf8(&response).map_err(|_| ScanError::Unavailable)?;
        let mut lines = headers.lines();
        let status = lines.next().ok_or(ScanError::Unavailable)?;
        if !status.starts_with("SPAMD/1.5 0 EX_OK") {
            return Err(ScanError::Unavailable);
        }
        let mut spam_flag = None;
        for line in lines {
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("Spam")
            {
                if spam_flag.is_some() {
                    return Err(ScanError::Unavailable);
                }
                let flag = value.split(';').next().unwrap_or_default().trim();
                spam_flag = match flag {
                    "True" => Some(true),
                    "False" => Some(false),
                    _ => return Err(ScanError::Unavailable),
                };
            }
        }
        match spam_flag {
            Some(true) => Err(ScanError::Spam),
            Some(false) => Ok(()),
            None => Err(ScanError::Unavailable),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClamdScanner {
    address: SocketAddr,
    timeout: Duration,
}

impl ClamdScanner {
    pub fn new(address: SocketAddr, timeout: Duration) -> io::Result<Self> {
        if timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ClamAV scanner timeout must be greater than zero",
            ));
        }
        Ok(Self { address, timeout })
    }

    pub async fn scan(&self, message: &[u8]) -> Result<(), ScanError> {
        timeout(self.timeout, self.scan_inner(message))
            .await
            .map_err(|_| ScanError::Unavailable)?
    }

    async fn scan_inner(&self, message: &[u8]) -> Result<(), ScanError> {
        let mut stream = TcpStream::connect(self.address)
            .await
            .map_err(|_| ScanError::Unavailable)?;
        Self::scan_stream(&mut stream, message).await
    }

    async fn scan_stream<S>(stream: &mut S, message: &[u8]) -> Result<(), ScanError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        stream
            .write_all(b"zINSTREAM\0")
            .await
            .map_err(|_| ScanError::Unavailable)?;
        for chunk in message.chunks(CLAMD_CHUNK_BYTES) {
            let length = u32::try_from(chunk.len()).map_err(|_| ScanError::Unavailable)?;
            stream
                .write_all(&length.to_be_bytes())
                .await
                .map_err(|_| ScanError::Unavailable)?;
            stream
                .write_all(chunk)
                .await
                .map_err(|_| ScanError::Unavailable)?;
        }
        stream
            .write_all(&0_u32.to_be_bytes())
            .await
            .map_err(|_| ScanError::Unavailable)?;
        let mut response = Vec::with_capacity(128);
        let mut limited = stream.take(CLAMD_REPLY_LIMIT as u64);
        limited
            .read_to_end(&mut response)
            .await
            .map_err(|_| ScanError::Unavailable)?;
        if response.len() == CLAMD_REPLY_LIMIT && !response.ends_with(&[0]) {
            return Err(ScanError::Unavailable);
        }
        let response = response.strip_suffix(&[0]).unwrap_or(&response);
        if response.ends_with(b": OK") {
            Ok(())
        } else if response.ends_with(b" FOUND") {
            Err(ScanError::Infected)
        } else {
            Err(ScanError::Unavailable)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_rejects_zero_timeout() {
        assert!(ClamdScanner::new("127.0.0.1:3310".parse().unwrap(), Duration::ZERO).is_err());
    }

    async fn fake_clamd_exchange(response: &'static [u8]) -> Result<(), ScanError> {
        let (mut client, mut server) = tokio::io::duplex(1024);
        let peer = tokio::spawn(async move {
            let mut command = [0_u8; 10];
            server.read_exact(&mut command).await.unwrap();
            assert_eq!(&command, b"zINSTREAM\0");
            let mut length = [0_u8; 4];
            server.read_exact(&mut length).await.unwrap();
            assert_eq!(u32::from_be_bytes(length), 4);
            let mut message = [0_u8; 4];
            server.read_exact(&mut message).await.unwrap();
            assert_eq!(&message, b"test");
            server.read_exact(&mut length).await.unwrap();
            assert_eq!(u32::from_be_bytes(length), 0);
            server.write_all(response).await.unwrap();
            server.shutdown().await.unwrap();
        });
        let result = ClamdScanner::scan_stream(&mut client, b"test").await;
        peer.await.unwrap();
        result
    }

    async fn fake_spamd_exchange(response: &'static [u8]) -> Result<(), ScanError> {
        let (mut client, mut server) = tokio::io::duplex(1024);
        let peer = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                server.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
            }
            assert!(String::from_utf8_lossy(&request).contains("Content-length: 4\r\n"));
            let mut message = [0_u8; 4];
            server.read_exact(&mut message).await.unwrap();
            assert_eq!(&message, b"test");
            server.write_all(response).await.unwrap();
        });
        let result = SpamAssassinScanner::check_stream(&mut client, b"test").await;
        peer.await.unwrap();
        result
    }

    #[tokio::test]
    async fn clamd_instream_clean_response_is_accepted() {
        assert!(matches!(fake_clamd_exchange(b"stream: OK\0").await, Ok(())));
    }

    #[tokio::test]
    async fn clamd_instream_infected_response_is_rejected() {
        assert!(matches!(
            fake_clamd_exchange(b"stream: Eicar FOUND\0").await,
            Err(ScanError::Infected)
        ));
    }

    #[tokio::test]
    async fn spamd_clean_result_is_accepted() {
        let result =
            fake_spamd_exchange(b"SPAMD/1.5 0 EX_OK\r\nSpam: False ; 0.0 / 5.0\r\n\r\n").await;
        assert!(matches!(result, Ok(())), "unexpected result: {result:?}");
    }

    #[tokio::test]
    async fn spamd_spam_result_is_rejected() {
        let result =
            fake_spamd_exchange(b"SPAMD/1.5 0 EX_OK\r\nSpam: True ; 8.2 / 5.0\r\n\r\n").await;
        assert!(
            matches!(result, Err(ScanError::Spam)),
            "unexpected result: {result:?}"
        );
    }
}
