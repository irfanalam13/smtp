use std::collections::HashMap;
use uuid::Uuid;

use crate::sync::{ChangeKind, SyncChange, SyncToken};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineRecord {
    pub message_id: Uuid,
    pub raw_message: Vec<u8>,
    pub dirty: bool,
    pub folder_id: Option<Uuid>,
    pub flags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfflineOperation {
    Upsert(OfflineRecord),
    Delete(Uuid),
}

#[derive(Clone, Debug, Default)]
pub struct OfflineMailbox {
    records: HashMap<Uuid, OfflineRecord>,
    pending: Vec<OfflineOperation>,
    cursor: SyncToken,
}

impl OfflineMailbox {
    pub fn apply_server_changes(&mut self, changes: &[SyncChange]) {
        for change in changes {
            let record_is_clean = self
                .records
                .get(&change.message_id)
                .is_some_and(|record| !record.dirty);
            if record_is_clean {
                match &change.kind {
                    ChangeKind::Delete => {
                        self.records.remove(&change.message_id);
                    }
                    ChangeKind::Move { folder_id } => {
                        if let Some(record) = self.records.get_mut(&change.message_id) {
                            record.folder_id = Some(*folder_id);
                        }
                    }
                    ChangeKind::Flags { flags } => {
                        if let Some(record) = self.records.get_mut(&change.message_id) {
                            record.flags.clone_from(flags);
                        }
                    }
                    ChangeKind::Upsert => {}
                }
            }
            self.cursor = self.cursor.max(change.token);
        }
    }

    pub fn queue_upsert(&mut self, record: OfflineRecord) {
        self.records.insert(record.message_id, record.clone());
        self.pending.push(OfflineOperation::Upsert(record));
    }

    pub fn queue_delete(&mut self, message_id: Uuid) {
        self.records.remove(&message_id);
        self.pending.push(OfflineOperation::Delete(message_id));
    }

    pub fn pending(&self) -> &[OfflineOperation] {
        &self.pending
    }
    pub fn cursor(&self) -> SyncToken {
        self.cursor
    }
    pub fn acknowledge(&mut self) {
        self.pending.clear();
    }

    pub fn acknowledge_through(&mut self, operation_count: usize) {
        let acknowledged = operation_count.min(self.pending.len());
        self.pending.drain(..acknowledged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    #[test]
    fn queues_local_mutations_until_acknowledged() {
        let mut cache = OfflineMailbox::default();
        cache.queue_delete(Uuid::new_v4());
        assert_eq!(cache.pending().len(), 1);
        cache.acknowledge_through(1);
        assert!(cache.pending().is_empty());
    }

    #[test]
    fn applies_remote_metadata_without_overwriting_dirty_records() {
        let message_id = Uuid::new_v4();
        let folder_id = Uuid::new_v4();
        let mut cache = OfflineMailbox::default();
        cache.queue_upsert(OfflineRecord {
            message_id,
            raw_message: b"local".to_vec(),
            dirty: true,
            folder_id: None,
            flags: vec![],
        });
        cache.apply_server_changes(&[
            SyncChange {
                token: SyncToken(1),
                message_id,
                kind: ChangeKind::Move { folder_id },
                occurred_at: Utc::now(),
            },
            SyncChange {
                token: SyncToken(2),
                message_id,
                kind: ChangeKind::Delete,
                occurred_at: Utc::now(),
            },
        ]);
        let local = cache
            .records
            .get(&message_id)
            .expect("preserve dirty record");
        assert_eq!(local.raw_message, b"local");
        assert_eq!(local.folder_id, None);
        assert_eq!(cache.cursor(), SyncToken(2));
    }

    #[test]
    fn applies_server_flags_and_moves_to_clean_records() {
        let message_id = Uuid::new_v4();
        let folder_id = Uuid::new_v4();
        let mut cache = OfflineMailbox::default();
        cache.records.insert(
            message_id,
            OfflineRecord {
                message_id,
                raw_message: b"server".to_vec(),
                dirty: false,
                folder_id: None,
                flags: vec![],
            },
        );
        cache.apply_server_changes(&[
            SyncChange {
                token: SyncToken(1),
                message_id,
                kind: ChangeKind::Move { folder_id },
                occurred_at: Utc::now(),
            },
            SyncChange {
                token: SyncToken(2),
                message_id,
                kind: ChangeKind::Flags {
                    flags: vec!["\\Seen".into(), "\\Flagged".into()],
                },
                occurred_at: Utc::now(),
            },
        ]);
        let remote = cache.records.get(&message_id).expect("cached record");
        assert_eq!(remote.folder_id, Some(folder_id));
        assert_eq!(remote.flags, ["\\Seen", "\\Flagged"]);
    }

    #[test]
    fn acknowledging_sent_prefix_preserves_new_operations() {
        let mut cache = OfflineMailbox::default();
        cache.queue_delete(Uuid::new_v4());
        let sent_count = cache.pending().len();
        cache.queue_delete(Uuid::new_v4());
        cache.acknowledge_through(sent_count);
        assert_eq!(cache.pending().len(), 1);
    }
}
