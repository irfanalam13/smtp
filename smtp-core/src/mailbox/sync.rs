use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SyncToken(pub u64);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChangeKind {
    Upsert,
    Delete,
    Move { folder_id: Uuid },
    Flags { flags: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncChange {
    pub token: SyncToken,
    pub message_id: Uuid,
    pub kind: ChangeKind,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct SyncLog {
    next_token: u64,
    changes: Vec<SyncChange>,
}

impl SyncLog {
    pub fn append(&mut self, message_id: Uuid, kind: ChangeKind) -> SyncChange {
        self.next_token = self.next_token.saturating_add(1);
        let change = SyncChange {
            token: SyncToken(self.next_token),
            message_id,
            kind,
            occurred_at: Utc::now(),
        };
        self.changes.push(change.clone());
        change
    }

    pub fn since(&self, token: SyncToken, limit: usize) -> Vec<SyncChange> {
        self.changes
            .iter()
            .filter(|change| change.token > token)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn current_token(&self) -> SyncToken {
        SyncToken(self.next_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returns_incremental_changes() {
        let mut log = SyncLog::default();
        let id = Uuid::new_v4();
        log.append(id, ChangeKind::Upsert);
        log.append(id, ChangeKind::Delete);
        assert_eq!(log.since(SyncToken(1), 10).len(), 1);
    }
}
