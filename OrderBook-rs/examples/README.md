# OrderBook-rs Examples

This directory contains **19 comprehensive examples** demonstrating various features and use cases of the OrderBook-rs library. Each example is designed to showcase specific functionality and best practices for different use cases from beginner tutorials to advanced performance testing.

## 📑 Quick Index

| Example | Description | Level |
|---------|-------------|-------|
| `prelude_demo` | Quick start with simplified imports | 🎓 Beginner |
| `basic_orderbook` | Comprehensive OrderBook introduction | 🎓 Beginner |
| `market_trades_demo` | Market order execution and trades | 🎓 Beginner |
| `depth_analysis` | Market depth & liquidity analysis | 💡 Advanced |
| `market_metrics` | Market metrics (VWAP, spread, imbalance) | 💡 Advanced |
| `market_impact_simulation` | Pre-trade impact & risk analysis | 💡 Advanced |
| `intelligent_order_placement` | Smart order placement for market makers | 💡 Advanced |
| `functional_iterators` | Functional-style depth analysis with iterators | 💡 Advanced |
| `aggregate_statistics` | Market condition detection & analytics | 💡 Advanced |
| `enriched_snapshots` | ⭐ **New!** Pre-calculated metrics for HFT | 💡 Advanced |
| `trade_listener_demo` | Real-time trade notifications | 💡 Advanced |
| `trade_listener_channels` | Multi-book trade routing | 💡 Advanced |
| `orderbook_snapshot_restore` | State persistence & recovery | 💡 Advanced |
| `multi_threaded_orderbook` | Concurrent operations (8 threads) | 🚀 Performance |
| `orderbook_hft_simulation` | HFT simulation (30 threads) | 🚀 Performance |
| `orderbook_contention_test` | Advanced stress testing | 🚀 Performance |
| `price_level_debug` | Low-level debugging | 🔧 Debug |
| `price_level_transition` | State transition testing | 🔧 Debug |

## Running Examples

All examples can be run from the `examples` directory:

```bash
cd examples
cargo run --bin <example_name>
```

For release mode (better performance):

```bash
cargo run --bin <example_name> --release
```

## Available Examples

### 📊 Depth Analysis (`depth_analysis.rs`)

**New!** Demonstrates the depth analysis methods for market making and liquidity analysis.

```bash
cargo run --bin depth_analysis
```

**Features demonstrated:**
- `price_at_depth()` - Find price level where cumulative depth reaches a target
- `cumulative_depth_to_target()` - Get both price and actual cumulative depth at target
- `total_depth_at_levels()` - Calculate total depth in first N price levels
- Market impact estimation for large orders
- Liquidity distribution analysis across price levels

**Use cases:**
- Market making strategies
- Order execution planning
- Liquidity analysis
- Price impact estimation

---

### 📈 Market Metrics (`market_metrics.rs`)

⭐ **New!** Comprehensive demonstration of market metrics for algorithmic trading and risk management.

```bash
cargo run --bin market_metrics
```

**Features demonstrated:**
- `mid_price()` - Average of best bid and ask
- `spread_absolute()` - Absolute spread in price units
- `spread_bps(multiplier)` - Spread in basis points with optional custom multiplier
- `vwap()` - Volume-Weighted Average Price for execution planning
- `micro_price()` - Volume-weighted price at best levels
- `order_book_imbalance()` - Buy/sell pressure indicator (-1.0 to 1.0)
- Custom spread calculations (bps, percentage, pips)

**Metrics explained:**
- **Mid Price**: Simple average for quick reference
- **Spread**: Measures liquidity cost (tight = good, wide = poor)
- **VWAP**: Expected execution price including slippage
- **Micro Price**: More accurate fair value than mid price
- **Imbalance**: Directional pressure (positive = bullish, negative = bearish)

**Practical applications:**
- **Liquidity Assessment**: Using spread_bps to gauge market conditions
- **Execution Planning**: VWAP calculation for optimal order sizing
- **Signal Generation**: Combining imbalance and micro price for trading signals
- **Risk Management**: Slippage estimation for position sizing

