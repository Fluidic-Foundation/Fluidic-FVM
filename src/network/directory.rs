use crate::crypto::{AccountId, KeyPair};
use crate::network::discovery::{EndpointScheme, PeerAnnouncement};
use crate::state::merkle::MerkleAccumulator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Default maximum age for a peer announcement: 7 days.
const DEFAULT_MAX_AGE_NS: u64 = 7 * 24 * 60 * 60 * 1_000_000_000;

/// Default upper bound on stored announcements per operator.
const DEFAULT_MAX_ANNOUNCEMENTS_PER_OPERATOR: usize = 4;

/// Default upper bound on total stored announcements.
const DEFAULT_MAX_ANNOUNCEMENTS: usize = 1024;

/// A thread-safe directory of signed peer endpoint announcements.
#[derive(Clone, Default)]
pub struct PeerDirectory {
    inner: Arc<Mutex<PeerDirectoryInner>>,
}

#[derive(Default, Serialize, Deserialize)]
struct PeerDirectoryInner {
    /// endpoint -> announcement
    by_endpoint: HashMap<String, PeerAnnouncement>,
    /// operator -> set of endpoints
    by_operator: HashMap<AccountId, Vec<String>>,
}

impl PeerDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a persisted directory from JSON.  Invalid entries are silently
    /// skipped so a corrupt cache does not prevent startup.
    pub fn load(path: &std::path::Path) -> Self {
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<PeerDirectoryInner>(&json) {
                Ok(inner) => Self {
                    inner: Arc::new(Mutex::new(inner)),
                },
                Err(e) => {
                    warn!("failed to parse peer cache {}: {}", path.display(), e);
                    Self::new()
                }
            },
            Err(e) => {
                warn!("failed to read peer cache {}: {}", path.display(), e);
                Self::new()
            }
        }
    }

    /// Persist the directory to JSON.
    pub fn save(&self, path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let inner = self.inner.lock().unwrap();
        match serde_json::to_string_pretty(&*inner) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    warn!("failed to write peer cache {}: {}", path.display(), e);
                }
            }
            Err(e) => warn!("failed to serialize peer cache: {}", e),
        }
    }

    /// Insert a batch of announcements after validating signatures and freshness.
    /// Returns the number of newly accepted announcements.
    pub fn insert_announcements(
        &self,
        announcements: &[PeerAnnouncement],
        max_age_ns: Option<u64>,
    ) -> usize {
        let max_age_ns = max_age_ns.unwrap_or(DEFAULT_MAX_AGE_NS);
        let now_ns = now_ns();
        let mut accepted = 0;
        let mut inner = self.inner.lock().unwrap();

        for ann in announcements {
            // Skip if the announcement itself is invalid.
            if !ann.verify() {
                warn!("rejected peer announcement for {}: invalid signature", ann.endpoint);
                continue;
            }
            // Skip stale announcements.
            if !ann.is_fresh(now_ns, max_age_ns) {
                warn!("rejected stale peer announcement for {}", ann.endpoint);
                continue;
            }
            // Skip unsupported schemes.
            if EndpointScheme::parse(&ann.endpoint).is_none() {
                warn!("rejected peer announcement with unsupported scheme: {}", ann.endpoint);
                continue;
            }

            let old = inner.by_endpoint.get(&ann.endpoint).cloned();
            // Replace only if newer (by timestamp) or no prior entry.
            if let Some(ref old) = old {
                if ann.timestamp_ns <= old.timestamp_ns {
                    continue;
                }
            }

            inner.by_endpoint.insert(ann.endpoint.clone(), ann.clone());
            accepted += 1;

            // Maintain operator index.
            let operator = ann.operator;
            let endpoint = ann.endpoint.clone();
            let timestamps: Vec<u64> = {
                let by_endpoint = &inner.by_endpoint;
                inner
                    .by_operator
                    .get(&operator)
                    .map(|ops| {
                        ops.iter()
                            .filter_map(|ep| by_endpoint.get(ep).map(|a| a.timestamp_ns))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let mut ops = inner
                .by_operator
                .get_mut(&operator)
                .map(|v| v.clone())
                .unwrap_or_default();
            if !ops.contains(&endpoint) {
                ops.push(endpoint.clone());
            }

            // Prune old endpoints for this operator if over limit.
            let mut removed = Vec::new();
            if ops.len() > DEFAULT_MAX_ANNOUNCEMENTS_PER_OPERATOR {
                let mut indexed: Vec<(usize, u64)> = timestamps.into_iter().enumerate().collect();
                // Include the newly added endpoint with current timestamp for pruning decisions.
                indexed.push((ops.len() - 1, now_ns));
                indexed.sort_by_key(|(_, ts)| *ts);
                let remove_indices: Vec<usize> = indexed
                    .into_iter()
                    .take(ops.len() - DEFAULT_MAX_ANNOUNCEMENTS_PER_OPERATOR)
                    .map(|(idx, _)| idx)
                    .collect();
                let mut keep = Vec::with_capacity(DEFAULT_MAX_ANNOUNCEMENTS_PER_OPERATOR);
                for (idx, ep) in ops.iter().cloned().enumerate() {
                    if remove_indices.contains(&idx) {
                        removed.push(ep);
                    } else {
                        keep.push(ep);
                    }
                }
                ops = keep;
            }

            inner.by_operator.insert(operator, ops);
            for ep in &removed {
                inner.by_endpoint.remove(ep);
            }

            info!(
                "learned peer {} via operator {} (ttl={})",
                ann.endpoint, ann.operator, ann.ttl
            );
        }

        // Global cap: drop oldest by timestamp.
        while inner.by_endpoint.len() > DEFAULT_MAX_ANNOUNCEMENTS {
            let oldest = inner
                .by_endpoint
                .iter()
                .min_by_key(|(_, a)| a.timestamp_ns)
                .map(|(ep, _)| ep.clone());
            if let Some(ep) = oldest {
                inner.by_endpoint.remove(&ep);
                for ops in inner.by_operator.values_mut() {
                    ops.retain(|x| x != &ep);
                }
            } else {
                break;
            }
        }
        inner.by_operator.retain(|_, ops| !ops.is_empty());

        accepted
    }

    /// Get the local node's own announcement for a given endpoint.
    pub fn get(&self, endpoint: &str) -> Option<PeerAnnouncement> {
        self.inner.lock().unwrap().by_endpoint.get(endpoint).cloned()
    }

    /// Return all known endpoints, optionally filtered by scheme.
    pub fn endpoints(
        &self,
        scheme: Option<EndpointScheme>,
        exclude: Option<&str>,
    ) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        inner
            .by_endpoint
            .values()
            .filter(|a| {
                if let Some(s) = scheme {
                    EndpointScheme::parse(&a.endpoint).map(|(es, _)| es) == Some(s)
                } else {
                    true
                }
            })
            .filter(|a| exclude.map(|e| e != a.endpoint).unwrap_or(true))
            .map(|a| a.endpoint.clone())
            .collect()
    }

    /// Return a bounded sample of announcements suitable for forwarding.
    pub fn sample_for_forward(
        &self,
        limit: usize,
        exclude: Option<&str>,
    ) -> Vec<PeerAnnouncement> {
        let inner = self.inner.lock().unwrap();
        let mut anns: Vec<PeerAnnouncement> = inner
            .by_endpoint
            .values()
            .filter(|a| exclude.map(|e| e != a.endpoint).unwrap_or(true))
            .cloned()
            .collect();
        // Simple deterministic shuffle by hash so every node doesn't forward the same subset.
        anns.sort_by(|a, b| a.hash().cmp(&b.hash()));
        anns.truncate(limit);
        anns
    }

    /// Return the Merkle root over all stored announcements.
    pub fn merkle_root(&self) -> [u8; 32] {
        let inner = self.inner.lock().unwrap();
        let items: Vec<(Vec<u8>, Vec<u8>)> = inner
            .by_endpoint
            .iter()
            .map(|(endpoint, ann)| {
                let key = endpoint.as_bytes().to_vec();
                let value = bincode::serialize(ann).unwrap_or_default();
                (key, value)
            })
            .collect();
        MerkleAccumulator::root(&items)
    }

    /// Export all announcements as a JSON-friendly Vec.
    pub fn all_announcements(&self) -> Vec<PeerAnnouncement> {
        self.inner.lock().unwrap().by_endpoint.values().cloned().collect()
    }

    /// Return the number of stored announcements.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().by_endpoint.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_retrieve() {
        let dir = PeerDirectory::new();
        let kp = KeyPair::generate();
        let ann = PeerAnnouncement::sign(
            &kp,
            "wss://example.com/api/ws",
            now_ns(),
            3,
        );
        assert_eq!(dir.insert_announcements(&[ann.clone()], None), 1);
        assert_eq!(dir.endpoints(None, None), vec!["wss://example.com/api/ws"]);
    }

    #[test]
    fn invalid_signature_rejected() {
        let dir = PeerDirectory::new();
        let mut ann = PeerAnnouncement::sign(
            &KeyPair::generate(),
            "wss://example.com/api/ws",
            now_ns(),
            3,
        );
        ann.signature = vec![1, 2, 3];
        assert_eq!(dir.insert_announcements(&[ann], None), 0);
    }

    #[test]
    fn stale_rejected() {
        let dir = PeerDirectory::new();
        let ann = PeerAnnouncement::sign(
            &KeyPair::generate(),
            "wss://example.com/api/ws",
            0,
            3,
        );
        assert_eq!(dir.insert_announcements(&[ann], Some(1_000)), 0);
    }

    #[test]
    fn newer_replaces_older() {
        let dir = PeerDirectory::new();
        let kp = KeyPair::generate();
        let old = PeerAnnouncement::sign(
            &kp,
            "wss://example.com/api/ws",
            now_ns() - 1_000_000_000,
            3,
        );
        let new = PeerAnnouncement::sign(
            &kp,
            "wss://example.com/api/ws",
            now_ns(),
            3,
        );
        dir.insert_announcements(&[old], None);
        assert_eq!(dir.insert_announcements(&[new.clone()], None), 1);
        assert_eq!(dir.get("wss://example.com/api/ws").unwrap().timestamp_ns, new.timestamp_ns);
    }

    #[test]
    fn sample_respects_exclude() {
        let dir = PeerDirectory::new();
        let kp = KeyPair::generate();
        let a = PeerAnnouncement::sign(&kp, "wss://a/api/ws", now_ns(), 3);
        let b = PeerAnnouncement::sign(&kp, "wss://b/api/ws", now_ns(), 3);
        dir.insert_announcements(&[a, b.clone()], None);
        let sample = dir.sample_for_forward(10, Some("wss://a/api/ws"));
        assert_eq!(sample.len(), 1);
        assert_eq!(sample[0].endpoint, "wss://b/api/ws");
    }
}
