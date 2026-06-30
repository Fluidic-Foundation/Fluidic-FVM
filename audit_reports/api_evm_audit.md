# Fluidic Whitepaper ↔ Implementation Audit
## HTTP/WebSocket API · EVM RPC Gateway · TypeScript SDK · Signal Submission · Bridges

- **Date:** 2026-06-30
- **Whitepaper:** `docs/FLUIDIC_WHITEPAPER.md`
- **Code audited:** `src/` (Rust `mesh_node`) and `sdk/typescript/`
- **Method:** Static read of source; claims drawn from whitepaper §3, §4, §8, §9, §13, §14. No runtime execution was performed — correctness bugs are reasoned from code and flagged as such.

### Legend
- ✅ Implemented and matches the whitepaper claim.
- 🟡 Partially implemented / implemented differently / works but diverges from the wording.
- ❌ Missing, contradicted, or broken.

---

## 1. Scorecard (at a glance)

| # | Area | Whitepaper claim | Status |
|---|------|------------------|--------|
| A | HTTP API | Rust `mesh_node` exposes an HTTP API (§14) | ✅ |
| B | WebSocket | Clients subscribe to **domain-specific** state feeds (§4.3) | 🟡 |
| C | EVM RPC | JSON-RPC compatible with MetaMask/Hardhat/Foundry (§13) | 🟡 |
| D | EVM RPC | Verifies ECDSA signature (§13.1) | ✅ |
| E | EVM RPC | Derives a Fluidic account from the ETH address (§13.2) | ✅ |
| F | EVM RPC | **Translates the tx into a Signal in a domain** (§13.3) | ❌ |
| G | EVM RPC | Returns a (synthetic) tx hash immediately (§13.4) | 🟡 |
| H | EVM RPC | **Polls the DAG for finalization** and updates receipt (§13.5) | ❌ |
| I | Signal model | Signal = from/domain/payload/predecessors/vector_clock/signature/nonce, Ed25519 (§3) | 🟡 |
| J | Ingest | "Verifies the signature" for every received Signal (§4.1, §9.1) | 🟡 |
| K | Ingest | Replay protection via sender-domain **nonce** (§3, §4.1) | ❌ |
| L | SDK | TypeScript SDK signs & submits Signals | ✅ |
| M | SDK | `fluidic.domain({...})` domain declaration API (§6) | ❌ |
| N | SDK/Node | Finalization depth `K` is consistent across SDK and node | ❌ |
| O | Bridges | Ethereum & Solana bridge domains (§8.3, §12.4) | ❌ |

---

## 2. HTTP / WebSocket API

The whitepaper (§14) claims a "Rust `mesh_node` with HTTP/WebSocket API". This is real and substantial.

**Evidence**
- HTTP server (axum), CORS, token-bucket rate limit, body limit, timeout: `src/api/server.rs:75-108`.
- Route table (health, state, balance, register, shift submit, status, certificate, quorum, ticks, operator, faucet, sync, ws): `src/api/routes.rs:16-38`.
- WebSocket upgrade handler: `src/api/routes.rs:891-896`; socket loop: `src/api/websocket.rs:5-27`.

| Claim | Status | Evidence | Notes |
|-------|--------|----------|-------|
| HTTP API exists | ✅ | `server.rs:87-93`, `routes.rs:16-38` | Rich REST surface, well beyond the minimum. |
| WebSocket exists | ✅ | `routes.rs:891-896`, `websocket.rs:5-27` | `/api/ws`, sends an initial snapshot then deltas + 30s ping. |
| Clients subscribe to **domain-specific feeds** (§4.3) | 🟡 | `websocket.rs:34-52`, `websocket.rs:54-73` | The WS stream is a **single global snapshot** of the one DEX pool (reserves, price, throughput, three apply-counters, pool accounts). There is **no per-domain subscription, filtering, or topic** — `handle_socket` ignores all client input. "Domain-specific feeds" is not implemented. |
| State reads honor `?min_tick=` for read-consistency | ✅ | `routes.rs:84-95`, `routes.rs:97-117` | Matches the SDK `min_tick` modes. |

**Specific mismatch (B):** §4.3 says "Clients subscribe to domain-specific feeds and see state updates as soon as their local operator synthesizes them." The implementation broadcasts one global pool snapshot to every socket (`websocket.rs:38-51`); a client cannot subscribe to, say, `game.world.arena-7` separately from a payment domain. With only one domain registered (see §6 below) this is moot today, but the architectural claim is unmet.

