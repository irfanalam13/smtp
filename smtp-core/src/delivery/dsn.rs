use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BounceClass {
    None,
    Temporary,
    Permanent,
    Policy,
    InvalidRecipient,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsnAction {
    Delivered,
    Delayed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DsnStatus {
    pub action: DsnAction,
    pub status: String,
    pub diagnostic_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DsnReport {
    pub original_message_id: Uuid,
    pub recipient: String,
    pub reporting_mta: String,
    pub status: DsnStatus,
    pub generated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedbackType {
    Abuse,
    Fraud,
    Virus,
    NotSpam,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackReport {
    pub feedback_type: FeedbackType,
    pub original_message_id: Option<String>,
    pub source_ip: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DeliveryTracker {
    reports: HashMap<Uuid, Vec<DsnReport>>,
}

impl DeliveryTracker {
    pub fn record(&mut self, message_id: Uuid, report: DsnReport) {
        self.reports.entry(message_id).or_default().push(report);
    }

    pub fn reports(&self, message_id: Uuid) -> &[DsnReport] {
        self.reports
            .get(&message_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DsnError {
    #[error("recipient is empty")]
    EmptyRecipient,
    #[error("status must be a three-part SMTP enhanced status code")]
    InvalidStatus,
    #[error("reporting MTA is empty")]
    EmptyReportingMta,
    #[error("feedback report is missing its Feedback-Type field")]
    MissingFeedbackType,
    #[error("recipient or reporting MTA contains control characters")]
    InvalidField,
    #[error("DSN is empty or exceeds the parser limit")]
    InvalidMessageSize,
    #[error("DSN is missing a required delivery-status field")]
    MissingDeliveryStatusField,
}

const MAX_DSN_BYTES: usize = 256 * 1024;

pub fn parse_delivery_status(
    original_message_id: Uuid,
    raw: &[u8],
) -> Result<Vec<DsnReport>, DsnError> {
    if raw.is_empty() || raw.len() > MAX_DSN_BYTES {
        return Err(DsnError::InvalidMessageSize);
    }
    let text = std::str::from_utf8(raw).map_err(|_| DsnError::InvalidField)?;
    let mut reports = Vec::new();
    let mut fields = Vec::new();
    let mut current_name = String::new();
    let mut current_value = String::new();
    let mut in_status_part = false;
    for line in text.lines() {
        if line.is_empty() {
            if in_status_part {
                push_dsn_field(&mut fields, &mut current_name, &mut current_value);
                if !fields.is_empty() {
                    reports.push(build_dsn_report(original_message_id, &fields)?);
                    fields.clear();
                }
            }
            in_status_part = true;
            continue;
        }
        if !in_status_part {
            continue;
        }
        if line.chars().next().is_some_and(char::is_whitespace) {
            if !current_name.is_empty() {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }
        push_dsn_field(&mut fields, &mut current_name, &mut current_value);
        let Some((name, value)) = line.split_once(':') else {
            return Err(DsnError::InvalidField);
        };
        current_name = name.trim().to_owned();
        current_value = value.trim().to_owned();
    }
    push_dsn_field(&mut fields, &mut current_name, &mut current_value);
    if in_status_part && !fields.is_empty() {
        reports.push(build_dsn_report(original_message_id, &fields)?);
    }
    if reports.is_empty() {
        return Err(DsnError::MissingDeliveryStatusField);
    }
    Ok(reports)
}

fn push_dsn_field(fields: &mut Vec<(String, String)>, name: &mut String, value: &mut String) {
    if !name.is_empty() {
        fields.push((std::mem::take(name), std::mem::take(value)));
    }
}

fn build_dsn_report(
    original_message_id: Uuid,
    fields: &[(String, String)],
) -> Result<DsnReport, DsnError> {
    let value = |name: &str| {
        fields
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let recipient = value("Final-Recipient")
        .and_then(|value| {
            value
                .split_once(';')
                .map(|(_, address)| address.trim().to_owned())
        })
        .ok_or(DsnError::MissingDeliveryStatusField)?;
    let reporting_mta = value("Reporting-MTA")
        .and_then(|value| {
            value
                .split_once(';')
                .map(|(_, host)| host.trim().to_owned())
        })
        .ok_or(DsnError::MissingDeliveryStatusField)?;
    let action = match value("Action").as_deref() {
        Some("delivered") => DsnAction::Delivered,
        Some("delayed") => DsnAction::Delayed,
        Some("failed") => DsnAction::Failed,
        _ => return Err(DsnError::MissingDeliveryStatusField),
    };
    let status = value("Status").ok_or(DsnError::MissingDeliveryStatusField)?;
    let diagnostic_code = value("Diagnostic-Code");
    generate_dsn(
        original_message_id,
        &recipient,
        &reporting_mta,
        DsnStatus {
            action,
            status,
            diagnostic_code,
        },
    )
}

pub fn parse_feedback_report(headers: &[(String, String)]) -> Result<FeedbackReport, DsnError> {
    let feedback_type = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Feedback-Type"))
        .map(
            |(_, value)| match value.trim().to_ascii_lowercase().as_str() {
                "abuse" => FeedbackType::Abuse,
                "fraud" => FeedbackType::Fraud,
                "virus" => FeedbackType::Virus,
                "not-spam" => FeedbackType::NotSpam,
                _ => FeedbackType::Other,
            },
        )
        .ok_or(DsnError::MissingFeedbackType)?;
    let header = |wanted: &str| {
        headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
            .map(|(_, value)| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    Ok(FeedbackReport {
        feedback_type,
        original_message_id: header("Original-Mail-From"),
        source_ip: header("Source-IP"),
    })
}

pub fn classify_bounce(status: &str, diagnostic_code: Option<&str>) -> BounceClass {
    if !valid_enhanced_status(status) {
        return BounceClass::None;
    }
    let first = status.split('.').next().unwrap_or_default();
    if status.starts_with("5.1.1") || diagnostic_code.is_some_and(|code| code.contains("550 5.1.1"))
    {
        BounceClass::InvalidRecipient
    } else if status.starts_with("5.7.") {
        BounceClass::Policy
    } else if first == "5" {
        BounceClass::Permanent
    } else if first == "4" {
        BounceClass::Temporary
    } else {
        BounceClass::None
    }
}

pub fn generate_dsn(
    original_message_id: Uuid,
    recipient: &str,
    reporting_mta: &str,
    status: DsnStatus,
) -> Result<DsnReport, DsnError> {
    if recipient.is_empty() {
        return Err(DsnError::EmptyRecipient);
    }
    if reporting_mta.is_empty() {
        return Err(DsnError::EmptyReportingMta);
    }
    if recipient.chars().any(char::is_control) || reporting_mta.chars().any(char::is_control) {
        return Err(DsnError::InvalidField);
    }
    if !valid_enhanced_status(&status.status) {
        return Err(DsnError::InvalidStatus);
    }
    Ok(DsnReport {
        original_message_id,
        recipient: recipient.to_owned(),
        reporting_mta: reporting_mta.to_owned(),
        status,
        generated_at: Utc::now(),
    })
}

fn valid_enhanced_status(status: &str) -> bool {
    let parts = status.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && matches!(parts[0], "2" | "4" | "5")
        && parts[1..].iter().all(|part| {
            (1..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_recipient_bounces_and_tracks_reports() {
        assert_eq!(
            classify_bounce("5.1.1", None),
            BounceClass::InvalidRecipient
        );
        let id = Uuid::new_v4();
        let report = generate_dsn(
            id,
            "user@example.test",
            "mx.example.test",
            DsnStatus {
                action: DsnAction::Failed,
                status: "5.1.1".into(),
                diagnostic_code: None,
            },
        )
        .unwrap();
        let mut tracker = DeliveryTracker::default();
        tracker.record(id, report);
        assert_eq!(tracker.reports(id).len(), 1);
    }

    #[test]
    fn parses_feedback_report_type() {
        let report = parse_feedback_report(&[
            ("Feedback-Type".into(), "abuse".into()),
            ("Source-IP".into(), "192.0.2.10".into()),
        ])
        .unwrap();
        assert_eq!(report.feedback_type, FeedbackType::Abuse);
        assert_eq!(report.source_ip.as_deref(), Some("192.0.2.10"));
    }

    #[test]
    fn rejects_malformed_enhanced_status_codes() {
        assert_eq!(classify_bounce("5.1.1\r\n", None), BounceClass::None);
        assert_eq!(classify_bounce("6.1.1", None), BounceClass::None);
        assert!(matches!(
            generate_dsn(
                Uuid::new_v4(),
                "user@example.test",
                "mx.example.test",
                DsnStatus {
                    action: DsnAction::Failed,
                    status: "5.1".into(),
                    diagnostic_code: None,
                }
            ),
            Err(DsnError::InvalidStatus)
        ));
    }

    #[test]
    fn parses_bounded_delivery_status_parts() {
        let reports = parse_delivery_status(
            Uuid::nil(),
            b"Content-Type: multipart/report\r\n\
              \r\n\
              Reporting-MTA: dns; mx.example.test\r\n\
              Final-Recipient: rfc822; user@example.test\r\n\
              Action: failed\r\n\
              Status: 5.1.1\r\n\
              Diagnostic-Code: smtp; 550 5.1.1 no such user\r\n",
        )
        .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].recipient, "user@example.test");
        assert_eq!(reports[0].status.status, "5.1.1");
    }

    #[test]
    fn rejects_unbounded_or_incomplete_delivery_status() {
        assert!(matches!(
            parse_delivery_status(Uuid::nil(), b"Reporting-MTA: dns; mx.example.test\r\n"),
            Err(DsnError::MissingDeliveryStatusField)
        ));
        assert!(matches!(
            parse_delivery_status(Uuid::nil(), &vec![b'x'; MAX_DSN_BYTES + 1]),
            Err(DsnError::InvalidMessageSize)
        ));
    }
}
