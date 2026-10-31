use std::{collections::BTreeMap, net::IpAddr, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use mail_auth::{MessageAuthenticator, hickory_resolver::system_conf::read_system_conf};
use mail_queue::{Queue, QueueConfig, model::EnqueueRequest};
use serde_json::Value;
use shared_types::{CanonicalMessage, MessageEnvelope, MimeLimits};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use thiserror::Error;
use tokio::time::timeout;
use uuid::Uuid;

use crate::dmarc_report::{
    AggregateAuthResults, AggregateRecord, AggregateReport, AggregateRow, DkimAuthResult,
    PolicyEvaluated, PublishedPolicy, ReportDateRange, ReportMetadata, SpfAuthResult,
};

const MAX_QUEUED_REPORT_MESSAGE_BYTES: usize = 10 * 1024 * 1024;
const REPORT_DNS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum ReportWorkerError {
    #[error("DMARC reporting configuration is invalid")]
    Configuration,
    #[error("DMARC report database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("DMARC report could not be encoded")]
    Report(#[from] crate::dmarc_report::ReportError),
    #[error("DMARC report could not be queued")]
    Queue(#[from] mail_queue::QueueError),
    #[error("DMARC reporting DNS validation is temporarily unavailable")]
    Resolver,
}

#[derive(Clone)]
struct ReportConfig {
    report_domain: String,
    sender: String,
    organization: String,
    contact: String,
}

impl ReportConfig {
    fn from_env() -> Result<Self, ReportWorkerError> {
        let report_domain = required_env("SMTP_DKIM_REPORT_DOMAIN")?.to_ascii_lowercase();
        let sender = required_env("SMTP_DKIM_REPORT_SENDER")?;
        let organization = required_env("SMTP_DKIM_REPORT_ORG_NAME")?;
        let contact = required_env("SMTP_DKIM_REPORT_CONTACT")?;
        let sender_domain = sender
            .rsplit_once('@')
            .map(|(_, domain)| domain.to_ascii_lowercase());
        if !valid_domain(&report_domain)
            || sender_domain.as_deref() != Some(report_domain.as_str())
            || !valid_mailbox(&sender)
            || organization.is_empty()
            || organization.len() > 255
            || organization.bytes().any(|byte| byte.is_ascii_control())
            || !valid_mailbox(&contact)
        {
            return Err(ReportWorkerError::Configuration);
        }
        Ok(Self {
            report_domain,
            sender,
            organization,
            contact,
        })
    }
}

/// Runs the RFC 9990 aggregate report outbox. Report data is captured at SMTP
/// evaluation time, daily reports are immutable/idempotent, and final delivery
/// uses the regular queue and outbound DKIM/TLS policy.
pub async fn run_dmarc_report_worker() -> Result<(), ReportWorkerError> {
    let config = ReportConfig::from_env()?;
    let report_authenticator = report_dns_authenticator()?;
    let queue_config = QueueConfig::from_env()?;
    let queue = Queue::connect(queue_config.clone()).await?;
    let pool = PgPoolOptions::new()
        .max_connections(queue_config.max_pool_connections)
        .connect(&queue_config.database_url)
        .await?;

    loop {
        if let Err(error) = prune_expired_reports(&pool).await {
            tracing::error!(%error, "could not prune expired DMARC report data");
        }
        match create_completed_reports(&pool, &config).await {
            Ok(created) if created > 0 => {
                tracing::info!(created, "created DMARC aggregate reports")
            }
            Ok(_) => {}
            Err(error) => tracing::error!(%error, "could not create DMARC aggregate reports"),
        }
        match enqueue_pending_reports(&pool, &queue, &config, &report_authenticator).await {
            Ok(enqueued) if enqueued > 0 => {
                tracing::info!(enqueued, "queued DMARC aggregate reports")
            }
            Ok(_) => {}
            Err(error) => tracing::error!(%error, "could not queue DMARC aggregate reports"),
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

fn report_dns_authenticator() -> Result<MessageAuthenticator, ReportWorkerError> {
    let (resolver_config, mut options) =
        read_system_conf().map_err(|_| ReportWorkerError::Resolver)?;
    options.timeout = Duration::from_secs(3);
    options.attempts = 2;
    options.cache_size = 512;
    options.num_concurrent_reqs = 1;
    options.validate = true;
    MessageAuthenticator::new(resolver_config, options).map_err(|_| ReportWorkerError::Resolver)
}

async fn prune_expired_reports(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dmarc_received_reports WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM dmarc_report_batches WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM dmarc_report_events WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    Ok(())
}

async fn create_completed_reports(
    pool: &PgPool,
    config: &ReportConfig,
) -> Result<u64, ReportWorkerError> {
    let rows = sqlx::query(
        "SELECT date_trunc('day', event.received_at) AS period,
                event.policy_domain, destination.address, destination.max_size,
                event.source_ip::text AS source_ip, event.header_from,
                event.envelope_from_domain, event.spf_domain, event.spf_scope, event.spf_result,
                event.dmarc_spf_result, event.dmarc_dkim_result, event.disposition,
                event.published_policy, event.subdomain_policy,
                event.spf_alignment, event.dkim_alignment, event.dkim_results,
                COUNT(*)::bigint AS message_count
         FROM dmarc_report_events AS event
         CROSS JOIN LATERAL unnest(
             event.report_destinations, event.report_destination_max_bytes
         ) AS destination(address, max_size)
         WHERE event.received_at < date_trunc('day', NOW())
           AND NOT EXISTS (
             SELECT 1 FROM dmarc_report_periods AS completed
             WHERE completed.period_begin = floor(
                 extract(epoch FROM date_trunc('day', event.received_at))
             )::bigint
           )
         GROUP BY date_trunc('day', event.received_at), event.policy_domain,
                  destination.address, destination.max_size, event.source_ip,
                  event.header_from, event.envelope_from_domain, event.spf_domain, event.spf_scope,
                  event.spf_result, event.dmarc_spf_result, event.dmarc_dkim_result,
                  event.disposition, event.published_policy, event.subdomain_policy,
                  event.spf_alignment, event.dkim_alignment, event.dkim_results
         ORDER BY period, event.policy_domain, destination.address, event.source_ip",
    )
    .fetch_all(pool)
    .await?;

    let mut groups: BTreeMap<ReportGroupKey, Vec<AggregateRecord>> = BTreeMap::new();
    for row in rows {
        let period: DateTime<Utc> = row.try_get("period")?;
        let period_begin = period.timestamp();
        let policy_domain: String = row.try_get("policy_domain")?;
        let destination: String = row.try_get("address")?;
        let max_size: i64 = row.try_get("max_size")?;
        let source_ip: String = row.try_get("source_ip")?;
        let header_from: String = row.try_get("header_from")?;
        let envelope_from: String = row.try_get("envelope_from_domain")?;
        let spf_domain: String = row.try_get("spf_domain")?;
        let spf_scope: String = row.try_get("spf_scope")?;
        let spf_result: String = row.try_get("spf_result")?;
        let dmarc_spf: String = row.try_get("dmarc_spf_result")?;
        let dmarc_dkim: String = row.try_get("dmarc_dkim_result")?;
        let disposition: String = row.try_get("disposition")?;
        let published_policy: String = row.try_get("published_policy")?;
        let subdomain_policy: String = row.try_get("subdomain_policy")?;
        let spf_alignment: String = row.try_get("spf_alignment")?;
        let dkim_alignment: String = row.try_get("dkim_alignment")?;
        let dkim_json: Value = row.try_get("dkim_results")?;
        let message_count: i64 = row.try_get("message_count")?;
        if !valid_mailbox(&destination)
            || max_size < 0
            || !valid_domain(&policy_domain)
            || !valid_domain(&header_from)
            || !valid_domain(&spf_domain)
        {
            continue;
        }
        let dkim = parse_dkim_results(dkim_json);
        let dkim_results = if dkim.is_empty() {
            Vec::new()
        } else {
            dkim.into_iter()
                .map(|(domain, result)| DkimAuthResult { domain, result })
                .collect()
        };
        let source_ip: IpAddr = source_ip
            .parse()
            .map_err(|_| ReportWorkerError::Configuration)?;
        let record = AggregateRecord {
            row: AggregateRow {
                source_ip: source_ip.to_string(),
                count: u64::try_from(message_count).unwrap_or(0).max(1),
                policy_evaluated: PolicyEvaluated {
                    disposition: disposition.clone(),
                    dkim: dmarc_dkim,
                    spf: dmarc_spf,
                },
            },
            identifiers: crate::dmarc_report::AggregateIdentifiers {
                envelope_from: (!envelope_from.is_empty()).then_some(envelope_from),
                header_from,
            },
            auth_results: AggregateAuthResults {
                spf: vec![SpfAuthResult {
                    domain: spf_domain,
                    scope: spf_scope,
                    result: spf_result,
                }],
                dkim: dkim_results,
            },
        };
        groups
            .entry(ReportGroupKey {
                period_begin,
                policy_domain,
                destination,
                max_size: u64::try_from(max_size).unwrap_or(0),
                published_policy,
                subdomain_policy,
                spf_alignment,
                dkim_alignment,
            })
            .or_default()
            .push(record);
    }

    let mut inserted = 0;
    let mut completed_periods = std::collections::BTreeSet::new();
    for (key, mut records) in groups {
        completed_periods.insert(key.period_begin);
        records.sort_by_cached_key(|record| serde_json::to_string(record).unwrap_or_default());
        let limit = if key.max_size == 0 {
            6 * 1024 * 1024
        } else {
            key.max_size.min(6 * 1024 * 1024) as usize
        };
        let parts = split_report_parts(config, &key, &records, limit)?;
        if parts.is_empty() {
            tracing::warn!(domain = %key.policy_domain,
                "a DMARC aggregate report row exceeds the published size limit");
            continue;
        }
        for (part_number, (id, attachment_gzip)) in parts.into_iter().enumerate() {
            let attachment_name = format!(
                "{}!{}!{}!{}!{}.xml.gz",
                config.report_domain,
                key.policy_domain,
                key.period_begin,
                key.period_begin + 86_400,
                id.simple()
            );
            let outcome = sqlx::query(
                "INSERT INTO dmarc_report_batches
                 (id, period_begin, period_end, policy_domain, report_destination,
                  published_policy, subdomain_policy, spf_alignment, dkim_alignment,
                  attachment_name, attachment_gzip, part_number)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (period_begin, period_end, policy_domain, report_destination,
                              published_policy, subdomain_policy, spf_alignment, dkim_alignment,
                              part_number)
                 DO NOTHING",
            )
            .bind(id)
            .bind(key.period_begin)
            .bind(key.period_begin + 86_400)
            .bind(&key.policy_domain)
            .bind(&key.destination)
            .bind(&key.published_policy)
            .bind(&key.subdomain_policy)
            .bind(&key.spf_alignment)
            .bind(&key.dkim_alignment)
            .bind(attachment_name)
            .bind(attachment_gzip)
            .bind(i32::try_from(part_number).unwrap_or(i32::MAX))
            .execute(pool)
            .await?;
            inserted += outcome.rows_affected();
        }
    }
    for period_begin in completed_periods {
        sqlx::query(
            "INSERT INTO dmarc_report_periods (period_begin) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(period_begin)
        .execute(pool)
        .await?;
    }
    Ok(inserted)
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct ReportGroupKey {
    period_begin: i64,
    policy_domain: String,
    destination: String,
    max_size: u64,
    published_policy: String,
    subdomain_policy: String,
    spf_alignment: String,
    dkim_alignment: String,
}

fn split_report_parts(
    config: &ReportConfig,
    key: &ReportGroupKey,
    records: &[AggregateRecord],
    max_attachment_bytes: usize,
) -> Result<Vec<(Uuid, Vec<u8>)>, ReportWorkerError> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let id = Uuid::new_v4();
    let report = AggregateReport::new(
        ReportMetadata {
            org_name: config.organization.clone(),
            email: config.contact.clone(),
            report_id: format!("{}@{}", id.simple(), config.report_domain),
            date_range: ReportDateRange {
                begin: key.period_begin,
                end: key.period_begin + 86_400,
            },
        },
        PublishedPolicy {
            domain: key.policy_domain.clone(),
            adkim: key.dkim_alignment.clone(),
            aspf: key.spf_alignment.clone(),
            p: key.published_policy.clone(),
            sp: key.subdomain_policy.clone(),
        },
        records.to_vec(),
    );
    if let Ok(report) = report {
        match report.to_gzip() {
            Ok(gzip) if gzip.len() <= max_attachment_bytes => return Ok(vec![(id, gzip)]),
            Err(crate::dmarc_report::ReportError::TooLarge) if records.len() > 1 => {}
            Err(crate::dmarc_report::ReportError::TooLarge) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
            _ => {}
        }
    } else if records.len() == 1 {
        return Err(crate::dmarc_report::ReportError::Malformed.into());
    }
    if records.len() == 1 {
        return Ok(Vec::new());
    }
    let midpoint = records.len() / 2;
    let mut parts = split_report_parts(config, key, &records[..midpoint], max_attachment_bytes)?;
    parts.extend(split_report_parts(
        config,
        key,
        &records[midpoint..],
        max_attachment_bytes,
    )?);
    Ok(parts)
}

async fn enqueue_pending_reports(
    pool: &PgPool,
    queue: &Queue,
    config: &ReportConfig,
    report_authenticator: &MessageAuthenticator,
) -> Result<u64, ReportWorkerError> {
    let rows = sqlx::query(
        "SELECT id, period_begin, period_end, created_at, policy_domain, report_destination,
                attachment_name, attachment_gzip
         FROM dmarc_report_batches
         WHERE queued_at IS NULL AND discarded_at IS NULL
         ORDER BY created_at, id
         LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    let mut queued = 0;
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let period_begin: i64 = row.try_get("period_begin")?;
        let period_end: i64 = row.try_get("period_end")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let policy_domain: String = row.try_get("policy_domain")?;
        let destination: String = row.try_get("report_destination")?;
        let attachment_name: String = row.try_get("attachment_name")?;
        let attachment_gzip: Vec<u8> = row.try_get("attachment_gzip")?;
        if !valid_mailbox(&destination) || !valid_domain(&policy_domain) {
            tracing::error!(batch_id = %id, "invalid stored DMARC report batch was skipped");
            continue;
        }
        let requested_destination = [destination.clone()];
        match timeout(
            REPORT_DNS_TIMEOUT,
            report_authenticator.authorized_report_addresses((
                policy_domain.as_str(),
                requested_destination.as_slice(),
            )),
        )
        .await
        {
            Ok(Ok(authorized)) if !authorized.is_empty() => {}
            Ok(Ok(_)) => {
                sqlx::query(
                    "UPDATE dmarc_report_batches
                     SET discarded_at = NOW(), discard_reason = 'report destination authorization absent'
                     WHERE id = $1 AND queued_at IS NULL AND discarded_at IS NULL",
                )
                .bind(id)
                .execute(pool)
                .await?;
                continue;
            }
            Ok(Err(_)) | Err(_) => return Err(ReportWorkerError::Resolver),
        }
        let raw = build_report_message(
            config,
            id,
            created_at,
            period_begin,
            period_end,
            &policy_domain,
            &destination,
            &attachment_name,
            &attachment_gzip,
        );
        if raw.len() > MAX_QUEUED_REPORT_MESSAGE_BYTES {
            tracing::error!(batch_id = %id, "DMARC report message exceeds the outbound size limit");
            continue;
        }
        let message = CanonicalMessage::parse(
            MessageEnvelope {
                sender: config.sender.clone(),
                recipients: vec![destination],
            },
            raw,
            MimeLimits::default(),
        )
        .map_err(|_| ReportWorkerError::Configuration)?;
        queue
            .enqueue(&EnqueueRequest::new(
                &message,
                id.to_string(),
                "dmarc-report-batch".to_owned(),
            ))
            .await?;
        sqlx::query(
            "UPDATE dmarc_report_batches SET queued_at = NOW() WHERE id = $1 AND queued_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;
        queued += 1;
    }
    Ok(queued)
}

#[allow(clippy::too_many_arguments)]
fn build_report_message(
    config: &ReportConfig,
    id: Uuid,
    created_at: DateTime<Utc>,
    begin: i64,
    end: i64,
    policy_domain: &str,
    destination: &str,
    attachment_name: &str,
    compressed_xml: &[u8],
) -> Vec<u8> {
    let boundary = format!("dmarc-report-{}", id.simple());
    let report_id = format!("{}@{}", id.simple(), config.report_domain);
    let date = created_at.to_rfc2822();
    let encoded = STANDARD.encode(compressed_xml);
    let wrapped = encoded
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n");
    format!(
        "From: DMARC reports <{}>\r\nTo: <{}>\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nSubject: Report Domain: {} Submitter: {} Report-ID: {}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n--{}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 7bit\r\n\r\nDMARC aggregate report for {} from {} through {}.\r\n\r\n--{}\r\nContent-Type: application/gzip\r\nContent-Transfer-Encoding: base64\r\nContent-Disposition: attachment; filename=\"{}\"\r\n\r\n{}\r\n--{}--\r\n",
        config.sender,
        destination,
        date,
        id.simple(),
        config.report_domain,
        policy_domain,
        config.report_domain,
        report_id,
        boundary,
        boundary,
        policy_domain,
        begin,
        end,
        boundary,
        attachment_name,
        wrapped,
        boundary
    )
    .into_bytes()
}

fn parse_dkim_results(value: Value) -> Vec<(String, String)> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let domain = item.get("domain")?.as_str()?;
            let result = item.get("result")?.as_str()?;
            if valid_domain(domain) && valid_auth_result(result) {
                Some((domain.to_owned(), result.to_owned()))
            } else {
                None
            }
        })
        .take(20)
        .collect()
}

fn required_env(name: &str) -> Result<String, ReportWorkerError> {
    std::env::var(name).map_err(|_| ReportWorkerError::Configuration)
}

fn valid_mailbox(value: &str) -> bool {
    if value.len() > 320 || value.bytes().any(|byte| byte.is_ascii_control()) {
        return false;
    }
    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'.' | b'!'
                        | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'/'
                        | b'='
                        | b'?'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'{'
                        | b'|'
                        | b'}'
                        | b'~'
                )
        })
        && valid_domain(domain)
}

fn valid_domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
        })
}

fn valid_auth_result(value: &str) -> bool {
    matches!(
        value,
        "pass" | "fail" | "softfail" | "neutral" | "temperror" | "permerror" | "none"
    )
}
