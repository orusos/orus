//! Brokered network egress with private-network denial.
//!
//! Apps cannot reach the host LAN, localhost, or link-local addresses.
//! Only public internet on ports 80/443 is allowed by default.
//! Additional destinations can be whitelisted in
//! `~/.orus/config/network.toml`.

use std::net::IpAddr;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkBrokerError {
    #[error("denied: {addr}:{port} — {reason}")]
    Denied {
        addr: IpAddr,
        port: u16,
        reason: &'static str,
    },

    #[error("port {0} not in default allow list (80, 443)")]
    PortDenied(u16),

    #[error("inbound connections are not allowed")]
    InboundDenied,

    #[error("config error: {0}")]
    Config(String),
}

/// Whitelist entry from `network.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct WhitelistEntry {
    pub addr: String,
    pub port: u16,
    #[serde(default)]
    pub comment: String,
}

/// Top-level network config loaded from `~/.orus/config/network.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub whitelist: Vec<WhitelistEntry>,
}

impl NetworkConfig {
    /// Load from a TOML file, returning default config if the file is absent.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&text).unwrap_or_default()
    }
}

/// Network broker enforcing egress policy.
#[derive(Debug)]
pub struct NetworkBroker {
    /// Default allowed ports (80, 443).
    default_ports: Vec<u16>,
    /// User-configured whitelist overrides.
    whitelist: Vec<WhitelistEntry>,
}

impl Default for NetworkBroker {
    fn default() -> Self {
        Self {
            default_ports: vec![80, 443],
            whitelist: Vec::new(),
        }
    }
}

impl NetworkBroker {
    /// Create a broker with the given config.
    pub fn with_config(config: &NetworkConfig) -> Self {
        Self {
            default_ports: vec![80, 443],
            whitelist: config.whitelist.clone(),
        }
    }

    /// Check whether an outbound connection to `addr:port` is allowed.
    pub fn check_egress(&self, addr: IpAddr, port: u16) -> Result<(), NetworkBrokerError> {
        // Check whitelist first — explicit overrides bypass all other checks
        if self.is_whitelisted(addr, port) {
            return Ok(());
        }

        // Deny private/reserved addresses
        if is_loopback(addr) {
            return Err(NetworkBrokerError::Denied {
                addr,
                port,
                reason: "loopback address",
            });
        }
        if is_private(addr) {
            return Err(NetworkBrokerError::Denied {
                addr,
                port,
                reason: "private network (RFC1918)",
            });
        }
        if is_link_local(addr) {
            return Err(NetworkBrokerError::Denied {
                addr,
                port,
                reason: "link-local address",
            });
        }
        if is_multicast(addr) {
            return Err(NetworkBrokerError::Denied {
                addr,
                port,
                reason: "multicast address",
            });
        }
        if is_unspecified(addr) {
            return Err(NetworkBrokerError::Denied {
                addr,
                port,
                reason: "unspecified address",
            });
        }

        // Check port allowlist
        if !self.default_ports.contains(&port) {
            return Err(NetworkBrokerError::PortDenied(port));
        }

        Ok(())
    }

    /// Inbound connections are always denied.
    pub fn check_inbound(&self) -> Result<(), NetworkBrokerError> {
        Err(NetworkBrokerError::InboundDenied)
    }

    fn is_whitelisted(&self, addr: IpAddr, port: u16) -> bool {
        self.whitelist.iter().any(|entry| {
            entry.port == port && entry.addr.parse::<IpAddr>().is_ok_and(|a| a == addr)
        })
    }
}

// --- IP classification helpers ---

fn is_loopback(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback(),
    }
}

fn is_private(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            // 10.0.0.0/8
            octets[0] == 10
            // 172.16.0.0/12
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            // 192.168.0.0/16
            || (octets[0] == 192 && octets[1] == 168)
        }
        IpAddr::V6(ip) => {
            // fc00::/7 — unique local addresses
            let segments = ip.segments();
            (segments[0] & 0xfe00) == 0xfc00
        }
    }
}

fn is_link_local(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            // 169.254.0.0/16
            octets[0] == 169 && octets[1] == 254
        }
        IpAddr::V6(ip) => {
            // fe80::/10
            let segments = ip.segments();
            (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

fn is_multicast(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => ip.is_multicast(),
        IpAddr::V6(ip) => ip.is_multicast(),
    }
}

fn is_unspecified(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => ip.is_unspecified(),
        IpAddr::V6(ip) => ip.is_unspecified(),
    }
}

