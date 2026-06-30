# Fluidic Security Audit — Whitepaper vs. Implementation

**Scope:** Signatures, replay protection, double-spend detection, slashing, synthesis certificates, and cryptographic accountability.
**Whitepaper:** `docs/FLUIDIC_WHITEPAPER.md`
**Codebase:** `src/`, `tests/`
**Date:** 2026-06-30
**Method:** Static read-only review of source and tests. No code executed.

Legend: ✅ implemented · 🟡 partial / weaker than claimed · ❌ missing / contradicted

---

## Executive Summary

| # | Whitepaper claim | Status |
|---|---|---|
| 1 | "Every Signal is signed" / ingest "Verifies the signature" (§3, §4.1, §9.1) | 🟡 — stateful & EVM verified; **commutative signatures are never verified** |
| 2 | Synthesis certificates are signed and verified (§4.3, §9.1) | ✅ |
| 3 | EVM gateway "Verifies the ECDSA signature" (§13.1) | ✅ |
| 4 | `nonce`: anti-replay counter scoped to the sender-domain pair (§3, §4.1) | ❌ — no sender-domain nonce is tracked or enforced for native Signals |
| 5 | Double-spend detection of concurrent same-balance spends (§4.2, §5) | ✅ core, 🟡 robustness (checks only seed balances, unbounded re-scan) |
| 6 | Finality after surviving `K` synthesis ticks (§5, §7.4) | ✅ |
| 7 | Slashing: conflicting certificates "burns the stake" (§8.1, §9.2) | 🟡 — equivocation detected & operator disqualified, but **stake is not burned** |
| 8 | Synthesis certificate emits "state root **and rejection set**" + "Rejection proofs" (§4.3) | 🟡 — rich root set present; **no rejection set / rejection proofs** |
| 9 | Cryptographic accountability / non-repudiable misbehavior (§9.1) | 🟡 — gaps: unsigned registrations, self-asserted stake, unverified commutative |
| 10 | Metabolic decay `B(t)=B(0)·e^(-λt)` redistributed to operators (§7.1, §10) | ❌ — linear (not exponential) and **never wired** to account balances in the running node |

The cryptographic core for **stateful transfers** (Ed25519 signing, vector-clock DAG ordering, double-spend rejection, K-tick finality) and **operator certificates** (signing, equivocation slashing, stake-weighted quorum) is genuinely implemented and tested. The gaps cluster in three areas: (a) **commutative signals and registrations bypass signature verification**, (b) **replay protection does not use the sender-domain nonce the whitepaper specifies**, and (c) **economic enforcement is soft** — stake is self-asserted and slashing disqualifies rather than burns.

---

## 1. Signatures

### 1.1 Signal signing primitives — ✅
- `KeyPair` / Ed25519 via `ed25519-dalek`, `AccountId = blake3(pubkey)` — `src/crypto/keys.rs:1-95`.
- `StatefulShift` and `CommutativeShift` carry an Ed25519 `signature` over domain-separated canonical bytes — `src/crypto/phase_shift.rs:114-139` (commutative), `:188-231` (stateful).
- `StatefulShift::verify` additionally binds signer to `from`: `AccountId::from_public_key(pk) == self.from` — `src/crypto/phase_shift.rs:225-231`.

### 1.2 Stateful shift verification — ✅
Verified at three layers:
- API submission: `src/api/routes.rs:319-327` (`KeyPair::verify` over `signing_bytes`).
- DAG insertion: `src/consensus/dag.rs:86-90` (`verify_signature`).
- Synthesis re-check before applying balance: `src/consensus/oscillator.rs:373-378`.