**What you'll learn:**
- Standard market microstructure metrics
- How to assess market liquidity
- Execution cost estimation techniques
- Building trading signals from order book data

---

### 🎯 Market Impact Simulation (`market_impact_simulation.rs`)

⭐ **New!** Pre-trade analysis tool for understanding market impact before order execution.

```bash
cargo run --bin market_impact_simulation
```

**Features demonstrated:**
- `market_impact()` - Comprehensive impact analysis before execution
- `simulate_market_order()` - Step-by-step fill simulation
- `liquidity_in_range()` - Price range liquidity analysis
- Pre-trade risk assessment workflow

**Key metrics provided:**
- **Average Execution Price**: VWAP across all fills
- **Slippage Analysis**: Absolute and basis points slippage
- **Liquidity Depth**: Levels consumed and available quantity
- **Fill Simulation**: Detailed breakdown of each fill
- **Cost Estimation**: Total execution cost calculation

**Use cases:**
- **Risk Management**: Assess order impact before execution
- **Smart Order Routing**: Compare liquidity across venues
- **Execution Strategy**: Determine optimal order sizing
- **Backtesting**: Realistic fill simulations for strategy testing
- **Compliance**: Pre-trade risk checks and reporting

**Practical applications:**
- Analyze different order sizes (100, 250, 500, 1000 units)
- Compare buy vs sell side liquidity
- Simulate exact execution fills
- Check liquidity in specific price ranges
- Generate pre-trade risk classifications (LOW/MEDIUM/HIGH)
- Recommend execution strategies (market order, split order, TWAP, etc.)

**What you'll learn:**
- How to assess market impact before trading
- Understanding slippage and execution costs
- Liquidity analysis techniques
- Pre-trade risk management workflows
- Smart order sizing strategies

---

### 🎯 Intelligent Order Placement (`intelligent_order_placement.rs`)

⭐ **New!** Smart order placement utilities for market makers and algorithmic traders.

```bash
cargo run --bin intelligent_order_placement
```

**Features demonstrated:**
- `queue_ahead_at_price()` - Check order queue depth at specific price
- `price_n_ticks_inside()` - Calculate price N ticks from best bid/ask
- `price_for_queue_position()` - Find price for target queue position
- `price_at_depth_adjusted()` - Optimal price for depth-based strategies

**Key concepts:**
- **Queue Position**: Understanding your place in the FIFO queue
- **Tick-Based Pricing**: Strategic pricing relative to best prices
- **Position Targeting**: Finding prices for specific competitive positions
- **Depth-Based Strategy**: Optimizing placement based on cumulative depth

**Use cases:**
- **Market Making**: Optimize quote placement for execution probability
- **Queue Optimization**: Maximize fills by targeting light queues
- **Competitive Positioning**: Balance aggressiveness vs. execution cost
- **Liquidity Provision**: Strategic placement at key depth levels
- **Smart Routing**: Compare and select optimal price levels

**Practical applications:**
- Analyze queue depth at each price level
- Calculate prices 1-3 ticks inside best bid/ask
- Target specific queue positions (1st, 2nd, 3rd, etc.)
- Find optimal prices based on target depth (e.g., just inside 100 units)
- Implement adaptive market making strategies
- Decision making: join heavy queue vs. place 1 tick inside

**What you'll learn:**
- How queue position affects execution probability
- Strategic pricing for market makers
- Trade-offs between price improvement and execution speed
- Depth-based order placement strategies
- Practical market making decision workflows

---

### 🔄 Functional Iterators (`functional_iterators.rs`)

⭐ **New!** Modern functional-style iterators for memory-efficient order book analysis.

```bash
cargo run --bin functional_iterators
```

**Features demonstrated:**
- `levels_with_cumulative_depth()` - Iterate with running depth totals
- `levels_until_depth()` - Auto-stop at target depth
- `levels_in_range()` - Filter by price range
- `find_level()` - Find first matching level with predicates

**Key concepts:**
- **Lazy Evaluation**: No upfront memory allocation
- **Composability**: Chain operations with standard iterator combinators
- **Short-circuiting**: Stop early when condition is met
- **Zero-copy**: Efficient traversal without intermediate collections
- **Functional Style**: Expressive, readable code

