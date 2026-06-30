# Fluidic Economics Audit — Whitepaper vs. Implementation

**Scope:** Metabolic incentives, rewards, staking, slashing, fees, token economics, operator economics.
**Whitepaper:** `docs/FLUIDIC_WHITEPAPER.md`
**Code audited:** `src/` and `tests/` (Rust `fluidic` crate)
**Legend:** ✅ implemented · 🟡 partial / mismatched · ❌ missing

## Executive summary

The cryptographic/consensus economics primitives (stake table, conflicting-certificate slashing, stake-weighted BFT quorum, proportional reward split) are implemented and tested. The **economic value layer is largely inert or absent**:

- **Metabolic decay is never wired to balances.** No production code path ever creates a `MetabolicStream`; `add_stream` appears only in tests/benches. In a running node `metabolic_burned == 0` every tick, so reward distribution is always 0.
- **Decay is linear, not exponential** as the whitepaper's `B(t) = B(0)·e^(−λt)` specifies.
- **No fee system exists** at all (domain fees, fee market, per-Signal fees) — only a metabolic multiplier field.
- **No supply cap / "1 billion fixed supply"** is enforced; accounts are minted freely via faucet/registration.
- **Staking does not escrow real WAVE**, and **slashing does not burn/confiscate stake** — it only flags the operator ineligible.
- **No bridge / bridge-bond** code.

---

## 1. Metabolic Incentives (Whitepaper §7.1, §10)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 1.1 | Idle value decays over time: `B(t) = B(0)·e^(−λt)` | 🟡 mismatch | Decay is **linear**, not exponential. `MetabolicStream::process` computes `burned = rate_per_tick · elapsed_ticks` (`src/value/metabolic.rs:83-92`). `domain_decay_rate_per_tick` is a fixed per-tick subunit amount derived once from the *initial* balance (`src/value/metabolic.rs:68-78`), so it never compounds against the shrinking balance. No `exp`/`e^` anywhere. |
| 1.2 | `λ` metabolic decay rate set per domain | 🟡 partial | A per-domain multiplier exists: `DomainPolicy.metabolic_multiplier_bp` (`src/consensus/domain.rs:25-27`) over a global base `BASE_METABOLIC_DECAY_BASIS_POINTS_PER_TICK = 1` (`src/value/metabolic.rs:17`). But it parameterizes a **linear** rate, not the exponential `λ`. Decay is driven by logical synthesis tick, not wall-clock `t`. |
| 1.3 | Decay applied each synthesis tick ("Metabolic Decay" step of `Synthesize`) | 🟡 partial | The synthesis loop calls `process_metabolic_degradation(tick)` first (`src/consensus/oscillator.rs:274-279`). The step exists, but see 1.4 — there are no streams to process. |
| 1.4 | "Penalizes passivity" / "Empty accounts and unused state naturally evaporate" | ❌ not wired | Only explicitly-added `MetabolicStream`s decay; ordinary account balances (`WaveField`/DAG balances) never decay. **No production path creates a stream** — `add_stream`/`MetabolicStream::new`/`for_domain` are referenced only in `src/value/metabolic.rs` tests (lines 146-198), `tests/metabolic_overhead.rs:72`, and `benches/metabolic_bench.rs:18-19`. The `mesh_node` binary and API never add streams, so `metabolic_burned` is always 0 in practice. |
| 1.5 | Decay "redistributed to operators **and liquidity providers**" | 🟡 partial | `RewardPool::distribute` credits **only staked operators**, proportional to stake (`src/operator/rewards.rs:25-47`). Liquidity providers receive nothing. Wired in `src/consensus/oscillator.rs:276-279`. |
| 1.6 | "Funds synthesis … **without inflating supply**" | 🟡 partial | Only `REWARD_BASIS_POINTS = 5000` (50%) of the burned amount is redistributed (`src/operator/rewards.rs:7,29`); the remaining 50% is dropped (neither credited nor tracked as recovered). Rewards accrue in a separate `RewardPool`, decoupled from the streams that were burned. Because 1.4 makes burn 0, this never actually funds anything. |

---