### 1.3 Commutative shift verification — ❌ (contradicts §4.1 "Verifies the signature", §9.1 "Every Signal is signed")
`CommutativeShift::verify` exists (`src/crypto/phase_shift.rs:133-138`) but is **never called outside unit tests** (confirmed by grep — only `dag.rs`, `oscillator.rs:373`, `certificate.rs` call any `verify*`).
- `submit_commutative` builds the shift and ingests it **without verifying the signature** — `src/api/routes.rs:409-455`.
- `ingest_commutative` only checks domain policy and de-duplicates by raw signature bytes; **no signature check** — `src/consensus/oscillator.rs:204-227`.
- Effect: any client can submit a commutative shift with empty/garbage `signature` and mutate pool/coordinate state (e.g. liquidity deltas) — there is no authentication on the commutative path. The commutative-root in the certificate then commits to unauthenticated deltas.

### 1.4 Synthesis certificate signing — ✅
`SynthesisCertificate::sign` / `verify` over `FLUIDIC:SYNTHESIS:v3` bytes — `src/consensus/certificate.rs:34-99`. Produced per tick by staked operators — `src/consensus/oscillator.rs:414-446`. Verified on ingest — `src/consensus/certificate.rs:240-245`.

### 1.5 EVM ECDSA verification — ✅ (§13.1)
`EvmTransaction::decode_raw` RLP-decodes and recovers the sender via `recover_from()`, enforcing chain id — `src/evm/mod.rs:81-109`. `eth_sendRawTransaction` routes through it — `src/api/evm_rpc.rs:380`. Wrong-chain / bad-signature txs are rejected (test `rejects_wrong_chain_id`, `src/evm/mod.rs:418-423`).

---

## 2. Replay Protection

**Whitepaper (§3, §4.1):** `nonce` is an "anti-replay counter scoped to the sender-domain pair"; ingest "Checks replay protection via the sender-domain nonce."

### 2.1 Native Signals — ❌ for the stated mechanism / 🟡 in aggregate
- `nonce` fields exist and are covered by signatures (`phase_shift.rs:82,121` commutative; `:152,203` stateful) but **no `(sender, domain) → nonce` table is maintained or checked anywhere**. Grep for replay/nonce tables returns only EVM nonces and the unrelated slash-nonce.
- Actual de-dup is `Oscillator::seen_signatures` (a `DashMap<Vec<u8>, ()>`) — `src/consensus/oscillator.rs:60,218-221,246-255`. This is:
  - **in-memory only and never persisted** (`persistence/mod.rs` does not serialize it) → replay protection resets on every node restart;
  - keyed on exact signature bytes → only blocks byte-identical replays, not nonce reuse.
- Stateful shifts get incidental monotonic protection from vector-clock validation (`own == own_tip+1`) — `src/consensus/dag.rs:215-250`, enforced at API in `src/api/routes.rs:331-341`. This is causal-clock based, **not** the sender-domain nonce described.
- Commutative shifts have **no** replay protection beyond signature de-dup (and signatures are unverified — see §1.3).

### 2.2 EVM — ✅
Strict per-sender nonce sequencing: `expected = nonces[from]`, mismatches fail, success increments — `src/evm/mod.rs:195-202, 263`. Nonces are persisted — `src/persistence/mod.rs:170-174, 309-312`.

---

## 3. Double-Spend Detection

**Whitepaper (§4.2, §5):** detect "two concurrent Signals spending the same unspent balance."

### 3.1 Core mechanism — ✅
`VectorClockDag::detect_double_spends` groups shifts by sender, and for **vector-clock-concurrent** pairs whose cumulative amount exceeds balance, flags the later one `DagError::DoubleSpend` — `src/consensus/dag.rs:329-364`. Synthesis marks the loser `Rejected` and excludes it — `src/consensus/oscillator.rs:309-318, 360-363`.
Tested: `dag.rs:400-421`, `tests/finality.rs:43-98` (exactly one finalizes, one rejected), cross-node merge in `tests/adversarial_load.rs:192-249`.

