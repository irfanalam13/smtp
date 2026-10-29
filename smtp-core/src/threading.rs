use std::collections::HashMap;

use thiserror::Error;
use uuid::Uuid;

const MAX_REFERENCES: usize = 100;
const MAX_MESSAGE_ID_BYTES: usize = 998;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadMessage {
    pub message_id: Uuid,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub subject: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversation {
    pub id: Uuid,
    pub message_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Default)]
pub struct ThreadIndex {
    messages: HashMap<Uuid, ThreadMessage>,
    message_conversations: HashMap<Uuid, Uuid>,
    header_to_conversation: HashMap<String, Uuid>,
    conversations: HashMap<Uuid, Vec<Uuid>>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ThreadIndexError {
    #[error("message identifier is malformed")]
    InvalidMessageId,
    #[error("message has too many references")]
    TooManyReferences,
    #[error("message is already present in the thread index")]
    DuplicateMessage,
}

impl ThreadIndex {
    pub fn insert(&mut self, mut message: ThreadMessage) -> Result<Uuid, ThreadIndexError> {
        if self.messages.contains_key(&message.message_id) {
            return Err(ThreadIndexError::DuplicateMessage);
        }
        if message.references.len() > MAX_REFERENCES {
            return Err(ThreadIndexError::TooManyReferences);
        }

        message.message_id_header = message
            .message_id_header
            .as_deref()
            .map(normalize_message_id)
            .transpose()?;
        message.in_reply_to = message
            .in_reply_to
            .as_deref()
            .map(normalize_message_id)
            .transpose()?;
        let mut normalized_references = Vec::with_capacity(message.references.len());
        for reference in &message.references {
            let normalized = normalize_message_id(reference)?;
            if !normalized_references.contains(&normalized) {
                normalized_references.push(normalized);
            }
        }
        message.references = normalized_references;

        let mut related_headers = message.references.clone();
        if let Some(in_reply_to) = &message.in_reply_to
            && !related_headers.contains(in_reply_to)
        {
            related_headers.push(in_reply_to.clone());
        }
        if let Some(message_id_header) = &message.message_id_header
            && !related_headers.contains(message_id_header)
        {
            related_headers.push(message_id_header.clone());
        }

        let mut related_conversations = Vec::new();
        for header in &related_headers {
            if let Some(conversation_id) = self.header_to_conversation.get(header)
                && !related_conversations.contains(conversation_id)
            {
                related_conversations.push(*conversation_id);
            }
        }
        let conversation_id = related_conversations
            .first()
            .copied()
            .unwrap_or_else(Uuid::new_v4);

        for other_id in related_conversations.into_iter().skip(1) {
            self.merge_conversations(conversation_id, other_id);
        }
        for header in related_headers {
            self.header_to_conversation.insert(header, conversation_id);
        }

        self.message_conversations
            .insert(message.message_id, conversation_id);
        self.conversations
            .entry(conversation_id)
            .or_default()
            .push(message.message_id);
        self.messages.insert(message.message_id, message);
        Ok(conversation_id)
    }

    pub fn conversation(&self, id: Uuid) -> Option<Conversation> {
        self.conversations.get(&id).map(|ids| Conversation {
            id,
            message_ids: ids.clone(),
        })
    }

    pub fn remove(&mut self, message_id: Uuid) -> bool {
        if self.messages.remove(&message_id).is_none() {
            return false;
        }
        let Some(conversation_id) = self.message_conversations.remove(&message_id) else {
            return false;
        };
        if let Some(message_ids) = self.conversations.get_mut(&conversation_id) {
            message_ids.retain(|id| *id != message_id);
            if message_ids.is_empty() {
                self.conversations.remove(&conversation_id);
                self.header_to_conversation
                    .retain(|_, id| *id != conversation_id);
            }
        }
        true
    }