## 2. Rewards (Whitepaper §8.1, §10)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 2.1 | Operators earn fees from the metabolic burn | 🟡 partial | Distribution logic exists and is correct (proportional split, rounding remainder retained) `src/operator/rewards.rs:25-47`; unit test `rewards_split_proportionally_to_stake` (`:78-91`). But burn input is always 0 at runtime (see 1.4), so accrual is effectively dead. |
| 2.2 | Operators earn from **domain-specific usage fees** | ❌ missing | No fee accrual anywhere; `RewardPool` is fed only by `distribute(metabolic_burned, …)` (`src/consensus/oscillator.rs:278`). Doc comment claims "from metabolic burn and fees" (`src/operator/rewards.rs:9`) but no fee path exists. |
| 2.3 | Slashed operators receive no rewards | ✅ implemented | `staked_operators()` filters out slashed entries (`src/operator/stake.rs:82-90,111-113`), and `distribute` iterates only those. Test `slashed_operator_gets_no_rewards` (`src/operator/rewards.rs:93-107`). |
| 2.4 | Rewards are claimable/withdrawable | 🟡 partial | `RewardPool::claim` exists (`src/operator/rewards.rs:53-58`) but is **not exposed by any API route**. The router only offers read-only `GET /api/operator/:id/rewards` → balance (`src/api/routes.rs:33,590-608`). Accrued rewards cannot be moved into a spendable balance. |
| 2.5 | Reward state is committed in the synthesis certificate | ✅ implemented | `reward_root` over the pool is included in each `SynthesisCertificate` and signed (`src/operator/rewards.rs:61-69`; `src/consensus/certificate.rs:26,46,86`; `src/consensus/oscillator.rs:422,438`). |

---

## 3. Staking (Whitepaper §8.1, §9, §11.1)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 3.1 | To become an operator, a node stakes WAVE | 🟡 partial | A permissioned `StakeTable` tracks operator stake (`src/operator/stake.rs:33-71`); eligibility requires `stake ≥ min_stake` and not slashed (`:111-113`). However staking is just a **signed announcement** — `StakeShift::sign` (`src/crypto/phase_shift.rs:270-283`) and `apply_stake` (`src/consensus/oscillator.rs:175-182`) set the table value but **do not deduct/escrow WAVE from any account balance**. The comment admits the announcement is "trusted" (`src/consensus/oscillator.rs:170-174`). No economic bond is actually locked. |
| 3.2 | Minimum stake to be eligible | ✅ implemented | `StakingConfig.min_stake` default `1_000_000_000_000_000_000` = 1,000,000 WAVE at 10¹² sub-units (`src/operator/stake.rs:14-21`). Enforced in `is_staked` (`:111-120`). |
| 3.3 | Stake-weighted BFT quorum (>2/3) | ✅ implemented | `quorum_threshold = total/3*2 + 1` (`src/operator/stake.rs:101-109`); used by `check_quorum` (`src/consensus/oscillator.rs:140-144`; `src/consensus/certificate.rs:277-287`). Tests in `tests/quorum.rs`. |
| 3.4 | Stake state survives restarts | ✅ implemented | `to_snapshot`/`from_snapshot` persist stake and slash state (`src/operator/stake.rs:158-192`); test `snapshot_roundtrip_preserves_stake_and_slash` (`:249-260`). |
| 3.5 | Quorum denominator consistency | 🟡 bug | `total_stake()` sums **all** entries including slashed ones (`src/operator/stake.rs:92-99`), so a slashed operator's stake still inflates the quorum threshold denominator while being excluded from `staked_operators()` numerator. This can make quorum unreachable after slashing. |

---

