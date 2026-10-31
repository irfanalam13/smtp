#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    smtp_imap::server::run_from_env().await
}
