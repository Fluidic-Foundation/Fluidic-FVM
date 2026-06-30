# Fluidic Consensus & Synthesis Audit

**Scope:** Whitepaper (`docs/FLUIDIC_WHITEPAPER.md`) vs. implementation (`src/`, `tests/`)
**Focus areas:** consensus, synthesis, vector-clock DAG, concurrency domains, finality
**Legend:** ✅ implemented · 🟡 partial / diverges · ❌ missing

---

## Executive Summary

The causal core — vector-clock DAG ordering, finality-by-depth, double-spend
rejection, signed synthesis certificates, and stake-weighted quorum/slashing —
is genuinely implemented and tested. The pieces that are **missing or materially
diverge** from the whitepaper are concentrated in:

1. **Metabolic decay** — dormant in production (no stream is ever created) and
   **linear**, not the exponential `B(t)=B(0)·e^(-λt)` the paper specifies.
2. **NTT "logarithmic-time" commutative merge** — the NTT is computed only as a
   round-trip *self-check*; real aggregation is plain O(n) sequential summation.
3. **Concurrency-domain model** — only one built-in domain; no developer
   declaration API, no `strict` quorum-per-signal policy, no per-domain
   isolation/gossip, no replication factor or operator set.
4. **Domain isolation / parallelism** — one global DAG, one global balance map,
   one global synthesis loop; "domains synthesized independently / in parallel"
   is not realized.
5. **Slashing** marks an operator ineligible but does **not** burn/zero stake.
6. **Bridges** and **domain-specific client feeds** are absent.

---

## 3. Signals vs. Transactions

| Whitepaper claim | Status | Evidence |
|---|---|---|
| Signal carries `from`, `domain`, `payload`, `predecessors`, `vector_clock`, `signature`, `nonce` | ✅ | `StatefulShift` has all fields — `src/crypto/phase_shift.rs:143-157`. Enum `Signal` — `src/crypto/phase_shift.rs:300-315`. |
| Ed25519 signature over canonical signing bytes | ✅ | `signing_bytes`/`verify_signature` — `src/crypto/phase_shift.rs:188-223`; keys — `src/crypto/keys.rs:87-94`. |
| `nonce` is an anti-replay counter **scoped to the sender-domain pair** | 🟡 | `nonce` is included in signing bytes but is **never validated** for monotonicity or uniqueness. Replay protection is done by deduplicating on the full signature instead — `src/consensus/oscillator.rs:218-221, 246-248`. No sender-domain nonce table exists. |
| Commutative payload is a Signal too | ✅ | `CommutativeShift` — `src/crypto/phase_shift.rs:75-139`. (Note: it has no `from`/`vector_clock`/`predecessors`, which is appropriate for commutative data.) |

---

## 4. The Wave-Field Engine

### 4.1 Ingest
| Claim | Status | Evidence |
|---|---|---|
| Verify the signature (at ingest) | 🟡 | The core engine `ingest_stateful`/`ingest_commutative` do **not** verify signatures; verification is deferred to `synthesize` (`src/consensus/oscillator.rs:291`, `367-378`) and to the HTTP route (`src/api/routes.rs:319-327`). So signatures *are* checked before state changes, but not at the ingest step the paper describes. |
| Replay protection via sender-domain nonce | 🟡/❌ | Done by **signature-hash dedup**, not nonce — `src/consensus/oscillator.rs:218-221, 246-248`. |
| Route Signal to the appropriate domain queue | 🟡 | Domain *policy* is looked up and unknown domains are rejected (`oscillator.rs:204-217, 229-245`), but there is a single global `pending_commutative` / `pending_stateful` queue (`oscillator.rs:56-59`), not per-domain queues. |
| Deduplicate using the Signal hash | 🟡 | Dedup key is the signature bytes, not the BLAKE3 signal hash — `oscillator.rs:218, 246`. |

### 4.2 Synthesis
| Claim | Status | Evidence |
|---|---|---|
| Synthesis happens continuously, not in blocks | ✅ | Interval-driven loop — `src/bin/mesh_node.rs:358-366`; monotonic tick — `oscillator.rs:272`. |
| Commutative ops batched and applied through an **NTT window** in **logarithmic time** | ❌ | `synthesize_commutative_batch` aggregates pools via a **plain sequential sum** (`src/field/wave_field.rs:116-123, 147-158`). The NTT forward+inverse transform is run only to *verify round-trip fidelity* (`wave_field.rs:127-143`) and to populate a `spectrum` vector that is never consumed for balances. Net effect: O(n) work for the real result plus O(n log n) of decorative NTT overhead — the opposite of the claimed speed-up. |
| Stateful ops inserted into Vector-Clock DAG; resolves deps, detects double-spends, rejects conflicts | ✅ | `VectorClockDag::insert` + `detect_double_spends` — `src/consensus/dag.rs:77-153, 329-364`; wired in `oscillator.rs:289-318`. |