**Use cases:**
- **Execution Planning**: Determine levels needed for target quantity
- **Liquidity Analysis**: Analyze depth distribution without allocations
- **Risk Assessment**: Calculate slippage efficiently
- **Market Quality Metrics**: Compute depth statistics on-the-fly
- **Smart Routing**: Compare venues without materializing full depth

**Practical applications:**
- Iterate through levels with cumulative depth tracking
- Find first level exceeding quantity threshold
- Calculate total liquidity in price range
- Analyze depth distribution by bands
- Build complex analysis pipelines with `.map()`, `.filter()`, `.take()`
- Early termination for performance optimization

**Benefits over traditional approaches:**
- **Memory efficient**: O(1) memory vs O(N) for vectors
- **Performance**: Can short-circuit, avoiding unnecessary work
- **Composable**: Works seamlessly with Rust iterator ecosystem
- **Expressive**: Functional style is more readable than imperative loops
- **Flexible**: Easy to combine with other iterator operations

**What you'll learn:**
- Modern functional programming patterns in Rust
- Lazy evaluation and its benefits
- Iterator composition techniques
- Zero-allocation data processing
- Building efficient analysis pipelines
- Short-circuit optimization strategies

---

### 📊 Aggregate Statistics (`aggregate_statistics.rs`)

⭐ **New!** Comprehensive statistical analysis for market condition detection and decision support.

```bash
cargo run --bin aggregate_statistics
```

**Features demonstrated:**
- `depth_statistics()` - Comprehensive depth metrics (volume, avg sizes, weighted prices, std dev)
- `buy_sell_pressure()` - Market pressure indicators
- `is_thin_book()` - Liquidity health checks
- `depth_distribution()` - Histogram of liquidity concentration
- `order_book_imbalance()` - Buy/sell pressure ratio

**Key concepts:**
- **Depth Statistics**: Volume distribution, size variability, weighted averages
- **Market Pressure**: Total volume on each side as sentiment indicators
- **Liquidity Health**: Detecting thin books and insufficient depth
- **Distribution Analysis**: Visualizing liquidity concentration across price ranges
- **Imbalance Detection**: Identifying directional bias in the book

**Use cases:**
- **Market Condition Detection**: Identify trends, pressure, and sentiment
- **Risk Management**: Monitor liquidity health and volatility
- **Strategy Adaptation**: Adjust parameters based on real-time conditions
- **Decision Support**: Data-driven trading decisions
- **Analytics & Reporting**: Generate market quality metrics

**Practical applications:**
- Determine safe order sizes based on depth statistics
- Assess market conditions before trading
- Detect liquidity risks and thin book warnings
- Analyze distribution of liquidity across price bands
- Identify directional opportunities from imbalance
- Risk assessment using level size variability

**Metrics provided:**
- **Volume metrics**: Total, average, min, max per level
- **Price metrics**: Weighted average prices
- **Variability metrics**: Standard deviation of level sizes
- **Pressure metrics**: Buy vs sell volume comparison
- **Distribution metrics**: Liquidity histogram by price bins
- **Imbalance metrics**: Directional bias (-1.0 to 1.0)

**What you'll learn:**
- Statistical analysis of order book depth
- Market condition detection techniques
- Liquidity risk assessment methods
- Distribution analysis for concentration detection
- Practical trading decision workflows
- Risk management using quantitative metrics

---

### 📸 Enriched Snapshots (`enriched_snapshots.rs`)

⭐ **New!** Pre-calculated metrics in snapshots for high-frequency trading and market data distribution.

```bash
cargo run --bin enriched_snapshots
```

**Features demonstrated:**
- `enriched_snapshot()` - Snapshot with all metrics pre-calculated
- `enriched_snapshot_with_metrics()` - Custom metric selection for optimization
- `MetricFlags` - Bitflags for controlling which metrics to calculate

**Key concepts:**
- **Single-Pass Calculation**: All metrics computed in one pass through data
- **Performance Optimization**: Better cache locality vs multiple passes
- **Metric Selection**: Calculate only needed metrics for speed
- **Pre-calculated Metrics**: Mid price, spread BPS, depth, VWAP, imbalance

