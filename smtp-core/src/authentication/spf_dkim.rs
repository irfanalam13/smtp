use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DnsRecord {
    A(String),
    Aaaa(String),
    Mx { preference: u16, host: String },
    Ptr(String),
    Txt(String),
}

#[derive(Clone, Debug, Default)]
pub struct DnsCache {
    records: HashMap<(String, String), Vec<DnsRecord>>,
}

impl DnsCache {
    pub fn insert(&mut self, name: &str, record_type: &str, records: Vec<DnsRecord>) {
        self.records.insert(
            (
                name.trim_end_matches('.').to_ascii_lowercase(),
                record_type.to_ascii_uppercase(),
            ),
            records,
        );
    }

    pub fn get(&self, name: &str, record_type: &str) -> Option<&[DnsRecord]> {
        self.records
            .get(&(
                name.trim_end_matches('.').to_ascii_lowercase(),
                record_type.to_ascii_uppercase(),
            ))
            .map(Vec::as_slice)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpfMechanism {
    All,
    Ip4(String),
    Ip6(String),
    A,
    Mx,
    Include(String),
    Exists(String),
    Unknown(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpfResult {
    Pass,
    Fail,
    SoftFail,
    Neutral,
    None,
    PermError,
    TempError,
}

pub fn evaluate_spf(record: &str, sender_ip: &str, _helo_domain: &str) -> SpfResult {
    let tokens = record.split_whitespace().collect::<Vec<_>>();
    if tokens.first().copied() != Some("v=spf1") {
        return SpfResult::None;
    }
    for token in tokens.into_iter().skip(1) {
        let (qualifier, mechanism) = match token.chars().next() {
            Some('+') | None => ('+', &token[1..]),
            Some(value @ ('-' | '~' | '?')) => (value, &token[1..]),
            Some(_) => ('+', token),
        };
        let matches = mechanism == "all"
            || mechanism
                .strip_prefix("ip4:")
                .is_some_and(|value| value == sender_ip)
            || mechanism
                .strip_prefix("ip6:")
                .is_some_and(|value| value == sender_ip);
        if matches {
            return match qualifier {
                '+' => SpfResult::Pass,
                '-' => SpfResult::Fail,
                '~' => SpfResult::SoftFail,
                '?' => SpfResult::Neutral,
                _ => SpfResult::PermError,
            };
        }
    }
    SpfResult::Neutral
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkimCanonicalization {
    Simple,
    Relaxed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DkimSignature {
    pub domain: String,
    pub selector: String,
    pub header: String,
}

pub trait DkimSigner {
    fn sign(&self, signing_input: &[u8]) -> Result<Vec<u8>, DkimError>;
}

pub trait DkimVerifier {
    fn verify(&self, signing_input: &[u8], signature: &[u8]) -> Result<bool, DkimError>;
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DkimError {
    #[error("invalid DKIM input")]
    InvalidInput,
    #[error("signing or verification failed")]
    Crypto,
}

pub fn sign_dkim_headers(
    headers: &[(String, String)],
    body: &[u8],
    domain: &str,
    selector: &str,
    canonicalization: DkimCanonicalization,
    signer: &dyn DkimSigner,
) -> Result<DkimSignature, DkimError> {
    if domain.is_empty() || selector.is_empty() || headers.is_empty() {
        return Err(DkimError::InvalidInput);
    }
    let mut input = canonicalize_body(body, canonicalization);
    for (name, value) in headers {
        input.extend_from_slice(&canonicalize_header(name, value, canonicalization));
    }
    let signature = signer.sign(&input)?;
    let encoded = BASE64.encode(signature);
    Ok(DkimSignature {
        domain: domain.to_ascii_lowercase(),
        selector: selector.to_owned(),
        header: format!(
            "v=1; a=rsa-sha256; d={domain}; s={selector}; c={}; h={}; b={encoded}",
            match canonicalization {
                DkimCanonicalization::Simple => "simple/simple",
                DkimCanonicalization::Relaxed => "relaxed/relaxed",
            },
            headers
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(":")
        ),
    })
}

pub fn verify_dkim(
    headers: &[(String, String)],
    body: &[u8],
    signature: &DkimSignature,
    canonicalization: DkimCanonicalization,
    verifier: &dyn DkimVerifier,
) -> Result<bool, DkimError> {
    let mut input = canonicalize_body(body, canonicalization);
    for (name, value) in headers {
        input.extend_from_slice(&canonicalize_header(name, value, canonicalization));
    }
    let value = signature
        .header
        .split("b=")
        .nth(1)
        .ok_or(DkimError::InvalidInput)?;
    let value = value.split(';').next().unwrap_or(value).trim();
    let decoded = BASE64.decode(value).map_err(|_| DkimError::InvalidInput)?;
    verifier.verify(&input, &decoded)
}

fn canonicalize_body(body: &[u8], mode: DkimCanonicalization) -> Vec<u8> {
    match mode {
        DkimCanonicalization::Simple => body.to_vec(),
        DkimCanonicalization::Relaxed => body
            .split(|byte| *byte == b'\n')
            .map(|line| {
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                let mut output = Vec::new();
                let mut whitespace = false;
                for byte in line {
                    if byte.is_ascii_whitespace() {
                        whitespace = true;
                    } else {
                        if whitespace && !output.is_empty() {
                            output.push(b' ');
                        }
                        whitespace = false;
                        output.push(*byte);
                    }
                }
                output
            })
            .collect::<Vec<_>>()
            .join(&b'\n'),
    }
}

fn canonicalize_header(name: &str, value: &str, mode: DkimCanonicalization) -> Vec<u8> {
    match mode {
        DkimCanonicalization::Simple => format!("{name}:{value}\r\n").into_bytes(),
        DkimCanonicalization::Relaxed => format!(
            "{}:{}\r\n",
            name.trim().to_ascii_lowercase(),
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
        )
        .into_bytes(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmarcDisposition {
    None,
    Quarantine,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmarcPolicy {
    pub domain: String,
    pub disposition: DmarcDisposition,
    pub subdomain_disposition: DmarcDisposition,
    pub percentage: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmarcResult {
    Pass,
    Fail(DmarcDisposition),
}

pub fn evaluate_dmarc(
    policy: &DmarcPolicy,
    header_from_domain: &str,
    spf_domain: Option<&str>,
    dkim_domain: Option<&str>,
) -> DmarcResult {
    let aligned = spf_domain.is_some_and(|domain| domain.eq_ignore_ascii_case(header_from_domain))
        || dkim_domain.is_some_and(|domain| domain.eq_ignore_ascii_case(header_from_domain));
    if aligned {
        DmarcResult::Pass
    } else {
        DmarcResult::Fail(policy.disposition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoSigner;
    impl DkimSigner for EchoSigner {
        fn sign(&self, input: &[u8]) -> Result<Vec<u8>, DkimError> {
            Ok(input.to_vec())
        }
    }
    struct EchoVerifier;
    impl DkimVerifier for EchoVerifier {
        fn verify(&self, input: &[u8], signature: &[u8]) -> Result<bool, DkimError> {
            Ok(input == signature)
        }
    }

    #[test]
    fn evaluates_spf_qualifiers() {
        assert_eq!(
            evaluate_spf("v=spf1 -all", "192.0.2.1", "mail.example"),
            SpfResult::Fail
        );
        assert_eq!(
            evaluate_spf("v=spf1 ip4:192.0.2.1", "192.0.2.1", "mail.example"),
            SpfResult::Pass
        );
    }

    #[test]
    fn signs_and_verifies_canonicalized_headers() {
        let headers = vec![("From".into(), " alice@example.test ".into())];
        let signature = sign_dkim_headers(
            &headers,
            b"body\r\n",
            "example.test",
            "mail",
            DkimCanonicalization::Relaxed,
            &EchoSigner,
        )
        .unwrap();
        assert!(
            verify_dkim(
                &headers,
                b"body\r\n",
                &signature,
                DkimCanonicalization::Relaxed,
                &EchoVerifier
            )
            .unwrap()
        );
    }

    #[test]
    fn requires_dmarc_alignment() {
        let policy = DmarcPolicy {
            domain: "example.test".into(),
            disposition: DmarcDisposition::Reject,
            subdomain_disposition: DmarcDisposition::Reject,
            percentage: 100,
        };
        assert_eq!(
            evaluate_dmarc(&policy, "example.test", Some("other.test"), None),
            DmarcResult::Fail(DmarcDisposition::Reject)
        );
    }
}
