use std::collections::HashSet;

use thiserror::Error;
use uuid::Uuid;

const MAX_SUGGESTION_BYTES: usize = 16 * 1024;
const MAX_SOURCE_MESSAGES: usize = 100;
const MAX_MODEL_VERSION_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftSuggestionKind {
    SmartReply,
    ComposeAssistance,
    Summary,
    Translation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DraftSuggestion {
    kind: DraftSuggestionKind,
    text: String,
    source_message_ids: Vec<Uuid>,
    model_version: String,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum DraftSuggestionError {
    #[error("suggestion text is empty")]
    EmptyText,
    #[error("suggestion text exceeds the configured limit")]
    TextTooLarge,
    #[error("suggestion text contains unsupported control characters")]
    InvalidText,
    #[error("suggestion has too many source messages")]
    TooManySources,
    #[error("suggestion contains an invalid or duplicate source message")]
    InvalidSources,
    #[error("model version is empty, too long, or contains control characters")]
    InvalidModelVersion,
}

impl DraftSuggestion {
    pub fn new(
        kind: DraftSuggestionKind,
        text: String,
        source_message_ids: Vec<Uuid>,
        model_version: String,
    ) -> Result<Self, DraftSuggestionError> {
        if text.trim().is_empty() {
            return Err(DraftSuggestionError::EmptyText);
        }
        if text.len() > MAX_SUGGESTION_BYTES {
            return Err(DraftSuggestionError::TextTooLarge);
        }
        if text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(DraftSuggestionError::InvalidText);
        }
        if source_message_ids.len() > MAX_SOURCE_MESSAGES {
            return Err(DraftSuggestionError::TooManySources);
        }
        let mut unique_sources = HashSet::with_capacity(source_message_ids.len());
        if source_message_ids
            .iter()
            .any(|id| id.is_nil() || !unique_sources.insert(*id))
        {
            return Err(DraftSuggestionError::InvalidSources);
        }
        if model_version.trim().is_empty()
            || model_version.len() > MAX_MODEL_VERSION_BYTES
            || model_version.chars().any(char::is_control)
        {
            return Err(DraftSuggestionError::InvalidModelVersion);
        }

        Ok(Self {
            kind,
            text,
            source_message_ids,
            model_version,
        })
    }

    pub fn kind(&self) -> DraftSuggestionKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn source_message_ids(&self) -> &[Uuid] {
        &self.source_message_ids
    }

    pub fn model_version(&self) -> &str {
        &self.model_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_suggestion() -> Result<DraftSuggestion, DraftSuggestionError> {
        DraftSuggestion::new(
            DraftSuggestionKind::SmartReply,
            "Thanks for the update.".to_owned(),
            vec![Uuid::new_v4()],
            "provider/model-v1".to_owned(),
        )
    }

    #[test]
    fn creates_bounded_reviewable_suggestion_without_send_semantics() {
        let source = Uuid::new_v4();
        let suggestion = DraftSuggestion::new(
            DraftSuggestionKind::SmartReply,
            "Thanks for the update.".to_owned(),
            vec![source],
            "provider/model-v1".to_owned(),
        )
        .expect("valid suggestion");

        assert_eq!(suggestion.kind(), DraftSuggestionKind::SmartReply);
        assert_eq!(suggestion.text(), "Thanks for the update.");
        assert_eq!(suggestion.source_message_ids(), &[source]);
        assert_eq!(suggestion.model_version(), "provider/model-v1");
    }

    #[test]
    fn rejects_empty_oversized_and_control_character_text() {
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                " \n\t".to_owned(),
                vec![],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::EmptyText)
        );
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                "x".repeat(MAX_SUGGESTION_BYTES + 1),
                vec![],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::TextTooLarge)
        );
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                "unsafe\u{0007}text".to_owned(),
                vec![],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::InvalidText)
        );
    }

    #[test]
    fn rejects_duplicate_nil_or_excessive_sources() {
        let source = Uuid::new_v4();
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                "A summary".to_owned(),
                vec![source, source],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::InvalidSources)
        );
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                "A summary".to_owned(),
                vec![Uuid::nil()],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::InvalidSources)
        );
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Summary,
                "A summary".to_owned(),
                vec![Uuid::new_v4(); MAX_SOURCE_MESSAGES + 1],
                "model-v1".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::TooManySources)
        );
    }

    #[test]
    fn rejects_invalid_model_version() {
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Translation,
                "Bonjour".to_owned(),
                vec![],
                " \t".to_owned(),
            )
            .err(),
            Some(DraftSuggestionError::InvalidModelVersion)
        );
        assert_eq!(
            DraftSuggestion::new(
                DraftSuggestionKind::Translation,
                "Bonjour".to_owned(),
                vec![],
                format!("v{}", "x".repeat(MAX_MODEL_VERSION_BYTES)),
            )
            .err(),
            Some(DraftSuggestionError::InvalidModelVersion)
        );
    }

    #[test]
    fn validates_a_regular_suggestion() {
        assert!(valid_suggestion().is_ok());
    }
}
