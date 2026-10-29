use std::{sync::Arc, time::Duration};

use crate::{
    error::QueueError,
    processor::{Processor, TestProcessorOutcome},
    store::Queue,
};

pub struct Worker<P> {
    pub queue: Queue,
    pub processor: Arc<P>,
    pub owner: String,
    pub poll_interval: Duration,
}

impl<P: Processor + 'static> Worker<P> {
    pub async fn run_until_cancelled(
        &self,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), QueueError> {
        while !*cancel.borrow() {
            self.queue.recover_expired().await?;
            if let Some(job) = self.queue.claim(&self.owner).await? {
                let outcome = self.processor.process(&job).await?;
                let result = match outcome {
                    TestProcessorOutcome::Success => Ok(TestProcessorOutcome::Success),
                    TestProcessorOutcome::TemporaryFailure => {
                        Ok(TestProcessorOutcome::TemporaryFailure)
                    }
                    TestProcessorOutcome::PermanentFailure => {
                        Ok(TestProcessorOutcome::PermanentFailure)
                    }
                };
                self.queue.complete(&job, result).await?;
            } else {
                tokio::time::sleep(self.poll_interval).await;
            }
        }
        Ok(())
    }
}
