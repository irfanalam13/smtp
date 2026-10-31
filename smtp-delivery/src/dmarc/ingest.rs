use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;
use uuid::Uuid;

use crate::dmarc_report::{AggregateReport, ReportError, parse_report};

#[derive(Debug, Error)]
pub enum ReportIngestError {
    #[error("DMARC aggregate report is invalid")]
    Invalid(#[from] ReportError),
    #[error("DMARC aggregate report could not be stored")]
    Database(#[from] sqlx::Error),
    #[error("DMARC aggregate report could not be encoded")]
    Encode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IngestOutcome {
    Stored,
    Duplicate,
}

/// Validates and stores a normalized external DMARC aggregate report. Original
/// attachment bytes are deliberately discarded; parsed data expires after 90
/// days and is deduplicated by report-id, submitter, and policy domain.
pub async fn ingest_report(
    pool: &PgPool,
    input: &[u8],
) -> Result<IngestOutcome, ReportIngestError> {
    let report = parse_report(input)?;
    let reporter_domain = report
        .report_metadata
        .email
        .rsplit_once('@')
        .map(|(_, domain)| domain.to_ascii_lowercase())
        .ok_or(ReportIngestError::Invalid(ReportError::Malformed))?;
    let fingerprint = report_fingerprint(&report);
    let parsed_report = serde_json::to_value(&report).map_err(|_| ReportIngestError::Encode)?;
    let inserted = sqlx::query(
        "INSERT INTO dmarc_received_reports
         (id, report_fingerprint, report_id, reporter_domain, policy_domain,
          period_begin, period_end, record_count, parsed_report)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (report_fingerprint) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(fingerprint.as_slice())
    .bind(&report.report_metadata.report_id)
    .bind(reporter_domain)
    .bind(&report.policy_published.domain)
    .bind(report.report_metadata.date_range.begin)
    .bind(report.report_metadata.date_range.end)
    .bind(i32::try_from(report.records.len()).unwrap_or(i32::MAX))
    .bind(parsed_report)
    .execute(pool)
    .await?;
    Ok(if inserted.rows_affected() == 1 {
        IngestOutcome::Stored
    } else {
        IngestOutcome::Duplicate
    })
}

pub async fn connect_from_env() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").map_err(|error| {
        sqlx::Error::Configuration(format!("DATABASE_URL is required: {error}").into())
    })?;
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
}

fn report_fingerprint(report: &AggregateReport) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"smtp-platform/dmarc-ingest/v1\0");
    let reporter_domain = report
        .report_metadata
        .email
        .rsplit_once('@')
        .map_or("", |(_, domain)| domain);
    for value in [
        report.report_metadata.report_id.as_str(),
        reporter_domain,
        report.policy_published.domain.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    digest.finalize().into()
}