**Use cases:**
- **High-Frequency Trading**: Low-latency snapshots with metrics
- **Market Data Distribution**: Send enriched snapshots to subscribers
- **Performance Critical Systems**: Reduce computational overhead
- **Analytics**: Consistent metrics at same timestamp
- **Risk Monitoring**: Quick liquidity and execution quality checks

**Practical applications:**
- Create snapshots with all metrics in single pass
- Select specific metrics for performance (only mid price + spread)
- Distribute enriched snapshots over network
- Reduce client-side calculations
- HFT trading decisions with pre-calculated data
- Market data feeds with embedded analytics

**Metrics included:**
- **Mid Price**: Average of best bid and ask
- **Spread (BPS)**: Spread in basis points
- **Total Depth**: Volume on each side
- **VWAP**: Volume-Weighted Average Price for top N levels
- **Imbalance**: Buy/sell pressure ratio (-1.0 to 1.0)

**Performance benefits:**
- **Single Pass**: One iteration vs 5+ separate passes
- **Cache Locality**: Better CPU cache utilization
- **Reduced Overhead**: Fewer allocations and computations
- **Lower Latency**: Critical for HFT applications
- **Flexibility**: Optional metric selection

**What you'll learn:**
- Performance optimization for HFT systems
- Single-pass data processing techniques
- Bitflags for feature selection
- Market data distribution patterns
- Reducing computational overhead
- Cache-friendly data structures

---

### 📘 Basic OrderBook (`basic_orderbook.rs`)

Comprehensive introduction to the OrderBook API covering all fundamental operations. Perfect starting point for new users.

```bash
cargo run --bin basic_orderbook
```

**Features demonstrated:**
- Creating an order book with market close timestamps
- Adding various types of limit orders (buy and sell)
- Adding iceberg orders with hidden quantities
- Market order submission and execution
- Limit order matching mechanics
- Order lookup and retrieval by ID
- Order cancellation
- Displaying book state and statistics
- Time-in-force order types (GTC, IOC, FOK)

**What you'll learn:**
- Basic OrderBook lifecycle
- Order management fundamentals
- How matching engine works
- Error handling patterns

---

### 🔀 Multi-threaded OrderBook (`multi_threaded_orderbook.rs`)

Performance test demonstrating the OrderBook's thread-safe operations and lock-free architecture under concurrent load.

```bash
cargo run --bin multi_threaded_orderbook
```

**Features demonstrated:**
- Concurrent order insertion from multiple threads
- Thread-safe order matching without locks
- Real-time performance metrics (operations per second)
- Synchronization using barriers
- Pre-population strategies for realistic testing
- Mixed workloads (adds, cancels, queries)

**Configuration:**
- 8 concurrent threads by default
- 5-second test duration
- 1000 pre-populated orders
- Automatic performance reporting

**What you'll learn:**
- How the lock-free architecture handles contention
- Thread-safe access patterns
- Performance characteristics under load
- Scalability across multiple cores

---

### ⚡ HFT Simulation (`orderbook_hft_simulation.rs`)

Realistic high-frequency trading simulation modeling real exchange behavior with makers, takers, and cancellers.

```bash
cargo run --bin orderbook_hft_simulation
```

**Features demonstrated:**
- 30 concurrent threads simulating HFT activity
  - 10 maker threads (creating liquidity)
  - 10 taker threads (consuming liquidity)
  - 10 canceller threads (order management)
- Custom order metadata with client/user/exchange IDs
- Realistic price distributions across 20 levels
- Order ID queue management for cancellations
- Comprehensive performance statistics
- Before/after OrderBook state comparison

**Configuration:**
- Symbol: BTC/USD
- Duration: 5 seconds
- Base bid: 9,900 | Base ask: 10,000
- 20 price levels with 5-unit spreads
- Pre-loaded with initial liquidity

**Performance metrics:**
- Orders added per second
- Orders matched per second
- Orders cancelled per second
- Total operations per second
- Final book state analysis

