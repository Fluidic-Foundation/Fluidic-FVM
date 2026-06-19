# Fluidic — Continuous-Wave State Engine (Research Prototype)

A Rust/Tokio reference implementation of the amended Fluidic architecture:

- **Number Theoretic Transform (NTT)** aggregation for *commutative*,
  state-independent operations: liquidity-pool balance shifts, continuous
  micro-payment streams, and data-throughput routing.
- **Vector-clock DAG ordering** for *state-dependent* operations such as unique
  balance exhaustion, guaranteeing strict causal consistency before any wave
  synthesis occurs.
- **Metabolic decay engine** for continuous time-based value burn.
- **TCP gossip mesh** for local sandboxed oscillator networks.

> **Status:** Functional research implementation. The core state engine, HTTP/WebSocket
> API, Ed25519-signed stateful shifts, and React dApp are real and run against a live
> local `mesh_node`. It is still not production financial infrastructure: no Sybil
> resistance, slashing, persistence layer, or formal safety proof is provided.

## Architecture

```
src/
├── crypto/        Ed25519 keypairs, signed phase-shifts
├── field/         State wave-field, account frequency coordinates
├── consensus/     NTT engine, vector-clock DAG, oscillator synthesis, mesh simulation
├── network/       Async in-process gossip, TCP gossip, zero-copy ring buffers
├── value/         Metabolic decay, continuous streams, spectrum band allocation
└── bin/           mesh_node — containerized oscillator node
```

## Build

```bash
cargo build --release
```

## Test

Unit and integration tests:

```bash
cargo test
```

10,000 overlapping transaction benchmark:

```bash
cargo test --test bench -- --nocapture
```

100,000 concurrent NTT stress test:

```bash
cargo test --test ntt_stress -- --nocapture
```

Metabolic decay overhead invariant (< 1%):

```bash
cargo test --test metabolic_overhead -- --nocapture
```

## Benchmark

Metabolic decay Criterion benchmark:

```bash
cargo bench --bench metabolic_bench
```

## Pitch Deck

Generate the 10-slide investor deck:

```bash
cd docs
npm install
node generate-pdf.js
```

Output: `docs/fluidic-pitch-deck.pdf`

## Local Sandboxed Mesh

Build and run the mesh node binary:

```bash
cargo run --release --bin mesh_node
```

Or deploy with Docker Compose:

```bash
cd docker
./partition_test.sh
```

The partition test spins up six oscillator containers, disconnects two of them
(~33%) for 15 seconds, then reconnects them and verifies that the surviving
nodes continued to synthesize the wave-field.

> **Note:** Docker must be available and the current user must have permission
> to access the Docker daemon. The partition test was validated syntactically
> but could not be executed in this environment due to daemon permissions.

## Fluidic DEX dApp

A real React/Vite dApp in `dapp/` connects to the live `mesh_node` API, derives
Ed25519 accounts in the browser, registers them with a faucet, and signs
stateful cross-token swaps that are ingested, gossiped, and synthesized by the
oscillator.

Run the node with the API server:

```bash
cargo run --release --bin mesh_node -- --api-port 8080
```

Run the dApp:

```bash
cd dapp
npm install
npm run dev
```

API endpoints:

- `GET  /api/state` — live pool reserves, price, throughput, pool account IDs
- `GET  /api/account/:id/balance` — WAVE/USDC balances for a registered account
- `POST /api/account/register` — register an Ed25519 pubkey, returns derived WAVE/USDC accounts and seeds a faucet
- `POST /api/shift/stateful` — submit a signed `StatefulShift` to be synthesized (returns the shift hash)
- `GET  /api/shift/:hash/status` — finality status: `unknown`, `accepted`, `finalized`, or `rejected`
- `GET  /api/ws` — WebSocket feed of pool state updates

A shift becomes **finalized** after surviving `FINALIZATION_DEPTH` synthesis ticks
without a conflicting double-spend being accepted into the DAG.

End-to-end swap test:

```bash
node /tmp/test-swap.mjs
```

## Key Design Decisions

1. **NTT-only for commutative ops.** The NTT is used to batch-sum deltas and
   verify that frequency-domain synthesis matches sequential aggregation. It
   does not replace causal ordering.
2. **DAG for stateful ops.** Every stateful phase-shift carries a vector clock
   and predecessor hashes. The oscillator topologically orders the DAG and
   rejects overdrafts, enforcing the conservation law.
3. **No blocks, no mempool.** Shifts are ingested as a continuous stream and
   synthesized in periodic windows. However, causal ordering for stateful
   operations is explicit and deterministic.
4. **Metabolic decay is integer-only.** Burn is computed as
   `rate_per_second * elapsed_ns / 1_000_000_000` using `u128` fixed-point
   arithmetic, so there is no floating-point drift.

## License

MIT OR Apache-2.0
