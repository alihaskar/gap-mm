//! Black-Scholes pricing model and Greeks calculation.
//!
//! This module provides a lightweight implementation of the Black-Scholes
//! option pricing model for use in implied volatility calculations.

use super::types::{IVParams, OptionType};
use std::f64::consts::PI;

/// Square root of 2, precomputed for efficiency.
const SQRT_2: f64 = std::f64::consts::SQRT_2;

/// Black-Scholes pricing model implementation.
///
/// Provides methods for calculating option prices and Greeks
/// using the Black-Scholes-Merton formula.
pub struct BlackScholes;

impl BlackScholes {
    /// Approximation of the error function (erf).
    ///
    /// Uses Abramowitz and Stegun approximation (formula 7.1.26)
    /// with maximum error of 1.5×10⁻⁷.
    ///
    /// # Arguments
    /// - `x`: Input value
    ///
    /// # Returns
    /// Approximation of erf(x)
    #[must_use]
    pub fn erf(x: f64) -> f64 {
        // Constants for the approximation
        const A1: f64 = 0.254829592;
        const A2: f64 = -0.284496736;
        const A3: f64 = 1.421413741;
        const A4: f64 = -1.453152027;
        const A5: f64 = 1.061405429;
        const P: f64 = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + P * x);
        let y = 1.0 - (((((A5 * t + A4) * t) + A3) * t + A2) * t + A1) * t * (-x * x).exp();

