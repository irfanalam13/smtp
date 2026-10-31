use std::{sync::Arc, time::Duration};

use mail_delivery::{DeliveryConfig, DnsResolver, SmtpProcessor, worker::DeliveryWorker};
use mail_queue::{Queue, QueueConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let config = DeliveryConfig::from_env()?;
    if !config.enabled {
        return Err("SMTP_DELIVERY_ENABLED=true is required to run the delivery worker".into());
    }
    let queue = Queue::connect(QueueConfig::from_env()?).await?;
    let processor = Arc::new(SmtpProcessor {
        config,
        resolver: Arc::new(DnsResolver::try_from_system_conf()?),
    });
    let worker = DeliveryWorker {
        queue,
        processor,
        owner: format!("delivery-{}-{}", std::process::id(), uuid::Uuid::new_v4()),
        poll_interval: Duration::from_secs(1),
    };
    let (shutdown, cancel) = tokio::sync::watch::channel(false);
    let run = worker.run_until_cancelled(cancel);
    tokio::pin!(run);
    tokio::select! {
        result = &mut run => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            shutdown.send(true)?;
            run.await?;
        }
    }
    Ok(())
}
