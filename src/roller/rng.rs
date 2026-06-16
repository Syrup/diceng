use rand::Rng;

/// Abstraction over random number generation for dice rolls.
///
/// Implement this trait to plug in a custom randomness source.
/// The [`roll`](DiceRng::roll) method must return a value in `[1, sides]`.
///
/// Two built-in implementations ship with diceng:
/// - [`RandomRng`] for real randomness (uses the `rand` crate)
/// - [`LehmerRng`] for deterministic, seeded rolls
///
/// ```
/// use diceng::roller::DiceRng;
///
/// struct FixedRng(u32);
/// impl DiceRng for FixedRng {
///     fn roll(&mut self, sides: u32) -> u32 {
///         self.0.min(sides).max(1)
///     }
/// }
/// ```
pub trait DiceRng {
    /// Roll a die with the given number of sides.
    ///
    /// Must return a value in `[1, sides]`. Returning 0 when `sides == 0`
    /// is allowed as a special case.
    fn roll(&mut self, sides: u32) -> u32;
}

/// Random number generator backed by the operating system's entropy source.
///
/// Each call to [`roll`](DiceRng::roll) produces an unpredictable result.
/// This is the default RNG used by [`diceng::roll()`](crate::roll).
///
/// ```
/// use diceng::roller::{RandomRng, DiceRng};
///
/// let mut rng = RandomRng::new();
/// let value = rng.roll(6); // 1-6, different every time
/// ```
pub struct RandomRng;

impl RandomRng {
    /// Create a new random RNG using the system's random source
    pub fn new() -> Self {
        Self
    }
}

impl Default for RandomRng {
    fn default() -> Self {
        Self::new()
    }
}

impl DiceRng for RandomRng {
    fn roll(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        rand::thread_rng().gen_range(1..=sides)
    }
}

/// Lehmer (Park-Miller) PRNG for deterministic, reproducible rolls.
///
/// Uses the MINSTD variant with constants `a = 48271` and `m = 2^31 - 1`.
/// Same seed always produces the same sequence of rolls.
///
/// This is the RNG used by [`diceng::roll_seeded()`](crate::roll_seeded).
///
/// ```
/// use diceng::roller::{LehmerRng, DiceRng};
///
/// let mut rng1 = LehmerRng::new(42);
/// let mut rng2 = LehmerRng::new(42);
///
/// // Same seed, same results
/// assert_eq!(rng1.roll(6), rng2.roll(6));
/// assert_eq!(rng1.roll(6), rng2.roll(6));
/// ```
pub struct LehmerRng {
    seed: u32,
}

impl LehmerRng {
    const M: u32 = 2147483647; // 2^31 - 1 (Mersenne prime)
    const A: u32 = 48271; // Known good multiplier

    /// Create a new Lehmer RNG with the given seed.
    ///
    /// Seed 0 is treated as 1. Values larger than `m` are reduced modulo `m`.
    pub fn new(seed: u32) -> Self {
        // Ensure seed is in valid range [1, M-1]
        let seed = if seed == 0 { 1 } else { seed % Self::M };
        Self { seed }
    }

    /// Get the current seed value
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Advance to the next seed value
    pub fn next(&mut self) {
        self.seed = ((self.seed as u64 * Self::A as u64) % Self::M as u64) as u32;
    }

    /// Generate a float in [0, 1)
    pub fn float(&mut self) -> f64 {
        self.next();
        (self.seed as f64) / (Self::M as f64)
    }
}

impl DiceRng for LehmerRng {
    fn roll(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        (self.float() * sides as f64).floor() as u32 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lehmer_deterministic() {
        let mut rng1 = LehmerRng::new(12345);
        let mut rng2 = LehmerRng::new(12345);

        for _ in 0..100 {
            assert_eq!(rng1.roll(6), rng2.roll(6));
        }
    }

    #[test]
    fn test_lehmer_range() {
        let mut rng = LehmerRng::new(42);
        for _ in 0..1000 {
            let val = rng.roll(6);
            assert!(val >= 1 && val <= 6, "Value {} out of range [1, 6]", val);
        }
    }

    #[test]
    fn test_random_rng_range() {
        let mut rng = RandomRng::new();
        for _ in 0..1000 {
            let val = rng.roll(20);
            assert!(val >= 1 && val <= 20, "Value {} out of range [1, 20]", val);
        }
    }

    #[test]
    fn test_lehmer_different_seeds() {
        let mut rng1 = LehmerRng::new(1);
        let mut rng2 = LehmerRng::new(2);

        // Different seeds should produce different sequences (with high probability)
        let mut same = 0;
        for _ in 0..100 {
            if rng1.roll(6) == rng2.roll(6) {
                same += 1;
            }
        }
        // Very unlikely to be all the same
        assert!(same < 95, "Too many identical rolls: {}", same);
    }
}
