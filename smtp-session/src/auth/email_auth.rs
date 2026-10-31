use std::{io, net::IpAddr, time::Duration};
use tokio::time::timeout;

use mail_auth::{
    AuthenticatedMessage, DkimOutput, DkimResult, DmarcResult, MessageAuthenticator, SpfResult,
    crypto::Algorithm,
    dmarc::{Policy, verify::DmarcParameters},
    hickory_resolver::system_conf::read_system_conf,
    spf::verify::SpfParameters,
};
use serde::Serialize;
use thiserror::Error;

const DNS_TIMEOUT: Duration = Duration::from_secs(3);
const MESSAGE_AUTH_BUDGET: Duration = Duration::from_secs(15);
const DNS_NEGATIVE_TTL: Duration = Duration::from_secs(3600);
const MAX_DKIM_SIGNATURES: usize = 4;
const MAX_REPORT_DESTINATIONS: usize = 8;

#[derive(Clone)]
pub struct EmailAuthenticator {
    resolver: MessageAuthenticator,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthenticationSummary {
    pub spf: &'static str,
    pub spf_domain: String,
    pub spf_scope: &'static str,
    pub dkim: Vec<DkimAuthenticationResult>,
    pub dmarc: &'static str,
    pub dmarc_spf: &'static str,
    pub dmarc_dkim: &'static str,
    pub policy: &'static str,
    pub published_policy: &'static str,
    pub subdomain_policy: &'static str,
    pub spf_alignment: &'static str,
    pub dkim_alignment: &'static str,
    pub author_domain: String,
    pub rua: Vec<ReportDestination>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportDestination {
    pub address: String,
    pub max_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DkimAuthenticationResult {
    pub result: &'static str,
    pub domain: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticationDisposition {
    Accept,
    Quarantine,
    Reject,
}

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("authentication message is malformed")]
    InvalidMessage,
    #[error("authentication DNS lookup failed temporarily")]
    Temporary,
}

impl EmailAuthenticator {
    pub fn from_system_configuration() -> io::Result<Self> {
        let (config, mut options) =
            read_system_conf().map_err(|error| io::Error::other(error.to_string()))?;
        options.timeout = DNS_TIMEOUT;
        options.attempts = 2;
        options.cache_size = 2048;
        options.num_concurrent_reqs = 1;
        // Validate DNSSEC chains for all authentication lookups. Insecure
        // delegations remain usable, while bogus answers fail validation.
        options.validate = true;
        options.positive_max_ttl = Some(Duration::from_secs(86_400));
        options.negative_max_ttl = Some(DNS_NEGATIVE_TTL);
        let resolver = MessageAuthenticator::new(config, options)
            .map_err(|error| io::Error::other(error.to_string()))?
            .with_max_negative_ttl(DNS_NEGATIVE_TTL);
        Ok(Self { resolver })
    }

    pub async fn evaluate(
        &self,
        raw_message: &[u8],
        peer_ip: IpAddr,
        helo_domain: &str,
        receiver_domain: &str,
        envelope_sender: &str,
    ) -> Result<(AuthenticationSummary, AuthenticationDisposition), AuthenticationError> {
        timeout(
            MESSAGE_AUTH_BUDGET,
            self.evaluate_inner(
                raw_message,
                peer_ip,
                helo_domain,
                receiver_domain,
                envelope_sender,
            ),
        )
        .await
        .map_err(|_| AuthenticationError::Temporary)?
    }

    async fn evaluate_inner(
        &self,
        raw_message: &[u8],
        peer_ip: IpAddr,
        helo_domain: &str,
        receiver_domain: &str,
        envelope_sender: &str,
    ) -> Result<(AuthenticationSummary, AuthenticationDisposition), AuthenticationError> {
        if count_dkim_headers(raw_message) > MAX_DKIM_SIGNATURES {
            return Err(AuthenticationError::InvalidMessage);
        }
        let message =
            AuthenticatedMessage::parse(raw_message).ok_or(AuthenticationError::InvalidMessage)?;
        let dkim = self.resolver.verify_dkim(&message).await;
        let spf = self
            .resolver
            .verify_spf(SpfParameters::mail_from(
                peer_ip,
                helo_domain,
                receiver_domain,
                envelope_sender,
            ))
            .await;
        // RFC 8301 deprecates rsa-sha1 for verification. The compatibility
        // library can report it, so it must not contribute to DMARC alignment.
        let dkim_for_dmarc = dkim
            .iter()
            .filter(|output| {
                !output
                    .signature()
                    .is_some_and(|signature| signature.a == Algorithm::RsaSha1)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mail_from_domain = if envelope_sender.is_empty() {
            helo_domain
        } else {
            envelope_sender
                .rsplit_once('@')
                .map_or(helo_domain, |(_, domain)| domain)
        };
        let dmarc = self
            .resolver
            .verify_dmarc(DmarcParameters::new(
                &message,
                &dkim_for_dmarc,
                mail_from_domain,
                &spf,
            ))
            .await;

        if matches!(dmarc.result(), DmarcResult::TempError(_)) {
            return Err(AuthenticationError::Temporary);
        }

        let result = dmarc.result();
        let disposition = match (&result, dmarc.policy()) {
            (DmarcResult::Fail(_), Policy::Reject) => AuthenticationDisposition::Reject,
            (DmarcResult::Fail(_), Policy::Quarantine) => AuthenticationDisposition::Quarantine,
            _ => AuthenticationDisposition::Accept,
        };
        let record = dmarc.record();
        let rua = if let Some(record) = record {
            let report_addresses = &record.rua()[..record.rua().len().min(MAX_REPORT_DESTINATIONS)];
            self.resolver
                .authorized_report_addresses((dmarc.domain(), report_addresses))
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|address| ReportDestination {
                    address: address.uri().to_owned(),
                    max_size: address.max_size(),
                })
                .collect()
        } else {
            Vec::new()
        };
        let summary = AuthenticationSummary {
            spf: spf_name(spf.result()),
            spf_domain: spf.domain().to_owned(),
            spf_scope: if envelope_sender.is_empty() {
                "helo"
            } else {
                "mfrom"
            },
            dkim: dkim
                .iter()
                .map(|output| DkimAuthenticationResult {
                    result: dkim_name(output),
                    domain: output.signature().map(|signature| signature.d.clone()),
                })
                .collect(),
            dmarc: dmarc_name(&result),
            dmarc_spf: dmarc_name(dmarc.spf_result()),
            dmarc_dkim: dmarc_name(dmarc.dkim_result()),
            policy: policy_name(dmarc.policy()),
            published_policy: record.map_or("none", |record| policy_name(record.p)),
            subdomain_policy: record.map_or("none", |record| policy_name(record.sp)),
            spf_alignment: record.map_or("r", |record| alignment_name(record.aspf)),
            dkim_alignment: record.map_or("r", |record| alignment_name(record.adkim)),
            author_domain: dmarc.domain().to_owned(),
            rua,
        };
        Ok((summary, disposition))
    }
}

fn count_dkim_headers(raw: &[u8]) -> usize {
    let header_end = raw
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap_or(raw.len());
    raw[..header_end]
        .split(|byte| *byte == b'\n')
        .filter(|line| {
            line.iter()
                .position(|byte| *byte == b':')
                .is_some_and(|colon| {
                    line[..colon]
                        .trim_ascii()
                        .eq_ignore_ascii_case(b"DKIM-Signature")
                })
        })
        .count()
}

fn spf_name(result: SpfResult) -> &'static str {
    match result {
        SpfResult::Pass => "pass",
        SpfResult::Fail => "fail",
        SpfResult::SoftFail => "softfail",
        SpfResult::Neutral => "neutral",
        SpfResult::TempError => "temperror",
        SpfResult::PermError => "permerror",
        SpfResult::None => "none",
    }
}

fn dkim_name(output: &DkimOutput<'_>) -> &'static str {
    if output
        .signature()
        .is_some_and(|signature| signature.a == Algorithm::RsaSha1)
    {
        return "permerror";
    }
    let result = output.result();
    match result {
        DkimResult::Pass => "pass",
        DkimResult::Fail(_) => "fail",
        DkimResult::Neutral(_) => "neutral",
        DkimResult::TempError(_) => "temperror",
        DkimResult::PermError(_) => "permerror",
        DkimResult::None => "none",
    }
}

fn dmarc_name(result: &DmarcResult) -> &'static str {
    match result {
        DmarcResult::Pass => "pass",
        DmarcResult::Fail(_) => "fail",
        DmarcResult::TempError(_) => "temperror",
        DmarcResult::PermError(_) => "permerror",
        DmarcResult::None => "none",
    }
}

fn policy_name(policy: Policy) -> &'static str {
    match policy {
        Policy::None | Policy::Unspecified => "none",
        Policy::Quarantine => "quarantine",
        Policy::Reject => "reject",
    }
}

fn alignment_name(alignment: mail_auth::dmarc::Alignment) -> &'static str {
    match alignment {
        mail_auth::dmarc::Alignment::Relaxed => "r",
        mail_auth::dmarc::Alignment::Strict => "s",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_dkim_dns_work_from_header_count() {
        let four = b"From: a@example.test\r\nDKIM-Signature: a\r\nDKIM-Signature: b\r\nDKIM-Signature: c\r\nDKIM-Signature: d\r\n\r\nbody";
        let five = b"From: a@example.test\r\nDKIM-Signature: a\r\nDKIM-Signature: b\r\nDKIM-Signature: c\r\nDKIM-Signature: d\r\nDKIM-Signature: e\r\n\r\nbody";
        assert_eq!(count_dkim_headers(four), 4);
        assert_eq!(count_dkim_headers(five), 5);
    }

    #[test]
    fn header_count_is_case_insensitive_and_stops_at_body() {
        let raw = b"From: a@example.test\r\ndKiM-sIgNaTuRe: x\r\n\r\nDKIM-Signature: body text";
        assert_eq!(count_dkim_headers(raw), 1);
    }

    #[test]
    fn summaries_use_protocol_result_tokens_without_dns_error_details() {
        assert_eq!(spf_name(SpfResult::TempError), "temperror");
        assert_eq!(
            dkim_name(&DkimResult::PermError(mail_auth::Error::Parse).into()),
            "permerror"
        );
        assert_eq!(
            dmarc_name(&DmarcResult::Fail(mail_auth::Error::NotAligned)),
            "fail"
        );
    }
}
