use crate::crypto::AccountId;
use dashmap::DashMap;

/// Unique identifier for a metabolic stream.
pub type StreamId = [u8; 32];

/// Basis-points denominator (10_000 = 100%).
pub const BASIS_POINTS_DENOMINATOR: u64 = 10_000;

/// Default exponential decay constant λ for the built-in DEX domain, expressed
/// in basis points per synthesis tick.  A value of `1` means each tick a
/// balance retains `(10_000 - 1) / 10_000 = 99.99%` of its value, i.e. 0.01%
/// decays away per tick.
pub const DEFAULT_DEX_LAMBDA_BP: u64 = 1;

/// Integer exponentiation by squaring for `u128`.
///
/// Uses `saturating_mul` so the result is always defined and identical across
/// all honest nodes even on overflow (consensus-critical determinism).  Callers
/// must keep `base^exp` within `u128` range to obtain mathematically exact
/// decay; see [`decayed_balance`].
pub fn pow(mut base: u128, mut exp: u64) -> u128 {
    let mut acc: u128 = 1;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = acc.saturating_mul(base);
        }
        exp >>= 1;
        if exp > 0 {
            base = base.saturating_mul(base);
        }
    }
    acc
}

/// Closed-form exponential decay of a balance over `elapsed` synthesis ticks:
///
/// ```text
/// B(elapsed) = B(0) * (10_000 - λ)^elapsed / 10_000^elapsed
/// ```
///
/// This is the discrete integer analogue of `B(t) = B(0) * e^(-λt)`.  All
/// arithmetic is integer-only and deterministic, so every honest node computes
/// the exact same remaining balance for a given `(balance, λ, elapsed)`.
pub fn decayed_balance(balance: u128, lambda_bp: u64, elapsed: u64) -> u128 {
    if balance == 0 || elapsed == 0 {
        return balance;
    }
    // Cap λ strictly below the denominator so a balance never fully vanishes in
    // a single tick and the retained fraction is always >= 1 / 10_000.
    let lambda_bp = lambda_bp.min(BASIS_POINTS_DENOMINATOR - 1);
    let retain = (BASIS_POINTS_DENOMINATOR - lambda_bp) as u128;
    let numerator = pow(retain, elapsed);
    let denominator = pow(BASIS_POINTS_DENOMINATOR as u128, elapsed);
    balance.saturating_mul(numerator) / denominator
}

/// A continuous value stream whose balance decays exponentially over synthesis
/// ticks following `B(t) = B(0) * e^(-λt)`.
///
/// Decay is driven by the logical synthesis tick, not wall-clock time, so every
/// honest node computes exactly the same remaining balance at the same tick.
#[derive(Clone, Debug)]
pub struct MetabolicStream {
    pub id: StreamId,
    pub owner: AccountId,
    /// Balance at `created_tick`, the anchor for the closed-form decay curve.
    pub initial_balance: u128,
    /// Synthesis tick at which the stream was created (decay anchor t=0).
    pub created_tick: u64,
    /// Per-domain decay constant λ in basis points per tick (capped < 10_000).
    pub lambda_bp: u64,
    /// Remaining balance after the most recent `process` call.
    pub remaining: u128,
    /// Last synthesis tick at which `process` advanced the stream.
    pub last_update_tick: u64,
}

impl MetabolicStream {
    /// Create a stream anchored at tick 0 with the given decay constant.
    pub fn new(id: StreamId, owner: AccountId, initial_balance: u128, lambda_bp: u64) -> Self {
        Self::new_at(id, owner, initial_balance, lambda_bp, 0)
    }

    /// Create a stream anchored at an explicit `created_tick`.
    pub fn new_at(
        id: StreamId,
        owner: AccountId,
        initial_balance: u128,
        lambda_bp: u64,
        created_tick: u64,
    ) -> Self {
        let lambda_bp = lambda_bp.min(BASIS_POINTS_DENOMINATOR - 1);
        Self {
            id,
            owner,
            initial_balance,
            created_tick,
            lambda_bp,
            remaining: initial_balance,
            last_update_tick: created_tick,
        }
    }

    /// Remaining balance at an absolute synthesis `tick`, computed from the
    /// closed-form exponential curve anchored at `created_tick`.
    pub fn remaining_at(&self, tick: u64) -> u128 {
        let elapsed = tick.saturating_sub(self.created_tick);
        decayed_balance(self.initial_balance, self.lambda_bp, elapsed)
    }

    /// Advance the stream to absolute synthesis `tick`.  Returns the value
    /// burned since the previous `process` call and whether the stream is now
    /// fully exhausted (remaining == 0).
    pub fn process(&mut self, tick: u64) -> (u128, bool) {
        let new_remaining = self.remaining_at(tick).min(self.remaining);
        let burned = self.remaining.saturating_sub(new_remaining);
        self.remaining = new_remaining;
        self.last_update_tick = tick;
        (burned, self.remaining == 0)
    }
}

/// Engine that owns all active metabolic streams and processes their decay
/// in a single pass over the oscillator's execution loop.
#[derive(Debug, Default)]
pub struct MetabolicDecayEngine {
    pub streams: DashMap<StreamId, MetabolicStream>,
    pub total_burned: std::sync::Mutex<u128>,
}