        sign * y
    }

    /// Standard normal cumulative distribution function (CDF).
    ///
    /// Calculates P(Z ≤ x) where Z is a standard normal random variable.
    ///
    /// # Arguments
    /// - `x`: Input value
    ///
    /// # Returns
    /// Probability that a standard normal variable is less than or equal to x
    #[must_use]
    pub fn norm_cdf(x: f64) -> f64 {
        0.5 * (1.0 + Self::erf(x / SQRT_2))
    }

    /// Standard normal probability density function (PDF).
    ///
    /// Calculates the density of the standard normal distribution at x.
    ///
    /// # Arguments
    /// - `x`: Input value
    ///
    /// # Returns
    /// Density value at x
    #[must_use]
    pub fn norm_pdf(x: f64) -> f64 {
        (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
    }

    /// Calculates the d1 parameter of the Black-Scholes formula.
    ///
    /// d1 = [ln(S/K) + (r + σ²/2)T] / (σ√T)
    ///
    /// # Arguments
    /// - `spot`: Current underlying price (S)
    /// - `strike`: Option strike price (K)
    /// - `rate`: Risk-free interest rate (r)
    /// - `time`: Time to expiration in years (T)
    /// - `vol`: Volatility (σ)
    ///
    /// # Returns
    /// The d1 parameter value
    #[must_use]
    pub fn d1(spot: f64, strike: f64, rate: f64, time: f64, vol: f64) -> f64 {
        let sqrt_time = time.sqrt();
        ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_time)
    }

    /// Calculates the d2 parameter of the Black-Scholes formula.
    ///
    /// d2 = d1 - σ√T
    ///
    /// # Arguments
    /// - `d1`: The d1 parameter
    /// - `vol`: Volatility (σ)
    /// - `time`: Time to expiration in years (T)
    ///
    /// # Returns
    /// The d2 parameter value
    #[must_use]
    pub fn d2(d1: f64, vol: f64, time: f64) -> f64 {
        d1 - vol * time.sqrt()
    }

    /// Calculates the theoretical option price using Black-Scholes formula.
    ///
    /// For calls: C = S·N(d1) - K·e^(-rT)·N(d2)
    /// For puts:  P = K·e^(-rT)·N(-d2) - S·N(-d1)
    ///
    /// # Arguments
    /// - `params`: Option parameters (spot, strike, time, rate, type)
    /// - `vol`: Volatility (σ)
    ///
    /// # Returns
    /// Theoretical option price
    #[must_use]
    pub fn price(params: &IVParams, vol: f64) -> f64 {
        // Handle edge cases
        if params.time_to_expiry <= 0.0 {
            return params.intrinsic_value();
        }

        if vol <= 0.0 {
            // With zero volatility, option is worth intrinsic value
            let discount = (-params.risk_free_rate * params.time_to_expiry).exp();
            return match params.option_type {
                OptionType::Call => (params.spot - params.strike * discount).max(0.0),
                OptionType::Put => (params.strike * discount - params.spot).max(0.0),
            };
        }

        let d1 = Self::d1(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.time_to_expiry,
            vol,
        );
        let d2 = Self::d2(d1, vol, params.time_to_expiry);
        let discount = (-params.risk_free_rate * params.time_to_expiry).exp();

        match params.option_type {
            OptionType::Call => {
                params.spot * Self::norm_cdf(d1) - params.strike * discount * Self::norm_cdf(d2)
            }
            OptionType::Put => {
                params.strike * discount * Self::norm_cdf(-d2) - params.spot * Self::norm_cdf(-d1)
            }
        }
    }

    /// Calculates vega (∂price/∂σ) - sensitivity to volatility.
    ///
    /// Vega = S · N'(d1) · √T
    ///
    /// Vega is always positive for both calls and puts.
    ///
    /// # Arguments
    /// - `params`: Option parameters
    /// - `vol`: Current volatility estimate
    ///
    /// # Returns
    /// Vega value (change in price per unit change in volatility)
    #[must_use]
    pub fn vega(params: &IVParams, vol: f64) -> f64 {
        if params.time_to_expiry <= 0.0 || vol <= 0.0 {
            return 0.0;
        }

        let d1 = Self::d1(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.time_to_expiry,
            vol,
        );
        params.spot * Self::norm_pdf(d1) * params.time_to_expiry.sqrt()
    }

    /// Calculates delta (∂price/∂S) - sensitivity to underlying price.
    ///
    /// For calls: Δ = N(d1)
    /// For puts:  Δ = N(d1) - 1
    ///
    /// # Arguments
    /// - `params`: Option parameters
    /// - `vol`: Volatility
    ///
    /// # Returns
    /// Delta value
    #[must_use]
    pub fn delta(params: &IVParams, vol: f64) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return match params.option_type {
                OptionType::Call => {
                    if params.spot > params.strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if params.spot < params.strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }

        let d1 = Self::d1(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.time_to_expiry,
            vol,
        );

        match params.option_type {
            OptionType::Call => Self::norm_cdf(d1),
            OptionType::Put => Self::norm_cdf(d1) - 1.0,
        }
    }

    /// Calculates gamma (∂²price/∂S²) - rate of change of delta.
    ///
    /// Γ = N'(d1) / (S · σ · √T)
    ///
    /// Gamma is always positive for both calls and puts.
    ///
    /// # Arguments
    /// - `params`: Option parameters
    /// - `vol`: Volatility
    ///
    /// # Returns
    /// Gamma value
    #[must_use]
    pub fn gamma(params: &IVParams, vol: f64) -> f64 {
        if params.time_to_expiry <= 0.0 || vol <= 0.0 {
            return 0.0;
        }

        let d1 = Self::d1(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.time_to_expiry,
            vol,
        );
        Self::norm_pdf(d1) / (params.spot * vol * params.time_to_expiry.sqrt())
    }

    /// Calculates theta (∂price/∂T) - time decay.
    ///
    /// Returns the daily theta (price change per day).
    ///
    /// # Arguments
    /// - `params`: Option parameters
    /// - `vol`: Volatility
    ///
    /// # Returns
    /// Theta value (negative for long positions, representing time decay)
    #[must_use]
    pub fn theta(params: &IVParams, vol: f64) -> f64 {
        if params.time_to_expiry <= 0.0 || vol <= 0.0 {
            return 0.0;
        }

        let d1 = Self::d1(
            params.spot,
            params.strike,
            params.risk_free_rate,
            params.time_to_expiry,
            vol,
        );
        let d2 = Self::d2(d1, vol, params.time_to_expiry);
        let discount = (-params.risk_free_rate * params.time_to_expiry).exp();
        let sqrt_time = params.time_to_expiry.sqrt();

        let term1 = -params.spot * Self::norm_pdf(d1) * vol / (2.0 * sqrt_time);

        let theta_annual = match params.option_type {
            OptionType::Call => {
                term1 - params.risk_free_rate * params.strike * discount * Self::norm_cdf(d2)
            }
            OptionType::Put => {
                term1 + params.risk_free_rate * params.strike * discount * Self::norm_cdf(-d2)
            }
        };

        // Convert to daily theta
        theta_annual / 365.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn test_erf() {
        // Test known values
        assert!((BlackScholes::erf(0.0) - 0.0).abs() < TOLERANCE);
        assert!((BlackScholes::erf(1.0) - 0.8427007929).abs() < 1e-5);
        assert!((BlackScholes::erf(-1.0) + 0.8427007929).abs() < 1e-5);
    }

    #[test]
    fn test_norm_cdf() {
        // N(0) = 0.5
        assert!((BlackScholes::norm_cdf(0.0) - 0.5).abs() < TOLERANCE);
        // N(-∞) ≈ 0, N(+∞) ≈ 1
        assert!(BlackScholes::norm_cdf(-10.0) < 1e-10);
        assert!(BlackScholes::norm_cdf(10.0) > 1.0 - 1e-10);
    }

    #[test]
    fn test_norm_pdf() {
        // PDF at 0 = 1/√(2π) ≈ 0.3989
        assert!((BlackScholes::norm_pdf(0.0) - 0.3989422804).abs() < TOLERANCE);
        // PDF is symmetric
        assert!((BlackScholes::norm_pdf(1.0) - BlackScholes::norm_pdf(-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn test_call_price_atm() {
        // ATM call with 25% vol, 1 year, no rates
        let params = IVParams::call(100.0, 100.0, 1.0, 0.0);
        let price = BlackScholes::price(&params, 0.25);
        // ATM call ≈ 0.4 * S * σ * √T for small σ
        assert!(price > 9.0 && price < 11.0);
    }

    #[test]
    fn test_put_price_atm() {
        // ATM put with 25% vol, 1 year, no rates
        let params = IVParams::put(100.0, 100.0, 1.0, 0.0);
        let price = BlackScholes::price(&params, 0.25);
        // Put-call parity: C - P = S - K*e^(-rT) = 0 when r=0 and S=K
        let call_params = IVParams::call(100.0, 100.0, 1.0, 0.0);
        let call_price = BlackScholes::price(&call_params, 0.25);
        assert!((price - call_price).abs() < TOLERANCE);
    }

    #[test]
    fn test_put_call_parity() {
        // C - P = S - K*e^(-rT)
        let spot = 100.0;
        let strike = 105.0;
        let time = 0.5;
        let rate = 0.05;
        let vol = 0.3;

        let call_params = IVParams::call(spot, strike, time, rate);
        let put_params = IVParams::put(spot, strike, time, rate);

        let call_price = BlackScholes::price(&call_params, vol);
        let put_price = BlackScholes::price(&put_params, vol);

        let expected_diff = spot - strike * (-rate * time).exp();
        assert!((call_price - put_price - expected_diff).abs() < TOLERANCE);
    }

    #[test]
    fn test_vega_positive() {
        let params = IVParams::call(100.0, 100.0, 0.25, 0.05);
        let vega = BlackScholes::vega(&params, 0.25);
        assert!(vega > 0.0);

        let put_params = IVParams::put(100.0, 100.0, 0.25, 0.05);
        let put_vega = BlackScholes::vega(&put_params, 0.25);
        assert!(put_vega > 0.0);

        // Vega should be same for call and put
        assert!((vega - put_vega).abs() < TOLERANCE);
    }

    #[test]
    fn test_delta_bounds() {
        let call_params = IVParams::call(100.0, 100.0, 0.25, 0.05);
        let call_delta = BlackScholes::delta(&call_params, 0.25);
        // Call delta should be between 0 and 1
        assert!(call_delta > 0.0 && call_delta < 1.0);

        let put_params = IVParams::put(100.0, 100.0, 0.25, 0.05);
        let put_delta = BlackScholes::delta(&put_params, 0.25);
        // Put delta should be between -1 and 0
        assert!(put_delta > -1.0 && put_delta < 0.0);

        // Delta relationship: call_delta - put_delta = 1
        assert!((call_delta - put_delta - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_gamma_positive() {
        let params = IVParams::call(100.0, 100.0, 0.25, 0.05);
        let gamma = BlackScholes::gamma(&params, 0.25);
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_theta_negative_for_long() {
        let params = IVParams::call(100.0, 100.0, 0.25, 0.0);
        let theta = BlackScholes::theta(&params, 0.25);
        // Theta is typically negative (time decay)
        assert!(theta < 0.0);
    }

    #[test]
    fn test_price_at_expiry() {
        // At expiry, option is worth intrinsic value
        let itm_call = IVParams::call(110.0, 100.0, 0.0, 0.05);
        let price = BlackScholes::price(&itm_call, 0.25);
        assert!((price - 10.0).abs() < TOLERANCE);

        let otm_call = IVParams::call(90.0, 100.0, 0.0, 0.05);
        let price = BlackScholes::price(&otm_call, 0.25);
        assert!(price.abs() < TOLERANCE);
    }

    #[test]
    fn test_deep_itm_call() {
        // Deep ITM call should be close to intrinsic
        let params = IVParams::call(150.0, 100.0, 0.25, 0.0);
        let price = BlackScholes::price(&params, 0.25);
        assert!(price > 50.0);
    }

    #[test]
    fn test_deep_otm_call() {
        // Deep OTM call should be close to 0
        let params = IVParams::call(50.0, 100.0, 0.25, 0.0);
        let price = BlackScholes::price(&params, 0.25);
        assert!(price < 0.01);
    }
}
