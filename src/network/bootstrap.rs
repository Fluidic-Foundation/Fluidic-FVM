use crate::network::genesis::{BootstrapRecord, GENESIS_OPERATORS};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Maximum age for a trusted bootstrap record: 30 days.
const MAX_BOOTSTRAP_AGE_NS: u64 = 30 * 24 * 60 * 60 * 1_000_000_000;

/// Resolve DNS TXT records for `seed` and return verified endpoints.
pub async fn resolve_txt_seeds(seed: &str) -> Vec<String> {
    let records = match hickory_resolver::TokioAsyncResolver::tokio_from_system_conf() {
        Ok(resolver) => {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                resolver.txt_lookup(seed),
            )
            .await
            {
                Ok(Ok(lookup)) => {
                    lookup
                        .iter()
                        .flat_map(|txt| txt.txt_data().iter().map(|b| String::from_utf8_lossy(b).to_string()))
                        .collect::<Vec<_>>()
                }
                Ok(Err(e)) => {
                    warn!("DNS TXT lookup failed for {}: {}", seed, e);
                    Vec::new()
                }
                Err(_) => {
                    warn!("DNS TXT lookup for {} timed out", seed);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!("failed to build DNS resolver: {}", e);
            Vec::new()
        }
    };

    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut endpoints = Vec::new();

    for rec in records {
        if let Some(bootstrap) = BootstrapRecord::from_txt(&rec) {
            if !bootstrap.verify() {
                warn!("rejected unverified bootstrap record: {}", rec);
                continue;
            }
            if now_ns.saturating_sub(bootstrap.timestamp_ns) > MAX_BOOTSTRAP_AGE_NS {
                warn!("rejected stale bootstrap record for {}", bootstrap.endpoint);
                continue;
            }
            info!("learned bootstrap endpoint {} from DNS", bootstrap.endpoint);
            endpoints.push(bootstrap.endpoint);
        } else {
            // Non-bootstrap TXT records are ignored.
        }
    }
    endpoints
}

/// Fetch a signed JSON bootstrap file from a URL and return verified endpoints.
pub async fn fetch_bootstrap_url(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            warn!("failed to build HTTP client: {}", e);
            return Vec::new();
        }
    };
    match client.get(url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("bootstrap URL returned {}", resp.status());
                return Vec::new();
            }
            match resp.json::<Vec<BootstrapRecord>>().await {
                Ok(records) => {
                    let now_ns = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0);
                    records
                        .into_iter()
                        .filter_map(|r| {
                            if !r.verify() {
                                warn!("rejected unverified bootstrap record from URL");
                                return None;
                            }
                            if now_ns.saturating_sub(r.timestamp_ns) > MAX_BOOTSTRAP_AGE_NS {
                                warn!("rejected stale bootstrap record from URL");
                                return None;
                            }
                            Some(r.endpoint)
                        })
                        .collect()
                }
                Err(e) => {
                    warn!("failed to parse bootstrap JSON: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!("failed to fetch bootstrap URL {}: {}", url, e);
            Vec::new()
        }
    }
}

/// Print the compiled genesis operators (useful for diagnostics).
pub fn list_genesis_operators() {
    for op in GENESIS_OPERATORS.iter() {
        println!("{} -> {}", op.name, op.account);
    }
}
