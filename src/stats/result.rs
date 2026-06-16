use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A probability distribution stored as a histogram.
///
/// Maps each possible value to its occurrence count. The [`total`](ProbabilitiesResult::total)
/// field tracks the sum of all counts (i.e., total number of outcomes).
///
/// Use [`probability()`](ProbabilitiesResult::probability) to get P(X = value),
/// [`cumulative_probability()`](ProbabilitiesResult::cumulative_probability) for P(X <= value),
/// and [`stats()`](ProbabilitiesResult::stats) for summary statistics.
///
/// ```
/// use diceng::stats::ProbabilitiesResult;
///
/// let mut dist = ProbabilitiesResult::new();
/// dist.add(3);
/// dist.add(4);
/// dist.add(4);
///
/// assert_eq!(dist.total, 3);
/// assert!((dist.probability(4) - 2.0 / 3.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilitiesResult {
    /// Distribution: value -> count
    pub distribution: HashMap<i64, u64>,
    /// Total number of trials
    pub total: u64,
}

impl ProbabilitiesResult {
    /// Create a new empty probability distribution
    pub fn new() -> Self {
        Self {
            distribution: HashMap::new(),
            total: 0,
        }
    }

    /// Add a single observation
    pub fn add(&mut self, value: i64) {
        *self.distribution.entry(value).or_insert(0) += 1;
        self.total += 1;
    }

    /// Add multiple observations of the same value
    /// Returns false if overflow would occur
    pub fn add_quantity(&mut self, value: i64, quantity: u64) -> bool {
        let entry = self.distribution.entry(value).or_insert(0);
        *entry = match entry.checked_add(quantity) {
            Some(v) => v,
            None => return false,
        };
        self.total = match self.total.checked_add(quantity) {
            Some(v) => v,
            None => return false,
        };
        true
    }

    /// Merge another result into this one
    /// Returns false if overflow would occur
    pub fn merge(&mut self, other: &ProbabilitiesResult) -> bool {
        for (value, count) in &other.distribution {
            let entry = self.distribution.entry(*value).or_insert(0);
            *entry = match entry.checked_add(*count) {
                Some(v) => v,
                None => return false,
            };
        }
        self.total = match self.total.checked_add(other.total) {
            Some(v) => v,
            None => return false,
        };
        true
    }

    /// Get the probability of a specific value
    pub fn probability(&self, value: i64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        *self.distribution.get(&value).unwrap_or(&0) as f64 / self.total as f64
    }

    /// Get cumulative probability (at most)
    pub fn cumulative_probability(&self, value: i64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let count: u64 = self
            .distribution
            .iter()
            .filter(|(&k, _)| k <= value)
            .map(|(_, &v)| v)
            .sum();
        count as f64 / self.total as f64
    }

    /// Get reverse cumulative probability (at least)
    pub fn reverse_cumulative_probability(&self, value: i64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let count: u64 = self
            .distribution
            .iter()
            .filter(|(&k, _)| k >= value)
            .map(|(_, &v)| v)
            .sum();
        count as f64 / self.total as f64
    }

    /// Compute statistics from the distribution
    pub fn stats(&self) -> Stats {
        Stats::from_distribution(self)
    }

    /// Bucket the distribution (group values into buckets of given size)
    pub fn bucket(&self, bucket_size: i64) -> ProbabilitiesResult {
        let mut result = ProbabilitiesResult::new();
        for (&value, &count) in &self.distribution {
            let bucket_key = (value as f64 / bucket_size as f64).ceil() as i64;
            result.add_quantity(bucket_key, count);
        }
        result
    }
}

impl Default for ProbabilitiesResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for a probability distribution.
///
/// Computed by [`ProbabilitiesResult::stats()`]. Includes min, max,
/// mean, standard deviation, variance, sorted distribution, and percentiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// Minimum possible value
    pub min: i64,
    /// Maximum possible value
    pub max: i64,
    /// Mean (average)
    pub mean: f64,
    /// Standard deviation
    pub stddev: f64,
    /// Variance
    pub variance: f64,
    /// Distribution as sorted vector of (value, probability) pairs
    pub distribution: Vec<(i64, f64)>,
    /// Percentiles (25, 50, 75)
    pub percentiles: HashMap<u8, i64>,
}

impl Stats {
    /// Compute statistics from a probability distribution
    pub fn from_distribution(result: &ProbabilitiesResult) -> Self {
        if result.total == 0 || result.distribution.is_empty() {
            return Self {
                min: 0,
                max: 0,
                mean: 0.0,
                stddev: 0.0,
                variance: 0.0,
                distribution: Vec::new(),
                percentiles: HashMap::new(),
            };
        }

        let min = *result.distribution.keys().min().unwrap();
        let max = *result.distribution.keys().max().unwrap();

        // Compute mean
        let sum: f64 = result
            .distribution
            .iter()
            .map(|(&v, &c)| v as f64 * c as f64)
            .sum();
        let mean = sum / result.total as f64;

        // Compute variance
        let variance_sum: f64 = result
            .distribution
            .iter()
            .map(|(&v, &c)| {
                let diff = v as f64 - mean;
                diff * diff * c as f64
            })
            .sum();
        let variance = variance_sum / result.total as f64;
        let stddev = variance.sqrt();

        // Build sorted distribution
        let mut distribution: Vec<(i64, f64)> = result
            .distribution
            .iter()
            .map(|(&v, &c)| (v, c as f64 / result.total as f64))
            .collect();
        distribution.sort_by_key(|&(v, _)| v);

        // Compute percentiles
        let mut percentiles = HashMap::new();
        for &p in &[25, 50, 75] {
            let target = result.total as f64 * p as f64 / 100.0;
            let mut cumulative = 0u64;
            for (value, count) in &distribution {
                cumulative += (*count * result.total as f64) as u64;
                if cumulative as f64 >= target {
                    percentiles.insert(p, *value);
                    break;
                }
            }
        }

        Self {
            min,
            max,
            mean,
            stddev,
            variance,
            distribution,
            percentiles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probabilities_result() {
        let mut result = ProbabilitiesResult::new();
        result.add(1);
        result.add(2);
        result.add(2);
        result.add(3);

        assert_eq!(result.total, 4);
        assert_eq!(result.probability(1), 0.25);
        assert_eq!(result.probability(2), 0.5);
        assert_eq!(result.probability(3), 0.25);
    }

    #[test]
    fn test_cumulative_probability() {
        let mut result = ProbabilitiesResult::new();
        for i in 1..=6 {
            result.add(i);
        }

        assert_eq!(result.cumulative_probability(3), 0.5);
        assert_eq!(result.reverse_cumulative_probability(4), 0.5);
    }

    #[test]
    fn test_stats() {
        let mut result = ProbabilitiesResult::new();
        for _ in 0..100 {
            result.add(3); // All 3s
        }

        let stats = result.stats();
        assert_eq!(stats.min, 3);
        assert_eq!(stats.max, 3);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.stddev, 0.0);
    }
}
