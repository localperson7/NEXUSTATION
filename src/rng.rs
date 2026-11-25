use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ENTROPY_COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

/// Deterministic pseudo-random number generator (Xoshiro256++) seeded via SplitMix64.
/// Keeps game logic 100% reproducible in tests while needing zero external RNG crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    /// Creates a deterministic generator from a 64-bit seed.
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let mut next_sm = || {
            sm = sm.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let mut state = [next_sm(), next_sm(), next_sm(), next_sm()];
        if state == [0; 4] {
            state[0] = 1;
        }
        Self { state }
    }

    /// Creates a generator seeded from wall-clock nanoseconds, PID, and an atomic counter.
    pub fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xCAFE_BABE_DEAD_BEEF);
        let pid = u64::from(std::process::id()).rotate_left(17);
        let seq = ENTROPY_COUNTER.fetch_add(0x517C_C1B7_2722_0A95, Ordering::Relaxed);
        Self::from_seed(nanos ^ pid ^ seq)
    }

    /// Generates the next 64-bit unsigned value.
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.state[0].wrapping_add(self.state[3]))
            .rotate_left(23)
            .wrapping_add(self.state[0]);

        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];

        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform index in `0..upper_exclusive`. Panics if `upper_exclusive == 0`.
    pub fn gen_range(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0, "upper_exclusive must be > 0");
        let bound = upper_exclusive as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let x = self.next_u64();
            if x >= threshold {
                return (x % bound) as usize;
            }
        }
    }

    /// Uniform integer in `min_inclusive..max_exclusive`.
    pub fn gen_range_i32(&mut self, min_inclusive: i32, max_exclusive: i32) -> i32 {
        assert!(min_inclusive < max_exclusive, "invalid range");
        let span = i64::from(max_exclusive) - i64::from(min_inclusive);
        min_inclusive + (self.gen_range(span as usize) as i32)
    }

    /// Float in `[0.0, 1.0)`.
    pub fn gen_f32(&mut self) -> f32 {
        let bits = (self.next_u32() >> 8) as f32;
        bits * (1.0 / ((1u32 << 24) as f32))
    }

    /// Returns `true` with `probability` in `[0.0, 1.0]`.
    pub fn gen_bool(&mut self, probability: f32) -> bool {
        self.gen_f32() < probability.clamp(0.0, 1.0)
    }

    /// In-place Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range(i + 1);
            slice.swap(i, j);
        }
    }
}
