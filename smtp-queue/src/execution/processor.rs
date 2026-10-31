use async_trait::async_trait;
use thiserror::Error;

use crate::{
    error::QueueError,
    model::{ClaimedDeliveryBatch, ClaimedJob, RecipientDeliveryResult, RecipientDeliveryState},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestProcessorOutcome {
    Success,
    TemporaryFailure,
    PermanentFailure,
}

#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, job: &ClaimedJob) -> Result<TestProcessorOutcome, QueueError>;
}

#[derive(Clone, Debug)]
pub struct TestProcessor {
    pub outcome: TestProcessorOutcome,
}

#[async_trait]
impl Processor for TestProcessor {
    async fn process(&self, _job: &ClaimedJob) -> Result<TestProcessorOutcome, QueueError> {
        Ok(self.outcome.clone())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DeliveryProcessingError {
    #[error("transient delivery failure: {diagnostic}")]
    Transient {
        diagnostic: String,
        smtp_code: Option<u16>,
    },
    #[error("permanent delivery failure: {diagnostic}")]
    Permanent {
        diagnostic: String,
        smtp_code: Option<u16>,
    },
    #[error("uncertain delivery outcome: {diagnostic}")]
    Uncertain { diagnostic: String },
    #[error("lease lost during delivery processing")]
    LeaseLost,
    #[error("internal delivery failure: {0}")]
    Internal(String),
}

impl DeliveryProcessingError {
    pub fn to_recipient_results(
        &self,
        batch: &ClaimedDeliveryBatch,
    ) -> Vec<RecipientDeliveryResult> {
        let (state, smtp_code, diag) = match self {
            Self::Transient {
                diagnostic,
                smtp_code,
            } => (
                RecipientDeliveryState::TemporaryFailure,
                *smtp_code,
                diagnostic.clone(),
            ),
            Self::Permanent {
                diagnostic,
                smtp_code,
            } => (
                RecipientDeliveryState::PermanentFailure,
                *smtp_code,
                diagnostic.clone(),
            ),
            Self::Uncertain { diagnostic } => {
                (RecipientDeliveryState::Uncertain, None, diagnostic.clone())
            }
            Self::LeaseLost => (
                RecipientDeliveryState::Uncertain,
                None,
                "lease lost during delivery".into(),
            ),
            Self::Internal(diagnostic) => (
                RecipientDeliveryState::Uncertain,
                None,
                format!("internal processor failure: {diagnostic}"),
            ),
        };

        batch
            .attempt_recipients
            .iter()
            .map(|recipient| RecipientDeliveryResult {
                delivery_id: recipient.delivery_id,
                state,
                smtp_code,
                diagnostic: Some(diag.clone()),
                remote_host: None,
            })
            .collect()
    }
}

#[async_trait]
pub trait DeliveryBatchProcessor: Send + Sync {
    async fn process_batch(
        &self,
        batch: &ClaimedDeliveryBatch,
    ) -> Result<Vec<RecipientDeliveryResult>, DeliveryProcessingError>;
}

#[derive(Clone, Debug)]
pub struct TestDeliveryBatchProcessor {
    pub default_result: Result<Vec<RecipientDeliveryResult>, DeliveryProcessingError>,
    pub delay: std::time::Duration,
}

impl TestDeliveryBatchProcessor {
    pub fn successful() -> Self {
        Self {
            default_result: Ok(vec![]),
            delay: std::time::Duration::ZERO,
        }
    }

    pub fn with_delay(delay: std::time::Duration) -> Self {
        Self {
            default_result: Ok(vec![]),
            delay,
        }
    }

    pub fn failing_transient(diagnostic: impl Into<String>) -> Self {
        Self {
            default_result: Err(DeliveryProcessingError::Transient {
                diagnostic: diagnostic.into(),
                smtp_code: Some(451),
            }),
            delay: std::time::Duration::ZERO,
        }
    }

    pub fn failing_permanent(diagnostic: impl Into<String>) -> Self {
        Self {
            default_result: Err(DeliveryProcessingError::Permanent {
                diagnostic: diagnostic.into(),
                smtp_code: Some(550),
            }),
            delay: std::time::Duration::ZERO,
        }
    }
}

#[async_trait]
impl DeliveryBatchProcessor for TestDeliveryBatchProcessor {
    async fn process_batch(
        &self,
        batch: &ClaimedDeliveryBatch,
    ) -> Result<Vec<RecipientDeliveryResult>, DeliveryProcessingError> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        match &self.default_result {
            Ok(results) if results.is_empty() => Ok(batch
                .attempt_recipients
                .iter()
                .map(|r| RecipientDeliveryResult {
                    delivery_id: r.delivery_id,
                    state: RecipientDeliveryState::AcceptedByRemote,
                    smtp_code: Some(250),
                    diagnostic: Some("simulated remote acceptance".into()),
                    remote_host: Some("simulated-mx.test".into()),
                })
                .collect()),
            Ok(results) => Ok(results.clone()),
            Err(err) => Err(err.clone()),
        }
    }
}
