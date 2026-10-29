use async_trait::async_trait;

use crate::{error::QueueError, model::ClaimedJob};

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