/// Parse a string as an IP address, returning `None` for hostnames.
/// In production, DNS resolution should happen first and then the resolved
/// IP should be checked.
pub fn parse_addr(s: &str) -> Option<IpAddr> {
    s.parse::<IpAddr>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_public_http() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "93.184.216.34".parse().unwrap(); // example.com
        assert!(broker.check_egress(addr, 80).is_ok());
        assert!(broker.check_egress(addr, 443).is_ok());
    }

    #[test]
    fn deny_public_non_standard_port() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "93.184.216.34".parse().unwrap();
        assert!(broker.check_egress(addr, 8080).is_err());
        assert!(broker.check_egress(addr, 22).is_err());
    }

    #[test]
    fn deny_localhost_v4() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(broker.check_egress(addr, 8080).is_err());
        assert!(broker.check_egress(addr, 80).is_err());
    }

    #[test]
    fn deny_localhost_v6() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "::1".parse().unwrap();
        assert!(broker.check_egress(addr, 443).is_err());
    }

    #[test]
    fn deny_rfc1918_10() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(broker.check_egress(addr, 443).is_err());
    }

    #[test]
    fn deny_rfc1918_172() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "172.16.0.1".parse().unwrap();
        assert!(broker.check_egress(addr, 80).is_err());
        // 172.15.x.x is NOT private
        let addr2: IpAddr = "172.15.0.1".parse().unwrap();
        assert!(broker.check_egress(addr2, 80).is_ok());
    }

    #[test]
    fn deny_rfc1918_192() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(broker.check_egress(addr, 80).is_err());
    }

    #[test]
    fn deny_link_local() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "169.254.1.1".parse().unwrap();
        assert!(broker.check_egress(addr, 80).is_err());
    }

    #[test]
    fn deny_multicast() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "224.0.0.1".parse().unwrap();
        assert!(broker.check_egress(addr, 80).is_err());
    }

    #[test]
    fn deny_unspecified() {
        let broker = NetworkBroker::default();
        let v4: IpAddr = "0.0.0.0".parse().unwrap();
        let v6: IpAddr = "::".parse().unwrap();
        assert!(broker.check_egress(v4, 80).is_err());
        assert!(broker.check_egress(v6, 80).is_err());
    }

    #[test]
    fn deny_inbound() {
        let broker = NetworkBroker::default();
        assert!(broker.check_inbound().is_err());
    }

    #[test]
    fn whitelist_overrides_deny() {
        let config = NetworkConfig {
            whitelist: vec![WhitelistEntry {
                addr: "127.0.0.1".to_owned(),
                port: 8080,
                comment: "local dev server".to_owned(),
            }],
        };
        let broker = NetworkBroker::with_config(&config);
        let addr: IpAddr = "127.0.0.1".parse().unwrap();
        // Whitelisted — allowed
        assert!(broker.check_egress(addr, 8080).is_ok());
        // Different port — still denied
        assert!(broker.check_egress(addr, 9090).is_err());
    }

    #[test]
    fn whitelist_overrides_port_restriction() {
        let config = NetworkConfig {
            whitelist: vec![WhitelistEntry {
                addr: "93.184.216.34".to_owned(),
                port: 8443,
                comment: "custom HTTPS".to_owned(),
            }],
        };
        let broker = NetworkBroker::with_config(&config);
        let addr: IpAddr = "93.184.216.34".parse().unwrap();
        assert!(broker.check_egress(addr, 8443).is_ok());
    }

    #[test]
    fn config_from_toml() {
        let toml = r#"
[[whitelist]]
addr = "10.0.0.5"
port = 3000
comment = "internal API"
"#;
        let config: NetworkConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.whitelist.len(), 1);
        assert_eq!(config.whitelist[0].addr, "10.0.0.5");
        assert_eq!(config.whitelist[0].port, 3000);
    }

    #[test]
    fn ipv6_private() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "fc00::1".parse().unwrap();
        assert!(broker.check_egress(addr, 443).is_err());
    }

    #[test]
    fn ipv6_link_local() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "fe80::1".parse().unwrap();
        assert!(broker.check_egress(addr, 443).is_err());
    }

    #[test]
    fn ipv6_public_allowed() {
        let broker = NetworkBroker::default();
        let addr: IpAddr = "2606:4700:4700::1111".parse().unwrap(); // Cloudflare DNS
        assert!(broker.check_egress(addr, 443).is_ok());
    }
}
