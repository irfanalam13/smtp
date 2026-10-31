use std::{sync::Arc, time::Duration};

use mail_queue::worker::Worker;
use mail_queue::{Queue, QueueConfig, TestProcessor, TestProcessorOutcome};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = QueueConfig::from_env()?;
    let queue = Queue::connect(config).await?;
    let (shutdown, cancel) = tokio::sync::watch::channel(false);
    let worker = Worker {
        queue,
        processor: Arc::new(TestProcessor {
            outcome: TestProcessorOutcome::Success,
        }),
        owner: format!("local-queue-worker-{}", std::process::id()),
        poll_interval: Duration::from_secs(1),
    };

    tracing::info!(
        owner = %worker.owner,
        "starting durable queue worker"
    );

    let run = worker.run_until_cancelled(cancel);
    tokio::pin!(run);
    tokio::select! {
        result = &mut run => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            tracing::info!("shutdown signal received; draining worker");
            shutdown.send(true)?;
            run.await?;
        }
    }
    tracing::info!("queue worker exited cleanly");
    Ok(())
}
