use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachmentBlob {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanVerdict {
    Clean,
    Infected,
}

pub trait AttachmentScanner: Send + Sync {
    fn scan(&self, content: &[u8]) -> Result<ScanVerdict, AttachmentError>;
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AttachmentError {
    #[error("attachment filename is invalid")]
    InvalidFilename,
    #[error("attachment content type is invalid")]
    InvalidContentType,
    #[error("attachment is infected")]
    Infected,
    #[error("attachment scanner failed")]
    ScannerFailed,
    #[error("attachment not found")]
    NotFound,
    #[error("attachment tenant encryption key is not configured")]
    MissingTenantKey,
    #[error("attachment upload is not found")]
    UploadNotFound,
    #[error("attachment chunk sequence is invalid")]
    InvalidChunkSequence,
    #[error("attachment exceeds configured size limit")]
    TooLarge,
    #[error("attachment chunk size is invalid")]
    InvalidChunkSize,
    #[error("attachment data integrity check failed")]
    IntegrityFailure,
    #[error("attachment encrypted content is invalid")]
    EncryptionFailure,
    #[error("attachment compressed data is invalid")]
    CompressionFailure,
    #[error("attachment offset exceeds content length")]
    InvalidOffset,
}

struct UploadSession {
    tenant_id: Uuid,
    filename: String,
    content_type: String,
    next_chunk: u64,
    content: Vec<u8>,
}

struct StoredContent {
    nonce: [u8; 24],
    key_id: Uuid,
    encrypted_compressed: Vec<u8>,
}

#[derive(Default)]
pub struct AttachmentStore {
    tenant_keys: HashMap<(Uuid, Uuid), [u8; 32]>,
    active_key_ids: HashMap<Uuid, Uuid>,
    contents: HashMap<(Uuid, [u8; 32]), StoredContent>,
    metadata: HashMap<Uuid, AttachmentBlob>,
    uploads: HashMap<Uuid, UploadSession>,
    max_attachment_bytes: usize,
    max_chunk_bytes: usize,
}

impl AttachmentStore {
    pub fn new(max_attachment_bytes: usize, max_chunk_bytes: usize) -> Self {
        Self {
            max_attachment_bytes,
            max_chunk_bytes,
            ..Self::default()
        }
    }

    pub fn set_tenant_key(&mut self, tenant_id: Uuid, key: [u8; 32]) {
        let key_id = Uuid::new_v4();
        self.tenant_keys.insert((tenant_id, key_id), key);
        self.active_key_ids.insert(tenant_id, key_id);
    }

    pub fn begin_upload(
        &mut self,
        tenant_id: Uuid,
        filename: &str,
        content_type: &str,
    ) -> Result<Uuid, AttachmentError> {
        validate_filename(filename)?;
        validate_content_type(content_type)?;
        if !self.active_key_ids.contains_key(&tenant_id) {
            return Err(AttachmentError::MissingTenantKey);
        }
        let id = Uuid::new_v4();
        self.uploads.insert(
            id,
            UploadSession {
                tenant_id,
                filename: filename.to_owned(),
                content_type: content_type.to_ascii_lowercase(),
                next_chunk: 0,
                content: Vec::new(),
            },
        );
        Ok(id)
    }

    pub fn upload_chunk(
        &mut self,
        upload_id: Uuid,
        sequence: u64,
        chunk: &[u8],
    ) -> Result<(), AttachmentError> {
        if self.max_chunk_bytes == 0 || chunk.len() > self.max_chunk_bytes {
            return Err(AttachmentError::InvalidChunkSize);
        }
        let upload = self
            .uploads
            .get_mut(&upload_id)
            .ok_or(AttachmentError::UploadNotFound)?;
        if sequence != upload.next_chunk {
            return Err(AttachmentError::InvalidChunkSequence);
        }
        let new_len = upload
            .content
            .len()
            .checked_add(chunk.len())
            .filter(|size| *size <= self.max_attachment_bytes)
            .ok_or(AttachmentError::TooLarge)?;
        upload.content.reserve(new_len - upload.content.len());
        upload.content.extend_from_slice(chunk);
        upload.next_chunk = upload
            .next_chunk
            .checked_add(1)
            .ok_or(AttachmentError::InvalidChunkSequence)?;
        Ok(())
    }

    pub fn complete_upload(
        &mut self,
        upload_id: Uuid,
        scanner: &dyn AttachmentScanner,
    ) -> Result<AttachmentBlob, AttachmentError> {
        let upload = self
            .uploads
            .remove(&upload_id)
            .ok_or(AttachmentError::UploadNotFound)?;
        if scanner.scan(&upload.content)? == ScanVerdict::Infected {
            return Err(AttachmentError::Infected);
        }

        let digest: [u8; 32] = Sha256::digest(&upload.content).into();
        let content_key = (upload.tenant_id, digest);
        if !self.contents.contains_key(&content_key) {
            let key_id = self
                .active_key_ids
                .get(&upload.tenant_id)
                .copied()
                .ok_or(AttachmentError::MissingTenantKey)?;
            let tenant_key = self
                .tenant_keys
                .get(&(upload.tenant_id, key_id))
                .ok_or(AttachmentError::MissingTenantKey)?;
            let compressed = compress(&upload.content)?;
            let nonce = create_nonce();
            let encryption_key = Key::from(*tenant_key);
            let cipher = XChaCha20Poly1305::new(&encryption_key);
            let encrypted_compressed = cipher
                .encrypt(
                    &XNonce::from(nonce),
                    Payload {
                        msg: &compressed,
                        aad: upload.tenant_id.as_bytes(),
                    },
                )
                .map_err(|_| AttachmentError::EncryptionFailure)?;
            self.contents.insert(
                content_key,
                StoredContent {
                    nonce,
                    key_id,
                    encrypted_compressed,
                },
            );
        }

        let blob = AttachmentBlob {
            id: Uuid::new_v4(),
            tenant_id: upload.tenant_id,
            filename: upload.filename,
            content_type: upload.content_type,
            size: upload.content.len(),
            sha256: digest,
        };
        self.metadata.insert(blob.id, blob.clone());
        Ok(blob)
    }

    pub fn put(
        &mut self,
        tenant_id: Uuid,
        filename: &str,
        content_type: &str,
        content: &[u8],
        scanner: &dyn AttachmentScanner,
    ) -> Result<AttachmentBlob, AttachmentError> {
        let upload_id = self.begin_upload(tenant_id, filename, content_type)?;
        if self.max_chunk_bytes == 0 {
            self.uploads.remove(&upload_id);
            return Err(AttachmentError::InvalidChunkSize);
        }
        for (sequence, chunk) in content.chunks(self.max_chunk_bytes).enumerate() {
            let sequence =
                u64::try_from(sequence).map_err(|_| AttachmentError::InvalidChunkSequence)?;
            if let Err(error) = self.upload_chunk(upload_id, sequence, chunk) {
                self.uploads.remove(&upload_id);
                return Err(error);
            }
        }
        self.complete_upload(upload_id, scanner)
    }

    pub fn abort_upload(&mut self, upload_id: Uuid) -> bool {
        self.uploads.remove(&upload_id).is_some()
    }

    pub fn read_chunk(
        &self,
        tenant_id: Uuid,
        attachment_id: Uuid,
        offset: usize,
        max_bytes: usize,
    ) -> Result<Vec<u8>, AttachmentError> {
        if max_bytes == 0 || max_bytes > self.max_chunk_bytes {
            return Err(AttachmentError::InvalidChunkSize);
        }
        let blob = self
            .metadata
            .get(&attachment_id)
            .filter(|blob| blob.tenant_id == tenant_id)
            .ok_or(AttachmentError::NotFound)?;
        if offset > blob.size {
            return Err(AttachmentError::InvalidOffset);
        }
        let bytes = self.decrypt_content(blob)?;
        let end = offset.saturating_add(max_bytes).min(bytes.len());
        Ok(bytes[offset..end].to_vec())
    }

    pub fn download_disposition(
        &self,
        tenant_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<String, AttachmentError> {
        let blob = self
            .metadata
            .get(&attachment_id)
            .filter(|blob| blob.tenant_id == tenant_id)
            .ok_or(AttachmentError::NotFound)?;
        Ok(format!(
            "attachment; filename*=UTF-8''{}",
            percent_encode_filename(&blob.filename)
        ))
    }

    pub fn text_preview(
        &self,
        tenant_id: Uuid,
        attachment_id: Uuid,
        max_bytes: usize,
    ) -> Result<String, AttachmentError> {
        let blob = self
            .metadata
            .get(&attachment_id)
            .filter(|blob| blob.tenant_id == tenant_id)
            .ok_or(AttachmentError::NotFound)?;
        if blob.content_type != "text/plain" || max_bytes == 0 {
            return Err(AttachmentError::InvalidContentType);
        }
        let bytes = self.decrypt_content(blob)?;
        Ok(String::from_utf8_lossy(&bytes[..bytes.len().min(max_bytes)]).into_owned())
    }

    fn decrypt_content(&self, blob: &AttachmentBlob) -> Result<Vec<u8>, AttachmentError> {
        let stored = self
            .contents
            .get(&(blob.tenant_id, blob.sha256))
            .ok_or(AttachmentError::NotFound)?;
        let tenant_key = self
            .tenant_keys
            .get(&(blob.tenant_id, stored.key_id))
            .ok_or(AttachmentError::MissingTenantKey)?;
        let encryption_key = Key::from(*tenant_key);
        let cipher = XChaCha20Poly1305::new(&encryption_key);
        let compressed = cipher
            .decrypt(
                &XNonce::from(stored.nonce),
                Payload {
                    msg: &stored.encrypted_compressed,
                    aad: blob.tenant_id.as_bytes(),
                },
            )
            .map_err(|_| AttachmentError::EncryptionFailure)?;
        let content = decompress_bounded(&compressed, self.max_attachment_bytes)?;
        let digest: [u8; 32] = Sha256::digest(&content).into();
        if digest != blob.sha256 || content.len() != blob.size {
            return Err(AttachmentError::IntegrityFailure);
        }
        Ok(content)
    }
}

fn validate_filename(filename: &str) -> Result<(), AttachmentError> {
    if filename.is_empty()
        || filename.len() > 255
        || filename == "."
        || filename == ".."
        || filename.contains(['/', '\\', '\0'])
        || filename.chars().any(char::is_control)
    {
        return Err(AttachmentError::InvalidFilename);
    }
    Ok(())
}

fn validate_content_type(content_type: &str) -> Result<(), AttachmentError> {
    let Some((media_type, subtype)) = content_type.split_once('/') else {
        return Err(AttachmentError::InvalidContentType);
    };
    if media_type.is_empty()
        || subtype.is_empty()
        || subtype.contains('/')
        || content_type.len() > 127
        || content_type.chars().any(|character| {
            !(character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '!' | '#' | '$' | '&' | '^' | '_' | '.' | '+' | '-'
                ))
                && character != '/'
        })
    {
        return Err(AttachmentError::InvalidContentType);
    }
    Ok(())
}

fn compress(content: &[u8]) -> Result<Vec<u8>, AttachmentError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(content)
        .map_err(|_| AttachmentError::CompressionFailure)?;
    encoder
        .finish()
        .map_err(|_| AttachmentError::CompressionFailure)
}

fn decompress_bounded(content: &[u8], max_bytes: usize) -> Result<Vec<u8>, AttachmentError> {
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut decoder = GzDecoder::new(content).take(limit);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|_| AttachmentError::CompressionFailure)?;
    if decoded.len() > max_bytes {
        return Err(AttachmentError::TooLarge);
    }
    Ok(decoded)
}