### 3.2 Robustness caveats — 🟡
- **Balances checked are seed balances, not running balances.** `detect_double_spends` and `apply_ordered` read `self.balances` (`dag.rs:343, 304`), which is only ever seeded (`seed_balance`, registration) and **never updated with applied transfers** — synthesis writes results to `simulated_balances`/wave-field but never back to `dag.balances` (`oscillator.rs:354-412`). Each tick re-executes the **entire** DAG history against the original seed. Detection is correct for a fixed seed but does not reflect committed state evolution.
- **Unbounded growth / no pruning.** DAG nodes are retained forever (even finalized/rejected); detection is O(n²) per sender over all nodes ever seen (`dag.rs:344-361`) — a denial-of-service and memory-growth vector at scale (the code comment acknowledges "acceptable for prototype scale").
- **Post-finality conflicts are not revisited.** `promote_to_finalized` flips status irreversibly (`dag.rs:160-181`); a conflicting concurrent shift arriving after promotion is still detectable in-tick, but a finalized shift's status is never revoked.

---

## 4. Finality (`K`-tick confirmation depth)

**Whitepaper (§5, §7.4):** Accepted → Finalized after surviving `K` ticks without a conflicting double-spend.

- ✅ `FINALIZATION_DEPTH = 3` plus per-domain depth (`dag.rs:74`, `domain.rs:34-43`). `promote_to_finalized` promotes `Accepted` nodes once `current_tick - inserted_at_tick >= finalization_depth` — `src/consensus/dag.rs:160-181`. Driven each synthesis tick — `oscillator.rs:320-323`. Test `tests/finality.rs:7-41`.
- 🟡 Minor: promotion checks only tick-survival; it relies on the same-tick double-spend pass having already marked losers `Rejected`. A double-spend that becomes *acceptable* only after a balance change cannot occur here because balances never evolve in the DAG (§3.2), so in practice this holds for the seed model.

---

## 5. Slashing

**Whitepaper (§8.1, §9.2):** "signing two conflicting synthesis certificates ... is detected and slashed"; "Signing conflicting synthesis results **burns the stake**."

### 5.1 Equivocation detection — ✅
`CertificateTracker::apply` keys on `(operator, tick)`; a second certificate with different `signing_bytes` triggers `slash(operator)` and returns `ConflictingCertificate` — `src/consensus/certificate.rs:232-273`. Idempotent for identical resubmits. Wired into the node: `Oscillator::ingest_certificate` passes `stake_table.slash` as the slash callback — `src/consensus/oscillator.rs:125-138`; peer certs are ingested in `src/bin/mesh_node.rs:259-266`. Tests: `tests/slashing.rs`.

### 5.2 "Burns the stake" — 🟡 (not implemented as described)
`StakeTable::slash` only sets `slash_nonce = Some(n)`; **the staked amount is left intact** — `src/operator/stake.rs:122-131`. Consequences: `is_staked` returns false (`stake.rs:111-120`) so the operator can no longer sign/quorum, and `staked_operators()` excludes them so rewards stop (`rewards.rs:34-46`, test `rewards.rs:93-107`). But there is **no economic destruction/confiscation of WAVE** — slashing is *disqualification*, not *burning*. Slash state is persisted (`stake.rs:159-192`, `persistence/mod.rs:183-184, 327-330`).

### 5.3 Scope limits — 🟡
- Slashing fires **only** for certificate equivocation. Invalid-signature certs and unstaked-operator certs are rejected without penalty (`certificate.rs:240-248`) — consistent with "nothing to slash," but invalid Signal signatures (incl. unverified commutative, §1.3) create **no** accountability record.
- Detection requires a single node to observe both conflicting certificates via gossip; there is no fraud-proof broadcast that forces all peers to slash.

---

## 6. Synthesis Certificates

**Whitepaper (§4.3):** each tick emits "Updated domain balances and state roots," "A synthesis certificate signed by the operator," and "Rejection proofs for invalid Signals."

