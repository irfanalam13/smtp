use std::io::{Read, Write};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REPORT_NAMESPACE: &str = "urn:ietf:params:xml:ns:auth:aggregate-report";
const MAX_REPORT_BYTES: usize = 10 * 1024 * 1024;
const MAX_REPORT_RECORDS: usize = 50_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "feedback")]
pub struct AggregateReport {
    #[serde(rename = "@xmlns")]
    namespace: String,
    pub version: String,
    pub report_metadata: ReportMetadata,
    pub policy_published: PublishedPolicy,
    #[serde(rename = "record")]
    pub records: Vec<AggregateRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReportMetadata {
    pub org_name: String,
    pub email: String,
    pub report_id: String,
    pub date_range: ReportDateRange,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReportDateRange {
    pub begin: i64,
    pub end: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublishedPolicy {
    pub domain: String,
    pub adkim: String,
    pub aspf: String,
    pub p: String,
    pub sp: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AggregateRecord {
    pub row: AggregateRow,
    pub identifiers: AggregateIdentifiers,
    pub auth_results: AggregateAuthResults,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AggregateRow {
    pub source_ip: String,
    pub count: u64,
    pub policy_evaluated: PolicyEvaluated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PolicyEvaluated {
    pub disposition: String,
    pub dkim: String,
    pub spf: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AggregateIdentifiers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope_from: Option<String>,
    pub header_from: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AggregateAuthResults {
    #[serde(default, rename = "spf")]
    pub spf: Vec<SpfAuthResult>,
    #[serde(default, rename = "dkim")]
    pub dkim: Vec<DkimAuthResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpfAuthResult {
    pub domain: String,
    pub scope: String,
    pub result: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DkimAuthResult {
    pub domain: String,
    pub result: String,
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("DMARC report is malformed")]
    Malformed,
    #[error("DMARC report exceeds its size or row limits")]
    TooLarge,
    #[error("DMARC report compression failed")]
    Compression,
}

impl AggregateReport {
    pub fn new(
        metadata: ReportMetadata,
        published_policy: PublishedPolicy,
        records: Vec<AggregateRecord>,
    ) -> Result<Self, ReportError> {
        let report = Self {
            namespace: REPORT_NAMESPACE.to_owned(),
            version: "1.0".to_owned(),
            report_metadata: metadata,
            policy_published: published_policy,
            records,
        };
        report.validate()?;
        Ok(report)
    }

    pub fn to_xml(&self) -> Result<Vec<u8>, ReportError> {
        self.validate()?;
        let xml = quick_xml::se::to_string(self).map_err(|_| ReportError::Malformed)?;
        let mut output = Vec::with_capacity(xml.len() + 39);
        output.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.extend_from_slice(xml.as_bytes());
        if output.len() > MAX_REPORT_BYTES {
            return Err(ReportError::TooLarge);
        }
        Ok(output)
    }

    pub fn to_gzip(&self) -> Result<Vec<u8>, ReportError> {
        let xml = self.to_xml()?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&xml)
            .map_err(|_| ReportError::Compression)?;
        encoder.finish().map_err(|_| ReportError::Compression)
    }

    fn validate(&self) -> Result<(), ReportError> {
        if self.namespace != REPORT_NAMESPACE
            || self.version != "1.0"
            || self.records.is_empty()
            || self.records.len() > MAX_REPORT_RECORDS
            || self.report_metadata.org_name.is_empty()
            || self.report_metadata.org_name.len() > 255
            || self.report_metadata.email.is_empty()
            || self.report_metadata.email.len() > 320
            || self
                .report_metadata
                .email
                .rsplit_once('@')
                .is_none_or(|(local, domain)| {
                    local.is_empty()
                        || local.contains('@')
                        || !valid_domain(domain)
                        || self
                            .report_metadata
                            .email
                            .bytes()
                            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
                })
            || self.report_metadata.report_id.is_empty()
            || self.report_metadata.report_id.len() > 255
            || self.report_metadata.date_range.begin < 0
            || self.report_metadata.date_range.end <= self.report_metadata.date_range.begin
            || self.report_metadata.date_range.end - self.report_metadata.date_range.begin
                > 31 * 86_400
            || !valid_domain(&self.policy_published.domain)
            || !matches!(self.policy_published.adkim.as_str(), "r" | "s")
            || !matches!(self.policy_published.aspf.as_str(), "r" | "s")
            || !valid_policy(&self.policy_published.p)
            || !valid_policy(&self.policy_published.sp)
            || self.records.iter().any(|record| {
                record.row.count == 0
                    || record.row.source_ip.parse::<std::net::IpAddr>().is_err()
                    || !valid_disposition(&record.row.policy_evaluated.disposition)
                    || !valid_auth_result(&record.row.policy_evaluated.dkim)
                    || !valid_auth_result(&record.row.policy_evaluated.spf)
                    || record
                        .identifiers
                        .envelope_from
                        .as_ref()
                        .is_some_and(|domain| !valid_domain(domain))
                    || !valid_domain(&record.identifiers.header_from)
                    || record.auth_results.dkim.len() > 20
                    || record.auth_results.spf.len() > 20
                    || record.auth_results.spf.iter().any(|auth| {
                        !valid_domain(&auth.domain)
                            || auth.scope != "mfrom" && auth.scope != "helo"
                            || !valid_auth_result(&auth.result)
                    })
                    || record
                        .auth_results
                        .dkim
                        .iter()
                        .any(|auth| !valid_domain(&auth.domain) || !valid_auth_result(&auth.result))
            })
        {
            return Err(ReportError::Malformed);
        }
        Ok(())
    }
}

/// Parses either RFC 9990 XML or a gzip-compressed XML report. Input and
/// decompressed content are independently bounded before deserialization.
pub fn parse_report(input: &[u8]) -> Result<AggregateReport, ReportError> {
    if input.len() > MAX_REPORT_BYTES {
        return Err(ReportError::TooLarge);
    }
    let xml = if input.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(input);
        let mut xml = Vec::new();
        decoder
            .by_ref()
            .take((MAX_REPORT_BYTES + 1) as u64)
            .read_to_end(&mut xml)
            .map_err(|_| ReportError::Malformed)?;
        if xml.len() > MAX_REPORT_BYTES {
            return Err(ReportError::TooLarge);
        }
        xml
    } else {
        input.to_vec()
    };
    let mut reader = quick_xml::Reader::from_reader(xml.as_slice());
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = true;
    loop {
        match reader.read_event().map_err(|_| ReportError::Malformed)? {
            quick_xml::events::Event::DocType(_) => return Err(ReportError::Malformed),
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    let report: AggregateReport =
        quick_xml::de::from_reader(xml.as_slice()).map_err(|_| ReportError::Malformed)?;
    report.validate()?;
    Ok(report)
}

fn valid_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
        })
}

fn valid_policy(value: &str) -> bool {
    matches!(value, "none" | "quarantine" | "reject")
}

fn valid_disposition(value: &str) -> bool {
    matches!(value, "none" | "quarantine" | "reject")
}

fn valid_auth_result(value: &str) -> bool {
    matches!(
        value,
        "pass" | "fail" | "softfail" | "neutral" | "temperror" | "permerror" | "none"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> AggregateReport {
        AggregateReport::new(
            ReportMetadata {
                org_name: "Receiver & Mail".to_owned(),
                email: "dmarc@example.net".to_owned(),
                report_id: "report-1@example.net".to_owned(),
                date_range: ReportDateRange {
                    begin: 1_800_000_000,
                    end: 1_800_086_400,
                },
            },
            PublishedPolicy {
                domain: "example.org".to_owned(),
                adkim: "r".to_owned(),
                aspf: "r".to_owned(),
                p: "quarantine".to_owned(),
                sp: "reject".to_owned(),
            },
            vec![AggregateRecord {
                row: AggregateRow {
                    source_ip: "192.0.2.25".to_owned(),
                    count: 3,
                    policy_evaluated: PolicyEvaluated {
                        disposition: "none".to_owned(),
                        dkim: "pass".to_owned(),
                        spf: "pass".to_owned(),
                    },
                },
                identifiers: AggregateIdentifiers {
                    envelope_from: Some("example.org".to_owned()),
                    header_from: "example.org".to_owned(),
                },
                auth_results: AggregateAuthResults {
                    spf: vec![SpfAuthResult {
                        domain: "example.org".to_owned(),
                        scope: "mfrom".to_owned(),
                        result: "pass".to_owned(),
                    }],
                    dkim: vec![DkimAuthResult {
                        domain: "example.org".to_owned(),
                        result: "pass".to_owned(),
                    }],
                },
            }],
        )
        .unwrap()
    }

    #[test]
    fn generated_xml_and_gzip_round_trip() {
        let report = fixture();
        let xml = report.to_xml().unwrap();
        let parsed_xml = parse_report(&xml).unwrap();
        assert_eq!(parsed_xml.report_metadata.report_id, "report-1@example.net");
        assert_eq!(parsed_xml.records[0].row.count, 3);
        assert!(String::from_utf8_lossy(&xml).contains("Receiver &amp; Mail"));

        let compressed = report.to_gzip().unwrap();
        let parsed_gzip = parse_report(&compressed).unwrap();
        assert_eq!(parsed_gzip.policy_published.domain, "example.org");
    }

    #[test]
    fn rejects_dtd_and_invalid_policy_domains() {
        let dtd = b"<!DOCTYPE feedback [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><feedback/>";
        assert!(matches!(parse_report(dtd), Err(ReportError::Malformed)));

        let mut report = fixture();
        report.policy_published.domain = "example.org\r\nBcc: victim@example.net".to_owned();
        assert!(matches!(report.to_xml(), Err(ReportError::Malformed)));
    }
}
