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
  -e PEERS="api.testnet.fluidic.foundation:7000" \
  -e FLUIDIC_DATA_DIR=/data \
  -v "$HOME/fluidic-data:/data" \
  ghcr.io/fluidic-foundation/fluidic-fvm/mesh-node:latest
```

Use a **unique numeric** `OSCILLATOR_ID`. The identity is deterministic, so two nodes with the same ID share a keypair and will slash each other. Mount `/data` so your snapshot and identity survive restarts.

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
  PEERS="api.testnet.fluidic.foundation:7000" \
  ./target/release/mesh_node
```

### Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `OSCILLATOR_ID` | `0` | Unique numeric node identity. Use a random number; do not reuse IDs across machines |
| `API_PORT` | `8080` | HTTP/WebSocket API port |
| `BIND_ADDR` | `0.0.0.0:7000` | TCP gossip bind address |
| `PEERS` | `''` | Comma-separated list of gossip peers to dial on startup |
| `SEED_DNS` | `''` | DNS name to resolve for bootstrap peers |
| `FLUIDIC_PSK` | `''` | Hex-encoded 32-byte PSK for authenticated gossip |
| `SYNTHESIS_INTERVAL_MS` | `1000` | How often a synthesis tick runs |
| `SNAPSHOT_INTERVAL_MS` | `5000` | How often the state snapshot is saved to disk |
| `ENABLE_GENERATOR` | `false` | Emit synthetic commutative traffic (do not enable on public testnet) |
| `FLUIDIC_DATA_DIR` | `./data` | Directory for snapshots, peer cache, and persisted identity |

## What happens when it starts

1. A deterministic Ed25519 keypair is derived from `OSCILLATOR_ID`.
2. The node seeds a genesis balance for its own operator account.
3. It locks that balance as stake so the node is immediately eligible to produce BFT synthesis certificates.
4. It opens the API server and joins the gossip mesh via `PEERS`.
5. Every `SYNTHESIS_INTERVAL_MS` it runs a synthesis tick: burns metabolic value, finalizes stateful/commutative/EVM shifts, signs a certificate, and gossips it to peers.

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
