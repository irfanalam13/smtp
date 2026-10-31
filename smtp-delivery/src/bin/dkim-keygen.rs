use std::{fs::OpenOptions, io::Write, path::PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use mail_auth::dkim::generate::DkimKeyPair;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let domain = args
        .next()
        .ok_or("usage: dkim-keygen <domain> <selector> <key-root>")?;
    let selector = args
        .next()
        .ok_or("usage: dkim-keygen <domain> <selector> <key-root>")?;
    let key_root = PathBuf::from(
        args.next()
            .ok_or("usage: dkim-keygen <domain> <selector> <key-root>")?,
    );
    if args.next().is_some() || !valid_domain(&domain) || !valid_selector(&selector) {
        return Err("invalid domain or selector".into());
    }
    let domain = domain.to_ascii_lowercase();
    let domain_dir = key_root.join(&domain);
    std::fs::create_dir_all(&domain_dir)?;
    let key_path = domain_dir.join(format!("{selector}.pk8"));
    let pair = DkimKeyPair::generate_ed25519()?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut key_file = options.open(key_path)?;
    key_file.write_all(pair.private_key())?;
    key_file.sync_all()?;
    println!(
        "{}.{} TXT \"v=DKIM1; k=ed25519; p={}\"",
        selector,
        domain,
        STANDARD.encode(pair.public_key())
    );
    eprintln!(
        "private key written with restrictive permissions; publish the TXT record before activating the selector"
    );
    Ok(())
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
