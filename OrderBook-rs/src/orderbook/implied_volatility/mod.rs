//! Implied volatility calculation from OrderBook data.
//!
//! This module provides functionality to calculate implied volatility (IV)
//! from option prices extracted from the order book using Black-Scholes
//! model inversion via Newton-Raphson numerical method.
//!
//! # Overview
//!
//! Implied Volatility (IV) is the option's "price" translated into different units.
//! The price of an option in USD and the IV in % are the same information in different units.
//!
//! # Price Extraction
//!
//! For a specific strike and expiry, the orderbook provides:
//! - Best bid (e.g., $4.50)
//! - Best ask (e.g., $4.70)
//!
//! The "market price" is typically the mid-price: `(bid + ask) / 2 = $4.60`
//!
//! # Black-Scholes Inversion
//!
//! Since there's no analytical solution to invert Black-Scholes, we use
//! Newton-Raphson root finding which converges quickly (3-5 iterations)
//! because vega (∂price/∂σ) is always positive.
//!
//! # Example
//!
//! ```ignore
//! use orderbook_rs::implied_volatility::{IVParams, OptionType, PriceSource};
//!
//! let params = IVParams {
//!     spot: 3000.0,
//!     strike: 3000.0,
//!     time_to_expiry: 30.0 / 365.0,
//!     risk_free_rate: 0.0,
//!     option_type: OptionType::Call,
//! };
//!
//! let result = book.implied_volatility(&params, PriceSource::MidPrice)?;
//! println!("IV: {:.2}%", result.iv * 100.0);
//! ```

mod black_scholes;
mod error;
mod integration;
mod solver;
mod types;

pub use black_scholes::BlackScholes;
pub use error::IVError;
pub use integration::IVConfig;
pub use solver::{SolverConfig, solve_iv, solve_iv_bisection};
pub use types::{IVParams, IVQuality, IVResult, OptionType, PriceSource};
