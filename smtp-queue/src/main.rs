use std::{sync::Arc, time::Duration};

use mail_queue::worker::Worker;
use mail_queue::{Queue, QueueConfig, TestProcessor, TestProcessorOutcome};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = QueueConfig::from_env()?;
    let queue = Queue::connect(config).await?;
    let (shutdown, cancel) = tokio::sync::watch::channel(false);
    let worker = Worker {
        queue,
        processor: Arc::new(TestProcessor {
            outcome: TestProcessorOutcome::Success,
        }),
        owner: format!("local-worker-{}", std::process::id()),
        poll_interval: Duration::from_secs(1),
    };
    tokio::select! {
        result = worker.run_until_cancelled(cancel) => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            shutdown.send(true)?;
        }
    }
    Ok(())
}