### 6.1 Structure & signing — ✅ (exceeds the minimal claim)
`SynthesisCertificate` commits to `commutative_root`, `stateful_root`, `balances_root`, `stake_root`, `reward_root`, `evm_root`, `metabolic_burned`, `tick`, `operator`, `timestamp_ns` — `src/consensus/certificate.rs:10-32`. Roots are canonical/deterministic (sorted) — `:115-174`. Only **staked** operators sign — `oscillator.rs:415-417`. Certificates are gossiped (`mesh_node.rs:369-375`) and exposed via API (`routes.rs:576-588, 730-760`).

### 6.2 Stake-weighted quorum — ✅ (extra, beyond whitepaper)
`QuorumView` aggregates stake per identical root-view; `check_quorum` requires `>2/3` total stake — `certificate.rs:177-198, 277-287`, threshold `stake.rs:101-109`. Tests: `tests/quorum.rs`.

### 6.3 Rejection set / rejection proofs — ❌
The certificate carries **no rejection set and no rejection proofs**. Rejections exist only as in-memory `DagError` values (`dag.rs:31-39`, `SynthesisResult.stateful_rejected`) and are logged (`mesh_node.rs:399-401`); they are never hashed into the certificate nor emitted as signed proofs. This directly misses the §4.3 "Rejection proofs for invalid Signals" and the §9.1 framing of rejections as non-repudiable evidence.

---

## 7. Cryptographic Accountability (cross-cutting)

**Whitepaper (§9.1):** "Every Signal is signed. Every synthesis certificate is signed. Invalid signatures and conflicting certificates are non-repudiable evidence of misbehavior."

### 7.1 Unsigned registrations forge state — ❌ (significant)
`RegistrationShift` has **no signature field** — `src/crypto/phase_shift.rs:236-244`. The gossip handler trusts it unconditionally: it registers `reg.account → reg.public_key` in the key registry and seeds 10,000 WAVE + 10,000 USDC — `src/bin/mesh_node.rs:222-235`, `src/consensus/oscillator.rs:186-202`. It never checks `reg.account == blake3(public_key)` or that the derived token accounts match. After PSK auth (§7.3), any peer can:
- overwrite/poison the `AccountId → VerifyingKey` mapping used to verify stateful shifts, and
- mint balances by registering arbitrary accounts.
This undermines the "every Signal is signed / accountable" guarantee at the membership layer.

### 7.2 Self-asserted stake — 🟡 (significant)
`apply_stake` verifies the `StakeShift` signature (operator owns the key) but **trusts the announced `amount` with no collateral check** — `src/consensus/oscillator.rs:170-182` (the comment concedes the announcement is "trusted"). A node can declare arbitrary stake, become `is_staked`, sign certificates, and contribute to the `>2/3` quorum — a Sybil/forgery path against the §8.1 staking and §9.2 economic-security assumptions. On boot each node auto-stakes a genesis balance to itself — `src/bin/mesh_node.rs:111-123`.

### 7.3 Gossip authentication is a shared secret — 🟡
Peer auth is a single network-wide pre-shared key proven via a keyed BLAKE3 hash (`Signal::Auth`) — `src/network/tcp_gossip.rs:13-14, 79-83, 230-259`. It is symmetric and non-attributable (no per-peer identity); once a peer authenticates, the ingest loop treats it as trusted (`mesh_node.rs:267-270`). Combined with §7.1/§7.2, an authenticated peer has broad unaccountable write power. If `FLUIDIC_PSK` is unset, no gossip auth runs at all (`mesh_node.rs:148-165`, `tcp_gossip.rs:95-108`).

### 7.4 What does hold — ✅
- Stateful transfers are end-to-end authenticated and signer-bound (§1.2).
- Certificates are signer-bound and equivocation is attributable and punished by disqualification (§5).
- EVM txs are ECDSA-verified (§1.5).

---

## 8. Metabolic Decay / Incentives (related accountability claim)

**Whitepaper (§7.1, §10):** `B(t) = B(0)·e^(-λt)`; decayed value redistributed to operators.

