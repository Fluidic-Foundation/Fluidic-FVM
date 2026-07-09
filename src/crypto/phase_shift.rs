use crate::crypto::keys::{AccountId, KeyPair};
use crate::field::coordinates::Coordinate;
use crate::network::discovery::PeerAnnouncement;
use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 32-byte identifier for a Concurrency Domain.
pub type DomainId = [u8; 32];

/// 32-byte identifier for a liquidity pool or throughput channel.
pub type PoolId = [u8; 32];

/// 32-byte BLAKE3 hash identifying a Signal.
pub type TxHash = [u8; 32];

/// 32-byte identifier for an oscillator node.
pub type OscillatorId = [u8; 32];

/// Default domain used by the WAVE/USDC DEX pool in the testnet.
pub const DEFAULT_DEX_DOMAIN: DomainId = [0x44, 0x45, 0x58, 0x5f, 0x57, 0x41, 0x56, 0x45,
                                          0x5f, 0x55, 0x53, 0x44, 0x43, 0x00, 0x00, 0x00,
                                          0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                          0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

fn default_domain() -> DomainId {
    DEFAULT_DEX_DOMAIN
}

/// Vector clock captures the happens-before relation across oscillator nodes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub BTreeMap<OscillatorId, u64>);

impl VectorClock {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn tick(&mut self, node: OscillatorId) -> u64 {
        let entry = self.0.entry(node).or_insert(0);
        *entry += 1;
        *entry
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (node, time) in &other.0 {
            let entry = self.0.entry(*node).or_insert(0);
            *entry = (*entry).max(*time);
        }
    }

    pub fn get(&self, node: &OscillatorId) -> u64 {
        self.0.get(node).copied().unwrap_or(0)
    }

    /// Returns true if `self` happened before `other` (strict causal precedence).
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        let all_leq = self
            .0
            .iter()
            .all(|(k, v)| other.0.get(k).unwrap_or(&0) >= v);
        let strictly_less = other
            .0
            .iter()
            .any(|(k, v)| *self.0.get(k).unwrap_or(&0) < *v);
        all_leq && strictly_less
    }

    /// Returns true if the two vector clocks are concurrent (neither causally precedes the other).
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happened_before(other) && !other.happened_before(self)
    }
}

/// A state-independent, commutative Signal payload that can be safely aggregated via NTT.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommutativeShift {
    #[serde(default = "default_domain")]
    pub domain: DomainId,
    pub from: AccountId,
    pub coordinate: Coordinate,
    pub delta: i128,
    pub pool_id: PoolId,
    pub nonce: u64,
    pub timestamp_ns: u64,
    #[serde(default)]
    pub first_seen_at_ns: u64,
    pub signature: Vec<u8>,
}

impl CommutativeShift {
    pub fn new(
        keypair: &KeyPair,
        domain: DomainId,
        coordinate: Coordinate,
        delta: i128,
        pool_id: PoolId,
        nonce: u64,
        timestamp_ns: u64,
    ) -> Self {
        let from = keypair.account_id();
        let mut shift = Self {
            domain,
            from,
            coordinate,
            delta,
            pool_id,
            nonce,
            timestamp_ns,
            first_seen_at_ns: 0,
            signature: Vec::new(),
        };
        let sig = keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(b"FLUIDIC:COMMUTATIVE:v3");
        buf.extend_from_slice(&self.domain);
        buf.extend_from_slice(self.from.as_bytes());
        buf.extend_from_slice(&self.coordinate.to_bytes());
        buf.extend_from_slice(&self.delta.to_le_bytes());
        buf.extend_from_slice(&self.pool_id);
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf
    }

    pub fn hash(&self) -> TxHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.signing_bytes());
        hasher.update(&self.signature);
        hasher.finalize().into()
    }

    pub fn verify(&self, public_key: &ed25519_dalek::VerifyingKey) -> bool {
        let Ok(sig) = Signature::from_slice(&self.signature) else {
            return false;
        };
        if !KeyPair::verify(public_key, &self.signing_bytes(), &sig) {
            return false;
        }
        AccountId::from_public_key(public_key) == self.from
    }
}

