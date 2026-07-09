//! Encrypted mempool support.
//!
//! Pending signals can be wrapped in `Signal::Encrypted` so that external
//! observers who do not know the network PSK cannot read transaction details
//! while they sit in the mempool.  Nodes that join the mesh already possess the
//! PSK (via `FLUIDIC_PSK`) and decrypt the payload before ingestion.
//!
//! This is a first-layer MEV mitigation: it hides content from passive network
//! observers.  Future upgrades can add threshold / identity-bound encryption so
//! even individual operators cannot read pending transactions ahead of time.

use crate::crypto::Signal;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};

/// Encrypted wire format for a mempool signal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedSignal {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
    pub tag: [u8; 16],
}

fn mempool_key(psk: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"FLUIDIC:ENCRYPTED_MEMPOOL:v1");
    hasher.update(psk);
    hasher.finalize().into()
}

/// Encrypt a signal using the network PSK.  Returns the encrypted payload and
/// the nonce used (the caller may need the nonce for debugging / indexing).
pub fn encrypt_signal(psk: &[u8], signal: &Signal) -> Result<EncryptedSignal, String> {
    let plaintext = bincode::serialize(signal).map_err(|e| format!("serialize failed: {}", e))?;
    let key = mempool_key(psk);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("key init failed: {}", e))?;
    let nonce = ChaCha20Poly1305::generate_nonce(OsRng);
    let mut full = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| format!("encrypt failed: {}", e))?;
    // `encrypt` appends the 16-byte Poly1305 tag to the ciphertext.
    if full.len() < 16 {
        return Err("ciphertext too short after encryption".to_string());
    }
    let tag_start = full.len() - 16;
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&full[tag_start..]);
    full.truncate(tag_start);
    Ok(EncryptedSignal {
        nonce: nonce.into(),
        ciphertext: full,
        tag,
    })
}

/// Decrypt an encrypted signal using the network PSK.
pub fn decrypt_signal(psk: &[u8], enc: &EncryptedSignal) -> Result<Signal, String> {
    let key = mempool_key(psk);
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("key init failed: {}", e))?;
    let nonce = Nonce::from_slice(&enc.nonce);
    let mut full = enc.ciphertext.clone();
    full.extend_from_slice(&enc.tag);
    let plaintext = cipher
        .decrypt(nonce, full.as_ref())
        .map_err(|e| format!("decrypt failed: {}", e))?;
    bincode::deserialize(&plaintext).map_err(|e| format!("deserialize failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::KeyPair;
    use crate::crypto::{CommutativeShift, DEFAULT_DEX_DOMAIN};
    use crate::field::coordinates::Coordinate;

    #[test]
    fn roundtrip_encrypted_signal() {
        let psk = b"test-psk";
        let kp = KeyPair::generate();
        let shift = CommutativeShift::new(
            &kp,
            DEFAULT_DEX_DOMAIN,
            Coordinate::from_scalar(1),
            100,
            [1u8; 32],
            1,
            0,
        );
        let signal = Signal::Commutative(shift);
        let enc = encrypt_signal(psk, &signal).unwrap();
        let dec = decrypt_signal(psk, &enc).unwrap();
        assert_eq!(signal, dec);
    }

    #[test]
    fn wrong_psk_fails() {
        let psk = b"correct-psk";
        let wrong = b"wrong-psk";
        let kp = KeyPair::generate();
        let shift = CommutativeShift::new(
            &kp,
            DEFAULT_DEX_DOMAIN,
            Coordinate::from_scalar(1),
            100,
            [1u8; 32],
            1,
            0,
        );
        let signal = Signal::Commutative(shift);
        let enc = encrypt_signal(psk, &signal).unwrap();
        assert!(decrypt_signal(wrong, &enc).is_err());
    }
}
