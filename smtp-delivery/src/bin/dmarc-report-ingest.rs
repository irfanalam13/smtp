use std::{env, error::Error, fs::File, io::Read};

use shared_types::{MimeLimits, parse_message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .ok_or("usage: dmarc-report-ingest <report.xml[.gz]>")?;
    let mut bytes = Vec::new();
    File::open(path)?
        .take(10 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    let reports = if is_xml_or_gzip(&bytes) {
        vec![bytes]
    } else {
        let parsed = parse_message(&bytes, MimeLimits::default())?;
        parsed
            .attachments
            .into_iter()
            .filter(|attachment| {
                attachment
                    .content_type
                    .eq_ignore_ascii_case("application/gzip")
                    || attachment.content_type.eq_ignore_ascii_case("text/xml")
                    || attachment.filename.as_deref().is_some_and(|filename| {
                        filename.ends_with(".xml") || filename.ends_with(".xml.gz")
                    })
            })
            .map(|attachment| attachment.content)
            .collect::<Vec<_>>()
    };
    if reports.is_empty() {
        return Err("no XML or gzip report attachment found".into());
    }
    let pool = mail_delivery::dmarc_report_ingest::connect_from_env().await?;
    for report in reports {
        let outcome = mail_delivery::dmarc_report_ingest::ingest_report(&pool, &report).await?;
        println!("{outcome:?}");
    }
    Ok(())
}

fn is_xml_or_gzip(input: &[u8]) -> bool {
    input.starts_with(&[0x1f, 0x8b])
        || input
            .iter()
            .copied()
            .skip_while(|byte| byte.is_ascii_whitespace())
            .take(64)
            .collect::<Vec<_>>()
            .starts_with(b"<?xml")
        || input
            .iter()
            .copied()
            .skip_while(|byte| byte.is_ascii_whitespace())
            .take(64)
            .collect::<Vec<_>>()
            .starts_with(b"<feedback")
}