/// A state-dependent Signal payload that must be ordered by the vector-clock DAG
/// before it can be applied to the wave-field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatefulShift {
    #[serde(default = "default_domain")]
    pub domain: DomainId,
    pub from: AccountId,
    pub to: AccountId,
    pub amount: u128,
    pub vector_clock: VectorClock,
    pub predecessors: Vec<TxHash>,
    pub nonce: u64,
    pub timestamp_ns: u64,
    #[serde(default)]
    pub first_seen_at_ns: u64,
    pub signature: Vec<u8>,
}

impl StatefulShift {
    pub fn new(
        keypair: &KeyPair,
        domain: DomainId,
        to: AccountId,
        amount: u128,
        vector_clock: VectorClock,
        predecessors: Vec<TxHash>,
        nonce: u64,
        timestamp_ns: u64,
    ) -> Self {
        let from = keypair.account_id();
        let mut shift = Self {
            domain,
            from,
            to,
            amount,
            vector_clock,
            predecessors,
            nonce,
            timestamp_ns,
            first_seen_at_ns: 0,
            signature: Vec::new(),
        };
        let sig = keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(b"FLUIDIC:STATEFUL:v2");
        buf.extend_from_slice(&self.domain);
        buf.extend_from_slice(self.from.as_bytes());
        buf.extend_from_slice(self.to.as_bytes());
        buf.extend_from_slice(&self.amount.to_le_bytes());
        // Vector clock is sorted by key (BTreeMap), so deterministic.
        for (node, time) in &self.vector_clock.0 {
            buf.extend_from_slice(node);
            buf.extend_from_slice(&time.to_le_bytes());
        }
        for pred in &self.predecessors {
            buf.extend_from_slice(pred);
        }
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf
    }

    pub fn hash(&self) -> TxHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.signing_bytes());
        hasher.update(&self.signature);
        hasher.finalize().into()
    }

    /// Verify that `public_key` produced a valid signature over the shift's
    /// canonical signing bytes. Does not enforce that the public key hashes to
    /// `self.from`; that check is the caller's responsibility if required.
    pub fn verify_signature(&self, public_key: &ed25519_dalek::VerifyingKey) -> bool {
        let Ok(sig) = Signature::from_slice(&self.signature) else {
            return false;
        };
        KeyPair::verify(public_key, &self.signing_bytes(), &sig)
    }

    pub fn verify(&self, public_key: &ed25519_dalek::VerifyingKey) -> bool {
        if !self.verify_signature(public_key) {
            return false;
        }
        // The signer must be the `from` account.
        AccountId::from_public_key(public_key) == self.from
    }
}

/// Registration event gossiped across the mesh so every node learns new
/// accounts and their derived token accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationShift {
    pub account: AccountId,
    pub public_key: [u8; 32],
    pub wave_account: AccountId,
    pub usdc_account: AccountId,
    pub nonce: u64,
    pub timestamp_ns: u64,
}

/// Stake event gossiped across the mesh so every node learns which operators
/// have locked collateral and are eligible to sign synthesis certificates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeShift {
    pub operator: AccountId,
    pub public_key: [u8; 32],
    pub amount: u128,
    pub nonce: u64,
    pub timestamp_ns: u64,
    pub signature: Vec<u8>,
}

impl StakeShift {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"FLUIDIC:STAKE:v1");
        bytes.extend_from_slice(&self.operator.0);
        bytes.extend_from_slice(&self.public_key);
        bytes.extend_from_slice(&self.amount.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes
    }

    pub fn sign(keypair: &KeyPair, amount: u128, nonce: u64, timestamp_ns: u64) -> Self {
        let operator = keypair.account_id();
        let mut shift = Self {
            operator,
            public_key: keypair.public_key().to_bytes(),
            amount,
            nonce,
            timestamp_ns,
            signature: Vec::new(),
        };
        let sig = keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn verify(&self) -> bool {
        let Ok(pk) = ed25519_dalek::VerifyingKey::from_bytes(&self.public_key) else {
            return false;
        };
        let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.signature) else {
            return false;
        };
        if !KeyPair::verify(&pk, &self.signing_bytes(), &sig) {
            return false;
        }
        AccountId::from_public_key(&pk) == self.operator
    }
}

