use std::{sync::Arc, time::Duration};

use mail_queue::{
    Queue, QueueError, RecipientDeliveryResult, RecipientDeliveryState, model::ClaimedDeliveryBatch,
};
use tokio::task::JoinSet;

use crate::{dns::Resolver, processor::SmtpProcessor};

pub struct DeliveryWorker<R> {
    pub queue: Queue,
    pub processor: Arc<SmtpProcessor<R>>,
    pub owner: String,
    pub poll_interval: Duration,
}

impl<R: Resolver + 'static> DeliveryWorker<R> {
    pub async fn run_until_cancelled(
        &self,
        mut cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), QueueError> {
        let concurrency = self.processor.config.max_concurrent;
        if concurrency == 0
            || self.owner.is_empty()
            || self.owner.len() > 128
            || self.poll_interval.is_zero()
        {
            return Err(QueueError::Configuration(
                "delivery worker configuration is invalid".into(),
            ));
        }
        let mut tasks = JoinSet::new();
        loop {
            if *cancel.borrow() {
                break;
            }

            let mut claimed = false;
            while tasks.len() < concurrency && !*cancel.borrow() {
                let delivery_mode = if let Some(relay) = &self.processor.config.smtp_relay {
                    match relay.mode {
                        crate::config::SmtpRelayMode::AuthenticatedTls => "relay",
                        crate::config::SmtpRelayMode::LocalMailpit => "mailpit",
                    }
                } else {
                    "direct_mx"
                };
                let batch = self
                    .queue
                    .claim_delivery_batch_for_mode(&self.owner, delivery_mode)
                    .await?;
                let Some(batch) = batch else {
                    break;
                };
                claimed = true;
                let queue = self.queue.clone();
                let processor = self.processor.clone();
                tasks.spawn(async move { process_claimed_batch(queue, processor, batch).await });
            }

            if tasks.is_empty() {
                tokio::select! {
                    changed = cancel.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(self.poll_interval) => {}
                }
            } else if tasks.len() >= concurrency || !claimed {
                tokio::select! {
                    result = tasks.join_next() => {
                        if let Some(Err(error)) = result {
                            tracing::error!(%error, "delivery task panicked");
                        }
                    }
                    changed = cancel.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(self.poll_interval), if !claimed => {}
                }
            }
        }

        while let Some(result) = tasks.join_next().await {
            if let Err(error) = result {
                tracing::error!(%error, "delivery task panicked during shutdown");
            }
        }
        Ok(())
    }
}

async fn process_claimed_batch<R: Resolver>(
    queue: Queue,
    processor: Arc<SmtpProcessor<R>>,
    batch: ClaimedDeliveryBatch,
) {
    let message_id = batch.message_id;
    tracing::info!(
        %message_id,
        recipient_count = batch.attempt_recipients.len(),
        "processing claimed delivery batch"
    );
    let renewal_interval = queue.config().lease_duration / 3;
    let process_result = tokio::select! {
        result = processor.process(&batch) => result,
        error = renew_delivery_lease(
            queue.clone(),
            batch.clone(),
            renewal_interval,
        ) => {
            tracing::error!(%message_id, %error, "delivery lease renewal failed; stopping SMTP processing");
            return;
        }
    };
    let results = match process_result {
        Ok(results) => results,
        Err(error) => {
            tracing::error!(%message_id, %error, "delivery processing failed; recording uncertain outcomes");
            batch
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::Uncertain,
                    smtp_code: None,
                    diagnostic: Some(
                        "delivery processor failed; remote acceptance is unknown".into(),
                    ),
                    remote_host: None,
                })
                .collect()
        }
    };
    let accepted = results
        .iter()
        .filter(|result| result.state == RecipientDeliveryState::AcceptedByRemote)
        .count();
    let temporary_failures = results
        .iter()
        .filter(|result| result.state == RecipientDeliveryState::TemporaryFailure)
        .count();
    let permanent_failures = results
        .iter()
        .filter(|result| result.state == RecipientDeliveryState::PermanentFailure)
        .count();
    let uncertain = results
        .iter()
        .filter(|result| result.state == RecipientDeliveryState::Uncertain)
        .count();
    if let Err(error) = queue.complete_delivery_batch(&batch, &results).await {
        tracing::error!(%message_id, %error, "failed to persist delivery batch result");
        return;
    }

    match queue.state(message_id).await {
        Ok(Some(state)) => {
            if state == mail_queue::QueueState::RetryScheduled {
                tracing::warn!(
                    %message_id,
                    queue_state = ?state,
                    accepted,
                    temporary_failures,
                    permanent_failures,
                    uncertain,
                    "delivery outcomes persisted; retry scheduled"
                );
            } else {
                tracing::info!(
                    %message_id,
                    queue_state = ?state,
                    accepted,
                    temporary_failures,
                    permanent_failures,
                    uncertain,
                    "final delivery status persisted"
                );
            }
        }
        Ok(None) => tracing::error!(%message_id, "delivery status disappeared after completion"),
        Err(error) => {
            tracing::error!(%message_id, %error, "failed to read final delivery status");
        }
    }
}

async fn renew_delivery_lease(
    queue: Queue,
    batch: ClaimedDeliveryBatch,
    interval: Duration,
) -> QueueError {
    loop {
        tokio::time::sleep(interval).await;
        match queue.renew_delivery_batch_lease(&batch).await {
            Ok(true) => {}
            Ok(false) => return QueueError::LeaseLost,
            Err(error) => return error,
        }
    }
}
