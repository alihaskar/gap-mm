//! Aggregate statistics for order book analysis
//!
//! This module provides comprehensive statistical analysis of order book depth,
//! helping quantitative traders detect market conditions, identify trends,
//! and make informed trading decisions.

use serde::{Deserialize, Serialize};

/// Depth statistics for one side of the order book
///
/// Provides comprehensive metrics about liquidity distribution and depth
/// characteristics. All quantities are in base units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepthStats {
    /// Total volume across all analyzed levels (in units)
    pub total_volume: u64,

    /// Number of price levels analyzed
    pub levels_count: usize,

    /// Average size per level (in units)
    pub avg_level_size: f64,

    /// Volume-weighted average price (in price units)
    pub weighted_avg_price: f64,

    /// Smallest level size found (in units)
    pub min_level_size: u64,

    /// Largest level size found (in units)
    pub max_level_size: u64,

    /// Standard deviation of level sizes (in units)
    pub std_dev_level_size: f64,
}

impl DepthStats {
    /// Creates a new `DepthStats` with zero values
    ///
    /// Useful as a default when no levels exist.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            total_volume: 0,
            levels_count: 0,
            avg_level_size: 0.0,
            weighted_avg_price: 0.0,
            min_level_size: 0,
            max_level_size: 0,
            std_dev_level_size: 0.0,
        }
    }

    /// Returns true if statistics represent an empty order book side
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.levels_count == 0 || self.total_volume == 0
    }
}

/// Distribution bin for depth distribution analysis
///
/// Represents a price range and the total volume within that range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionBin {
    /// Minimum price of this bin (inclusive, in price units)
    pub min_price: u128,

    /// Maximum price of this bin (exclusive, in price units)
    pub max_price: u128,

    /// Total volume in this price range (in units)
    pub volume: u64,

    /// Number of price levels in this bin
    pub level_count: usize,
}

impl DistributionBin {
    /// Returns the midpoint price of this bin
    #[must_use]
    pub fn midpoint(&self) -> u128 {
        (self.min_price + self.max_price) / 2
    }

    /// Returns the width of this bin in price units
    #[must_use]
    pub fn width(&self) -> u128 {
        self.max_price.saturating_sub(self.min_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_stats_zero() {
        let stats = DepthStats::zero();

        assert_eq!(stats.total_volume, 0);
        assert_eq!(stats.levels_count, 0);
        assert_eq!(stats.avg_level_size, 0.0);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_depth_stats_not_empty() {
        let stats = DepthStats {
            total_volume: 100,
            levels_count: 5,
            avg_level_size: 20.0,
            weighted_avg_price: 50000.0,
            min_level_size: 10,
            max_level_size: 30,
            std_dev_level_size: 5.0,
        };

        assert!(!stats.is_empty());
    }

    #[test]
    fn test_distribution_bin_midpoint() {
        let bin = DistributionBin {
            min_price: 100,
            max_price: 200,
            volume: 50,
            level_count: 3,
        };

        assert_eq!(bin.midpoint(), 150);
        assert_eq!(bin.width(), 100);
    }
}