---

## 3. EVM RPC Gateway (§13)

Router: `evm_rpc_router()` mounts a single `POST /rpc` (`src/api/evm_rpc.rs:65-67`). Dispatch table: `evm_rpc.rs:108-133`.

### 3.1 The five-step §13 pipeline

| Step (§13) | Status | Evidence | Finding |
|------------|--------|----------|---------|
| 1. Verify the ECDSA signature | ✅ | `src/evm/mod.rs:81-109` | `decode_raw` RLP-decodes then `recover_from()` — recovery fails on a bad signature, so the signature is effectively verified. Chain-id is also enforced (`mod.rs:85-91`). |
| 2. Derive a Fluidic account from the ETH address | ✅ | `src/evm/mod.rs:122-130`, `evm_rpc.rs:352-366` | `blake3("fluidic:evm-account:v1" ‖ addr)`. Used by `eth_getBalance` and faucet. |
| 3. **Translate the tx into a Signal in the appropriate domain** | ❌ | `evm_rpc.rs:369-409`, `src/evm/mod.rs:159-294`, `src/consensus/oscillator.rs:394-402` | The tx is **not** turned into a `Signal`. It is pushed into a separate `EvmPool` and executed by a real `revm` interpreter on its own `InMemoryDB`. It never becomes `Signal::Stateful`, never enters a Concurrency Domain, and is not subject to domain policy. The only shared state is the raw balance map. |
| 4. Return a (synthetic) tx hash immediately | 🟡 | `evm_rpc.rs:382,395,408` | Returns immediately ✅, but the hash is the **genuine** keccak256 Ethereum tx hash (`tx.hash`), not a "synthetic" one. Functionally fine; contradicts the word "synthetic". |
| 5. **Poll the DAG for finalization** and update the receipt | ❌ | `src/evm/mod.rs:176-294`, `oscillator.rs:394-402` | Receipts are produced by `EvmPool::synthesize` (revm), keyed to the synthesis `tick` as a pseudo-block. EVM txs **never touch the Vector-Clock DAG**, so there is no DAG-based finalization for them. The certificate carries a separate `evm_root` (`src/consensus/certificate.rs:28`), parallel to `stateful_root`. |

**Specific mismatch (F & H, the headline divergence):** §13 presents EVM compatibility as "translate the transaction into a Signal … poll the DAG for finalization." In reality the gateway runs a **parallel execution engine** (revm) that bypasses the Signal abstraction, the Concurrency Domains, and the Vector-Clock DAG entirely (`oscillator.rs:394-402` runs `evm_pool.synthesize` after, and independently of, the DAG topological apply at `oscillator.rs:344-392`). The "under the hood it's a Signal in a domain" narrative does not hold.

### 3.2 JSON-RPC method coverage (§13: "compatible with MetaMask, Hardhat, Foundry") — 🟡

Implemented (`evm_rpc.rs:108-133`): `eth_blockNumber`, `eth_getBlockByNumber`, `eth_getBlockByHash`, `eth_chainId`, `net_version`, `eth_gasPrice`, `eth_getBalance`, `eth_getCode`, `eth_call`, `eth_sendRawTransaction`, `eth_getTransactionByHash`, `eth_getTransactionReceipt`, `eth_getTransactionCount`, `eth_estimateGas`, `eth_getLogs`, `web3_clientVersion`.

Missing methods these tools commonly require:
- `eth_feeHistory`, `eth_maxPriorityFeePerGas` — EIP-1559 fee estimation used by modern MetaMask and by Foundry/`cast`. Absent ⇒ EIP-1559 flows fall back or error.
- `eth_accounts`, `eth_sendTransaction` — needed by node-managed signing flows (Hardhat default network).
- `eth_getStorageAt`, `eth_syncing`.
- Filters / subscriptions: `eth_newFilter`, `eth_getFilterChanges`, `eth_uninstallFilter`, `eth_subscribe` — used by ethers/web3 event watchers.
- `eth_getBlockByNumber` returns **transaction hashes only**, never full tx objects (it ignores the `fullTx` boolean), and blocks omit `baseFeePerGas` (`block_json`, `evm_rpc.rs:173-234`).

Verdict: the **core send/read path** for a client that signs locally and uses legacy gas works; "compatible with MetaMask, Hardhat, Foundry" is an overstatement → 🟡.

### 3.3 Concrete bugs found in the gateway

