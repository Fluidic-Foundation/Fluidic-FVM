use crate::crypto::{AccountId, KeyPair};
use crate::network::discovery::PeerAnnouncement;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hybrid signing key combining Ed25519 (fast, widely supported today) and
/// CRYSTALS-Dilithium (NIST PQC, quantum-resistant).
///
/// Peer announcements and bootstrap records are signed with both algorithms so
/// the network remains secure even if one primitive is broken in the future.
pub struct HybridKeypair {
    pub ed25519: KeyPair,
    pub dilithium: pqc_dilithium::Keypair,
}

/// A public hybrid identity that can verify bootstrap records.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisOperator {
    pub name: String,
    pub ed25519_public_key: [u8; 32],
    #[serde(with = "serde_base64")]
    pub dilithium_public_key: Vec<u8>,
    pub account: AccountId,
}

mod serde_base64 {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        URL_SAFE_NO_PAD.encode(v).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        URL_SAFE_NO_PAD.decode(s).map_err(serde::de::Error::custom)
    }
}

/// A signed bootstrap endpoint record: `v1|endpoint|timestamp_ns|ed25519_sig|dilithium_sig`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapRecord {
    pub version: u32,
    pub endpoint: String,
    pub timestamp_ns: u64,
    #[serde(with = "serde_base64")]
    pub ed25519_signature: Vec<u8>,
    #[serde(with = "serde_base64")]
    pub dilithium_signature: Vec<u8>,
}

impl HybridKeypair {
    pub fn generate(name: impl Into<String>) -> (Self, GenesisOperator) {
        let ed25519 = KeyPair::generate();
        let dilithium = pqc_dilithium::Keypair::generate();
        let op = GenesisOperator {
            name: name.into(),
            ed25519_public_key: ed25519.public_key().to_bytes(),
            dilithium_public_key: dilithium.public.to_vec(),
            account: ed25519.account_id(),
        };
        (
            Self {
                ed25519,
                dilithium,
            },
            op,
        )
    }

    /// Sign a bootstrap endpoint record with both algorithms.
    pub fn sign_bootstrap(
        &self,
        endpoint: impl Into<String>,
        timestamp_ns: u64,
    ) -> BootstrapRecord {
        let endpoint = endpoint.into();
        let ed25519_signature = self
            .ed25519
            .sign(&bootstrap_signing_bytes(1, &endpoint, timestamp_ns))
            .to_bytes()
            .to_vec();
        let dilithium_signature = self
            .dilithium
            .sign(&bootstrap_signing_bytes(1, &endpoint, timestamp_ns))
            .to_vec();
        BootstrapRecord {
            version: 1,
            endpoint,
            timestamp_ns,
            ed25519_signature,
            dilithium_signature,
        }
    }

    /// Sign a peer announcement with both algorithms.
    pub fn sign_peer_announcement(
        &self,
        ann: &PeerAnnouncement,
    ) -> SignedPeerAnnouncement {
        let dilithium_signature = self.dilithium.sign(&ann.signing_bytes(),
        ).to_vec();
        SignedPeerAnnouncement {
            announcement: ann.clone(),
            dilithium_signature,
        }
    }
}

/// A peer announcement augmented with a Dilithium signature for post-quantum
/// authenticity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedPeerAnnouncement {
    pub announcement: PeerAnnouncement,
    #[serde(with = "serde_base64")]
    pub dilithium_signature: Vec<u8>,
}

/// Embedded genesis operators for the Fluidic testnet.  These public keys are
/// compiled into every node binary; compromised operators can be rotated by a
/// new software release.
pub const GENESIS_OPERATORS_JSON: &str = include_str!("../../genesis/operators.json");

lazy_static::lazy_static! {
    pub static ref GENESIS_OPERATORS: Vec<GenesisOperator> = {
        serde_json::from_str(GENESIS_OPERATORS_JSON)
            .expect("compiled genesis operators must be valid JSON")
    };
    pub static ref GENESIS_BY_ACCOUNT: HashMap<AccountId, GenesisOperator> = {
        GENESIS_OPERATORS.iter().map(|op| (op.account, op.clone())).collect()
    };
}