    fn merge_conversations(&mut self, target: Uuid, other: Uuid) {
        if target == other {
            return;
        }
        if let Some(other_messages) = self.conversations.remove(&other) {
            let target_messages = self.conversations.entry(target).or_default();
            for message_id in other_messages {
                if !target_messages.contains(&message_id) {
                    target_messages.push(message_id);
                }
                self.message_conversations.insert(message_id, target);
            }
        }
        for conversation_id in self.header_to_conversation.values_mut() {
            if *conversation_id == other {
                *conversation_id = target;
            }
        }
    }
}

fn normalize_message_id(raw: &str) -> Result<String, ThreadIndexError> {
    let value = raw.trim();
    let value = if value.starts_with('<') && value.ends_with('>') && value.len() >= 3 {
        &value[1..value.len() - 1]
    } else if value.contains(['<', '>']) {
        return Err(ThreadIndexError::InvalidMessageId);
    } else {
        value
    };
    if value.is_empty()
        || value.len() > MAX_MESSAGE_ID_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(ThreadIndexError::InvalidMessageId);
    }
    let Some((left, right)) = value.split_once('@') else {
        return Err(ThreadIndexError::InvalidMessageId);
    };
    if left.is_empty()
        || right.is_empty()
        || right.contains('@')
        || left.contains(['<', '>', '@'])
        || right.contains(['<', '>'])
    {
        return Err(ThreadIndexError::InvalidMessageId);
    }
    Ok(format!("{left}@{}", right.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: Uuid, header: &str, references: &[&str]) -> ThreadMessage {
        ThreadMessage {
            message_id: id,
            message_id_header: Some(header.into()),
            in_reply_to: None,
            references: references.iter().map(|value| (*value).into()).collect(),
            subject: "Topic".into(),
        }
    }

    #[test]
    fn groups_replies_by_references_and_normalizes_domain_case() {
        let mut index = ThreadIndex::default();
        let first = Uuid::new_v4();
        let thread = index
            .insert(message(first, "<one@EXAMPLE.test>", &[]))
            .unwrap();
        let second = Uuid::new_v4();
        assert_eq!(
            index
                .insert(message(
                    second,
                    "<two@example.test>",
                    &[" <one@example.test> "]
                ))
                .unwrap(),
            thread
        );
        assert_eq!(index.conversation(thread).unwrap().message_ids.len(), 2);
    }

    #[test]
    fn late_ancestor_joins_the_provisional_conversation() {
        let mut index = ThreadIndex::default();
        let child = Uuid::new_v4();
        let provisional = index
            .insert(message(
                child,
                "<child@example.test>",
                &["<root@example.test>"],
            ))
            .unwrap();
        let root = Uuid::new_v4();
        assert_eq!(
            index
                .insert(message(root, "<root@example.test>", &[]))
                .unwrap(),
            provisional
        );
        assert_eq!(
            index.conversation(provisional).unwrap().message_ids.len(),
            2
        );
    }

    #[test]
    fn references_that_connect_existing_threads_merge_membership() {
        let mut index = ThreadIndex::default();
        let first_id = Uuid::new_v4();
        let first_conversation = index
            .insert(message(first_id, "<first@example.test>", &[]))
            .unwrap();
        let second_id = Uuid::new_v4();
        let second_conversation = index
            .insert(message(second_id, "<second@example.test>", &[]))
            .unwrap();
        assert_ne!(first_conversation, second_conversation);

        let connector = Uuid::new_v4();
        let merged = index
            .insert(message(
                connector,
                "<connector@example.test>",
                &["<first@example.test>", "<second@example.test>"],
            ))
            .unwrap();
        assert_eq!(merged, first_conversation);
        assert_eq!(index.conversation(merged).unwrap().message_ids.len(), 3);
        assert!(index.conversation(second_conversation).is_none());
    }

    #[test]
    fn invalid_headers_and_duplicate_internal_ids_are_rejected() {
        let mut index = ThreadIndex::default();
        let id = Uuid::new_v4();
        assert_eq!(
            index.insert(message(id, "bad header", &[])),
            Err(ThreadIndexError::InvalidMessageId)
        );
        index
            .insert(message(id, "<good@example.test>", &[]))
            .unwrap();
        assert_eq!(
            index.insert(message(id, "<other@example.test>", &[])),
            Err(ThreadIndexError::DuplicateMessage)
        );
    }

    #[test]
    fn removing_last_message_removes_conversation_headers() {
        let mut index = ThreadIndex::default();
        let id = Uuid::new_v4();
        let conversation = index
            .insert(message(id, "<one@example.test>", &[]))
            .unwrap();
        assert!(index.remove(id));
        assert!(index.conversation(conversation).is_none());
        assert!(
            index
                .insert(message(
                    Uuid::new_v4(),
                    "<reply@example.test>",
                    &["<one@example.test>"]
                ))
                .is_ok()
        );
    }
}
