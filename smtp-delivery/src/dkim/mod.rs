use std::path::Path;

use mail_auth::{
    AuthenticatedMessage,
    crypto::Ed25519Key,
    dkim::{Canonicalization, DkimSigner},
    headers::HeaderWriter,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("message has no single valid From identity")]
    InvalidFrom,
    #[error("DKIM signing key configuration is unavailable")]
    Unavailable,
    #[error("DKIM signing key configuration is invalid")]
    InvalidKey,
}

/// Signs an outbound RFC 5322 message with the active Ed25519 key mounted for
/// its From domain. Key material is read from a secret-manager projected file,
/// never from PostgreSQL or ordinary application configuration.
pub async fn sign_message(key_root: &Path, message: &[u8]) -> Result<Vec<u8>, SigningError> {
    let authenticated = AuthenticatedMessage::parse(message).ok_or(SigningError::InvalidFrom)?;
    let mut from_domains = authenticated
        .from_addresses()
        .iter()
        .map(|address| {
            address
                .rsplit_once('@')
                .map(|(_, domain)| domain.to_ascii_lowercase())
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(SigningError::InvalidFrom)?;
    from_domains.sort();
    from_domains.dedup();
    if from_domains.len() != 1 {
        return Err(SigningError::InvalidFrom);
    }
    let domain = from_domains.pop().ok_or(SigningError::InvalidFrom)?;
    if !valid_dns_name(&domain) {
        return Err(SigningError::InvalidFrom);
    }

    let domain_dir = key_root.join(&domain);
    let selector_path = domain_dir.join("active-selector");
    let selector = tokio::fs::read_to_string(selector_path)
        .await
        .map_err(|_| SigningError::Unavailable)?;
    let selector = selector.trim();
    if !valid_selector(selector) {
        return Err(SigningError::InvalidKey);
    }
    let key_path = domain_dir.join(format!("{selector}.pk8"));
    let metadata = tokio::fs::metadata(&key_path)
        .await
        .map_err(|_| SigningError::Unavailable)?;
    if !metadata.is_file() || !private_file_permissions(&metadata) {
        return Err(SigningError::InvalidKey);
    }
    let key_bytes = tokio::fs::read(key_path)
        .await
        .map_err(|_| SigningError::Unavailable)?;
    let key = Ed25519Key::from_pkcs8_der(&key_bytes).map_err(|_| SigningError::InvalidKey)?;
    let signature = DkimSigner::from_key(key)
        .domain(&domain)
        .selector(selector)
        .headers(["From", "To", "Subject", "Date", "Message-ID"])
        .header_canonicalization(Canonicalization::Relaxed)
        .body_canonicalization(Canonicalization::Relaxed)
        .sign(message)
        .map_err(|_| SigningError::InvalidKey)?;
    let header = signature.to_header();
    let mut signed = Vec::with_capacity(header.len() + message.len());
    signed.extend_from_slice(header.as_bytes());
    signed.extend_from_slice(message);
    Ok(signed)
}

fn valid_dns_name(domain: &str) -> bool {
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn valid_selector(selector: &str) -> bool {
    !selector.is_empty()
        && selector.len() <= 63
        && selector
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(unix)]
fn private_file_permissions(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    matches!(metadata.permissions().mode() & 0o777, 0o400 | 0o600)
}

#[cfg(not(unix))]
fn private_file_permissions(metadata: &std::fs::Metadata) -> bool {
    metadata.permissions().readonly()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn signs_with_active_domain_key_without_modifying_original_bytes() {
        let root = std::env::temp_dir().join(format!("smtp-dkim-{}", Uuid::new_v4()));
        let domain_dir = root.join("example.test");
        tokio::fs::create_dir_all(&domain_dir).await.unwrap();
        tokio::fs::write(domain_dir.join("active-selector"), "current\n")
            .await
            .unwrap();
        // RFC 8410 Ed25519 PKCS#8 envelope around a deterministic test seed.
        let mut pkcs8 = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        pkcs8.extend_from_slice(&[0_u8; 32]);
        let key_path = domain_dir.join("current.pk8");
        tokio::fs::write(&key_path, pkcs8).await.unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let raw = b"From: sender@example.test\r\nTo: receiver@example.org\r\nSubject: test\r\n\r\nbody\r\n";
        let signed = sign_message(&root, raw).await.unwrap();
        let header_end = signed
            .windows(2)
            .position(|window| window == b"\r\n")
            .unwrap();
        let header = String::from_utf8_lossy(&signed[..header_end]);
        assert!(header.starts_with("DKIM-Signature:"), "{header}");
        assert!(header.contains("a=ed25519-sha256"), "{header}");
        assert!(header.contains("d=example.test"), "{header}");
        assert!(header.contains("s=current"), "{header}");
        assert!(signed.ends_with(raw));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn refuses_keys_with_group_or_world_permissions() {
        let root = std::env::temp_dir().join(format!("smtp-dkim-{}", Uuid::new_v4()));
        let domain_dir = root.join("example.test");
        tokio::fs::create_dir_all(&domain_dir).await.unwrap();
        tokio::fs::write(domain_dir.join("active-selector"), "current")
            .await
            .unwrap();
        let key_path = domain_dir.join("current.pk8");
        tokio::fs::write(&key_path, [0_u8; 48]).await.unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();
            let error = sign_message(&root, b"From: sender@example.test\r\n\r\nbody\r\n")
                .await
                .unwrap_err();
            assert!(matches!(error, SigningError::InvalidKey));
        }
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
