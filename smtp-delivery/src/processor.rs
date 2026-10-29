use std::sync::Arc;

use async_trait::async_trait;
use mail_queue::{
    QueueError,
    model::ClaimedJob,
    processor::{Processor, TestProcessorOutcome},
};

use crate::{
    client::deliver,
    config::DeliveryConfig,
    dns::{DnsResolver, Resolver},
    outcome::DeliveryOutcome,
};

pub struct SmtpProcessor<R = DnsResolver> {
    pub config: DeliveryConfig,
    pub resolver: Arc<R>,
}

#[async_trait]
impl<R: Resolver + 'static> Processor for SmtpProcessor<R> {
    async fn process(&self, job: &ClaimedJob) -> Result<TestProcessorOutcome, QueueError> {
        match deliver(job, &self.config, self.resolver.as_ref()).await {
            Ok(report) if report.outcome == DeliveryOutcome::DeliveredAccepted => {
                Ok(TestProcessorOutcome::Success)
            }
            Err(crate::outcome::DeliveryError::PermanentFailure { .. }) => {
                Ok(TestProcessorOutcome::PermanentFailure)
            }
            Err(
                crate::outcome::DeliveryError::SecurityFailure { .. }
                | crate::outcome::DeliveryError::UnsafeDestination
                | crate::outcome::DeliveryError::Tls
                | crate::outcome::DeliveryError::MessageTooLarge,
            ) => Ok(TestProcessorOutcome::PermanentFailure),
            Err(crate::outcome::DeliveryError::Disabled) => {
                Err(QueueError::Processor("real delivery is disabled".into()))
            }
            Err(_) => Ok(TestProcessorOutcome::TemporaryFailure),
            Ok(_) => Ok(TestProcessorOutcome::TemporaryFailure),
        }
    }
}
