use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use async_trait::async_trait;
use trust_dns_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};

use crate::outcome::DeliveryError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MxCandidate {
    pub host: String,
    pub preference: u16,
    pub addresses: Vec<IpAddr>,
}

#[derive(Clone, Debug)]
pub struct DestinationPolicy {
    pub require_public: bool,
    pub allow_loopback_test: bool,
}

impl DestinationPolicy {
    pub fn allows(&self, address: IpAddr) -> bool {
        if self.allow_loopback_test && address.is_loopback() {
            return true;
        }
        if !self.require_public {
            return true;
        }
        match address {
            IpAddr::V4(ip) => is_public_v4(ip),
            IpAddr::V6(ip) => is_public_v6(ip),
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.octets()[0] == 0
        || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
        || (ip.octets()[0] == 198 && (18..=19).contains(&ip.octets()[1])))
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        || (ip.segments()[0] & 0xffc0) == 0xfe80)
}

#[async_trait]
pub trait Resolver: Send + Sync {
    async fn resolve_mx(
        &self,
        domain: &str,
        timeout: Duration,
        max_candidates: usize,
    ) -> Result<Vec<MxCandidate>, DeliveryError>;
}

#[derive(Clone)]
pub struct DnsResolver {
    resolver: TokioAsyncResolver,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            resolver: TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()),
        }
    }
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Resolver for DnsResolver {
    async fn resolve_mx(
        &self,
        domain: &str,
        timeout_duration: Duration,
        max_candidates: usize,
    ) -> Result<Vec<MxCandidate>, DeliveryError> {
        if domain.is_empty() || domain.len() > 253 || !domain.is_ascii() {
            return Err(DeliveryError::Dns);
        }
        let lookup = tokio::time::timeout(timeout_duration, self.resolver.mx_lookup(domain))
            .await
            .map_err(|_| DeliveryError::Dns)?
            .map_err(|_| DeliveryError::Dns)?;
        let mut records = lookup
            .iter()
            .map(|record| {
                (
                    record.preference(),
                    record
                        .exchange()
                        .to_utf8()
                        .trim_end_matches('.')
                        .to_ascii_lowercase(),
                )
            })
            .filter(|(_, host)| !host.is_empty() && host.len() <= 253 && host.is_ascii())
            .collect::<Vec<_>>();
        records.sort_by_key(|(preference, host)| (*preference, host.clone()));
        records.truncate(max_candidates);
        let mut candidates = Vec::new();
        for (preference, host) in records {
            let addresses = tokio::time::timeout(
                timeout_duration,
                tokio::net::lookup_host((host.as_str(), 0)),
            )
            .await
            .map_err(|_| DeliveryError::Dns)?
            .map_err(|_| DeliveryError::Dns)?
            .map(|address| address.ip())
            .collect::<Vec<_>>();
            if !addresses.is_empty() {
                candidates.push(MxCandidate {
                    host,
                    preference,
                    addresses,
                });
            }
        }
        if candidates.is_empty() {
            return Err(DeliveryError::NoCandidate);
        }
        Ok(candidates)
    }
}

pub fn recipient_domain(recipient: &str) -> Result<&str, DeliveryError> {
    let (_, domain) = recipient.rsplit_once('@').ok_or(DeliveryError::Dns)?;
    if domain.is_empty()
        || domain.len() > 253
        || !domain.is_ascii()
        || domain.contains(char::is_whitespace)
    {
        return Err(DeliveryError::Dns);
    }
    Ok(domain)
}

pub fn socket_candidates(candidate: &MxCandidate, policy: &DestinationPolicy) -> Vec<SocketAddr> {
    candidate
        .addresses
        .iter()
        .copied()
        .filter(|ip| policy.allows(*ip))
        .map(|ip| SocketAddr::new(ip, 25))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_unsafe_addresses_by_default() {
        let policy = DestinationPolicy {
            require_public: true,
            allow_loopback_test: false,
        };
        assert!(!policy.allows("127.0.0.1".parse().unwrap()));
        assert!(!policy.allows("10.0.0.1".parse().unwrap()));
        assert!(!policy.allows("169.254.1.1".parse().unwrap()));
        assert!(!policy.allows("224.0.0.1".parse().unwrap()));
        assert!(policy.allows("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn loopback_requires_explicit_fixture_mode() {
        assert!(
            DestinationPolicy {
                require_public: true,
                allow_loopback_test: true
            }
            .allows("127.0.0.1".parse().unwrap())
        );
        assert_eq!(recipient_domain("a@example.test").unwrap(), "example.test");
    }
}