**What you'll learn:**
- Real-world exchange simulation
- High-throughput order processing
- Thread coordination in HFT scenarios
- Performance benchmarking techniques

---

### 🔥 Contention Test (`orderbook_contention_test.rs`)

Advanced stress testing analyzing OrderBook performance under various contention patterns and workload distributions.

```bash
cargo run --bin orderbook_contention_test
```

**Features demonstrated:**
- **Read/Write Ratio Test**: Analyzes performance with different ratios (0%, 25%, 50%, 75%, 95% reads)
- **Hot Spot Contention Test**: Measures performance when operations concentrate on specific price levels
- **Price Level Distribution Test**: Tests with varying numbers of active price levels (1-1000)
- Lock-free architecture benefits under extreme contention
- Performance metrics across different access patterns
- Scalability analysis with detailed statistics

**Configuration:**
- 12 concurrent threads
- 3-second test duration per scenario
- Pre-populated order books for each test
- Comprehensive performance reporting

**Test scenarios:**
1. **Read/Write Patterns**: Understanding cache effects and contention
2. **Hot Spots**: Performance when activity concentrates (0%-100% concentration)
3. **Price Distribution**: How the number of levels affects throughput

**What you'll learn:**
- How lock-free data structures handle contention
- Optimal workload distributions
- Cache effects in concurrent systems
- Performance tuning strategies

---

### 💹 Market Trades Demo (`market_trades_demo.rs`)

Demonstrates market order execution, trade generation, and the TradeListener event system.

```bash
cargo run --bin market_trades_demo
```

**Features demonstrated:**
- Creating OrderBook with TradeListener callback
- Adding bid and ask limit orders to build liquidity
- Executing market orders (buy and sell)
- Real-time trade capture with TradeListener
- Trade information extraction and display
- Transaction details (price, quantity, timestamps)
- Partial fills and complete fills
- Order matching mechanics visualization

**Step-by-step walkthrough:**
1. Set up order book with trade listener
2. Add bid orders (buy side liquidity)
3. Add ask orders (sell side liquidity)
4. Display initial book state
5. Execute market orders
6. Display all captured trades with details

**What you'll learn:**
- How market orders execute against the book
- Trade event system integration
- Best practices for trade tracking
- Understanding the matching engine output

---

### 📸 Snapshot & Restore (`orderbook_snapshot_restore.rs`)

Demonstrates OrderBook state persistence through snapshots with checksum validation and JSON serialization.

```bash
cargo run --bin orderbook_snapshot_restore
```

**Features demonstrated:**
- Creating OrderBook snapshots with top N levels
- Checksum-protected snapshot packages
- JSON serialization and deserialization
- Snapshot validation and integrity checking
- Restoring OrderBook state from snapshots
- Handling snapshot format versions
- Error handling for corrupted snapshots

**Snapshot features:**
- Captures configurable number of price levels (default: top 10)
- Includes format version for compatibility
- SHA-256 checksum for data integrity
- JSON format for easy storage/transmission
- Complete bid and ask level preservation

**What you'll learn:**
- State persistence strategies
- Data integrity with checksums
- Serialization best practices
- Disaster recovery patterns
- Snapshot validation techniques

---

### 📻 Trade Listener Demo (`trade_listener_demo.rs`)

Real-time trade monitoring using the TradeListener callback system for immediate trade notifications.

```bash
cargo run --bin trade_listener_demo
```

**Features demonstrated:**
- Creating OrderBook with TradeListener callback
- Real-time trade event notifications
- Displaying trade information as matches occur
- Crossing limit orders that generate trades
- Trade details extraction (price, quantity, sides)
- Multi-step order execution patterns

**Walkthrough:**
1. Set up order book with real-time trade listener
2. Add initial liquidity (bids and asks)
3. Execute crossing limit orders
4. Watch trades display in real-time
5. Final book state analysis

**What you'll learn:**
- Event-driven order book integration
- Real-time trade monitoring patterns
- Trade listener callback design
- Immediate notification handling

---

### 📡 Trade Listener Channels (`trade_listener_channels.rs`)

Advanced trade listener pattern using channels for multi-book management and async communication.

```bash
cargo run --bin trade_listener_channels
```

