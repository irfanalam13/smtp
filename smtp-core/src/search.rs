use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchDocument {
    pub message_id: Uuid,
    pub folder_id: Uuid,
    pub subject: String,
    pub sender: String,
    pub recipients: Vec<String>,
    pub body: String,
    pub attachment_names: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub attachment_name: Option<String>,
    pub has_attachment: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    pub message_id: Uuid,
    pub score: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailboxSearchHit {
    pub message_id: Uuid,
    pub folder_id: Uuid,
    pub uid: u64,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub internal_date: DateTime<Utc>,
    pub rank: f32,
}

#[derive(Debug, Error)]
pub enum MailboxSearchError {
    #[error("search query must contain 1 to 512 characters")]
    InvalidQuery,
    #[error("search page limit must be from 1 to 100")]
    InvalidLimit,
    #[error("fuzzy search requires at least three non-space characters")]
    FuzzyQueryTooShort,
    #[error("mailbox UID returned by the database is invalid")]
    InvalidUid,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub async fn search_mailbox(
    pool: &PgPool,
    tenant_id: Uuid,
    mailbox_id: Uuid,
    query: &str,
    fuzzy: bool,
    limit: u16,
    offset: u32,
) -> Result<Vec<MailboxSearchHit>, MailboxSearchError> {
    if query.trim().is_empty() || query.chars().count() > 512 {
        return Err(MailboxSearchError::InvalidQuery);
    }
    if !(1..=100).contains(&limit) {
        return Err(MailboxSearchError::InvalidLimit);
    }
    if fuzzy
        && query
            .chars()
            .filter(|character| !character.is_whitespace())
            .count()
            < 3
    {
        return Err(MailboxSearchError::FuzzyQueryTooShort);
    }

    let rows = sqlx::query(
        "WITH criteria AS (
             SELECT websearch_to_tsquery('simple', $3) AS terms
         )
         SELECT message.id, message.folder_id, message.uid, message.subject,
                message.sender_address, message.internal_date,
                ts_rank_cd(message.search_vector, criteria.terms) AS rank
         FROM mailbox_messages AS message
         JOIN mailbox_folders AS folder
           ON folder.id = message.folder_id AND folder.mailbox_id = $2
         JOIN mailboxes AS mailbox
           ON mailbox.id = folder.mailbox_id AND mailbox.tenant_id = $1
         CROSS JOIN criteria
         WHERE message.deleted_at IS NULL
           AND (
               message.search_vector @@ criteria.terms
               OR ($4 AND (
                   message.subject % $3
                   OR message.sender_address % $3
               ))
           )
         ORDER BY rank DESC, message.internal_date DESC, message.uid DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(tenant_id)
    .bind(mailbox_id)
    .bind(query)
    .bind(fuzzy)
    .bind(i64::from(limit))
    .bind(i64::from(offset))
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|row| {
            let uid: i64 = row.try_get("uid")?;
            Ok(MailboxSearchHit {
                message_id: row.try_get("id")?,
                folder_id: row.try_get("folder_id")?,
                uid: u64::try_from(uid).map_err(|_| MailboxSearchError::InvalidUid)?,
                subject: row.try_get("subject")?,
                sender: row.try_get("sender_address")?,
                internal_date: row.try_get("internal_date")?,
                rank: row.try_get("rank")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod mailbox_search_tests {
    use super::*;

    #[tokio::test]
    async fn query_limits_fail_before_database_access() {
        let pool = PgPool::connect_lazy("postgres://unused:unused@localhost/unused").unwrap();
        assert!(matches!(
            search_mailbox(&pool, Uuid::new_v4(), Uuid::new_v4(), "   ", false, 10, 0).await,
            Err(MailboxSearchError::InvalidQuery)
        ));
        assert!(matches!(
            search_mailbox(
                &pool,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "query",
                false,
                101,
                0
            )
            .await,
            Err(MailboxSearchError::InvalidLimit)
        ));
        assert!(matches!(
            search_mailbox(&pool, Uuid::new_v4(), Uuid::new_v4(), "ab", true, 10, 0).await,
            Err(MailboxSearchError::FuzzyQueryTooShort)
        ));
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchIndex {
    documents: HashMap<Uuid, SearchDocument>,
}

impl SearchIndex {
    pub fn upsert(&mut self, document: SearchDocument) {
        self.documents.insert(document.message_id, document);
    }

    pub fn remove(&mut self, message_id: Uuid) -> bool {
        self.documents.remove(&message_id).is_some()
    }

    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let mut results: Vec<_> = self
            .documents
            .values()
            .filter_map(|document| {
                if query.from.as_ref().is_some_and(|value| {
                    !document
                        .sender
                        .to_ascii_lowercase()
                        .contains(&value.to_ascii_lowercase())
                }) || query.subject.as_ref().is_some_and(|value| {
                    !document
                        .subject
                        .to_ascii_lowercase()
                        .contains(&value.to_ascii_lowercase())
                }) || query.attachment_name.as_ref().is_some_and(|value| {
                    !document.attachment_names.iter().any(|name| {
                        name.to_ascii_lowercase()
                            .contains(&value.to_ascii_lowercase())
                    })
                }) || query
                    .has_attachment
                    .is_some_and(|expected| expected != !document.attachment_names.is_empty())
                {
                    return None;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    document.subject,
                    document.sender,
                    document.recipients.join(" "),
                    document.body
                )
                .to_ascii_lowercase();
                let score = query
                    .text
                    .as_ref()
                    .map(|text| {
                        text.split_whitespace()
                            .filter(|term| haystack.contains(&term.to_ascii_lowercase()))
                            .count() as u32
                    })
                    .unwrap_or(1);
                if query.text.is_some() && score == 0 {
                    None
                } else {
                    Some(SearchResult {
                        message_id: document.message_id,
                        score,
                    })
                }
            })
            .collect();
        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then(left.message_id.cmp(&right.message_id))
        });
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_body_metadata_and_attachments() {
        let id = Uuid::new_v4();
        let mut index = SearchIndex::default();
        index.upsert(SearchDocument {
            message_id: id,
            subject: "Quarterly report".into(),
            sender: "alice@example.test".into(),
            body: "Revenue increased".into(),
            attachment_names: vec!["report.pdf".into()],
            ..Default::default()
        });
        assert_eq!(
            index.search(&SearchQuery {
                attachment_name: Some("PDF".into()),
                ..Default::default()
            })[0]
                .message_id,
            id
        );
    }
}
