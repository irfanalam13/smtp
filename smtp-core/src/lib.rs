pub mod ai_features;
pub mod ai_retrieval;
pub mod ai_suggestions;
pub mod arc;
pub mod attachments;
pub mod authentication;
pub mod authentication_pipeline;
pub mod bimi;
pub mod compliance;
pub mod config;
pub mod context;
pub mod database;
pub mod dsn;
pub mod errors;
pub mod events;
pub mod logging;
pub mod mailbox;
pub mod mailbox_store;
pub mod mime;
pub mod offline;
pub mod regional_topology;
pub mod rules;
pub mod runtime;
pub mod search;
pub mod secrets;
pub mod sync;
pub mod threading;

pub use ai_features::{EmailFeatures, extract_email_features};
pub use ai_retrieval::{
    EmbeddedMessage, RetrievalScope, SemanticHit, SemanticSearchError, semantic_search,
};
pub use ai_suggestions::{DraftSuggestion, DraftSuggestionError, DraftSuggestionKind};
pub use arc::{
    ArcChain, ArcError, ArcInstance, ArcResult, ArcVerifier, validate_arc_chain, verify_arc_chain,
};
pub use attachments::{
    AttachmentBlob, AttachmentError, AttachmentScanner, AttachmentStore, ScanVerdict,
};
pub use authentication::{
    DkimCanonicalization, DkimSignature, DkimSigner, DkimVerifier, DmarcDisposition, DmarcPolicy,
    DmarcResult, DnsCache, DnsRecord, SpfMechanism, SpfResult, evaluate_dmarc, evaluate_spf,
    sign_dkim_headers, verify_dkim,
};
pub use authentication_pipeline::{
    AuthenticationDisposition, AuthenticationInput, AuthenticationReport, authenticate,
};
pub use bimi::{BimiError, BimiPolicy, BimiResult, parse_bimi_record, validate_bimi_policy};
pub use compliance::{ComplianceFixture, ComplianceResult, run_compliance_fixture};
pub use config::{ConfigError, Environment, PlatformConfig};
pub use context::{CorrelationId, OperationContext, RequestId, TraceId};
pub use database::{DatabaseConfig, DatabaseError};
pub use dsn::{
    BounceClass, DeliveryTracker, DsnAction, DsnError, DsnReport, DsnStatus, FeedbackReport,
    FeedbackType, classify_bounce, generate_dsn, parse_feedback_report,
};
pub use errors::{CoreError, ErrorClass};
pub use events::{EventEnvelope, EventId, EventType, ProducerIdentity, SchemaVersion};
pub use events::{EventError, EventPublisher, PublishReceipt};
pub use mailbox::{
    MailboxError, MailboxFlag, MailboxMessage, validate_local_part, validate_mailbox_name,
    validate_message_size,
};
pub use mailbox_store::{
    CreateMailbox, CreateMailboxLabel, CreateSentSubmission, MailboxChange, MailboxFolder,
    MailboxKind, MailboxLabel, MailboxOutboxEvent, MailboxPermission, MailboxQuota,
    MailboxQuotaWarning, MailboxRecord, MailboxRuleRecord, MailboxStatus, MailboxStore,
    MailboxStoreError, QuarantineRequest, SentRecipientEvent,
};
pub use mime::{MimeError, MimeLimits, ParsedAttachment, ParsedMessage, parse_message};
pub use offline::{OfflineMailbox, OfflineOperation, OfflineRecord};
pub use regional_topology::{
    RegionDefinition, RegionRole, RegionalService, RegionalTopology, RegionalTopologyError,
};
pub use rules::{
    MailRule, RuleAction, RuleCondition, RuleContext, RuleDirection, RuleError, RuleField,
    RuleMatchOperator, RulePlan, RulePlanner, RuleSizeOperator,
};
pub use runtime::{Shutdown, ShutdownError, ShutdownOutcome, ShutdownReceiver};
pub use search::{
    MailboxSearchError, MailboxSearchHit, SearchDocument, SearchIndex, SearchQuery, SearchResult,
    search_mailbox,
};
pub use secrets::{EnvSecretProvider, SecretError, SecretKey, SecretProvider, SecretValue};
pub use sync::{ChangeKind, SyncChange, SyncLog, SyncToken};
pub use threading::{Conversation, ThreadIndex, ThreadIndexError, ThreadMessage};

#[cfg(test)]
mod tests {
    use super::config::PlatformConfig;

    #[test]
    fn core_exports_typed_configuration() {
        let config = PlatformConfig::default();
        assert_eq!(config.environment, super::Environment::Development);
    }
}