/// Constraints an intent places on its execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentConstraint {
    /// Transfer at least `min_amount` to `to` before `deadline_tick`.
    Transfer { to: AccountId, min_amount: u128 },
    /// Swap `from_token` for `to_token`, receiving at least `min_out`
    /// while respecting `max_slippage_bp` basis points of slippage.
    Swap {
        from_token: AccountId,
        to_token: AccountId,
        min_out: u128,
        max_slippage_bp: u64,
    },
}

/// An intent declares a desired outcome and a reward for the solver that
/// produces a valid fill.  Intents are matched during synthesis, producing
/// ordinary stateful shifts on behalf of the owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentShift {
    pub owner: AccountId,
    pub intent_id: TxHash,
    #[serde(default = "default_domain")]
    pub domain: DomainId,
    pub deadline_tick: u64,
    pub constraint: IntentConstraint,
    pub solver_reward: u128,
    pub nonce: u64,
    pub timestamp_ns: u64,
    pub signature: Vec<u8>,
}

impl IntentShift {
    pub fn new(
        keypair: &KeyPair,
        domain: DomainId,
        deadline_tick: u64,
        constraint: IntentConstraint,
        solver_reward: u128,
        nonce: u64,
        timestamp_ns: u64,
    ) -> Self {
        let owner = keypair.account_id();
        let mut shift = Self {
            owner,
            intent_id: [0u8; 32],
            domain,
            deadline_tick,
            constraint,
            solver_reward,
            nonce,
            timestamp_ns,
            signature: Vec::new(),
        };
        shift.intent_id = shift.hash();
        let sig = keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(b"FLUIDIC:INTENT:v1");
        buf.extend_from_slice(self.owner.as_bytes());
        buf.extend_from_slice(&self.domain);
        buf.extend_from_slice(&self.deadline_tick.to_le_bytes());
        match &self.constraint {
            IntentConstraint::Transfer { to, min_amount } => {
                buf.push(0);
                buf.extend_from_slice(to.as_bytes());
                buf.extend_from_slice(&min_amount.to_le_bytes());
            }
            IntentConstraint::Swap {
                from_token,
                to_token,
                min_out,
                max_slippage_bp,
            } => {
                buf.push(1);
                buf.extend_from_slice(from_token.as_bytes());
                buf.extend_from_slice(to_token.as_bytes());
                buf.extend_from_slice(&min_out.to_le_bytes());
                buf.extend_from_slice(&max_slippage_bp.to_le_bytes());
            }
        }
        buf.extend_from_slice(&self.solver_reward.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf
    }

    pub fn hash(&self) -> TxHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.signing_bytes());
        hasher.finalize().into()
    }

    pub fn verify(&self, public_key: &ed25519_dalek::VerifyingKey) -> bool {
        let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.signature) else {
            return false;
        };
        if !KeyPair::verify(public_key, &self.signing_bytes(), &sig) {
            return false;
        }
        AccountId::from_public_key(public_key) == self.owner
    }
}

/// A solver fill for an open intent.  During synthesis the fill is validated
/// against the intent constraints and, if valid, executed as a stateful shift
/// from the intent owner to the beneficiary, with the solver rewarded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentFillShift {
    pub intent_id: TxHash,
    pub solver: AccountId,
    pub fill_amount: u128,
    pub nonce: u64,
    pub timestamp_ns: u64,
    pub signature: Vec<u8>,
}