**Features demonstrated:**
- TradeListener with channel-based communication
- Multi-book management with `BookManager`
- Symbol-aware trade routing
- Async trade processing patterns
- Managing multiple OrderBooks simultaneously
- Real-world trading system architecture

**Architecture:**
- `BookManager` for centralized trade handling
- Channel-based trade distribution
- Symbol identification in trade events
- Thread-safe multi-book operations

**What you'll learn:**
- Production-ready trade listener patterns
- Multi-book trading system design
- Channel-based event distribution
- Scalable trade processing architecture

---

### 🔍 Price Level Debug (`price_level_debug.rs`)

Low-level debugging tool for analyzing price level distribution and concurrent operations.

```bash
cargo run --bin price_level_debug
```

**Features demonstrated:**
- Price level distribution analysis
- Concurrent operations with 4 threads
- Order pre-population strategies
- Book state verification with snapshots
- Memory usage patterns
- Debug-level performance testing

**Configuration:**
- 4 threads (optimized for debugging)
- 1-second test duration
- 100 price levels
- 10 orders per level minimum
- Detailed state reporting

**What you'll learn:**
- Price level internal mechanics
- Debugging concurrent issues
- State inspection techniques
- Performance analysis tools

---

### 🔄 Price Level Transition (`price_level_transition.rs`)

Tests OrderBook behavior during price level transitions and varying distributions.

```bash
cargo run --bin price_level_transition
```

**Features demonstrated:**
- Testing with multiple price level configurations (100, 5)
- Transitioning between different distributions
- Verifying state consistency across changes
- Snapshot-based state validation
- Concurrent operations during transitions
- Performance across different scenarios

**Test cases:**
1. High distribution: 100 price levels
2. Low distribution: 5 price levels
3. Transition handling between configurations

**What you'll learn:**
- How OrderBook handles varying distributions
- State consistency during changes
- Performance characteristics at extremes
- Debugging distribution-related issues

---

### 🎯 Prelude Demo (`prelude_demo.rs`)

Demonstrates the convenience of the prelude module for streamlined imports and quick prototyping.

```bash
cargo run --bin prelude_demo
```

**Features demonstrated:**
- Single-line import with `use orderbook_rs::prelude::*`
- All commonly used types available immediately
- Quick OrderBook creation and operations
- `DefaultOrderBook` type alias usage
- Utility functions from prelude (`current_time_millis`)
- Clean, concise code without verbose imports

**Types available via prelude:**
- `OrderBook` - Main order book structure
- `OrderId` - Order identifier
- `Side` - Buy/Sell enum
- `TimeInForce` - Order duration types
- `DefaultOrderBook` - Type alias for `OrderBook<()>`
- `TradeListener` - Trade event callback
- `TradeResult` - Trade execution results
- `current_time_millis` - Timestamp utility

**What you'll learn:**
- Prelude module best practices
- Quick prototyping patterns
- Reducing boilerplate imports
- Type aliases for common use cases

---

## Example Categories

### 🎓 For Beginners (Start Here!)
1. **`prelude_demo.rs`** - Quick start with simplified imports
2. **`basic_orderbook.rs`** - Comprehensive introduction to all core concepts
3. **`market_trades_demo.rs`** - Understanding order execution and trades

**Recommended order:** Start with `prelude_demo.rs` to understand the basics, then move to `basic_orderbook.rs` for a comprehensive overview, and finally `market_trades_demo.rs` to see trades in action.

---

### 🚀 For Performance Testing
1. **`multi_threaded_orderbook.rs`** - Basic concurrent operations (8 threads)
2. **`orderbook_hft_simulation.rs`** - Realistic HFT simulation (30 threads)
3. **`orderbook_contention_test.rs`** - Advanced stress testing with multiple scenarios

**Use these to:** Benchmark performance, test scalability, understand lock-free architecture benefits.

---