impl MetabolicDecayEngine {
    pub fn new() -> Self {
        Self {
            streams: DashMap::new(),
            total_burned: std::sync::Mutex::new(0),
        }
    }

    pub fn add_stream(&self, stream: MetabolicStream) {
        self.streams.insert(stream.id, stream);
    }

    /// Process every active stream once, burn the deterministic tick-based
    /// amount, remove exhausted streams, and return the total burned in tick.
    pub fn process_metabolic_degradation(&self, tick: u64) -> u128 {
        let mut tick_burn = 0u128;
        self.streams.retain(|_id, stream| {
            let (burned, exhausted) = stream.process(tick);
            tick_burn = tick_burn.saturating_add(burned);
            !exhausted
        });
        *self.total_burned.lock().unwrap() += tick_burn;
        tick_burn
    }

    pub fn active_stream_count(&self) -> usize {
        self.streams.len()
    }

    pub fn total_burned(&self) -> u128 {
        *self.total_burned.lock().unwrap()
    }

    /// Record externally-computed burn (e.g. wave-field decay) into the running
    /// total so reporting surfaces (API, persistence) stay accurate.
    pub fn record_burn(&self, amount: u128) {
        *self.total_burned.lock().unwrap() += amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::KeyPair;

    #[test]
    fn pow_uses_exponentiation_by_squaring() {
        assert_eq!(pow(1, 0), 1);
        assert_eq!(pow(7, 1), 7);
        assert_eq!(pow(2, 10), 1024);
        assert_eq!(pow(9_999, 2), 99_980_001);
        assert_eq!(pow(10_000, 3), 1_000_000_000_000);
    }

    #[test]
    fn decayed_balance_matches_closed_form() {
        // λ = 100 bp (1% per tick), retain 9_900/10_000.
        let initial = 1_000_000u128;
        assert_eq!(decayed_balance(initial, 100, 0), initial);
        assert_eq!(decayed_balance(initial, 100, 1), 990_000);
        // 1_000_000 * 9_900^2 / 10_000^2 = 1_000_000 * 98_010_000 / 100_000_000
        assert_eq!(decayed_balance(initial, 100, 2), 980_100);
        // 1_000_000 * 9_900^3 / 10_000^3
        assert_eq!(decayed_balance(initial, 100, 3), 970_299);
    }

    #[test]
    fn lambda_is_capped_below_denominator() {
        // λ >= 10_000 is clamped to 9_999 so at least 1/10_000 always survives.
        let owner = KeyPair::generate().account_id();
        let stream = MetabolicStream::new([9u8; 32], owner, 1_000_000, 50_000);
        assert_eq!(stream.lambda_bp, BASIS_POINTS_DENOMINATOR - 1);
        // One tick at the capped rate retains 1/10_000 of the balance.
        assert_eq!(stream.remaining_at(1), 100);
    }

    #[test]
    fn stream_remaining_follows_exponential_curve() {
        let owner = KeyPair::generate().account_id();
        let stream = MetabolicStream::new([1u8; 32], owner, 1_000_000, 100);
        assert_eq!(stream.remaining_at(0), 1_000_000);
        assert_eq!(stream.remaining_at(1), 990_000);
        assert_eq!(stream.remaining_at(2), 980_100);
        assert_eq!(stream.remaining_at(5), decayed_balance(1_000_000, 100, 5));
    }

    #[test]
    fn process_returns_incremental_burn_each_tick() {
        let owner = KeyPair::generate().account_id();
        let mut stream = MetabolicStream::new([2u8; 32], owner, 1_000_000, 100);

        let (burned, exhausted) = stream.process(1);
        assert_eq!(burned, 10_000); // 1_000_000 - 990_000
        assert!(!exhausted);
        assert_eq!(stream.remaining, 990_000);

        let (burned, exhausted) = stream.process(2);
        assert_eq!(burned, 9_900); // 990_000 - 980_100
        assert!(!exhausted);
        assert_eq!(stream.remaining, 980_100);

        // Re-processing the same tick burns nothing (idempotent).
        let (burned, _) = stream.process(2);
        assert_eq!(burned, 0);
    }

    #[test]
    fn engine_accumulates_exponential_burn() {
        let engine = MetabolicDecayEngine::new();
        let owner = KeyPair::generate().account_id();
        engine.add_stream(MetabolicStream::new([3u8; 32], owner, 1_000_000, 100));

        let burned = engine.process_metabolic_degradation(1);
        assert_eq!(burned, 10_000);
        assert_eq!(engine.total_burned(), 10_000);

        let burned = engine.process_metabolic_degradation(2);
        assert_eq!(burned, 9_900);
        assert_eq!(engine.total_burned(), 19_900);
        assert_eq!(engine.active_stream_count(), 1);
    }

    #[test]
    fn engine_removes_fully_exhausted_streams() {
        let engine = MetabolicDecayEngine::new();
        let owner = KeyPair::generate().account_id();
        // A tiny balance with the maximal capped rate decays to zero quickly.
        engine.add_stream(MetabolicStream::new([4u8; 32], owner, 1, 9_999));
        let burned = engine.process_metabolic_degradation(1);
        assert_eq!(burned, 1);
        assert_eq!(engine.active_stream_count(), 0);
    }
}
