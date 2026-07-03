use crate::crypto::{AccountId, KeyPair};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A signed advertisement of a reachable Fluidic gossip endpoint.
///
/// Endpoints may be raw TCP (`tcp://host:7000`) for publicly reachable mesh
/// nodes, or WebSocket (`wss://host/api/ws`) for user-run leaf nodes behind
/// NAT.  Each announcement is signed by the operator so nodes can verify it
/// without trusting the transport that delivered it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    /// Reachable endpoint, e.g. `tcp://1.2.3.4:7000` or `wss://host/api/ws`.
    pub endpoint: String,
    /// Operator account identity derived from `public_key`.
    pub operator: AccountId,
    /// Ed25519 public key used to verify `signature`.
    pub public_key: [u8; 32],
    /// Nanosecond timestamp when the announcement was created.
    pub timestamp_ns: u64,
    /// Remaining hops this announcement may travel.  Decremented on every
    /// forward; zero-TTL announcements are still valid locally but not relayed.
    pub ttl: u32,
    /// Ed25519 signature over `signing_bytes()`.
    pub signature: Vec<u8>,
}

impl PeerAnnouncement {
    /// Build an announcement and sign it with the operator's keypair.
    pub fn sign(
        keypair: &KeyPair,
        endpoint: impl Into<String>,
        timestamp_ns: u64,
        ttl: u32,
    ) -> Self {
        let operator = keypair.account_id();
        let public_key = keypair.public_key().to_bytes();
        let mut ann = Self {
            endpoint: endpoint.into(),
            operator,
            public_key,
            timestamp_ns,
            ttl,
            signature: Vec::new(),
        };
        let sig = keypair.sign(&ann.signing_bytes());
        ann.signature = sig.to_bytes().to_vec();
        ann
    }

    /// Canonical bytes covered by the operator signature.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"FLUIDIC:PEER:v1");
        bytes.extend_from_slice(self.endpoint.as_bytes());
        bytes.extend_from_slice(&self.operator.0);
        bytes.extend_from_slice(&self.public_key);
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes.extend_from_slice(&self.ttl.to_le_bytes());
        bytes
    }

    /// Verify the operator signature.  Does **not** check freshness or TTL.
    pub fn verify(&self) -> bool {
        let Ok(vk) = VerifyingKey::from_bytes(&self.public_key) else {
            return false;
        };
        if AccountId::from_public_key(&vk) != self.operator {
            return false;
        }
        let Ok(sig) = Signature::from_slice(&self.signature) else {
            return false;
        };
        KeyPair::verify(&vk, &self.signing_bytes(), &sig)
    }

    /// Deterministic content hash, useful for deduplication.
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"fluidic:peer-announcement:v1");
        hasher.update(&self.signing_bytes());
        hasher.update(&self.signature);
        hasher.finalize().into()
    }

    /// True if the announcement is not older than `max_age_ns`.
    pub fn is_fresh(&self, now_ns: u64, max_age_ns: u64) -> bool {
        now_ns.saturating_sub(self.timestamp_ns) <= max_age_ns
    }
}

/// Schemes supported for gossip endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointScheme {
    Tcp,
    Ws,
    Wss,
}

impl EndpointScheme {
    /// Parse the scheme prefix of an endpoint string.
    pub fn parse(endpoint: &str) -> Option<(Self, &str)> {
        if let Some(rest) = endpoint.strip_prefix("tcp://") {
            return Some((Self::Tcp, rest));
        }
        if let Some(rest) = endpoint.strip_prefix("wss://") {
            return Some((Self::Wss, rest));
        }
        if let Some(rest) = endpoint.strip_prefix("ws://") {
            return Some((Self::Ws, rest));
        }
        None
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EndpointScheme::Tcp => "tcp",
            EndpointScheme::Ws => "ws",
            EndpointScheme::Wss => "wss",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_roundtrip() {
        let kp = KeyPair::generate();
        let ann = PeerAnnouncement::sign(&kp, "tcp://1.2.3.4:7000", 1_700_000_000_000_000_000, 3);
        assert!(ann.verify());
    }

    #[test]
    fn wrong_key_fails() {
        let kp = KeyPair::generate();
        let mut ann = PeerAnnouncement::sign(&kp, "wss://example.com/api/ws", 1_700_000_000_000_000_000, 3);
        let other = KeyPair::generate();
        ann.public_key = other.public_key().to_bytes();
        ann.operator = other.account_id();
        assert!(!ann.verify());
    }

    #[test]
    fn tamper_endpoint_fails() {
        let kp = KeyPair::generate();
        let mut ann = PeerAnnouncement::sign(&kp, "tcp://1.2.3.4:7000", 1_700_000_000_000_000_000, 3);
        ann.endpoint = "tcp://5.6.7.8:7000".to_string();
        assert!(!ann.verify());
    }

    #[test]
    fn freshness() {
        let kp = KeyPair::generate();
        let ann = PeerAnnouncement::sign(&kp, "tcp://1.2.3.4:7000", 1_700_000_000_000_000_000, 3);
        assert!(ann.is_fresh(1_700_000_000_000_000_000, 86_400_000_000_000));
        assert!(!ann.is_fresh(1_700_100_000_000_000_000, 86_400_000_000_000));
    }

    #[test]
    fn parse_schemes() {
        assert_eq!(EndpointScheme::parse("tcp://1.2.3.4:7000").map(|(s, _)| s), Some(EndpointScheme::Tcp));
        assert_eq!(EndpointScheme::parse("wss://h/api/ws").map(|(s, _)| s), Some(EndpointScheme::Wss));
        assert!(EndpointScheme::parse("http://h").is_none());
    }
}