### 💡 For Advanced Features
1. **`enriched_snapshots.rs`** - ⭐ **New!** Pre-calculated metrics for HFT
2. **`aggregate_statistics.rs`** - Market condition detection & analytics
3. **`functional_iterators.rs`** - Functional-style depth analysis with iterators
4. **`intelligent_order_placement.rs`** - Smart order placement for market makers
5. **`market_impact_simulation.rs`** - Pre-trade impact & risk analysis
6. **`market_metrics.rs`** - Market metrics (VWAP, spread, imbalance)
7. **`depth_analysis.rs`** - Market depth and liquidity analysis
8. **`trade_listener_demo.rs`** - Real-time event notifications
9. **`trade_listener_channels.rs`** - Multi-book trade routing
10. **`orderbook_snapshot_restore.rs`** - State persistence and recovery

**Use these to:** Build market-making bots, optimize order placement, implement advanced trading strategies, assess pre-trade risk, manage multiple order books, generate trading signals, perform efficient depth analysis, detect market conditions, distribute market data with pre-calculated metrics.

---

### 🔧 For Debugging & Analysis
1. **`price_level_debug.rs`** - Low-level price level inspection
2. **`price_level_transition.rs`** - Testing state transitions

**Use these to:** Debug issues, understand internals, analyze performance bottlenecks.

---

## Building All Examples

Compile all examples at once:

```bash
cd examples
cargo build --bins
```

For optimized release builds:

```bash
cargo build --bins --release
```

---

## Running with Logging

All examples support logging via the `tracing` framework. Log output is enabled by default with INFO level.

To adjust log levels, set the `RUST_LOG` environment variable:

```bash
# Show only warnings and errors
RUST_LOG=warn cargo run --bin depth_analysis

# Show debug information
RUST_LOG=debug cargo run --bin depth_analysis

# Show everything including trace
RUST_LOG=trace cargo run --bin depth_analysis
```

---

## Helper Utilities (`helpers.rs`)

The `helpers.rs` module provides common utilities shared across examples:

**Functions available:**
- `setup_orders_for_read_write_test()` - Pre-populate order book for testing
- `setup_orders_for_test()` - Generic order setup with configurable levels
- Order generation utilities
- Random data creation helpers
- Display formatting functions
- Test data setup patterns

These helpers reduce code duplication and provide consistent testing patterns across examples.

## Requirements

- Rust 1.70 or higher
- Dependencies are managed via the workspace `Cargo.toml`

## Documentation

For detailed API documentation, see:
- [OrderBook API docs](https://docs.rs/orderbook-rs)
- [Main README](../README.md)

## 💡 Tips for Learning

**Recommended learning path:**
1. Start with `prelude_demo` to understand the API basics
2. Study `basic_orderbook` for comprehensive coverage
3. Run `market_trades_demo` to see trades in action
4. Learn `market_metrics` for trading signals and risk management
5. Explore `enriched_snapshots` for performance optimization
6. Review `aggregate_statistics` for market condition detection
7. Analyze `functional_iterators` for efficient depth analysis
8. Study `market_impact_simulation` for pre-trade risk assessment
9. Review `intelligent_order_placement` for market making strategies
10. Examine `depth_analysis` for advanced liquidity analysis
11. Try `multi_threaded_orderbook` to understand concurrency
12. Dive into `orderbook_hft_simulation` for realistic scenarios

**Performance testing:**
- Always use `--release` flag for accurate benchmarks
- Start with `multi_threaded_orderbook` for baseline metrics
- Use `orderbook_contention_test` to analyze bottlenecks
- Compare results across different hardware

**Debugging tips:**
- Use `RUST_LOG=debug` for detailed logging
- `price_level_debug` helps understand internal state
- Check snapshots with `orderbook_snapshot_restore`

## Contributing Examples

When adding new examples:
1. Place the file in `src/bin/`
2. Add documentation comments at the top of the file
3. Update this README with:
   - Entry in Quick Index table
   - Detailed section with features
   - Category assignment
4. Ensure it compiles without warnings (`cargo clippy`)
5. Test with `cargo run --bin <name>`
6. Add to appropriate category based on complexity

## 📞 Need Help?

- Check the [main documentation](https://docs.rs/orderbook-rs)
- Read the [project README](../README.md)
- Open an issue on [GitHub](https://github.com/joaquinbejar/OrderBook-rs/issues)