impl IntentFillShift {
    pub fn new(keypair: &KeyPair, intent_id: TxHash, fill_amount: u128, nonce: u64, timestamp_ns: u64) -> Self {
        let solver = keypair.account_id();
        let mut shift = Self {
            intent_id,
            solver,
            fill_amount,
            nonce,
            timestamp_ns,
            signature: Vec::new(),
        };
        let sig = keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(b"FLUIDIC:INTENT_FILL:v1");
        buf.extend_from_slice(&self.intent_id);
        buf.extend_from_slice(self.solver.as_bytes());
        buf.extend_from_slice(&self.fill_amount.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf
    }

    pub fn hash(&self) -> TxHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.signing_bytes());
        hasher.finalize().into()
    }

    pub fn verify(&self, public_key: &ed25519_dalek::VerifyingKey) -> bool {
        let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.signature) else {
            return false;
        };
        if !KeyPair::verify(public_key, &self.signing_bytes(), &sig) {
            return false;
        }
        AccountId::from_public_key(public_key) == self.solver
    }
}

/// Registers an agent account controlled by an owner.  The owner delegates
/// limited authority to the agent key; the agent can sign shifts on the
/// owner's behalf until `expiry_tick`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistrationShift {
    pub agent: AccountId,
    pub owner: AccountId,
    pub public_key: [u8; 32],
    pub expiry_tick: u64,
    pub nonce: u64,
    pub timestamp_ns: u64,
    pub signature: Vec<u8>,
}

impl AgentRegistrationShift {
    pub fn new(
        owner_keypair: &KeyPair,
        agent_public_key: &ed25519_dalek::VerifyingKey,
        expiry_tick: u64,
        nonce: u64,
        timestamp_ns: u64,
    ) -> Self {
        let owner = owner_keypair.account_id();
        let agent = AccountId::from_public_key(agent_public_key);
        let mut shift = Self {
            agent,
            owner,
            public_key: agent_public_key.to_bytes(),
            expiry_tick,
            nonce,
            timestamp_ns,
            signature: Vec::new(),
        };
        let sig = owner_keypair.sign(&shift.signing_bytes());
        shift.signature = sig.to_bytes().to_vec();
        shift
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(b"FLUIDIC:AGENT_REG:v1");
        buf.extend_from_slice(self.agent.as_bytes());
        buf.extend_from_slice(self.owner.as_bytes());
        buf.extend_from_slice(&self.public_key);
        buf.extend_from_slice(&self.expiry_tick.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf
    }

    pub fn verify(&self, owner_public_key: &ed25519_dalek::VerifyingKey) -> bool {
        let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.signature) else {
            return false;
        };
        if !KeyPair::verify(owner_public_key, &self.signing_bytes(), &sig) {
            return false;
        }
        AccountId::from_public_key(owner_public_key) == self.owner
    }
}

/// A cryptographically signed Signal injected into a Concurrency Domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Commutative(CommutativeShift),
    Stateful(StatefulShift),
    Registration(RegistrationShift),
    Stake(StakeShift),
    AgentRegistration(AgentRegistrationShift),
    Intent(IntentShift),
    IntentFill(IntentFillShift),
    /// Gossip probe: timestamp of the sender, used to estimate network RTT.
    Ping { timestamp_ns: u64, nonce: u64 },
    /// Response to a gossip probe.
    Pong { timestamp_ns: u64, nonce: u64 },
    /// Encrypted mempool payload.  Decrypted by nodes that know the network PSK.
    Encrypted(crate::crypto::encrypted::EncryptedSignal),
    /// Signed synthesis certificate gossiped between operators.
    Certificate(crate::consensus::certificate::SynthesisCertificate),
    /// Gossip authentication handshake. Sent immediately after connecting.
    /// The proof is a keyed hash proving knowledge of the shared network key.
    Auth { proof: [u8; 32] },
    /// One or more signed peer endpoint announcements exchanged after
    /// handshake so nodes can discover peers without a central registry.
    PeerAnnounce(Vec<PeerAnnouncement>),
}

