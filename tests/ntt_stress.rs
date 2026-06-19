use fluidic::consensus::TuningForkMeshSimulation;
use std::time::Instant;

const NTT_SIZE: usize = 2048;
const UPDATE_COUNT: usize = 100_000;
const THREADS: usize = 32;

/// Multi-threaded stress test: 100,000 concurrent updates to the same
/// localized coordinate spectrum bin. The forward/inverse Cooley-Tukey
/// butterfly operations must recover the exact algebraic total with zero
/// rounding or floating-point drift.
#[test]
fn hundred_k_concurrent_updates_same_bin() {
    let sim = TuningForkMeshSimulation::new(NTT_SIZE);
    let start = Instant::now();

    let (algebraic, recovered) = sim.stress_bin(UPDATE_COUNT, THREADS);

    let elapsed = start.elapsed();
    println!("\n=== NTT Stress Test ===");
    println!("NTT size:              {}", NTT_SIZE);
    println!("Concurrent updates:    {}", UPDATE_COUNT);
    println!("Worker threads:        {}", THREADS);
    println!("Algebraic total:       {}", algebraic);
    println!("Recovered bin value:   {}", recovered);
    println!("Elapsed:               {:?}", elapsed);
    println!("=======================\n");

    assert_eq!(
        algebraic, recovered,
        "wave-field synthesis diverged from algebraic total: {} != {}",
        algebraic, recovered
    );
}