| Bug | Severity | Evidence | Detail |
|-----|----------|----------|--------|
| `u256_to_hex` emits **decimal** digits behind a `0x` prefix | High | `evm_rpc.rs:169-171` | `format!("0x{}", u.to_string().trim_start_matches("0x"))`. `ethers_core::U256::to_string()` is **decimal**, so e.g. `1 ETH = 10^18` is rendered `0x1000000000000000000` (a different number). This corrupts `value` & `gasPrice` in `eth_getTransactionByHash` (`evm_rpc.rs:420-424`) and `effectiveGasPrice` in receipts (`evm_rpc.rs:543-545`). Should be `format!("0x{:x}", u)`. |
| `net_version` returns a **hex** string | Medium | `evm_rpc.rs:116`, `332-334` | `net_version` returns `chain_id()` = `"0xf1d1c"`. The JSON-RPC convention is a **decimal** network-id string (e.g. `"991004"`). Clients that `parseInt(netVersion)` will misread it. |
| `eth_call`/`eth_estimateGas` parse `value` as `U256::from_str` | Low | `evm_rpc.rs:640-644`, `705-709` | `primitive-types` `U256::from_str` is hex-without-`0x`; a `"0x.."` value silently `unwrap_or_default()`s to 0. Hex value args are dropped. |

`eth_gasPrice` returns `0x0` (`evm_rpc.rs:117`) — consistent with §11.3 "no global gas market" ✅.

---

## 4. Signal model & submission (§3, §4.1, §9)

### 4.1 Signal composition (§3) — 🟡

§3 says a Signal is `{from, domain, payload, predecessors, vector_clock, signature, nonce}`, Ed25519-signed.

- `Signal` enum: `src/crypto/phase_shift.rs:300-315` — variants `Commutative`, `Stateful`, `Registration`, `Stake`, `Ping`, `Pong`, `Certificate`, `Auth`.
- `StatefulShift`: `phase_shift.rs:143-157` — has `domain, from, to, amount, vector_clock, predecessors, nonce, timestamp_ns, signature` ✅ (matches §3 closely; `payload` = `to`+`amount`).
- `CommutativeShift`: `phase_shift.rs:75-87` — has `domain, coordinate, delta, pool_id, nonce, timestamp_ns, signature` but **no `from`, no `vector_clock`, no `predecessors`**.
- Ed25519 throughout: signing `crypto/keys.rs` + `phase_shift.rs:109-110,183-184`; SDK `@noble/curves/ed25519` `sdk/typescript/src/crypto.ts:75-81` ✅.

**Mismatch (I):** the §3 "every Signal has `from` … `vector_clock` … `predecessors`" generalization does not hold for commutative signals — they are anonymous coordinate deltas with no signer identity field.

### 4.2 Ingest pipeline (§4.1: verify sig → replay check via nonce → route to domain → dedup by hash)

| §4.1 step | Status | Evidence | Finding |
|-----------|--------|----------|---------|
| Verify the signature | 🟡 | stateful: `routes.rs:311-327` (API) + `dag.rs:86-90` + `oscillator.rs:367-378` (synthesis re-verify) ✅; commutative: **none** | **Commutative signals are never signature-verified** — not in `submit_commutative` (`routes.rs:409-460`), not in `ingest_commutative` (`oscillator.rs:204-227`), not in synthesis. They are accepted and applied to pool coordinates unauthenticated. Contradicts §4.1 and §9.1 ("Every Signal is signed"). |
| Replay protection via **nonce** | ❌ | `oscillator.rs:60` (`seen_signatures`), `218-221`, `246-248` | De-duplication is by **raw signature bytes** (`seen_signatures: DashMap<Vec<u8>,()>`), not by a sender-domain nonce counter. The `nonce` field is carried and signed but is **never checked** for replay/monotonicity in ingest. Stateful ordering is instead enforced by the vector clock (`dag.rs:215-259`). The §3/§4.1 "anti-replay counter scoped to the sender-domain pair" is not implemented as described. |
| Route to the appropriate domain queue | ✅ | `oscillator.rs:204-217`, `229-245` | Unknown domains are rejected; policy gates commutative vs stateful and FIFO vs DAG. |
| Deduplicate using the Signal hash | 🟡 | `oscillator.rs:218,246`; `dag.rs:92-94` | Dedup is by signature bytes at ingest and by hash inside the DAG — effective, but keyed on signature rather than the "Signal hash" the paper names. |