impl Signal {
    pub fn hash(&self) -> TxHash {
        match self {
            Signal::Commutative(s) => s.hash(),
            Signal::Stateful(s) => s.hash(),
            Signal::Registration(s) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&s.account.0);
                hasher.update(&s.public_key);
                hasher.update(&s.wave_account.0);
                hasher.update(&s.usdc_account.0);
                hasher.update(&s.nonce.to_le_bytes());
                hasher.update(&s.timestamp_ns.to_le_bytes());
                hasher.finalize().into()
            }
            Signal::Stake(s) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&s.operator.0);
                hasher.update(&s.public_key);
                hasher.update(&s.amount.to_le_bytes());
                hasher.update(&s.nonce.to_le_bytes());
                hasher.update(&s.timestamp_ns.to_le_bytes());
                hasher.finalize().into()
            }
            Signal::AgentRegistration(s) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(s.agent.as_bytes());
                hasher.update(s.owner.as_bytes());
                hasher.update(&s.public_key);
                hasher.update(&s.expiry_tick.to_le_bytes());
                hasher.update(&s.nonce.to_le_bytes());
                hasher.update(&s.timestamp_ns.to_le_bytes());
                hasher.finalize().into()
            }
            Signal::Intent(s) => s.hash(),
            Signal::IntentFill(s) => s.hash(),
            Signal::Ping { timestamp_ns, nonce } | Signal::Pong { timestamp_ns, nonce } => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&timestamp_ns.to_le_bytes());
                hasher.update(&nonce.to_le_bytes());
                hasher.finalize().into()
            }
            Signal::Certificate(c) => c.hash(),
            Signal::Auth { proof } => *proof,
            Signal::Encrypted(enc) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"fluidic:signal:encrypted:v1");
                hasher.update(&enc.nonce);
                hasher.update(&enc.ciphertext);
                hasher.update(&enc.tag);
                hasher.finalize().into()
            }
            Signal::PeerAnnounce(anns) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"fluidic:signal:peer-announce:v1");
                for ann in anns {
                    hasher.update(&ann.hash());
                }
                hasher.finalize().into()
            }
        }
    }

    pub fn timestamp_ns(&self) -> u64 {
        match self {
            Signal::Commutative(s) => s.timestamp_ns,
            Signal::Stateful(s) => s.timestamp_ns,
            Signal::Registration(s) => s.timestamp_ns,
            Signal::Stake(s) => s.timestamp_ns,
            Signal::AgentRegistration(s) => s.timestamp_ns,
            Signal::Intent(s) => s.timestamp_ns,
            Signal::IntentFill(s) => s.timestamp_ns,
            Signal::Ping { timestamp_ns, .. } | Signal::Pong { timestamp_ns, .. } => *timestamp_ns,
            Signal::Certificate(c) => c.timestamp_ns,
            Signal::Auth { .. } => 0,
            Signal::Encrypted(_) => 0,
            Signal::PeerAnnounce(anns) => {
                anns.first().map(|a| a.timestamp_ns).unwrap_or(0)
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commutative_sign_verify() {
        let kp = KeyPair::generate();
        let coord = Coordinate::from_scalar(7);
        let pool = [1u8; 32];
        let shift = CommutativeShift::new(&kp, DEFAULT_DEX_DOMAIN, coord, 100, pool, 1, 0);
        assert!(shift.verify(&kp.public_key()));
    }

    #[test]
    fn stateful_sign_verify_rejects_wrong_signer() {
        let kp = KeyPair::generate();
        let other = KeyPair::generate();
        let shift = StatefulShift::new(
            &kp,
            DEFAULT_DEX_DOMAIN,
            other.account_id(),
            50,
            VectorClock::new(),
            vec![],
            1,
            0,
        );
        assert!(shift.verify(&kp.public_key()));
        assert!(!shift.verify(&other.public_key()));
    }

    #[test]
    fn vector_clock_ordering() {
        let node_a = [1u8; 32];
        let node_b = [2u8; 32];
        let mut vc_a = VectorClock::new();
        vc_a.tick(node_a);

        let mut vc_b = vc_a.clone();
        vc_b.tick(node_b);

        assert!(vc_a.happened_before(&vc_b));
        assert!(!vc_b.happened_before(&vc_a));
        assert!(vc_a.concurrent_with(&vc_a)); // identical is not strictly before
    }
}
