# OrderBook-rs User Guide

Complete guide for using the OrderBook-rs library in your trading systems.

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Core Concepts](#core-concepts)
5. [Basic Operations](#basic-operations)
6. [Advanced Features](#advanced-features)
7. [Performance Optimization](#performance-optimization)
8. [Best Practices](#best-practices)
9. [Examples](#examples)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

OrderBook-rs is a high-performance, lock-free order book implementation for financial trading systems. It provides:

- **Lock-free architecture** using crossbeam-skiplist for concurrent access
- **Multiple order types**: Limit, Market, Iceberg, FOK, IOC
- **Real-time metrics**: VWAP, spread, imbalance, depth statistics
- **Market impact simulation** for pre-trade analysis
- **Intelligent order placement** strategies
- **Enriched snapshots** with pre-calculated metrics
- **Trade notifications** with listener pattern

**Performance characteristics:**
- Single-threaded: ~1M orders/second
- 30-thread HFT simulation: ~600K orders/second
- Low latency: <1µs for order operations
- Lock-free: No contention in concurrent scenarios

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
orderbook-rs = "0.4"
pricelevel = "0.4"
```

For simplified imports, use the prelude:

```rust
use orderbook_rs::prelude::*;
```

---

## Quick Start

### Creating an OrderBook

```rust
use orderbook_rs::prelude::*;

// Create order book
let book = OrderBook::<()>::new("BTC/USD");

// Add buy order
let order_id = OrderId::new();
let result = book.add_limit_order(
    order_id,
    50000,  // price
    10,     // quantity
    Side::Buy,
    TimeInForce::Gtc,
    None    // no extra data
);

// Add sell order
let order_id2 = OrderId::new();
book.add_limit_order(
    order_id2,
    50100,  // price
    10,     // quantity
    Side::Sell,
    TimeInForce::Gtc,
    None
)?;

// Get best bid/ask
if let Some(best_bid) = book.best_bid() {
    println!("Best bid: {}", best_bid);
}
if let Some(best_ask) = book.best_ask() {
    println!("Best ask: {}", best_ask);
}
```

### Executing Market Orders

```rust
// Execute market buy order
let order_id = OrderId::new();
let result = book.add_market_order(
    order_id,
    20,  // quantity
    Side::Buy,
    None
)?;

// Check execution
println!("Filled: {} units", result.filled_quantity);
println!("Average price: {}", result.average_price());
```

---

## Core Concepts

### Order Types

**Limit Orders:**
- Placed at specific price level
- Only execute at or better than limit price
- Can be partially filled

**Market Orders:**
- Execute immediately at best available price
- Consume liquidity from order book
- May experience slippage

**Iceberg Orders:**
- Hide large orders by showing only visible portion
- Replenish visible quantity as filled
- Reduce market impact

**Time-In-Force:**
- `Gtc` (Good-Till-Cancel): Remain until filled or cancelled
- `Ioc` (Immediate-Or-Cancel): Fill immediately or cancel
- `Fok` (Fill-Or-Kill): Fill completely or cancel entirely

### Sides

- `Side::Buy`: Bid side (buyers)
- `Side::Sell`: Ask side (sellers)

### Price Levels

Prices are represented as `u64` in base units (e.g., cents, satoshis).

Example: $500.00 = 50000 (in cents)

---

## Basic Operations

### Adding Orders

```rust
// Limit order
let order_id = OrderId::new();
book.add_limit_order(
    order_id,
    50000,           // price
    100,             // quantity
    Side::Buy,
    TimeInForce::Gtc,
    None
)?;

// Iceberg order (visible: 10, total: 100)
let order_id = OrderId::new();
book.add_iceberg_order(
    order_id,
    50000,           // price
    100,             // total quantity
    10,              // visible quantity
    Side::Buy,
    TimeInForce::Gtc,
    None
)?;

// Market order
let order_id = OrderId::new();
book.add_market_order(
    order_id,
    50,              // quantity
    Side::Buy,
    None
)?;
```

### Cancelling Orders

```rust
// Cancel specific order
book.cancel_order(order_id)?;

// Cancel all orders on one side
book.cancel_all_orders_for_side(Side::Buy)?;
```

### Querying Order Book State

```rust
// Best prices
let best_bid = book.best_bid();
let best_ask = book.best_ask();

// Spread
let spread = book.spread_absolute();
let spread_bps = book.spread_bps();

// Depth
let bid_depth = book.total_depth_at_levels(Side::Buy, 5);
let ask_depth = book.total_depth_at_levels(Side::Sell, 5);

// Check if order exists
let exists = book.has_order(&order_id);
```

---

## Advanced Features

### 1. Market Metrics

Calculate key trading metrics for decision making.

```rust
// VWAP (Volume-Weighted Average Price)
let vwap = book.vwap(Side::Buy, 10);  // Top 10 levels

// Mid price
let mid = book.mid_price();

// Spread in basis points
let spread_bps = book.spread_bps();

// Order book imbalance (-1.0 to 1.0)
let imbalance = book.order_book_imbalance(5);  // Top 5 levels

// Micro price (imbalance-adjusted)
let micro_price = book.micro_price();
```

**Use cases:**
- Trading signal generation
- Fair value calculation
- Market condition detection
- Risk assessment

### 2. Market Impact Simulation

Simulate order execution to assess pre-trade impact.

```rust
// Simulate market order
let simulation = book.simulate_market_order(Side::Buy, 1000);

println!("Average price: {}", simulation.average_price);
println!("Total cost: {}", simulation.total_cost);
println!("Price impact: {:.2}%", simulation.price_impact_percentage);
println!("Levels consumed: {}", simulation.levels_consumed);

// Decide based on impact
if simulation.price_impact_percentage < 0.5 {
    // Execute order
    book.add_market_order(order_id, 1000, Side::Buy, None)?;
} else {
    // Impact too high, use limit order instead
    book.add_limit_order(
        order_id,
        simulation.average_price as u64,
        1000,
        Side::Buy,
        TimeInForce::Gtc,
        None
    )?;
}
```

**Use cases:**
- Pre-trade risk assessment
- Order type selection
- Position sizing
- Execution strategy

### 3. Intelligent Order Placement

Optimize order placement for market makers and smart routing.

```rust
// Get queue position at specific price
let queue_ahead = book.queue_ahead_at_price(50000, Side::Buy);
println!("Orders ahead: {}", queue_ahead);

// Calculate price N ticks inside
let price = book.price_n_ticks_inside(Side::Buy, 3);  // 3 ticks inside best bid

// Find price for queue position
let target_price = book.price_for_queue_position(Side::Buy, 100);

// Get depth-adjusted price
let adjusted_price = book.price_at_depth_adjusted(Side::Buy, 1000, 0.95);
```

**Use cases:**
- Market maker order placement
- Smart order routing
- Execution optimization
- Liquidity provision

### 4. Functional Iterators

Efficient, lazy evaluation for depth analysis.

```rust
// Iterate until cumulative depth reached
let levels: Vec<_> = book
    .levels_until_depth(Side::Buy, 1000)
    .collect();

// Iterate with cumulative depth tracking
for level in book.levels_with_cumulative_depth(Side::Sell, 10) {
    println!("Price: {}, Size: {}, Cumulative: {}", 
             level.price, level.size, level.cumulative);
}

// Iterate within price range
let levels: Vec<_> = book
    .levels_in_range(Side::Buy, 49000, 50000)
    .collect();

// Combine with functional operations
let total_volume: u64 = book
    .levels_until_depth(Side::Buy, 5000)
    .map(|level| level.size)
    .sum();
```

**Benefits:**
- Zero-allocation iteration
- Lazy evaluation (compute only what's needed)
- Composable operations
- Short-circuit optimization

### 5. Aggregate Statistics

Comprehensive statistical analysis for market condition detection.

```rust
// Depth statistics
let stats = book.depth_statistics(Side::Buy, 10);

println!("Total volume: {}", stats.total_volume);
println!("Average level size: {:.2}", stats.avg_level_size);
println!("Weighted avg price: {:.2}", stats.weighted_avg_price);
println!("Std dev: {:.2}", stats.std_dev_level_size);
println!("Min/Max: {} / {}", stats.min_level_size, stats.max_level_size);

// Market pressure
let (buy_pressure, sell_pressure) = book.buy_sell_pressure();
println!("Buy pressure: {}, Sell pressure: {}", buy_pressure, sell_pressure);

// Thin book detection
let is_thin = book.is_thin_book(1000, 5);  // Threshold: 1000, levels: 5
if is_thin {
    println!("⚠️ Low liquidity detected!");
}

// Depth distribution
let distribution = book.depth_distribution(Side::Buy, 5);
for bin in distribution {
    println!("Price range: {} - {}, Volume: {}, Levels: {}", 
             bin.min_price, bin.max_price, bin.volume, bin.level_count);
}
```

**Use cases:**
- Market condition detection
- Risk management
- Strategy adaptation
- Trading decision support

### 6. Enriched Snapshots

Pre-calculated metrics in snapshots for high-frequency trading.

```rust
// Snapshot with all metrics
let snapshot = book.enriched_snapshot(10);

println!("Mid price: {:?}", snapshot.mid_price);
println!("Spread: {:?} bps", snapshot.spread_bps);
println!("Bid depth: {}", snapshot.bid_depth_total);
println!("Ask depth: {}", snapshot.ask_depth_total);
println!("Imbalance: {}", snapshot.order_book_imbalance);
println!("VWAP bid: {:?}", snapshot.vwap_bid);
println!("VWAP ask: {:?}", snapshot.vwap_ask);

// Custom metrics for performance
use orderbook_rs::MetricFlags;

let snapshot = book.enriched_snapshot_with_metrics(
    10,
    MetricFlags::MID_PRICE | MetricFlags::SPREAD
);

// Serialize for distribution
let json = serde_json::to_string(&snapshot)?;
```

**Benefits:**
- Single pass through data (vs 5+ passes)
- Better cache locality
- Lower latency
- Consistent timestamp for all metrics
- Optional metric selection

---

## Performance Optimization

### 1. Choose the Right Data Types

```rust
// Use u64 for prices and quantities (base units)
let price: u64 = 50000;  // $500.00 in cents
let quantity: u64 = 100;

// Use f64 only for calculated metrics
let vwap: f64 = book.vwap(Side::Buy, 10).unwrap_or(0.0);
```

### 2. Minimize Allocations

```rust
// Use iterators instead of collecting
let sum: u64 = book
    .levels_until_depth(Side::Buy, 1000)
    .map(|level| level.size)
    .sum();  // No allocation

// Instead of:
let levels: Vec<_> = book.levels_until_depth(Side::Buy, 1000).collect();
let sum: u64 = levels.iter().map(|level| level.size).sum();  // Allocates Vec
```

### 3. Use Enriched Snapshots for Multiple Metrics

```rust
// ❌ Inefficient: Multiple passes
let mid = book.mid_price();
let spread = book.spread_bps();
let depth = book.total_depth_at_levels(Side::Buy, 10);
let vwap = book.vwap(Side::Buy, 10);

// ✅ Efficient: Single pass
let snapshot = book.enriched_snapshot(10);
let mid = snapshot.mid_price;
let spread = snapshot.spread_bps;
let depth = snapshot.bid_depth_total;
let vwap = snapshot.vwap_bid;
```

### 4. Batch Operations

```rust
// Add multiple orders efficiently
let orders = vec![
    (OrderId::new(), 50000, 10),
    (OrderId::new(), 49990, 20),
    (OrderId::new(), 49980, 30),
];

for (id, price, qty) in orders {
    book.add_limit_order(id, price, qty, Side::Buy, TimeInForce::Gtc, None)?;
}
```

### 5. Use Appropriate Depth Limits

```rust
// Only analyze what you need
let stats = book.depth_statistics(Side::Buy, 5);  // Top 5 levels only

// Instead of:
let stats = book.depth_statistics(Side::Buy, 0);  // All levels (slower)
```

---

## Best Practices

### 1. Error Handling

```rust
use orderbook_rs::OrderBookError;

match book.add_limit_order(order_id, price, qty, Side::Buy, TimeInForce::Gtc, None) {
    Ok(result) => {
        println!("Order added successfully");
    }
    Err(OrderBookError::DuplicateOrderId { order_id }) => {
        eprintln!("Order {} already exists", order_id);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### 2. Trade Notifications

```rust
use orderbook_rs::prelude::*;

// Define trade listener
let listener = |trade: &TradeResult| {
    println!("Trade executed:");
    println!("  Symbol: {}", trade.symbol);
    for event in &trade.events {
        println!("  Price: {}, Quantity: {}", event.price, event.quantity);
    }
};

// Execute order with listener
book.add_market_order_with_listener(
    OrderId::new(),
    100,
    Side::Buy,
    None,
    listener
)?;
```

### 3. State Management

```rust
// Create snapshot for persistence
let snapshot = book.create_snapshot(10);
let json = serde_json::to_string(&snapshot)?;

// Save to file/database
std::fs::write("orderbook_snapshot.json", json)?;

// Restore later
let json = std::fs::read_to_string("orderbook_snapshot.json")?;
let snapshot: OrderBookSnapshot = serde_json::from_str(&json)?;
book.restore_from_snapshot(snapshot)?;
```

### 4. Concurrent Access

```rust
use std::sync::Arc;

// Share order book across threads
let book = Arc::new(OrderBook::<()>::new("BTC/USD"));

// Clone Arc for each thread
let book_clone = Arc::clone(&book);
std::thread::spawn(move || {
    // Use book_clone in thread
    let _ = book_clone.add_limit_order(
        OrderId::new(),
        50000,
        10,
        Side::Buy,
        TimeInForce::Gtc,
        None
    );
});
```

### 5. Custom Extra Data

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderMetadata {
    user_id: String,
    strategy: String,
}

let book = OrderBook::<OrderMetadata>::new("BTC/USD");

let metadata = OrderMetadata {
    user_id: "user123".to_string(),
    strategy: "market_maker".to_string(),
};

book.add_limit_order(
    OrderId::new(),
    50000,
    10,
    Side::Buy,
    TimeInForce::Gtc,
    Some(metadata)
)?;
```

---

## Examples

### Example 1: Simple Market Maker

```rust
use orderbook_rs::prelude::*;

fn market_maker_strategy(book: &OrderBook) -> Result<(), OrderBookError> {
    // Get current market state
    let snapshot = book.enriched_snapshot(5);
    
    if let (Some(mid), Some(spread_bps)) = (snapshot.mid_price, snapshot.spread_bps) {
        // Only make markets if spread is tight enough
        if spread_bps < 20.0 {
            let offset = 5.0;
            let bid_price = (mid - offset) as u64;
            let ask_price = (mid + offset) as u64;
            
            // Place orders
            book.add_limit_order(
                OrderId::new(),
                bid_price,
                10,
                Side::Buy,
                TimeInForce::Gtc,
                None
            )?;
            
            book.add_limit_order(
                OrderId::new(),
                ask_price,
                10,
                Side::Sell,
                TimeInForce::Gtc,
                None
            )?;
            
            println!("Market making: bid @ {}, ask @ {}", bid_price, ask_price);
        } else {
            println!("Spread too wide: {:.2} bps", spread_bps);
        }
    }
    
    Ok(())
}
```

### Example 2: Smart Order Execution

```rust
fn execute_large_order(
    book: &OrderBook,
    quantity: u64,
    side: Side
) -> Result<(), OrderBookError> {
    // Simulate to assess impact
    let simulation = book.simulate_market_order(side, quantity);
    
    println!("Simulation results:");
    println!("  Average price: {}", simulation.average_price);
    println!("  Price impact: {:.2}%", simulation.price_impact_percentage);
    
    // Decide execution strategy
    if simulation.price_impact_percentage < 0.5 {
        // Low impact: use market order
        println!("Executing market order");
        book.add_market_order(OrderId::new(), quantity, side, None)?;
    } else if simulation.price_impact_percentage < 2.0 {
        // Medium impact: use limit order at VWAP
        println!("Executing limit order at VWAP");
        book.add_limit_order(
            OrderId::new(),
            simulation.average_price as u64,
            quantity,
            side,
            TimeInForce::Gtc,
            None
        )?;
    } else {
        // High impact: split order
        println!("Splitting order due to high impact");
        let chunk_size = quantity / 4;
        for _ in 0..4 {
            book.add_limit_order(
                OrderId::new(),
                simulation.average_price as u64,
                chunk_size,
                side,
                TimeInForce::Gtc,
                None
            )?;
        }
    }
    
    Ok(())
}
```

### Example 3: Liquidity Monitoring

```rust
fn monitor_liquidity(book: &OrderBook) {
    let stats_bid = book.depth_statistics(Side::Buy, 10);
    let stats_ask = book.depth_statistics(Side::Sell, 10);
    
    println!("Liquidity Report:");
    println!("  Bid side:");
    println!("    Total volume: {}", stats_bid.total_volume);
    println!("    Std dev: {:.2}", stats_bid.std_dev_level_size);
    
    println!("  Ask side:");
    println!("    Total volume: {}", stats_ask.total_volume);
    println!("    Std dev: {:.2}", stats_ask.std_dev_level_size);
    
    // Check for thin book
    if book.is_thin_book(1000, 5) {
        println!("⚠️ WARNING: Thin book detected!");
        println!("  Recommendation: Reduce position sizes");
    }
    
    // Check for imbalance
    let imbalance = book.order_book_imbalance(5);
    if imbalance.abs() > 0.3 {
        if imbalance > 0.0 {
            println!("📈 Strong buy pressure detected");
        } else {
            println!("📉 Strong sell pressure detected");
        }
    }
}
```

---

## Troubleshooting

### Common Issues

**Issue: Order not added**

```rust
// Check for duplicate order ID
match book.add_limit_order(order_id, price, qty, Side::Buy, TimeInForce::Gtc, None) {
    Err(OrderBookError::DuplicateOrderId { .. }) => {
        // Generate new ID
        let new_id = OrderId::new();
        book.add_limit_order(new_id, price, qty, Side::Buy, TimeInForce::Gtc, None)?;
    }
    Ok(result) => { /* success */ }
    Err(e) => eprintln!("Error: {}", e),
}
```

**Issue: Market order not filled**

```rust
// Check available liquidity first
let depth = book.total_depth_at_levels(Side::Sell, 0);  // All levels
if depth < quantity {
    println!("Insufficient liquidity: {} available, {} needed", depth, quantity);
    // Use limit order instead
    book.add_limit_order(order_id, price, quantity, Side::Buy, TimeInForce::Gtc, None)?;
} else {
    book.add_market_order(order_id, quantity, Side::Buy, None)?;
}
```

**Issue: Performance degradation**

```rust
// Use enriched snapshots instead of multiple metric calls
// ❌ Slow
let mid = book.mid_price();
let spread = book.spread_bps();
let vwap = book.vwap(Side::Buy, 10);

// ✅ Fast
let snapshot = book.enriched_snapshot(10);
```

**Issue: Memory usage**

```rust
// Limit snapshot depth
let snapshot = book.create_snapshot(10);  // Only top 10 levels

// Instead of:
let snapshot = book.create_snapshot(0);  // All levels (high memory)
```

### Debug Tips

```rust
// Enable logging
std::env::set_var("RUST_LOG", "debug");
tracing_subscriber::fmt::init();

// Check order book state
println!("Bid levels: {}", book.bid_levels());
println!("Ask levels: {}", book.ask_levels());
println!("Total orders: {}", book.bid_levels() + book.ask_levels());

// Verify order exists
if book.has_order(&order_id) {
    println!("Order {} exists", order_id);
} else {
    println!("Order {} not found", order_id);
}
```

---

## Performance Benchmarks

Based on Apple M4 Max processor:

**Single-threaded operations:**
- Add limit order: ~1.2M ops/sec
- Cancel order: ~1.5M ops/sec
- Market order: ~900K ops/sec
- Best bid/ask: ~15M ops/sec

**Multi-threaded (30 threads):**
- Total throughput: ~600K orders/sec
- Per-thread: ~20K orders/sec
- Zero contention (lock-free)

**Metrics calculation:**
- VWAP: ~2µs (10 levels)
- Depth statistics: ~3µs (10 levels)
- Enriched snapshot: ~5µs (all metrics)

**Memory usage:**
- Base order book: ~1KB
- Per order: ~120 bytes
- 10,000 orders: ~1.2MB

---

## Further Reading

- [API Documentation](https://docs.rs/orderbook-rs)
- [Examples Directory](../examples/README.md)
- [GitHub Repository](https://github.com/joaquinbejar/OrderBook-rs)
- [Performance Analysis](../README.md#performance-analysis)

---

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/joaquinbejar/OrderBook-rs/issues
- Email: jb@taunais.com

---

**Version:** 0.4.8  
**Last Updated:** October 2025  
**License:** MIT
