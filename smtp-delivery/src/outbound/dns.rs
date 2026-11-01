use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use async_trait::async_trait;
use hickory_resolver::{
    TokioResolver,
    config::ResolverConfig,
    net::{DnsError, NetError, runtime::TokioRuntimeProvider},
    proto::rr::RData,
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
            return false;
        }
        match address {
            IpAddr::V4(ip) => is_public_v4(ip),
            IpAddr::V6(ip) => is_public_v6(ip),
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !(octets[0] == 0
        || ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192
            && ((octets[1] == 0 && octets[2] == 0)
                || (octets[1] == 0 && octets[2] == 2)
                || (octets[1] == 88 && octets[2] == 99)))
        || (octets[0] == 198
            && ((18..=19).contains(&octets[1]) || (octets[1] == 51 && octets[2] == 100)))
        || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        || octets[0] >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let global_unicast = (segments[0] & 0xe000) == 0x2000;
    !(!global_unicast
        || ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xff00) == 0xff00
        || (segments[0] == 0x2001 && segments[1] <= 0x01ff)
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || segments[0] == 0x2002)
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
    resolver: TokioResolver,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            resolver: TokioResolver::builder_with_config(
                ResolverConfig::default(),
                TokioRuntimeProvider::default(),
            )
            .build()
            .expect("default DNS resolver configuration is valid"),
        }
    }

    pub fn try_from_system_conf() -> Result<Self, DeliveryError> {
        let resolver = TokioResolver::builder_tokio()
            .map_err(|_| DeliveryError::Dns)?
            .build()
            .map_err(|_| DeliveryError::Dns)?;
        Ok(Self { resolver })
    }

    async fn resolve_addresses(
        &self,
        host: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<IpAddr>, DeliveryError> {
        tokio::time::timeout(timeout_duration, self.resolver.lookup_ip(host))
            .await
            .map_err(|_| DeliveryError::Dns)?
            .map(|lookup| lookup.iter().collect())
            .map_err(|_| DeliveryError::Dns)
    }

    async fn resolve_implicit_mx(
        &self,
        domain: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<MxCandidate>, DeliveryError> {
        let addresses = self
            .resolve_addresses(domain, timeout_duration)
            .await
            .map_err(|error| match error {
                DeliveryError::Dns => DeliveryError::NoCandidate,
                other => other,
            })?;
        if addresses.is_empty() {
            return Err(DeliveryError::NoCandidate);
        }
        Ok(vec![MxCandidate {
            host: domain.to_ascii_lowercase(),
            preference: 0,
            addresses,
        }])
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
        if !valid_dns_name(domain) || max_candidates == 0 {
            return Err(DeliveryError::Dns);
        }
        let lookup = tokio::time::timeout(timeout_duration, self.resolver.mx_lookup(domain))
            .await
            .map_err(|_| DeliveryError::Dns)?;
        let lookup = match lookup {
            Ok(lookup) => lookup,
            Err(NetError::Dns(DnsError::NoRecordsFound(_))) => {
                return self.resolve_implicit_mx(domain, timeout_duration).await;
            }
            Err(_) => return Err(DeliveryError::Dns),
        };
        let mut records = Vec::new();
        for record in lookup.answers() {
            let RData::MX(mx) = &record.data else {
                continue;
            };
            let exchange = mx.exchange.to_utf8();
            if exchange == "." {
                return Err(DeliveryError::NullMx);
            }
            let host = exchange.trim_end_matches('.').to_ascii_lowercase();
            if valid_dns_name(&host) {
                records.push((mx.preference, host));
            }
        }
        if records.is_empty() {
            return self.resolve_implicit_mx(domain, timeout_duration).await;
        }
        records.sort_by_key(|(preference, host)| (*preference, host.clone()));
        records.truncate(max_candidates);
        let mut candidates = Vec::new();
        for (preference, host) in records {
            let addresses = self.resolve_addresses(&host, timeout_duration).await?;
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
    let (local, domain) = recipient.rsplit_once('@').ok_or(DeliveryError::Dns)?;
    if recipient.len() > 254 || !valid_dot_atom(local) || !valid_dns_name(domain) {
        return Err(DeliveryError::Dns);
    }
    Ok(domain)
}

fn valid_dot_atom(local: &str) -> bool {
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'/'
                        | b'='
                        | b'?'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'{'
                        | b'|'
                        | b'}'
                        | b'~'
                        | b'.'
                )
        })
}

pub(crate) fn valid_dns_name(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 || !domain.is_ascii() {
        return false;
    }
    let normalized = domain.strip_suffix('.').unwrap_or(domain);
    !normalized.is_empty()
        && normalized.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
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
        for address in [
            "192.0.2.1",
            "192.88.99.1",
            "198.51.100.1",
            "203.0.113.1",
            "240.0.0.1",
        ] {
            assert!(
                !policy.allows(address.parse().unwrap()),
                "allowed {address}"
            );
        }
        assert!(policy.allows("8.8.8.8".parse().unwrap()));
        assert!(policy.allows("2001:4860:4860::8888".parse().unwrap()));
        for address in [
            "::ffff:8.8.8.8",
            "2001:db8::1",
            "2001::1",
            "2002:0808:0808::1",
            "2001:4860::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            if address == "2001:4860::1" {
                assert!(policy.allows(address.parse().unwrap()));
            } else {
                assert!(
                    !policy.allows(address.parse().unwrap()),
                    "allowed {address}"
                );
            }
        }
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
        assert!(
            !DestinationPolicy {
                require_public: false,
                allow_loopback_test: true
            }
            .allows("10.0.0.1".parse().unwrap())
        );
        assert_eq!(recipient_domain("a@example.test").unwrap(), "example.test");
        assert!(recipient_domain("a@bad..example").is_err());
        assert!(recipient_domain("a@example.test\r\nRCPT TO:<evil>").is_err());
    }
}