### 4.3 Stateful submission path — ✅ (mostly)

`submit_stateful` (`routes.rs:305-407`): hex-parses, looks up the sender's key in the registry, verifies the Ed25519 signature over `signing_bytes()`, validates the vector clock against observed causal history (`dag.validate_vector_clock`), optionally synthesizes the pool payout, records the shift, and ingests. Solid and matches the security intent. Note: an unknown sender ⇒ `401` (`routes.rs:312-318`), so the sender must have been registered/gossiped first.

---

## 5. TypeScript SDK (`sdk/typescript/`)

### 5.1 What matches — ✅
- Ed25519 keypair, account/wave/usdc derivation: `src/crypto.ts:11-91`.
- Canonical signing-byte construction byte-for-byte matches the Rust domains/tags `FLUIDIC:STATEFUL:v2`, `FLUIDIC:COMMUTATIVE:v2`, `FLUIDIC:STAKE:v1`: `src/shifts.ts:59-67,104-114,163-170` vs `phase_shift.rs:114-124,188-206,259-268`. Vector-clock is sorted by node key in both (`shifts.ts:30-40` vs `phase_shift.rs:195-199`).
- Client REST surface + `min_tick` modes (`none|latest|quorum|number`): `src/client.ts:46-59,104-191`; documented in `README.md:46-62`.
- WebSocket subscribe to `/api/ws`: `src/client.ts:225-269`.
- EVM provider over `POST /rpc`: `src/evm.ts:7-83`.

### 5.2 Mismatches

| Claim / expectation | Status | Evidence | Finding |
|---------------------|--------|----------|---------|
| Finalization depth `K` consistent SDK↔node | ❌ | `sdk/.../constants.ts:5` (`FINALIZATION_DEPTH = 6`) vs `src/consensus/dag.rs:74` (`FINALIZATION_DEPTH = 3`) and `domain.rs:40` (`finalization_depth: 3`) | The SDK advertises **6**; the node finalizes at **3** and reports `confirmations: 3` for finalized shifts (`routes.rs:647-653`). Any SDK logic keyed to 6 is wrong. |
| `submitCommutative` sends the signed `domain` | 🟡 | `src/client.ts:120-131` | The POST body omits the `domain` field, so the node defaults it to `DEFAULT_DEX_DOMAIN` (`routes.rs:413-416`). A commutative shift built for a non-default domain is signed over that domain but submitted/recorded under the DEX domain ⇒ hash and domain diverge from what `hashCommutativeShift` computed. (Low impact today because the node never verifies commutative signatures.) |
| `fluidic.domain({ id, policy, replicationFactor, operators })` (§6) | ❌ | no match in `sdk/typescript/src/*` | There is **no** `domain()` method or any domain-declaration API in the SDK. The §6 example code does not exist. |
| EVM "bridge" naming | 🟡 | `README.md:82-89` | The README calls `FluidicEvmProvider` an "EVM RPC bridge". It is an RPC client, not a cross-chain bridge (see §7). Cosmetic but misleading. |

---

## 6. Concurrency Domains & policy (§6) — context for the above

`DomainRegistry` seeds **only** the DEX domain and exposes no public route to declare more (`src/consensus/domain.rs:58-65`; no `domain` route in `routes.rs:16-38`).

- §6 policy values are `"commutative" | "causal" | "strict"`; the code models a domain as `{commutative: bool, stateful: bool, ordering: Dag|Fifo, finalization_depth, metabolic_multiplier_bp}` (`domain.rs:18-28`). There is **no "strict" / operator-quorum-per-Signal policy**, no `replicationFactor`, and no `operators[]` list at the domain level. → 🟡/❌ for §6 as written.

---

## 7. Bridges (§8.3, §12.4, §14 roadmap) — ❌ not implemented

- No bridge module, type, or route exists anywhere in `src/` or `sdk/typescript/src/`. A repo-wide search for `bridge|solana|inbound|outbound` in `src/` matches **only** the gossip layer's `inbound`/`outbound` channels (`src/network/tcp_gossip.rs:23-28,…`) — unrelated to cross-chain bridging.
- The only "bridge" strings are docs/marketing: whitepaper, audit reports, landing pages, `sdk/typescript/README.md:82` ("EVM RPC bridge").
- §8.3 ("Bridges connect Fluidic domains to external systems: Ethereum, Solana…") and §12.4 ("Each external chain gets a bridge domain… Inbound transfers become Signals… Outbound transfers are signed by bridge operators") describe a role that has **no code**. There is no `bridge.eth.inbound` domain, no external-finality watcher, no outbound relayer, no Solana integration.

