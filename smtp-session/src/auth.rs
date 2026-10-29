use async_trait::async_trait;

#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(&self, username: &str, password: &str) -> bool;
}