pub fn bootstrap_signing_bytes(version: u32, endpoint: &str, timestamp_ns: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(b"FLUIDIC:BOOTSTRAP:v");
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(endpoint.as_bytes());
    bytes.extend_from_slice(&timestamp_ns.to_le_bytes());
    bytes
}

impl BootstrapRecord {
    /// Encode as a compact DNS TXT string.
    pub fn to_txt(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}",
            self.version,
            self.endpoint,
            self.timestamp_ns,
            URL_SAFE_NO_PAD.encode(&self.ed25519_signature),
            URL_SAFE_NO_PAD.encode(&self.dilithium_signature)
        )
    }

    /// Parse a compact DNS TXT string.
    pub fn from_txt(s: &str) -> Option<Self> {
        let mut parts = s.split('|');
        let version: u32 = parts.next()?.strip_prefix('v')?.parse().ok()?;
        let endpoint = parts.next()?.to_string();
        let timestamp_ns: u64 = parts.next()?.parse().ok()?;
        let ed25519_signature = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;
        let dilithium_signature = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;
        Some(Self {
            version,
            endpoint,
            timestamp_ns,
            ed25519_signature,
            dilithium_signature,
        })
    }

    /// Verify the record against the compiled genesis operator set.
    pub fn verify(&self) -> bool {
        if self.version != 1 {
            return false;
        }
        if self.ed25519_signature.len() != 64 {
            return false;
        }
        if self.dilithium_signature.len() != pqc_dilithium::SIGNBYTES {
            return false;
        }
        let msg = bootstrap_signing_bytes(self.version, &self.endpoint, self.timestamp_ns);

        for op in GENESIS_OPERATORS.iter() {
            let Ok(ed_pk) = ed25519_dalek::VerifyingKey::from_bytes(
                &op.ed25519_public_key) else { continue; };
            let Ok(ed_sig) = ed25519_dalek::Signature::from_slice(
                &self.ed25519_signature) else { return false; };
            if !KeyPair::verify(
                &ed_pk,
                &msg,
                &ed_sig,
            ) {
                continue;
            }
            if pqc_dilithium::verify(
                &self.dilithium_signature,
                &msg,
                &op.dilithium_public_key,
            )
            .is_ok()
            {
                return true;
            }
        }
        false
    }
}

impl SignedPeerAnnouncement {
    /// Verify both the embedded Ed25519 announcement signature and the
    /// Dilithium signature using the genesis operator set.
    pub fn verify(&self) -> bool {
        if !self.announcement.verify() {
            return false;
        }
        if self.dilithium_signature.len() != pqc_dilithium::SIGNBYTES {
            return false;
        }
        let Ok(ed_pk) = ed25519_dalek::VerifyingKey::from_bytes(
            &self.announcement.public_key) else {
            return false;
        };
        let operator = AccountId::from_public_key(
            &ed_pk,
        );
        let Some(op) = GENESIS_BY_ACCOUNT.get(&operator) else {
            return false;
        };
        pqc_dilithium::verify(
            &self.dilithium_signature,
            &self.announcement.signing_bytes(),
            &op.dilithium_public_key,
        )
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_record_roundtrip() {
        let (kp, _op) = HybridKeypair::generate("test");
        let record = kp.sign_bootstrap("wss://example.com/api/ws", 1_700_000_000_000_000_000);
        let txt = record.to_txt();
        let parsed = BootstrapRecord::from_txt(&txt).unwrap();
        assert_eq!(parsed.endpoint, record.endpoint);
        assert_eq!(parsed.timestamp_ns, record.timestamp_ns);
        assert!(!parsed.dilithium_signature.is_empty());
        assert!(!parsed.ed25519_signature.is_empty());
    }

    #[test]
    fn malformed_bootstrap_record_fails() {
        assert!(BootstrapRecord::from_txt("not-a-record").is_none());
    }
}