**Note:** §14's roadmap explicitly lists "Bridge domains for Ethereum and Solana" as **future** work, and the §14 "functional" list does **not** claim bridges. So the *roadmap* is internally honest; the mismatch is between the **architecture sections (§8.3/§12.4), which present bridges as an existing role**, and the absent implementation.

---

## 8. Prioritized mismatch list

### P0 — Core architecture / security / correctness
1. **EVM execution bypasses Signals and the DAG (§13.3, §13.5).** Txs run in a separate `revm` `EvmPool`, not as `Signal`s in a domain, and never enter the Vector-Clock DAG. Evidence: `evm_rpc.rs:369-409`, `src/evm/mod.rs:159-294`, `oscillator.rs:394-402`. → claims F & H are ❌.
2. **Commutative signals are never signature-verified.** Unauthenticated mutation of pool coordinates, contradicting §4.1/§9.1. Evidence: `routes.rs:409-460`, `oscillator.rs:204-227`.
3. **`u256_to_hex` produces invalid (decimal-as-hex) numbers** for `value`/`gasPrice`/`effectiveGasPrice` in `eth_getTransactionByHash` and receipts. Evidence: `evm_rpc.rs:169-171`, `420-424`, `543-545`. Breaks EVM-tooling correctness.

### P1 — Functional divergence
4. **Finalization depth mismatch:** SDK `FINALIZATION_DEPTH = 6` vs node `3`. Evidence: `constants.ts:5` vs `dag.rs:74`, `domain.rs:40`.
5. **Replay protection is signature-dedup, not nonce-based (§3, §4.1).** The signed `nonce` is unused for replay/monotonicity. Evidence: `oscillator.rs:60,218-221,246-248`.
6. **WebSocket feed is a single global snapshot, not domain-specific (§4.3).** Evidence: `websocket.rs:34-73`.
7. **RPC method gaps undercut "MetaMask/Hardhat/Foundry compatible" (§13):** no `eth_feeHistory`, `eth_maxPriorityFeePerGas`, `eth_accounts`, `eth_sendTransaction`, storage/filter/subscription methods; blocks return hash-only txs and no `baseFeePerGas`. Evidence: `evm_rpc.rs:108-133`, `173-234`.
8. **`net_version` returns hex** instead of a decimal network id. Evidence: `evm_rpc.rs:116,332-334`.

### P2 — Wording / surface / roadmap
9. **SDK `submitCommutative` drops the `domain` field** ⇒ custom-domain commutative hash/domain divergence. Evidence: `client.ts:120-131`.
10. **No domain-declaration API (§6).** `fluidic.domain({...})` absent in SDK; only the DEX domain seeded; no "strict"/quorum policy, `replicationFactor`, or `operators[]`. Evidence: `domain.rs:18-65`, SDK has no `domain()`.
11. **"Synthetic transaction hash" (§13.4)** is actually the genuine keccak Ethereum hash. Cosmetic. Evidence: `evm_rpc.rs:382,395,408`.
12. **Bridges (Ethereum/Solana) unimplemented (§8.3, §12.4).** Consistent with the §14 roadmap's "future" framing, but the architecture sections imply an existing role. Evidence: no bridge code in `src/` or `sdk/`.

---

## 9. Appendix — files inspected

**Rust (`src/`)**
- `api/server.rs`, `api/routes.rs`, `api/websocket.rs`, `api/evm_rpc.rs`, `api/state.rs` (referenced)
- `evm/mod.rs`, `evm/executor.rs`
- `crypto/mod.rs`, `crypto/phase_shift.rs`
- `consensus/oscillator.rs`, `consensus/dag.rs`, `consensus/domain.rs`, `consensus/certificate.rs`
- `network/tcp_gossip.rs` (bridge/inbound search), `bin/mesh_node.rs` (referenced)

**TypeScript (`sdk/typescript/src/`)**
- `client.ts`, `evm.ts`, `crypto.ts`, `shifts.ts`, `swap.ts`, `types.ts`, `constants.ts`, `index.ts`, `README.md`

*Correctness items (notably the `u256_to_hex` and `net_version` findings) were derived from source reading, not runtime execution, and should be confirmed with a live `/rpc` call.*