## 4. Slashing (Whitepaper §8.1, §9)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 4.1 | Signing two conflicting synthesis certificates is detected | ✅ implemented | `CertificateTracker::apply` compares `signing_bytes` per `(operator, tick)` and triggers slash on divergence (`src/consensus/certificate.rs:232-273`). Wired via `ingest_certificate` (`src/consensus/oscillator.rs:125-138`). Test `conflicting_certificates_slash_operator` (`tests/slashing.rs:6-31`); idempotent identical certs do not slash (`tests/slashing.rs:38-54`). |
| 4.2 | Slashing **burns the stake** ("burns the stake", "makes Byzantine behavior expensive") | 🟡 mismatch | `slash()` only records a `slash_nonce` flag (`src/operator/stake.rs:122-131`); it does **not** zero or burn `entry.stake`. The operator becomes ineligible but the staked amount is neither confiscated nor redistributed. Since stake was never escrowed from a balance (3.1), there is no economic loss — slashing is purely an exclusion flag. |
| 4.3 | Reject invalid-signature / unstaked-operator certificates | ✅ implemented | `SlashingReason::{InvalidSignature, UnstakedOperator, ConflictingCertificate}` (`src/consensus/certificate.rs:200-206`); enforced in `apply` (`:240-256`). |
| 4.4 | Duplicate identity (same node ID) self-slash | ✅ implemented (operationally) | Two nodes sharing an `OSCILLATOR_ID` produce conflicting certs and slash each other (documented `README.md:34`, `docker-compose.yml:12`); follows from 4.1. |

---

## 5. Fees & Fee Market (Whitepaper §3 table, §11.1, §11.3)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 5.1 | Domain fees paid by dApps to reserve concurrency domains | ❌ missing | `DomainPolicy` has no fee fields (`src/consensus/domain.rs:18-28`); `DomainRegistry::register` is free and unmetered (`:67-69`). |
| 5.2 | Per-domain fee market: flat per-Signal, percentage, or metabolic-only | ❌ missing | No fee model exists. Grep for `fee`/`Fee`/`FEE` across `src/` yields only a doc comment in `src/operator/rewards.rs:9`. Signals carry no fee field (`src/crypto/phase_shift.rs`). The only per-domain economic knob is `metabolic_multiplier_bp`. |
| 5.3 | "No global gas market" / predictable economics | 🟡 partial | True that there is no global gas market — but only because no fee market of any kind is implemented. EVM gas is metered by `revm` for receipts (`gas_used`, `effective_gas_price` in `src/evm/mod.rs:54-56,205-212,281-283`) but gas is **not** charged/burned against WAVE; it is informational. |
| 5.4 | Signal failure cost = "Signal rejected, no burn" | ✅ implemented | Rejected stateful shifts are collected into `stateful_rejected` with no balance/burn side effect (`src/consensus/oscillator.rs:289-318,367-386`). |
| 5.5 | Reserving domain capacity by staking (closest analogue) | 🟡 partial | `Spectrum`/`BandLease` lets an owner stake `staked_amount` for a frequency band with a `throughput_quota` (`src/value/spectrum.rs:5-119`). This is a reservation-via-stake mechanism but is **not a fee**, is not connected to `DomainRegistry`/domains, and is exercised only in unit tests. |

---

## 6. Token Economics (Whitepaper §11)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 6.1 | Native token WAVE with fixed-point sub-units (10¹²) | ✅ implemented | `WAVE_PRECISION = 1_000_000_000_000` (`src/field/wave_field.rs:7`); `WAVE_SUBUNIT` re-export (`src/lib.rs:35-36`). |
| 6.2 | **Fixed supply of 1 billion WAVE; no further issuance** | ❌ missing | No total-supply constant, cap, or accounting anywhere (no `TOTAL_SUPPLY`/`max_supply`/`1_000_000_000` cap). Accounts are minted ad hoc: registration seeds 10,000 WAVE-equiv per account (`src/consensus/oscillator.rs:186-202`), API register/faucet seeds 1,000 WAVE + 1,000 USDC each (`src/api/routes.rs:169-170,213`), genesis seeds 1,000,000 WAVE (`src/bin/mesh_node.rs:111`). Supply is unbounded and uncapped. |
| 6.3 | "Operator rewards come from metabolic decay and usage fees, **not minting**" | 🟡/❌ | Rewards come from decay distribution only (no fees, 2.2), and that path is inert (1.4). Meanwhile new WAVE is freely minted via faucet/registration, contradicting the non-inflationary claim. |
| 6.4 | Token use: staking | 🟡 partial | See §3 — registry exists, no real lock. |
| 6.5 | Token use: domain fees | ❌ missing | See §5.1–5.2. |
| 6.6 | Token use: metabolic redistribution | 🟡 partial | See §1.5–1.6, §2.1. |
| 6.7 | Token use: **bridge bonds** | ❌ missing | No bridge module/bonds anywhere in `src/`. Whitepaper §14 lists bridges as future roadmap, consistent with absence, but the §11.1 token-use claim is unbacked. |

