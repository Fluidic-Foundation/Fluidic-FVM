use futures_util::StreamExt;
use mainline::{Dht, Id};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{info, warn};

/// Default Fluidic network identifier used to derive the DHT infohash.
pub const DEFAULT_NETWORK_ID: &str = "fluidic-testnet-v1";

/// Maximum time to spend waiting for DHT peer lookups on startup.
const DHT_LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

/// Derive a deterministic 20-byte Mainline DHT target from a network id.
/// The target is the first 20 bytes of SHA256("fluidic:dht:<network_id>").
pub fn network_infohash(network_id: &str) -> Id {
    let mut hasher = Sha256::new();
    hasher.update(b"fluidic:dht:");
    hasher.update(network_id.as_bytes());
    let hash = hasher.finalize();
    let mut id = [0u8; 20];
    id.copy_from_slice(&hash[..20]);
    Id::from_bytes(id).expect("20-byte slice is valid")
}

/// A lightweight BitTorrent Mainline DHT client for Fluidic bootstrap.
#[derive(Clone)]
pub struct DhtDiscovery {
    dht: Dht,
    infohash: Id,
}

impl DhtDiscovery {
    /// Create a DHT client bootstrapped against the public Mainline DHT.
    pub fn new(network_id: &str) -> Result<Self, std::io::Error> {
        let dht = Dht::client()?;
        let infohash = network_infohash(network_id);
        info!("dht bootstrap: infohash={} network_id={}", infohash, network_id);
        Ok(Self { dht, infohash })
    }

    /// Lookup peers advertised for this network.  Returns after the timeout or
    /// when the DHT query is exhausted, whichever comes first.
    pub async fn lookup_peers(&self) -> Vec<SocketAddr> {
        let async_dht = self.dht.clone().as_async();
        let mut stream = async_dht.get_peers(self.infohash);
        let mut peers = Vec::new();
        let deadline = tokio::time::Instant::now() + DHT_LOOKUP_TIMEOUT;

        loop {
            match tokio::time::timeout_at(deadline, stream.next()).await {
                Ok(Some(addrs)) => {
                    for addr in addrs {
                        peers.push(SocketAddr::V4(addr));
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    info!("dht bootstrap: lookup timeout reached");
                    break;
                }
            }
        }
        peers
    }

    /// Announce this node's TCP gossip port to the DHT so other nodes can find it.
    /// Only public nodes should call this; leaf/WSS nodes will be unreachable via
    /// raw TCP and should not pollute the DHT.
    pub async fn announce_peer(&self, port: u16) {
        let async_dht = self.dht.clone().as_async();
        match async_dht.announce_peer(self.infohash, Some(port)).await {
            Ok(target) => info!("announced tcp peer to DHT target={} port={}", target, port),
            Err(e) => warn!("dht announce failed: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infohash_is_deterministic() {
        let a = network_infohash(DEFAULT_NETWORK_ID);
        let b = network_infohash(DEFAULT_NETWORK_ID);
        assert_eq!(a, b);
    }

    #[test]
    fn infohash_differs_by_network() {
        let a = network_infohash("net-a");
        let b = network_infohash("net-b");
        assert_ne!(a, b);
    }
}
