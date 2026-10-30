use std::collections::HashSet;

use thiserror::Error;
use uuid::Uuid;

const MAX_EMBEDDING_DIMENSIONS: usize = 2_048;
const MAX_AUTHORIZED_MAILBOXES: usize = 1_000;
const MAX_CANDIDATES: usize = 10_000;
const MAX_RESULTS: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalScope {
    tenant_id: Uuid,
    mailbox_ids: HashSet<Uuid>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedMessage {
    tenant_id: Uuid,
    mailbox_id: Uuid,
    message_id: Uuid,
    embedding: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticHit {
    pub mailbox_id: Uuid,
    pub message_id: Uuid,
    pub similarity: f32,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum SemanticSearchError {
    #[error("tenant identifier must not be nil")]
    InvalidTenant,
    #[error("retrieval scope must contain authorized mailboxes")]
    EmptyScope,
    #[error("retrieval scope contains an invalid or duplicate mailbox")]
    InvalidScope,
    #[error("retrieval scope exceeds the mailbox limit")]
    ScopeTooLarge,
    #[error("embedding dimensions are outside the supported bounds")]
    InvalidDimensions,
    #[error("embedding contains a non-finite or zero vector")]
    InvalidEmbedding,
    #[error("candidate set exceeds the configured limit")]
    TooManyCandidates,
    #[error("result limit is outside the supported bounds")]
    InvalidResultLimit,
    #[error("authorized candidate contains an invalid identifier")]
    InvalidCandidate,
    #[error("authorized candidate embedding dimensions do not match the query")]
    DimensionMismatch,
    #[error("authorized candidate message is duplicated")]
    DuplicateCandidate,
}

impl RetrievalScope {
    pub fn new(tenant_id: Uuid, mailbox_ids: Vec<Uuid>) -> Result<Self, SemanticSearchError> {
        if tenant_id.is_nil() {
            return Err(SemanticSearchError::InvalidTenant);
        }
        if mailbox_ids.is_empty() {
            return Err(SemanticSearchError::EmptyScope);
        }
        if mailbox_ids.len() > MAX_AUTHORIZED_MAILBOXES {
            return Err(SemanticSearchError::ScopeTooLarge);
        }
        let unique_mailboxes: HashSet<_> = mailbox_ids.iter().copied().collect();
        if unique_mailboxes.len() != mailbox_ids.len() || unique_mailboxes.contains(&Uuid::nil()) {
            return Err(SemanticSearchError::InvalidScope);
        }
        Ok(Self {
            tenant_id,
            mailbox_ids: unique_mailboxes,
        })
    }
}

impl EmbeddedMessage {
    pub fn new(tenant_id: Uuid, mailbox_id: Uuid, message_id: Uuid, embedding: Vec<f32>) -> Self {
        Self {
            tenant_id,
            mailbox_id,
            message_id,
            embedding,
        }
    }
}

pub fn semantic_search(
    query_embedding: &[f32],
    scope: &RetrievalScope,
    candidates: &[EmbeddedMessage],
    limit: usize,
) -> Result<Vec<SemanticHit>, SemanticSearchError> {
    if candidates.len() > MAX_CANDIDATES {
        return Err(SemanticSearchError::TooManyCandidates);
    }
    if limit == 0 || limit > MAX_RESULTS {
        return Err(SemanticSearchError::InvalidResultLimit);
    }
    validate_embedding(query_embedding)?;

    let mut ranked = Vec::new();
    let mut seen = HashSet::new();
    for candidate in candidates.iter().filter(|candidate| {
        candidate.tenant_id == scope.tenant_id && scope.mailbox_ids.contains(&candidate.mailbox_id)
    }) {
        if candidate.mailbox_id.is_nil() || candidate.message_id.is_nil() {
            return Err(SemanticSearchError::InvalidCandidate);
        }
        if candidate.embedding.len() != query_embedding.len() {
            return Err(SemanticSearchError::DimensionMismatch);
        }
        validate_embedding(&candidate.embedding)?;
        if !seen.insert((candidate.mailbox_id, candidate.message_id)) {
            return Err(SemanticSearchError::DuplicateCandidate);
        }
        ranked.push(SemanticHit {
            mailbox_id: candidate.mailbox_id,
            message_id: candidate.message_id,
            similarity: cosine_similarity(query_embedding, &candidate.embedding),
        });
    }

    ranked.sort_by(|left, right| {
        right
            .similarity
            .total_cmp(&left.similarity)
            .then_with(|| left.mailbox_id.cmp(&right.mailbox_id))
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    ranked.truncate(limit);
    Ok(ranked)
}

fn validate_embedding(embedding: &[f32]) -> Result<(), SemanticSearchError> {
    if embedding.is_empty() || embedding.len() > MAX_EMBEDDING_DIMENSIONS {
        return Err(SemanticSearchError::InvalidDimensions);
    }
    if embedding.iter().any(|value| !value.is_finite())
        || embedding.iter().all(|value| *value == 0.0)
    {
        return Err(SemanticSearchError::InvalidEmbedding);
    }
    Ok(())
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let mut dot_product = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left_value, right_value) in left.iter().zip(right) {
        let left_value = f64::from(*left_value);
        let right_value = f64::from(*right_value);
        dot_product += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }
    (dot_product / (left_norm.sqrt() * right_norm.sqrt())) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(
        tenant_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        embedding: &[f32],
    ) -> EmbeddedMessage {
        EmbeddedMessage::new(tenant_id, mailbox_id, message_id, embedding.to_vec())
    }

    #[test]
    fn ranks_only_messages_in_the_explicit_tenant_and_mailbox_scope() {
        let tenant_id = Uuid::new_v4();
        let allowed_mailbox = Uuid::new_v4();
        let denied_mailbox = Uuid::new_v4();
        let allowed_message = Uuid::new_v4();
        let scope = RetrievalScope::new(tenant_id, vec![allowed_mailbox]).expect("scope");
        let candidates = vec![
            message(tenant_id, allowed_mailbox, allowed_message, &[1.0, 0.0]),
            message(tenant_id, allowed_mailbox, Uuid::new_v4(), &[0.0, 1.0]),
            message(tenant_id, denied_mailbox, Uuid::new_v4(), &[0.99, 0.01]),
            message(
                Uuid::new_v4(),
                allowed_mailbox,
                Uuid::new_v4(),
                &[0.99, 0.01],
            ),
        ];

        let hits = semantic_search(&[1.0, 0.0], &scope, &candidates, 10).expect("search");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].message_id, allowed_message);
        assert!((hits[0].similarity - 1.0).abs() < f32::EPSILON);
        assert!((hits[1].similarity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn returns_no_message_content_and_uses_deterministic_tie_breaking() {
        let tenant_id = Uuid::new_v4();
        let mailbox_id = Uuid::new_v4();
        let scope = RetrievalScope::new(tenant_id, vec![mailbox_id]).expect("scope");
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let candidates = vec![
            message(tenant_id, mailbox_id, second, &[1.0, 0.0]),
            message(tenant_id, mailbox_id, first, &[1.0, 0.0]),
        ];
        let hits = semantic_search(&[1.0, 0.0], &scope, &candidates, 1).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].message_id, first);
    }

    #[test]
    fn rejects_invalid_scope_embeddings_and_result_bounds() {
        let tenant_id = Uuid::new_v4();
        let mailbox_id = Uuid::new_v4();
        let scope = RetrievalScope::new(tenant_id, vec![mailbox_id]).expect("scope");

        assert_eq!(
            RetrievalScope::new(Uuid::nil(), vec![mailbox_id]).err(),
            Some(SemanticSearchError::InvalidTenant)
        );
        assert_eq!(
            RetrievalScope::new(tenant_id, vec![]).err(),
            Some(SemanticSearchError::EmptyScope)
        );
        assert_eq!(
            RetrievalScope::new(tenant_id, vec![mailbox_id, mailbox_id]).err(),
            Some(SemanticSearchError::InvalidScope)
        );
        assert_eq!(
            semantic_search(&[], &scope, &[], 1).err(),
            Some(SemanticSearchError::InvalidDimensions)
        );
        assert_eq!(
            semantic_search(&[0.0, 0.0], &scope, &[], 1).err(),
            Some(SemanticSearchError::InvalidEmbedding)
        );
        assert_eq!(
            semantic_search(&[f32::NAN], &scope, &[], 1).err(),
            Some(SemanticSearchError::InvalidEmbedding)
        );
        assert_eq!(
            semantic_search(&[1.0], &scope, &[], 0).err(),
            Some(SemanticSearchError::InvalidResultLimit)
        );
    }

    #[test]
    fn skips_unauthorized_candidates_before_validating_their_vectors() {
        let tenant_id = Uuid::new_v4();
        let mailbox_id = Uuid::new_v4();
        let scope = RetrievalScope::new(tenant_id, vec![mailbox_id]).expect("scope");
        let candidates = vec![message(Uuid::new_v4(), mailbox_id, Uuid::new_v4(), &[])];

        assert!(
            semantic_search(&[1.0], &scope, &candidates, 1)
                .expect("unauthorized row is ignored")
                .is_empty()
        );
    }

    #[test]
    fn rejects_malformed_authorized_candidates_and_duplicate_rows() {
        let tenant_id = Uuid::new_v4();
        let mailbox_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let scope = RetrievalScope::new(tenant_id, vec![mailbox_id]).expect("scope");
        let malformed = vec![message(tenant_id, mailbox_id, message_id, &[1.0])];
        assert_eq!(
            semantic_search(&[1.0, 0.0], &scope, &malformed, 1).err(),
            Some(SemanticSearchError::DimensionMismatch)
        );

        let duplicate = vec![
            message(tenant_id, mailbox_id, message_id, &[1.0, 0.0]),
            message(tenant_id, mailbox_id, message_id, &[0.0, 1.0]),
        ];
        assert_eq!(
            semantic_search(&[1.0, 0.0], &scope, &duplicate, 2).err(),
            Some(SemanticSearchError::DuplicateCandidate)
        );
    }
}