---

## 7. Operator Economics (Whitepaper §8.1, §14)

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 7.1 | Operators run the Wave-Field engine and produce signed synthesis certificates | ✅ implemented | `Oscillator::synthesize` signs a `SynthesisCertificate` when the operator is staked (`src/consensus/oscillator.rs:414-446`); cert signing/verify (`src/consensus/certificate.rs:34-99`). |
| 7.2 | Operator registry & staking **contracts** | 🟡 partial | Implemented as an in-memory permissioned `StakeTable` (`src/operator/stake.rs`), explicitly "permissioned operator model… not yet… permissionless mainnet" (`src/lib.rs:11-13`). On-chain staking contracts remain roadmap (Whitepaper §14, item 1). |
| 7.3 | Certificates gossiped; quorum formed; tick finalized | ✅ implemented | Gossip + quorum check in node loop (`src/bin/mesh_node.rs:369-380`); `GET /api/quorum/:tick` (`src/api/routes.rs:543-574`). |
| 7.4 | Causal finality after `K` ticks | ✅ implemented | `FINALIZATION_DEPTH = 3` default and per-domain `finalization_depth` (`src/consensus/dag.rs:73-74`; `src/consensus/domain.rs:24,38-43`); `promote_to_finalized` (`src/consensus/dag.rs:160-180`). |
| 7.5 | Operator earns net positive economics for honest work | ❌ effectively missing | With burn=0 (1.4), no fees (2.2), unclaimable rewards (2.4), and no real stake lock (3.1), there is no functioning operator profit/loss loop in the current code. |

---

## Key mismatches (prioritized)

1. **Metabolic decay is dead at runtime** — no production code creates `MetabolicStream`s, so burn and all downstream rewards are always 0. (`src/value/metabolic.rs`, callers only in tests/benches.)
2. **Decay model is linear, not exponential** `e^(−λt)`. (`src/value/metabolic.rs:68-92`.)
3. **No fee system** of any kind — directly contradicts §11.1/§11.3. (`src/consensus/domain.rs`, `src/crypto/phase_shift.rs`.)
4. **No fixed/1B supply cap; unbounded faucet minting** — contradicts §11.2. (`src/api/routes.rs:169-170,213`, `src/consensus/oscillator.rs:186-202`.)
5. **Staking doesn't escrow WAVE and slashing doesn't burn stake** — undermines the §9 "Byzantine behavior is expensive" guarantee. (`src/operator/stake.rs:64-71,122-131`.)
6. **Rewards omit liquidity providers and are unclaimable** via API; only 50% of burn is redistributed. (`src/operator/rewards.rs:7,25-47,53`.)
7. **Slashed stake still counts toward the quorum denominator.** (`src/operator/stake.rs:92-99`.)
8. **Bridge bonds absent.** (No bridge code in `src/`.)

## What is solid (matches whitepaper)

- Conflicting-certificate detection → slash flag, with idempotency. (`src/consensus/certificate.rs:232-273`, `tests/slashing.rs`.)
- Stake-weighted >2/3 BFT quorum over certificate root-views. (`src/operator/stake.rs:101-109`, `tests/quorum.rs`, `tests/network_partition.rs`.)
- Proportional reward split excluding slashed operators (logic correct, input inert). (`src/operator/rewards.rs`.)
- Per-domain `K`-tick causal finality. (`src/consensus/dag.rs`, `src/consensus/domain.rs`.)
- Certificates commit stake-root and reward-root, signed and verifiable. (`src/consensus/certificate.rs`.)
- WAVE sub-unit precision (10¹²). (`src/field/wave_field.rs:7`, `src/lib.rs:35-36`.)