### 4.3 Output
| Claim | Status | Evidence |
|---|---|---|
| Updated balances and state roots | ✅ | Balances synced to wave-field (`oscillator.rs:404-412`); roots in certificate (`certificate.rs:115-174`). |
| Synthesis certificate signed by the operator | ✅ | `SynthesisCertificate::sign` — `certificate.rs:36-71`; emitted in `oscillator.rs:414-446`. |
| Rejection **proofs** for invalid Signals | 🟡 | Rejections are tracked as a `Vec<DagError>` / `rejected` map (`dag.rs:48-49`, `oscillator.rs:292-318`); they are diagnostic enums, not cryptographic proofs. |
| Clients subscribe to **domain-specific feeds** | ❌ | The WebSocket broadcasts a single global DEX snapshot (price/reserves), with no domain selector — `src/api/websocket.rs:34-73`. |

---

## 5. The Vector-Clock DAG

| Claim | Status | Evidence |
|---|---|---|
| Each Signal carries a vector clock (map node→count) | ✅ | `VectorClock(BTreeMap<OscillatorId,u64>)` — `src/crypto/phase_shift.rs:30-72`. |
| A ≺ B iff componentwise ≤ with ≥1 strict inequality | ✅ | `happened_before` — `phase_shift.rs:56-66`; `concurrent_with` — `:69-71`. |
| Clock counts Signals "**that node has emitted in this domain**" | 🟡 | Two divergences: (a) the clock key is conflated — `validate_or_derive` keys the sender's own entry by **account id** (`sender.0`), while the DAG merges arbitrary node ids — `dag.rs:215-250`. (b) Clocks are **not scoped per domain**; `max_clock`/`account_tips` are global across one DAG — `dag.rs:50-53, 118-122`. |
| Insert verifies all declared predecessors exist | ✅ | `dag.rs:96-103`. |
| Insert checks signature validity | ✅ | `dag.rs:86-90`. |
| Insert verifies account balances and domain invariants | 🟡 | Balance conservation is enforced during apply, not at insert — `dag.rs:300-324` and `oscillator.rs:380-389`. Insert itself does no balance check. |
| Insert detects double-spends (concurrent spends of same balance) | ✅ | `detect_double_spends` (pairwise `concurrent_with`, deterministic tie-break by `inserted_at_tick` then hash) — `dag.rs:329-364`; invoked each tick — `oscillator.rs:309-318`. |
| Accepted on insert; Finalized after surviving `K` ticks without a conflicting double-spend | 🟡 | `ShiftStatus::{Accepted,Finalized}` — `dag.rs:4-13`. `promote_to_finalized` promotes purely on **tick depth** (`current_tick - inserted_at_tick >= depth`) — `dag.rs:160-181`; it does not re-evaluate conflicts at promotion time. In practice double-spend detection runs every tick and marks the later shift `Rejected` *before* promotion, so an accepted shift survives — behavior matches, mechanism is depth-only rather than "no-conflict-observed". |
| `K` is confirmation depth in synthesis ticks | ✅ | `FINALIZATION_DEPTH = 3` (`dag.rs:74`); per-domain override threaded through `oscillator.rs:297-304`. Validated by `tests/finality.rs:8-41`. |
| Natural parallelism: unrelated causal histories synthesized independently | ❌ | Single global DAG; `synthesize` walks one global `topological_order` sequentially — `oscillator.rs:344-392`. No independent/parallel synthesis of disjoint histories. |
| Local finality: a domain finalizes without waiting on other domains | 🟡 | Finality is per-shift by tick depth, but there is one DAG and one global balance map shared by all domains (`dag.rs:43-54`), so this is incidental, not domain-scoped isolation. |

**Tested:** `tests/finality.rs` (finalization depth + double-spend → exactly one finalizes, one rejected, conservation holds); `tests/convergence.rs` (independent nodes converge to identical balances/roots).

---

## 6. Concurrency Domains

