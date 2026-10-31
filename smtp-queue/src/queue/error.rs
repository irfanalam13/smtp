use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("invalid queue configuration: {0}")]
    Configuration(String),
    #[error("invalid enqueue request: {0}")]
    InvalidInput(String),
    #[error("idempotency key conflicts with an existing request")]
    IdempotencyConflict,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("queue lease is missing or no longer owned")]
    LeaseLost,
    #[error("processor error: {0}")]
    Processor(String),
}