- ❌ **Not exponential.** `MetabolicStream::process` burns a **linear** `rate_per_tick · elapsed` — `src/value/metabolic.rs:83-92`. No `e^(-λt)` curve is implemented.
- ❌ **Not wired in production.** `MetabolicDecayEngine::add_stream` is called **only in unit tests** (grep). The running node never creates streams, so `process_metabolic_degradation(tick)` always returns 0 (`oscillator.rs:275`), `metabolic_burned` is always 0, and `RewardPool::distribute` distributes nothing (`oscillator.rs:276-279`, `rewards.rs:25-47`). Account balances in the wave-field never decay; streams are a separate, unused structure. The certificate's `metabolic_burned` field is therefore always 0 in practice.
- ✅ Reward split math (50% basis points, stake-proportional, slashed excluded) is correct *when* fed a non-zero burn (`rewards.rs:5-47`, tests `:78-107`).

---

## 9. Notable Secondary Findings

- **DAG balances never committed** (§3.2): `dag.balances` stays at seed values; full-history re-execution every tick. Correctness depends on the DAG never being pruned.
- **Stateful re-verification trusts registry binding:** `dag.insert` calls `verify_signature` (not `verify`), relying on the caller looking up the key by `shift.from` (`oscillator.rs:291, 367`). Sound *only if* the registry is trustworthy — which §7.1 shows it is not under gossip.
- **No persisted replay/seen-signature state** (§2.1): restart reopens a replay window for native signals.
- **Packet-size DoS guard exists** (`MAX_PACKET_SIZE`, `tcp_gossip.rs:17, 312-315`) but there is no rate-limiting or per-peer accounting on ingest.

---

## 10. Recommendations (priority order)

1. **Verify commutative signatures** at ingest/submission and bind them to a signer account (`ingest_commutative`, `submit_commutative`). (§1.3)
2. **Sign `RegistrationShift`** and verify `account == blake3(pubkey)` + derived-account derivation before registering keys or seeding balances. (§7.1)
3. **Implement sender-domain nonce replay protection** with a persisted `(sender, domain) → last_nonce` table, or document that vector-clock monotonicity replaces it (and persist the de-dup set). (§2.1)
4. **Make slashing economic** — zero/confiscate stake on equivocation, and persist a fraud proof — to match "burns the stake." (§5.2)
5. **Collateralize stake** instead of trusting announced amounts (verify against an on-chain/genesis balance). (§7.2)
6. **Add a rejection set / rejection proofs** to the synthesis certificate. (§6.3)
7. **Reconcile metabolic decay**: either wire streams to real account balances with the documented exponential curve, or revise the whitepaper to describe the linear, opt-in stream model. (§8)
8. **Bound DAG growth** (prune finalized history, commit balances) to remove the O(n²) re-scan and seed-only balance check. (§3.2)

---

## Appendix — Key File Map

| Concern | Primary files |
|---|---|
| Keys / Signals / signing bytes | `src/crypto/keys.rs`, `src/crypto/phase_shift.rs` |
| DAG, double-spend, finality | `src/consensus/dag.rs` |
| Certificates, slashing, quorum | `src/consensus/certificate.rs` |
| Synthesis orchestration | `src/consensus/oscillator.rs` |
| Stake / rewards | `src/operator/stake.rs`, `src/operator/rewards.rs` |
| Metabolic decay | `src/value/metabolic.rs` |
| API submission paths | `src/api/routes.rs`, `src/api/state.rs` |
| EVM gateway | `src/evm/mod.rs`, `src/api/evm_rpc.rs` |
| Gossip / auth | `src/network/tcp_gossip.rs`, `src/bin/mesh_node.rs` |
| Persistence | `src/persistence/mod.rs` |
| Tests | `tests/finality.rs`, `tests/slashing.rs`, `tests/quorum.rs`, `tests/adversarial_load.rs` |
