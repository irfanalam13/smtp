use mail_queue::{Queue, QueueConfig};
use smtp_ingress::{Config, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::default();
    if std::env::var_os("DATABASE_URL").is_some() {
        config.queue = Some(Queue::connect(QueueConfig::from_env()?).await?);
    }
    let server = Server::bind(config).await?;
    eprintln!("SMTP test server listening on {}", server.local_addr()?);
    server
        .run(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
