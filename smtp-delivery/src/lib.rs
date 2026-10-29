pub mod client;
pub mod config;
pub mod dns;
pub mod framing;
pub mod outcome;
pub mod processor;
pub mod reply;

pub use client::{DeliveryReport, deliver};
pub use config::{DeliveryConfig, TlsPolicy};
pub use dns::{DestinationPolicy, DnsResolver, MxCandidate, Resolver};
pub use outcome::{DeliveryError, DeliveryOutcome};
