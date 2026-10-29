use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::{CoreError, context::CorrelationId};

pub type EventId = Uuid;
pub type SchemaVersion = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublishReceipt {
    pub event_id: EventId,
}

pub trait EventPublisher: Send + Sync {
    fn publish<T: Serialize>(&self, event: &EventEnvelope<T>)
    -> Result<PublishReceipt, EventError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("event serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("event transport is unavailable")]
    Unavailable,
    #[error("event publication was cancelled")]
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventType(String);

impl EventType {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
        {
            return Err(CoreError::InvalidEvent(
                "event type must be a bounded token",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 128
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProducerIdentity(String);

impl ProducerIdentity {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || value.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(CoreError::InvalidEvent("producer identity must be bounded"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 128
            && !self.0.bytes().any(|byte| byte.is_ascii_control())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventEnvelope<T> {
    pub event_id: EventId,
    pub event_type: EventType,
    pub schema_version: SchemaVersion,
    pub occurred_at: DateTime<Utc>,
    pub producer: ProducerIdentity,
    pub correlation_id: Option<CorrelationId>,
    pub payload: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        event_type: EventType,
        schema_version: SchemaVersion,
        producer: ProducerIdentity,
        correlation_id: Option<CorrelationId>,
        payload: T,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            schema_version,
            occurred_at: Utc::now(),
            producer,
            correlation_id,
            payload,
        }
    }

    pub fn to_json(&self) -> Result<String, CoreError>
    where
        T: Serialize,
    {
        Ok(serde_json::to_string(self)?)
    }

    pub fn from_json(value: &str) -> Result<Self, CoreError>
    where
        T: DeserializeOwned,
    {
        let envelope: Self = serde_json::from_str(value)?;
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version == 0 || !self.event_type.is_valid() || !self.producer.is_valid() {
            return Err(CoreError::InvalidEvent("event metadata is invalid"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_without_losing_identity() {
        let envelope = EventEnvelope::new(
            EventType::new("platform.test").unwrap(),
            1,
            ProducerIdentity::new("test-service").unwrap(),
            Some(CorrelationId::generate()),
            serde_json::json!({"state": "ready"}),
        );
        let event_id = envelope.event_id;
        envelope.validate().unwrap();
        let decoded =
            EventEnvelope::<serde_json::Value>::from_json(&envelope.to_json().unwrap()).unwrap();
        assert_eq!(decoded.event_id, event_id);
        assert_eq!(decoded.payload["state"], "ready");
    }

    #[test]
    fn invalid_event_type_is_rejected() {
        assert!(EventType::new("message status").is_err());
    }
}
