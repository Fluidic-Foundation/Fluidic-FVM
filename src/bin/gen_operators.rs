use fluidic::network::HybridKeypair;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let out_dir = PathBuf::from("genesis");
    std::fs::create_dir_all(&out_dir).expect("create genesis dir");

    let mut operators = Vec::with_capacity(count);
    let mut keys = Vec::with_capacity(count);

    for i in 0..count {
        let name = format!("fluidic-foundation-{}", i);
        let (kp, op) = HybridKeypair::generate(&name);
        operators.push(op.clone());

        // Save secrets before moving keypair into keys vec.
        let ed_secret = hex::encode(kp.ed25519.to_bytes());
        std::fs::write(out_dir.join(format!("operator_{}.ed25519.secret", i)), ed_secret)
            .expect("write ed25519 secret");
        std::fs::write(
            out_dir.join(format!("operator_{}.dilithium.secret", i)),
            kp.dilithium.expose_secret(),
        )
        .expect("write dilithium secret");

        keys.push((name.clone(), kp));

        println!("generated {} -> {}", name, op.account);
    }

    // Save public operator list.
    let ops_json = serde_json::to_string_pretty(&operators).expect("serialize operators");
    std::fs::write(out_dir.join("operators.json"), ops_json).expect("write operators.json");

    // Sign bootstrap records for common endpoints.
    let endpoints: Vec<String> = std::env::args()
        .skip(2)
        .collect();

    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let mut txt_records = Vec::new();
    for endpoint in &endpoints {
        // Use operator 0 as the canonical signer for DNS records.
        let record = keys[0].1.sign_bootstrap(endpoint, timestamp_ns);
        txt_records.push(record.to_txt());
    }

    if !txt_records.is_empty() {
        std::fs::write(
            out_dir.join("bootstrap_dns_txt.txt"),
            txt_records.join("\n"),
        )
        .expect("write bootstrap txt");
        println!("wrote {} bootstrap TXT record(s)", txt_records.len());
    }

    println!("genesis operators written to {}/", out_dir.display());
    println!("embed operators.json in the binary and publish bootstrap_dns_txt.txt as DNS TXT records.");
}
