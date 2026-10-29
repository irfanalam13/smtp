use std::time::Duration;

use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct Shutdown {
    sender: watch::Sender<bool>,
}

#[derive(Debug)]
pub struct ShutdownReceiver {
    receiver: watch::Receiver<bool>,
}

impl Shutdown {
    pub fn new() -> (Self, ShutdownReceiver) {
        let (sender, receiver) = watch::channel(false);
        (Self { sender }, ShutdownReceiver { receiver })
    }

    pub fn trigger(&self) -> Result<(), ShutdownError> {
        self.sender
            .send(true)
            .map_err(|_| ShutdownError::NoReceivers)
    }
}

impl ShutdownReceiver {
    pub async fn wait(mut self) -> Result<(), ShutdownError> {
        if *self.receiver.borrow() {
            return Ok(());
        }
        self.receiver
            .changed()
            .await
            .map_err(|_| ShutdownError::SenderDropped)
    }

    pub async fn wait_with_deadline(
        mut self,
        timeout: Duration,
    ) -> Result<ShutdownOutcome, ShutdownError> {
        if *self.receiver.borrow() {
            return Ok(ShutdownOutcome::Triggered);
        }
        tokio::time::timeout(timeout, self.receiver.changed())
            .await
            .map_err(|_| ShutdownError::DeadlineExceeded)?
            .map_err(|_| ShutdownError::SenderDropped)?;
        Ok(ShutdownOutcome::Triggered)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownOutcome {
    Triggered,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ShutdownError {
    #[error("shutdown signal has no active receivers")]
    NoReceivers,
    #[error("shutdown sender was dropped before signaling")]
    SenderDropped,
    #[error("shutdown deadline expired")]
    DeadlineExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_signal_is_observable() {
        let (shutdown, receiver) = Shutdown::new();
        shutdown.trigger().unwrap();
        assert_eq!(
            receiver
                .wait_with_deadline(Duration::from_millis(10))
                .await
                .unwrap(),
            ShutdownOutcome::Triggered
        );
    }
}