| Claim | Status | Evidence |
|---|---|---|
| Domain = key + synthesis policy; unknown domains rejected | ✅ | `DomainId`, `DomainPolicy`, `DomainRegistry` — `src/consensus/domain.rs:18-78`; unknown-domain rejection at ingest — `oscillator.rs:204-217, 229-245`. |
| Developer declares a domain (`fluidic.domain({ id, policy, replicationFactor, operators })`) | ❌ | No API route registers domains — router list `src/api/routes.rs:16-38`. `DomainRegistry::new()` seeds **only** the built-in DEX domain (`domain.rs:58-65`); `register()` exists but is never exposed or called outside seeding. No `replicationFactor`, no `operators` field anywhere. |
| Policy values `"commutative" | "causal" | "strict"` | 🟡 | Code models orthogonal `commutative: bool` / `stateful: bool` flags plus `OrderingMode::{Dag, Fifo}` (`domain.rs:5-28`) — not the paper's three-way enum. `causal` ≈ `OrderingMode::Dag` ✅; `commutative` ≈ flag ✅; `strict` ❌. |
| **Strict** domains require explicit operator quorum per Signal | ❌ | No per-signal quorum path. Quorum exists only over *synthesis certificates* per tick (`certificate.rs:277-287`), not gating individual signals. |
| Commutative domains scale near-linearly with node count | 🟡 | Commutative signals supported, but aggregation is sequential (`wave_field.rs:116-123`); the scaling property is not demonstrated. |
| Domains gossip independently; a surge in one domain does not raise latency in another | ❌ | Single global gossip channel (no per-domain topics in `src/network/tcp_gossip.rs`) and a single shared synthesis loop / global pending queues (`oscillator.rs:56-59, 263-461`). No isolation between domains. |

---

## 7. The State Transition Function

`State' = Synthesize(State, Signals, Policy, Registry)` → `Oscillator::synthesize(&registry)` (`oscillator.rs:263-461`).

| Per-tick step | Status | Evidence |
|---|---|---|
| 1. Metabolic Decay | 🟡/❌ | Called at `oscillator.rs:274-279`, but the stream set is **always empty in production** (see §10) so it burns 0. |
| 2. Commutative Merge (NTT windows) | 🟡 | Applied (`oscillator.rs:326-342`) but via sequential sum, not NTT (see §4.2). |
| 3. DAG Apply (topo order, validate balances, reject double-spends) | ✅ | `oscillator.rs:344-392`; topo order `dag.rs:263-298`. |
| 4. Finalize (promote shifts past `K`) | ✅ | `oscillator.rs:320-323` → `dag.rs:160-181`. |
| 5. Emit Certificate (sign state root + rejection set) | 🟡 | Certificate signed over roots — `oscillator.rs:414-446`. Certificate does **not** include the rejection set (only counts/roots) — `certificate.rs:10-32`. |
| Honest nodes converge without a global consensus round | ✅ | `tests/convergence.rs:59-123`. Caveat: `SynthesisResult` also carries wall-clock `elapsed_ms`/`throughput_per_sec`/`avg_latency_ms` (`oscillator.rs:39-44`) which are non-deterministic, but the consensus-relevant `final_balances`/roots are deterministic. |

---

## 8. Network Architecture

| Claim | Status | Evidence |
|---|---|---|
| Operators stake WAVE, produce synthesis certificates | ✅ | `StakeTable` (`src/operator/stake.rs`), certificate signing (`oscillator.rs:414-446`). |
| Misbehavior (two conflicting certificates) detected and slashed | ✅ | `CertificateTracker::apply` slashes on conflicting `signing_bytes` for same `(operator,tick)` — `certificate.rs:232-273`; `tests/slashing.rs`. |
| Honest operators earn fees from metabolic burn | 🟡 | `RewardPool::distribute` routes 50% of burn to staked operators (`src/operator/rewards.rs:25-47`), but burn is ~0 in production (§10). |
| Clients submit signals + subscribe to feeds; light nodes verify certs | 🟡 | Submit endpoints exist (`routes.rs:22-23`); cert verification exists (`certificate.rs:94-99`); but feeds are global, not domain-specific (§4.3). |
| Bridges (Ethereum/Solana/DB/IoT) emit Signals on external events | ❌ | No bridge module. (An EVM JSON-RPC translation layer exists under `src/evm` + `src/api/evm_rpc.rs`, but that is the RPC gateway of §13, not a bridge domain.) |

---

## 9. Security Model

| Claim | Status | Evidence |
|---|---|---|
| Cryptographic accountability (every signal & certificate signed; conflicts non-repudiable) | ✅ | Signal/cert signing & verification — `phase_shift.rs:188-231`, `certificate.rs:36-99`. |
| Stake-weighted BFT quorum (>2/3) | ✅ | `quorum_threshold = total/3*2 + 1` — `stake.rs:102-109`; `check_quorum` — `certificate.rs:277-287`; `tests/quorum.rs`, `tests/network_partition.rs`. |
| Economic slashing **burns** the stake | 🟡 | `StakeTable::slash` sets `slash_nonce` (operator becomes ineligible via `is_staked`) — `stake.rs:111-131` — but the staked **amount is not zeroed/burned**; `get_stake` still returns the original amount. So it is disqualification, not stake-burning. |
| Causal finality after `K` ticks; **clients choose their own `K`** | 🟡 | `K` comes from the domain policy / global const (`dag.rs:74`, `domain.rs:24`), not from a per-client/per-request risk parameter. |
| Byzantine Causally Consistent (honest nodes agree on causal deps, reject conflicting spends) | ✅ (conceptually) | Demonstrated by convergence + double-spend tests; certificate quorum adds the >2/3 agreement layer. |

