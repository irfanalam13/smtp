use std::{
    fs,
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use mail_auth::{
    MessageAuthenticator, crypto::Ed25519Key, hickory_resolver::system_conf::read_system_conf,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or(usage())?;
    let domain = args.next().ok_or(usage())?.to_ascii_lowercase();
    let selector = args.next().ok_or(usage())?;
    let key_root = PathBuf::from(args.next().ok_or(usage())?);
    let confirmation = args.next();
    if args.next().is_some() || !valid_domain(&domain) || !valid_selector(&selector) {
        return Err(usage().into());
    }
    let domain_dir = key_root.join(&domain);
    ensure_real_directory(&domain_dir)?;
    let key_path = domain_dir.join(format!("{selector}.pk8"));
    ensure_private_key_file(&key_path)?;

    match command.as_str() {
        "activate" if confirmation.is_none() => {
            activate(&domain, &selector, &domain_dir, &key_path).await?
        }
        "retire" if confirmation.as_deref() == Some("--dns-removed-after-ttl") => {
            retire(&selector, &domain_dir, &key_path)?
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}

async fn activate(
    domain: &str,
    selector: &str,
    domain_dir: &Path,
    key_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let private = fs::read(key_path)?;
    let key = Ed25519Key::from_pkcs8_der(&private)?;
    let expected_public = STANDARD.encode(key.public_key());
    let (resolver_config, mut options) = read_system_conf()?;
    options.timeout = Duration::from_secs(3);
    options.attempts = 2;
    options.cache_size = 128;
    options.validate = true;
    let resolver = MessageAuthenticator::new(resolver_config, options)?;
    let owner = format!("{selector}._domainkey.{domain}");
    let lookup = resolver.resolver().txt_lookup(owner.clone()).await?;
    let records = lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            mail_auth::hickory_resolver::proto::rr::RData::TXT(record) => {
                let mut value = Vec::new();
                for chunk in &record.txt_data {
                    value.extend_from_slice(chunk);
                }
                String::from_utf8(value).ok()
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let matching = records
        .iter()
        .filter(|record| {
            let tags = record
                .split(';')
                .filter_map(|tag| tag.trim().split_once('='))
                .map(|(name, value)| (name.trim(), value.trim()))
                .collect::<std::collections::HashMap<_, _>>();
            tags.get("v") == Some(&"DKIM1")
                && tags
                    .get("k")
                    .is_some_and(|algorithm| *algorithm == "ed25519")
                && tags
                    .get("p")
                    .is_some_and(|public| *public == expected_public)
        })
        .count();
    if records.len() != 1 || matching != 1 {
        return Err(
            format!("DNS TXT at {owner} does not uniquely publish this Ed25519 key").into(),
        );
    }
    write_active_selector(domain_dir, selector)?;
    println!("activated {selector}._domainkey.{domain} after DNSSEC-aware key match");
    Ok(())
}

fn retire(
    selector: &str,
    domain_dir: &Path,
    key_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let active_path = domain_dir.join("active-selector");
    if fs::symlink_metadata(&active_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("active-selector must not be a symlink".into());
    }
    let active = fs::read_to_string(&active_path).unwrap_or_default();
    if active.trim() == selector {
        return Err("activate another selector before retiring the active key".into());
    }
    fs::remove_file(key_path)?;
    fs::File::open(domain_dir)?.sync_all()?;
    println!(
        "retired local private key for {selector}; DNS revocation and TTL wait were confirmed by operator"
    );
    Ok(())
}

fn write_active_selector(domain_dir: &Path, selector: &str) -> io::Result<()> {
    let active_path = domain_dir.join("active-selector");
    if fs::symlink_metadata(&active_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "active-selector must not be a symlink",
        ));
    }
    let temporary_path = domain_dir.join(format!(".active-selector-{}", Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary_path)?;
    file.write_all(selector.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&temporary_path, &active_path)?;
    fs::File::open(domain_dir)?.sync_all()?;
    Ok(())
}

fn ensure_real_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DKIM domain key path must be a real directory",
        ));
    }
    Ok(())
}

fn ensure_private_key_file(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DKIM key must be a regular non-symlink file",
        ));
    }
    let permissions = metadata.permissions().mode() & 0o777;
    if permissions != 0o400 && permissions != 0o600 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "DKIM key permissions must be 0400 or 0600",
        ));
    }
    Ok(())
}

fn usage() -> &'static str {
    "usage: dkim-keyctl activate <domain> <selector> <key-root> | dkim-keyctl retire <domain> <selector> <key-root> --dns-removed-after-ttl"
}

fn valid_domain(domain: &str) -> bool {
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
