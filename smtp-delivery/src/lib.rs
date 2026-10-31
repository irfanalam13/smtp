pub mod dkim;
pub mod dmarc;
pub mod outbound;

// Keep the established public module paths while grouping implementations by
// delivery capability.
pub use dmarc::{
    aggregate_report as dmarc_report, ingest as dmarc_report_ingest, worker as dmarc_report_worker,
};
pub use outbound::{client, config, dns, framing, outcome, processor, reply, worker};

pub use outbound::client::{
    DeliveryReport, RecipientDeliveryReport, RecipientDeliveryStatus, deliver,
};
pub use outbound::config::{DeliveryConfig, TlsPolicy};
pub use outbound::dns::{DestinationPolicy, DnsResolver, MxCandidate, Resolver};
pub use outbound::outcome::{DeliveryError, DeliveryOutcome};
pub use outbound::processor::SmtpProcessor;