---

## 10. Metabolic Incentives

| Claim | Status | Evidence |
|---|---|---|
| Decay formula `B(t) = B(0) · e^(-λt)` (exponential) | ❌ | Implementation is **linear**: `burned = rate_per_tick · elapsed_ticks`, capped at remaining — `src/value/metabolic.rs:83-92`. No exponential / `e^(-λt)` term anywhere. `rate_per_tick` is a fixed per-tick amount (`metabolic.rs:55-78`), so balance decays linearly to zero, not asymptotically. |
| Decay actually applied to idle value | ❌ | `MetabolicStream`s are only ever created in unit tests. `add_stream`/`for_domain` have **no production caller** (grep shows only `src/value/metabolic.rs` tests). `mesh_node.rs` never seeds a stream, so `process_metabolic_degradation` iterates an empty map and returns 0 — `metabolic.rs:117-126`, `oscillator.rs:274-275`. Account balances are never linked to streams. |
| Decay redistributed to operators (and LPs) | 🟡 | `RewardPool::distribute` splits 50% of burn across staked operators by stake weight — `rewards.rs:25-47`. LPs are not a separate recipient class. Moot while burn is 0. |
| Penalizes passivity / funds synthesis / reduces spam | ❌ | None realized, since no decay runs in production. |

---

## 11–14. Other claims (consensus-adjacent)

| Claim | Status | Evidence |
|---|---|---|
| §13 EVM RPC gateway translates txns into Signals, returns synthetic hash, polls DAG for finalization | 🟡 | EVM executor + RPC present (`src/evm/`, `src/api/evm_rpc.rs`, ~28 KB) and EVM txns are synthesized into the same balance map (`oscillator.rs:394-402`); not deeply audited here. |
| §14 "Vector-Clock DAG with finality depth" | ✅ | As above. |
| §14 "Finality and adversarial-load test suites" | ✅ | `tests/finality.rs`, `tests/adversarial_load.rs`, `tests/quorum.rs`, `tests/slashing.rs`, `tests/convergence.rs`, `tests/network_partition.rs`, `tests/metabolic_overhead.rs`. |
| §14 "Synthesis certificates and slashing conditions" (listed as roadmap, but implemented) | ✅ | Implemented — `certificate.rs`. |

---

## Prioritized Mismatch List

1. **❌ Metabolic decay is dormant and linear** (`metabolic.rs:83-92`; no production `add_stream`). Whitepaper promises exponential `e^(-λt)` decay funding operators and curbing spam — none of it runs. **High impact** (core economic claim).
2. **❌ NTT "logarithmic-time" commutative merge is decorative** (`wave_field.rs:108-161`). The headline math claim (§4.2) is not how balances are computed.
3. **❌ Concurrency-domain model is largely unbuilt**: no declaration API, no `strict` policy, no per-domain gossip/queues/isolation, no replication factor/operators (`domain.rs`, `routes.rs:16-38`, `tcp_gossip.rs`). Undercuts §6 horizontal-scaling thesis.
4. **❌ No real parallel/independent synthesis** — single global DAG + balance map + synthesis loop (`oscillator.rs:263-461`).
5. **🟡 Slashing disqualifies but does not burn stake** (`stake.rs:122-131`).
6. **🟡 Replay protection by signature dedup, not sender-domain nonce; nonce unvalidated** (`oscillator.rs:218, 246`).
7. **🟡 Vector clock not domain-scoped and key conflated (account id vs node id)** (`dag.rs:215-250`).
8. **🟡 Client-selectable `K`, domain-specific feeds, rejection *proofs*, and bridges** are absent/partial.

## What Is Solid (no action needed)

- Vector-clock causal precedence + DAG topological apply (`dag.rs`).
- Finalization-by-depth with deterministic double-spend rejection and value conservation (`dag.rs`, `tests/finality.rs`).
- Signed synthesis certificates, equivocation detection, stake-weighted >2/3 quorum, partition→heal convergence (`certificate.rs`, `tests/quorum.rs`, `tests/slashing.rs`, `tests/network_partition.rs`).
- Deterministic multi-node convergence on balances/roots (`tests/convergence.rs`).
- Ed25519 signing/verification and account derivation (`crypto/keys.rs`, `crypto/phase_shift.rs`).
