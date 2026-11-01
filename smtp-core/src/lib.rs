pub mod ai;
pub mod attachments;
pub mod authentication;
pub mod delivery;
pub mod events;
pub mod mailbox;
pub mod message;
pub mod platform;
pub mod validation;

// Keep the original module paths available to existing service crates while
// the implementation is organized under feature-focused directories.
pub use ai::{features as ai_features, retrieval as ai_retrieval, suggestions as ai_suggestions};
pub use authentication::{arc, bimi, pipeline as authentication_pipeline};
pub use delivery::dsn;
pub use mailbox::{offline, rules, search, store as mailbox_store, sync, threading};
pub use message::mime;
pub use platform::{
    config, context, database, errors, logging, regional_topology, runtime, secrets,
};
pub use validation::compliance;

pub use ai::features::{EmailFeatures, extract_email_features};
pub use ai::retrieval::{
    EmbeddedMessage, RetrievalScope, SemanticHit, SemanticSearchError, semantic_search,
};
pub use ai::suggestions::{DraftSuggestion, DraftSuggestionError, DraftSuggestionKind};
pub use attachments::{
    AttachmentBlob, AttachmentError, AttachmentScanner, AttachmentStore, ScanVerdict,
};
pub use authentication::arc::{
    ArcChain, ArcError, ArcInstance, ArcResult, ArcVerifier, validate_arc_chain, verify_arc_chain,
};
pub use authentication::bimi::{
    BimiError, BimiPolicy, BimiResult, parse_bimi_record, validate_bimi_policy,
};
pub use authentication::pipeline::{
    AuthenticationDisposition, AuthenticationInput, AuthenticationReport, authenticate,
};
pub use authentication::{
    DkimCanonicalization, DkimSignature, DkimSigner, DkimVerifier, DmarcDisposition, DmarcPolicy,
    DmarcResult, DnsCache, DnsRecord, SpfMechanism, SpfResult, evaluate_dmarc, evaluate_spf,
    sign_dkim_headers, verify_dkim,
};
pub use mailbox::offline::{OfflineMailbox, OfflineOperation, OfflineRecord};
pub use mailbox::rules::{
    MailRule, RuleAction, RuleCondition, RuleContext, RuleDirection, RuleError, RuleField,
    RuleMatchOperator, RulePlan, RulePlanner, RuleSizeOperator,
};
pub use mailbox::search::{
    MailboxSearchError, MailboxSearchHit, SearchDocument, SearchIndex, SearchQuery, SearchResult,
    search_mailbox,
};
pub use mailbox::store::{
    CreateMailbox, CreateMailboxLabel, CreateSentSubmission, MailboxChange, MailboxFolder,
    MailboxKind, MailboxLabel, MailboxOutboxEvent, MailboxPermission, MailboxQuota,
    MailboxQuotaWarning, MailboxRecord, MailboxRuleRecord, MailboxStatus, MailboxStore,
    MailboxStoreError, QuarantineRequest, SentRecipientEvent,
};
pub use mailbox::sync::{ChangeKind, SyncChange, SyncLog, SyncToken};
pub use mailbox::threading::{Conversation, ThreadIndex, ThreadIndexError, ThreadMessage};
pub use mailbox::{
    MailboxError, MailboxFlag, MailboxMessage, validate_local_part, validate_mailbox_name,
    validate_message_size,
};
pub use message::mime::{
    CANONICAL_MESSAGE_SCHEMA_VERSION, CanonicalMessage, IndexedAttachmentMetadata,
    IndexedMessageMetadata, MAX_CANONICAL_MESSAGE_BYTES, MessageEnvelope, MimeError, MimeLimits,
    ParsedAttachment, ParsedMessage, parse_canonical_message, parse_message,
};
pub use platform::config::{ConfigError, Environment, PlatformConfig};
pub use platform::context::{CorrelationId, OperationContext, RequestId, TraceId};
pub use platform::database::{DatabaseConfig, DatabaseError};
pub use platform::errors::{CoreError, ErrorClass};
pub use platform::regional_topology::{
    RegionDefinition, RegionRole, RegionalService, RegionalTopology, RegionalTopologyError,
};
pub use platform::runtime::{Shutdown, ShutdownError, ShutdownOutcome, ShutdownReceiver};
pub use platform::secrets::{
    EnvSecretProvider, SecretError, SecretKey, SecretProvider, SecretValue,
};
pub use validation::compliance::{ComplianceFixture, ComplianceResult, run_compliance_fixture};

pub use delivery::dsn::{
    BounceClass, DeliveryTracker, DsnAction, DsnError, DsnReport, DsnStatus, FeedbackReport,
    FeedbackType, classify_bounce, generate_dsn, parse_delivery_status, parse_feedback_report,
};
pub use events::{EventEnvelope, EventId, EventType, ProducerIdentity, SchemaVersion};
pub use events::{EventError, EventPublisher, PublishReceipt};

#[cfg(test)]
mod tests {
    use super::config::PlatformConfig;

    #[test]
    fn core_exports_typed_configuration() {
        let config = PlatformConfig::default();
        assert_eq!(config.environment, super::Environment::Development);
    }
}
