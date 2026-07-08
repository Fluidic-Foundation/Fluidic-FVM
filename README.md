# Fluidic-FVM

Reference Rust implementation of the Fluidic mesh node.

This repo is for users and operators who want to run a Fluidic oscillator node. It contains the core state engine, HTTP/WebSocket API, TCP gossip mesh, and a containerized `mesh_node` binary.

> The landing page, dApp, TypeScript SDK, faucet, and testnet infrastructure live in separate repositories.

## Quick start

### Docker (recommended)

```bash
docker run -d --name fluidic-node \
  --restart unless-stopped \
  -p 8080:8080 -p 7000:7000 \
  -e OSCILLATOR_ID=12345 \
  -e FLUIDIC_DATA_DIR=/data \
  -e PEERS=hayabusa.proxy.rlwy.net:34754 \
  -v "$HOME/fluidic-data:/data" \
  ghcr.io/fluidic-foundation/fluidic-fvm/mesh-node:latest
```

Use a **unique numeric** `OSCILLATOR_ID`. The identity is deterministic, so two nodes with the same ID share a keypair and will slash each other. Mount `/data` so your snapshot and identity survive restarts.

By default this runs as a **light client** (client mode): it follows the operator mesh, verifies synthesis certificates, and exposes the API, but does not synthesize or stake. If you advertise a `PUBLIC_ENDPOINT`, the node runs as a full operator instead. Set `FLUIDIC_CLIENT_MODE=false` to force full-node behavior behind NAT.

The node also discovers peers via the public BitTorrent Mainline DHT and LAN mDNS, so the explicit `PEERS` list is a bootstrap convenience rather than a hard requirement. If you have been running older/broken builds, delete `$HOME/fluidic-data/snapshot.json` and restart with a fresh snapshot to avoid stale state.

Or use the installer:

```bash
curl -sSL https://raw.githubusercontent.com/Fluidic-Foundation/Fluidic-FVM/main/scripts/run-node.sh | bash
```

### Manual build

```bash
git clone https://github.com/Fluidic-Foundation/Fluidic-FVM.git
cd Fluidic-FVM
cargo build --release --bin mesh_node
OSCILLATOR_ID=12345 API_PORT=8080 BIND_ADDR=0.0.0.0:7000 \
  ./target/release/mesh_node
```

### Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `OSCILLATOR_ID` | `0` | Unique numeric node identity. Use a random number; do not reuse IDs across machines |
| `API_PORT` | `8080` | HTTP/WebSocket API port |
| `BIND_ADDR` | `0.0.0.0:7000` | TCP gossip bind address |
| `PEERS` | `''` | Comma-separated list of gossip peers to dial on startup |
| `SEED_DNS` | `''` | Legacy DNS name to resolve for bootstrap peers |
| `BOOTSTRAP_DNS` | `''` | DNS domain with signed genesis TXT bootstrap records (e.g. `seeds.testnet.fluidic.foundation`) |
| `BOOTSTRAP_URL` | `''` | HTTPS URL serving signed JSON bootstrap records |
| `DHT_BOOTSTRAP` | `true` | Query the public BitTorrent Mainline DHT for peers |
| `MDNS_BOOTSTRAP` | `true` | Browse LAN mDNS for `_fluidic._tcp` peers |
| `FLUIDIC_NETWORK_ID` | `fluidic-testnet-v1` | Network identifier used to derive the DHT infohash |
| `FLUIDIC_PSK` | `''` | Hex-encoded 32-byte PSK for authenticated gossip |
| `SYNTHESIS_INTERVAL_MS` | `1000` | How often a synthesis tick runs |
| `SNAPSHOT_INTERVAL_MS` | `5000` | How often the state snapshot is saved to disk |
| `ENABLE_GENERATOR` | `false` | Emit synthetic commutative traffic (do not enable on public testnet) |
| `FLUIDIC_DATA_DIR` | `./data` | Directory for snapshots, peer cache, and persisted identity |
| `PUBLIC_ENDPOINT` | `''` | Publicly reachable endpoint advertised to peers (`tcp://`, `wss://`, `ws://`). Setting this enables full operator mode |
| `FLUIDIC_CLIENT_MODE` | `auto` | `true` = light client, `false` = full operator, default = light client when `PUBLIC_ENDPOINT` is empty |
| `SYNC_PEERS` | `''` | Comma-separated HTTP API URLs to sync state from before synthesizing (required for safe full-node join) |

## What happens when it starts

1. A deterministic Ed25519 keypair is derived from `OSCILLATOR_ID`.
2. The node decides its role:
   - **Light client** (default when `PUBLIC_ENDPOINT` is empty): follows operator certificates, verifies quorum via the light client, and exposes the API. It does not stake or synthesize.
   - **Full operator** (when `PUBLIC_ENDPOINT` is advertised): connects to `SYNC_PEERS` (or `PEERS`), downloads the current state, and only then stakes genesis balance and synthesizes.
3. It discovers peers through signed genesis bootstrap records (DNS TXT / HTTPS), the public BitTorrent Mainline DHT, and LAN mDNS, plus any explicit `PEERS`.
4. It opens the API server and joins the gossip mesh.
5. Every `SYNTHESIS_INTERVAL_MS` it runs a synthesis tick. Operators burn metabolic value, finalize shifts, sign certificates, and gossip them. Light clients ingest and verify certificates from operators.

Your node is online when you see `API server listening on 0.0.0.0:8080`. Point a browser or the SDK at `http://localhost:8080`.

## API endpoints

- `GET  /api/health` — liveness check
- `GET  /api/state` — live pool reserves, price, throughput, pool account IDs
- `GET  /api/account/:id/balance` — WAVE/USDC balances for a registered account
- `POST /api/account/register` — register an Ed25519 pubkey, returns derived WAVE/USDC accounts and seeds a faucet
- `POST /api/shift/stateful` — submit a signed `StatefulShift` to be synthesized
- `GET  /api/shift/:hash/status` — finality status: `unknown`, `accepted`, `finalized`, or `rejected`
- `GET  /api/ws` — WebSocket feed of pool state updates

## Build and test

```bash
cargo build --release --bin mesh_node
cargo test
```

## Architecture

```
src/
├── api/           HTTP/WebSocket API and API state
├── bin/           mesh_node — containerized oscillator node
├── bridge/        Cross-chain bridge utilities
├── consensus/     NTT engine, vector-clock DAG, oscillator synthesis
├── crypto/        Ed25519 keypairs, signed phase-shifts
├── evm/           EVM transaction execution
├── field/         State wave-field, account frequency coordinates
├── light_client/  Light client verification
├── network/       Async gossip, TCP gossip, zero-copy ring buffers
├── operator/      Staking and rewards
├── persistence/   Snapshot save/load
├── state/         Merkle tree and account state
└── value/         Metabolic decay, continuous streams, spectrum bands
```

## License

MIT OR Apache-2.0