fn create_nonce() -> [u8; 24] {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut nonce = [0; 24];
    nonce[..16].copy_from_slice(first.as_bytes());
    nonce[16..].copy_from_slice(&second.as_bytes()[..8]);
    nonce
}

fn percent_encode_filename(filename: &str) -> String {
    let mut encoded = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Clean;

    impl AttachmentScanner for Clean {
        fn scan(&self, _: &[u8]) -> Result<ScanVerdict, AttachmentError> {
            Ok(ScanVerdict::Clean)
        }
    }

    struct Infected;

    impl AttachmentScanner for Infected {
        fn scan(&self, _: &[u8]) -> Result<ScanVerdict, AttachmentError> {
            Ok(ScanVerdict::Infected)
        }
    }

    fn store() -> (AttachmentStore, Uuid) {
        let tenant_id = Uuid::new_v4();
        let mut store = AttachmentStore::new(1024, 4);
        store.set_tenant_key(tenant_id, [7; 32]);
        (store, tenant_id)
    }

    #[test]
    fn uploads_in_order_and_returns_authenticated_download_chunks() {
        let (mut store, tenant_id) = store();
        let upload = store
            .begin_upload(tenant_id, "résumé.txt", "text/plain")
            .unwrap();
        assert_eq!(
            store.upload_chunk(upload, 1, b"no"),
            Err(AttachmentError::InvalidChunkSequence)
        );
        store.upload_chunk(upload, 0, b"abcd").unwrap();
        store.upload_chunk(upload, 1, b"efgh").unwrap();
        let blob = store.complete_upload(upload, &Clean).unwrap();
        assert_eq!(store.read_chunk(tenant_id, blob.id, 2, 4).unwrap(), b"cdef");
        assert_eq!(
            store.download_disposition(tenant_id, blob.id).unwrap(),
            "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.txt"
        );
        assert_eq!(store.text_preview(tenant_id, blob.id, 5).unwrap(), "abcde");
        assert_eq!(
            store.read_chunk(Uuid::new_v4(), blob.id, 0, 4),
            Err(AttachmentError::NotFound)
        );
    }

    #[test]
    fn encrypts_compresses_and_deduplicates_only_within_tenant() {
        let (mut store, tenant_id) = store();
        let first = store
            .put(
                tenant_id,
                "a.txt",
                "text/plain",
                b"repeated payload",
                &Clean,
            )
            .unwrap();
        let second = store
            .put(
                tenant_id,
                "b.txt",
                "text/plain",
                b"repeated payload",
                &Clean,
            )
            .unwrap();
        let other_tenant = Uuid::new_v4();
        store.set_tenant_key(other_tenant, [8; 32]);
        store.set_tenant_key(tenant_id, [9; 32]);
        let third = store
            .put(
                other_tenant,
                "a.txt",
                "text/plain",
                b"repeated payload",
                &Clean,
            )
            .unwrap();
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.sha256, third.sha256);
        assert_eq!(store.contents.len(), 2);
        assert_eq!(
            store.read_chunk(tenant_id, second.id, 0, 4).unwrap(),
            b"repe"
        );
        assert_eq!(
            store.read_chunk(other_tenant, first.id, 0, 4),
            Err(AttachmentError::NotFound)
        );
    }

    #[test]
    fn rejects_invalid_metadata_and_scanner_verdicts() {
        let (mut store, tenant_id) = store();
        assert_eq!(
            store.begin_upload(tenant_id, "../unsafe.txt", "text/plain"),
            Err(AttachmentError::InvalidFilename)
        );
        assert_eq!(
            store.begin_upload(tenant_id, "file.txt", "text/plain\r\nx: y"),
            Err(AttachmentError::InvalidContentType)
        );
        assert_eq!(
            store.put(tenant_id, "bad.txt", "text/plain", b"bad", &Infected),
            Err(AttachmentError::Infected)
        );
        assert_eq!(
            store.put(tenant_id, "oversize", "text/plain", &[0; 1025], &Clean),
            Err(AttachmentError::TooLarge)
        );
    }
}
