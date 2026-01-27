//! Types for implied volatility calculation.

use serde::{Deserialize, Serialize};

/// Option type for IV calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    /// Call option (right to buy the underlying at strike price).
    Call,
    /// Put option (right to sell the underlying at strike price).
    Put,
}

/// Price source for IV calculation from order book.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceSource {
    /// Simple mid price: `(bid + ask) / 2`.
    #[default]
    MidPrice,
    /// Volume-weighted mid price based on quantities at best bid/ask.
    WeightedMid,
    /// Last traded price from the order book.
    LastTrade,
}

/// IV calculation quality indicator based on liquidity.
///
/// Quality is determined by the bid-ask spread at calculation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IVQuality {
    /// Spread < 100 bps (1%), high liquidity.
    High,
    /// Spread 100-500 bps (1-5%), moderate liquidity.
    Medium,
    /// Spread > 500 bps (5%), low liquidity.
    Low,
    /// Interpolated from nearby strikes (not directly calculated).
    Interpolated,
}

/// Parameters for IV calculation.
///
/// These parameters define the option contract and market conditions
/// needed to calculate implied volatility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IVParams {
    /// Underlying spot price in price units.
    pub spot: f64,
    /// Option strike price in price units.
    pub strike: f64,
    /// Time to expiration in years (e.g., 30 days = 30.0 / 365.0).
    pub time_to_expiry: f64,
    /// Risk-free interest rate (annualized, e.g., 0.05 for 5%).
    pub risk_free_rate: f64,
    /// Option type (Call or Put).
    pub option_type: OptionType,
}

impl IVParams {
    /// Creates new IV parameters.
    ///
    /// # Arguments
    /// - `spot`: Underlying spot price in price units
    /// - `strike`: Option strike price in price units
    /// - `time_to_expiry`: Time to expiration in years
    /// - `risk_free_rate`: Risk-free interest rate (annualized)
    /// - `option_type`: Call or Put
    #[must_use]
    pub fn new(
        spot: f64,
        strike: f64,
        time_to_expiry: f64,
        risk_free_rate: f64,
        option_type: OptionType,
    ) -> Self {
        Self {
            spot,
            strike,
            time_to_expiry,
            risk_free_rate,
            option_type,
        }
    }

    /// Creates parameters for a call option.
    #[must_use]
    pub fn call(spot: f64, strike: f64, time_to_expiry: f64, risk_free_rate: f64) -> Self {
        Self::new(
            spot,
            strike,
            time_to_expiry,
            risk_free_rate,
            OptionType::Call,
        )
    }

    /// Creates parameters for a put option.
    #[must_use]
    pub fn put(spot: f64, strike: f64, time_to_expiry: f64, risk_free_rate: f64) -> Self {
        Self::new(
            spot,
            strike,
            time_to_expiry,
            risk_free_rate,
            OptionType::Put,
        )
    }

    /// Calculates the intrinsic value of the option.
    ///
    /// For calls: max(0, spot - strike)
    /// For puts: max(0, strike - spot)
    #[must_use]
    pub fn intrinsic_value(&self) -> f64 {
        match self.option_type {
            OptionType::Call => (self.spot - self.strike).max(0.0),
            OptionType::Put => (self.strike - self.spot).max(0.0),
        }
    }

    /// Returns true if the option is in-the-money.
    #[must_use]
    pub fn is_itm(&self) -> bool {
        self.intrinsic_value() > 0.0
    }

    /// Returns true if the option is at-the-money (within 0.1% of strike).
    #[must_use]
    pub fn is_atm(&self) -> bool {
        (self.spot - self.strike).abs() / self.strike < 0.001
    }

    /// Returns true if the option is out-of-the-money.
    #[must_use]
    pub fn is_otm(&self) -> bool {
        !self.is_itm() && !self.is_atm()
    }
}

/// Result of IV calculation.
///
/// Contains the calculated implied volatility along with metadata
/// about the calculation quality and inputs used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IVResult {
    /// Calculated implied volatility (0.0 - 1.0+, e.g., 0.25 = 25%).
    pub iv: f64,
    /// Price used for calculation in price units.
    pub price_used: f64,
    /// Bid-ask spread at calculation time in basis points.
    pub spread_bps: f64,
    /// Number of solver iterations to converge.
    pub iterations: u32,
    /// Calculation quality based on liquidity.
    pub quality: IVQuality,
}

impl IVResult {
    /// Creates a new IV result.
    #[must_use]
    pub fn new(
        iv: f64,
        price_used: f64,
        spread_bps: f64,
        iterations: u32,
        quality: IVQuality,
    ) -> Self {
        Self {
            iv,
            price_used,
            spread_bps,
            iterations,
            quality,
        }
    }

    /// Returns the IV as a percentage (e.g., 25.0 for 25%).
    #[must_use]
    pub fn iv_percent(&self) -> f64 {
        self.iv * 100.0
    }

    /// Returns true if the calculation quality is high.
    #[must_use]
    pub fn is_high_quality(&self) -> bool {
        self.quality == IVQuality::High
    }

    /// Returns true if the calculation quality is acceptable (High or Medium).
    #[must_use]
    pub fn is_acceptable_quality(&self) -> bool {
        matches!(self.quality, IVQuality::High | IVQuality::Medium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_type_serialization() {
        let call = OptionType::Call;
        let json = serde_json::to_string(&call).unwrap();
        assert_eq!(json, "\"Call\"");

        let put = OptionType::Put;
        let json = serde_json::to_string(&put).unwrap();
        assert_eq!(json, "\"Put\"");
    }

    #[test]
    fn test_price_source_default() {
        let source = PriceSource::default();
        assert_eq!(source, PriceSource::MidPrice);
    }

    #[test]
    fn test_iv_params_intrinsic_value() {
        // ITM call
        let params = IVParams::call(110.0, 100.0, 0.25, 0.05);
        assert!((params.intrinsic_value() - 10.0).abs() < 1e-10);
        assert!(params.is_itm());

        // OTM call
        let params = IVParams::call(90.0, 100.0, 0.25, 0.05);
        assert!((params.intrinsic_value() - 0.0).abs() < 1e-10);
        assert!(params.is_otm());

        // ITM put
        let params = IVParams::put(90.0, 100.0, 0.25, 0.05);
        assert!((params.intrinsic_value() - 10.0).abs() < 1e-10);
        assert!(params.is_itm());

        // OTM put
        let params = IVParams::put(110.0, 100.0, 0.25, 0.05);
        assert!((params.intrinsic_value() - 0.0).abs() < 1e-10);
        assert!(params.is_otm());
    }

    #[test]
    fn test_iv_params_atm() {
        let params = IVParams::call(100.0, 100.0, 0.25, 0.05);
        assert!(params.is_atm());
        assert!(!params.is_itm());
        assert!(!params.is_otm());
    }

    #[test]
    fn test_iv_result_percent() {
        let result = IVResult::new(0.25, 10.0, 50.0, 5, IVQuality::High);
        assert!((result.iv_percent() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_iv_result_quality() {
        let high = IVResult::new(0.25, 10.0, 50.0, 5, IVQuality::High);
        assert!(high.is_high_quality());
        assert!(high.is_acceptable_quality());

        let medium = IVResult::new(0.25, 10.0, 200.0, 5, IVQuality::Medium);
        assert!(!medium.is_high_quality());
        assert!(medium.is_acceptable_quality());

        let low = IVResult::new(0.25, 10.0, 600.0, 5, IVQuality::Low);
        assert!(!low.is_high_quality());
        assert!(!low.is_acceptable_quality());
    }
}
