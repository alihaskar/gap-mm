## Description

### Context
Implied Volatility (IV) is the option's "price" translated into different units. It's like temperature: you can measure it in Celsius or Fahrenheit — it's the same physical quantity expressed differently. The price of an option in USD and the IV in % are the same information in different units.

### Proposed Functionality

#### 1. Price Extraction from OrderBook
For a specific strike and expiry, the orderbook has:
- **Best bid**: e.g. $4.50
- **Best ask**: e.g. $4.70

The "market price" is typically the **mid-price**: `(bid + ask) / 2 = $4.60`

Price source options to implement:
| Source | Pros | Cons |
|--------|------|------|
| Mid price | Simple | Spread can be huge |
| Last trade | "Real" price | Can be stale |
| Weighted mid | Considers depth | More complex |

#### 2. Black-Scholes Inversion via Numerical Search
Black-Scholes: `(S, K, T, r, σ) → Price`

We need to invert the function to obtain σ (IV). Since there's no analytical solution, we use **root finding** (Newton-Raphson or bisection):

```
Given: S=3000, K=3000, T=30 days, r=0, Market_price=150

Find σ such that: BS(S, K, T, r, σ) = Market_price

Newton-Raphson method:
- Try σ=0.50 → BS gives $140 (too low)
- Try σ=0.60 → BS gives $168 (too high)  
- Try σ=0.55 → BS gives $154 (close)
- Try σ=0.53 → BS gives $150 (found!)

Result: IV = 53%
```

Converges quickly (3-5 iterations) because vega (∂price/∂σ) is always positive.

#### 3. IV Surface Construction
Repeat for each point in the option chain:

```
           Strike
Expiry    2800   2900   3000   3100   3200
7 days     62%    58%    55%    54%    56%
30 days    55%    52%    50%    51%    53%
90 days    48%    47%    46%    47%    48%
```

### Practical Problems to Consider

#### Illiquid Strikes
For very OTM strikes, the orderbook may be empty or have huge spreads:
- Bid: $0.05, Ask: $2.00 → Mid: $1.025 (meaningless)

Solutions:
- Discard points with spread > X%
- Use only bid for OTM puts and ask for OTM calls
- Interpolate from nearby liquid strikes

#### Surface Arbitrage
Calculating IV point by point can generate surfaces with arbitrage:
- **Calendar arbitrage**: 30-day IV < 7-day IV
- **Butterfly arbitrage**: surface not convex in strike

Solution: **smoothing** with parametric models (SVI, SABR) that guarantee no-arbitrage.

#### Temporal Synchronization
If spot moves while calculating the surface, IVs from different strikes will be misaligned. An **atomic snapshot** of the complete orderbook is needed.

### Proposed API

```rust
/// Module for implied volatility calculation
pub mod implied_volatility;

/// Parameters for IV calculation
pub struct IVParams {
    /// Underlying spot price
    pub spot: f64,
    /// Option strike price
    pub strike: f64,
    /// Time to expiration in years
    pub time_to_expiry: f64,
    /// Risk-free interest rate
    pub risk_free_rate: f64,
    /// Option type (Call/Put)
    pub option_type: OptionType,
}

/// Price source for IV calculation
pub enum PriceSource {
    /// Simple mid price: (bid + ask) / 2
    MidPrice,
    /// Volume-weighted mid price
    WeightedMid,
    /// Last traded price
    LastTrade,
}

/// IV calculation result
pub struct IVResult {
    /// Calculated implied volatility (0.0 - 1.0+)
    pub iv: f64,
    /// Price used for calculation
    pub price_used: f64,
    /// Bid-ask spread at calculation time (in basis points)
    pub spread_bps: f64,
    /// Number of solver iterations
    pub iterations: u32,
    /// Calculation quality (based on liquidity)
    pub quality: IVQuality,
}

/// IV calculation quality indicator
pub enum IVQuality {
    /// Spread < 1%, high liquidity
    High,
    /// Spread 1-5%, moderate liquidity
    Medium,
    /// Spread > 5%, low liquidity
    Low,
    /// Interpolated from nearby strikes
    Interpolated,
}

impl<T> OrderBook<T> {
    /// Calculates implied volatility for an option
    pub fn implied_volatility(
        &self,
        params: &IVParams,
        price_source: PriceSource,
    ) -> Result<IVResult, IVError>;
}
```

### Dependencies
- Consider integration with `OptionStratLib` for Black-Scholes model and vega calculation
- Alternatively, implement a lightweight Newton-Raphson solver internally

### Tasks
- [ ] Implement price extraction (mid, weighted mid)
- [ ] Implement Newton-Raphson solver for BS inversion
- [ ] Add quality validation based on spread
- [ ] Implement illiquid strike filtering
- [ ] Add atomic snapshot support
- [ ] Unit and integration tests
- [ ] Documentation with examples
