use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;
use tracing::info;

/// mDNS service type for Fluidic mesh nodes.
const SERVICE_TYPE: &str = "_fluidic._tcp.local.";

/// Browse the local network for Fluidic peers for `duration` and return a
/// deduplicated list of endpoints.  Endpoints are read from the TXT property
/// `endpoint` if present, otherwise constructed as `tcp://<host>:<port>`.
pub fn browse_for(duration: Duration) -> Result<Vec<String>, mdns_sd::Error> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse(SERVICE_TYPE)?;
    let mut endpoints = Vec::new();
    let mut seen = HashSet::new();
    let start = std::time::Instant::now();

    while start.elapsed() < duration {
        let remaining = duration - start.elapsed();
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let endpoint = info
                    .get_property_val_str("endpoint")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        let host = info
                            .addresses
                            .iter()
                            .next()
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| info.host.clone());
                        format!("tcp://{}:{}", host, info.port)
                    });
                if seen.insert(endpoint.clone()) {
                    info!("mdns discovered {}", endpoint);
                    endpoints.push(endpoint);
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = mdns.shutdown();
    Ok(endpoints)
}

/// Announce a local Fluidic service via mDNS so LAN peers can discover it.
/// Returns the daemon handle; the caller should keep it alive for the lifetime
/// of the process.
pub fn announce(
    instance_name: &str,
    endpoint: &str,
    ip: IpAddr,
    port: u16,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let mdns = ServiceDaemon::new()?;
    let host_name = format!("{}.local.", ip);
    let properties = [("endpoint", endpoint)];
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        instance_name,
        &host_name,
        ip,
        port,
        &properties[..],
    )?;
    mdns.register(service)?;
    info!("mdns announced {} on {} port {}", endpoint, ip, port);
    Ok(mdns)
}
