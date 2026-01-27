# Rust Engine - Bybit WebSocket Orderbook Streaming

High-performance Rust engine for streaming Bybit orderbook data with Python bindings.

## 🚀 Features

- **Native WebSocket Implementation**: Direct connection to Bybit spot and perpetual futures markets
- **OrderBook-rs Integration**: Leverages high-performance lock-free orderbook engine (200K+ ops/sec)
- **Python Bindings**: PyO3-based Python module with clean API
- **Enriched Metrics**: Real-time market analysis (mid price, imbalance, depth, spread)
- **Change Detection**: Callbacks fire only when best bid/ask changes (efficient)
- **Dual Mode**: Standalone Rust binary + Python module

## 📋 Requirements

- Rust 1.75.0+ (tested on 1.89.0)
- Python 3.12+
- Poetry (Python dependency management)

## 🛠 Installation

### Build Rust Binary (Standalone)

```bash
cd rust_engine
cargo build --release
cargo run --release  # Test WebSocket streams
```

### Build Python Module

```bash
cd rust_engine
poetry run maturin develop --release
```

This installs `rust_engine` into your Poetry virtualenv.

## 📖 Usage

### Python API

```python
from rust_engine import TradingNode

def on_market_data(data):
    """Called when best bid/ask changes"""
    print(f"[{data['source']}] {data['symbol']}")
    print(f"  Bid: {data['bid']:.2f} | Ask: {data['ask']:.2f}")
    print(f"  Mid: {data['mid_price']:.2f}")
    print(f"  Imbalance: {data['imbalance']:+.3f}")
    print(f"  Depth(5): {data['bid_depth_5']}/{data['ask_depth_5']}")

# Create node
node = TradingNode()

# Start streaming (blocks until interrupted)
node.start_stream(on_market_data, symbol="BTCUSDT")
```

### Data Dictionary

Each callback receives:

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | str | Trading pair (e.g., "BTCUSDT") |
| `source` | str | Market source ("bybit_spot" or "bybit_linear_perp") |
| `bid` | float | Best bid price |
| `ask` | float | Best ask price |
| `spread` | float | Absolute spread (ask - bid) |
| `mid_price` | float\|None | Fair value (avg of bid/ask) |
| `spread_bps` | float\|None | Spread in basis points |
| `imbalance` | float | Buy/sell pressure (-1.0 to +1.0) |
| `bid_depth_5` | int | Total quantity in top 5 bid levels |
| `ask_depth_5` | int | Total quantity in top 5 ask levels |

### Interpreting Imbalance

- **Positive** (+0.5): Strong buy pressure (more bids than asks)
- **Near Zero** (0.0): Balanced market
- **Negative** (-0.5): Strong sell pressure (more asks than bids)

## 🏗 Architecture

```
Bybit WebSocket ━━━┳━━━> [Spot orderbook.200.BTCUSDT]
                    ┃
                    ┃     ┌─────────────────┐
                    ┃━━━━>│  BybitMessage   │
                    ┃     │   (mpsc chan)   │
                    ┃     └────────┬────────┘
                    ┃              │
                    ┃              v
                    ┃     ┌─────────────────┐
                    ┃     │ OrderBookState  │
                    ┃     │  (OrderBook-rs) │
                    ┃     └────────┬────────┘
                    ┃              │
Bybit WebSocket ━━━┻━━━> [Linear perp]     │
                                   │
                                   v
                          ┌─────────────────┐
                          │ MarketUpdate    │
                          │ (enriched data) │
                          └────────┬────────┘
                                   │
                                   v
                            Python Callback
```

## 🎯 Key Design Decisions

1. **Native WebSocket**: Built from scratch (no lotusx/barter) for full control
2. **Price Scaling**: 1e8 multiplier for sub-cent precision (u128 prices, u64 quantities)
3. **Level Mapping**: Price levels → Synthetic OrderIDs via LevelKey(side, price)
4. **Change Filtering**: Only trigger callbacks when best bid/ask actually changes
5. **Rustls + Ring**: Pure Rust TLS (no Windows credential dependencies)

## 📊 Performance

- **WebSocket**: Sub-millisecond latency for orderbook updates
- **OrderBook-rs**: 200K+ operations/second (lock-free architecture)
- **Callback Overhead**: <1ms GIL acquisition + Python invocation
- **Memory**: ~1KB base + ~120 bytes per order

## 🧪 Testing

### Test Rust Binary

```bash
cd rust_engine
cargo run --release --bin bybit_stream
```

Output:
```
[bybit_spot] Connected to wss://stream.bybit.com/v5/public/spot
[bybit_linear_perp] Connected to wss://stream.bybit.com/v5/public/linear
[bybit_spot] BTCUSDT | Best Bid: 87953.90 | Best Ask: 87954.00 | Spread: 0.10
```

### Test Python Integration

```bash
poetry run python test_feed.py
```

Output:
```
[bybit_spot] BTCUSDT | Bid: 87953.90 | Ask: 87954.00 | Mid: 87953.95 | 
Spread: 0.0000 bps | Imbalance: -0.695 | Depth(5): 0/1
```

## 🔧 Development Workflow

### Modify Rust Code

```bash
# 1. Edit src/bybit.rs or src/lib.rs
vim src/bybit.rs

# 2. Rebuild Python module
cd rust_engine
poetry run maturin develop --release

# 3. Test
cd ..
poetry run python test_feed.py
```

### Modify Python Code Only

```bash
# No rebuild needed!
poetry run python your_strategy.py
```

## 📚 OrderBook-rs Features Available

The engine uses OrderBook-rs which provides:

- **Market Metrics**: VWAP, spread (absolute/bps), micro price
- **Depth Analysis**: Cumulative depth, liquidity distribution
- **Market Impact**: Pre-trade slippage simulation
- **Statistics**: Volume, average sizes, std dev, pressure indicators
- **Enriched Snapshots**: All metrics in single pass (cache-friendly)

See [OrderBook-rs docs](../OrderBook-rs/doc/USER_GUIDE.md) for full capabilities.

## 🐛 Troubleshooting

**Import Error:**
```bash
# Rebuild the module
cd rust_engine
poetry run maturin develop --release
```

**Connection Issues:**
- Check internet connectivity
- Bybit may rate-limit WebSocket connections
- Ensure no firewall blocking wss:// connections

**Performance Issues:**
- Use release builds: `--release` flag
- OrderBook-rs is optimized for 50-100 price levels
- Consider reducing callback frequency if Python is slow

## 📈 Next Steps (Future Extensions)

- Add Binance, OKX support
- Multi-symbol streaming
- Reconnection logic with exponential backoff
- Performance metrics (latency tracking, gap detection)
- Expose more OrderBook-rs features (VWAP, market impact simulation)
- Add Python asyncio support for non-blocking callbacks

## 📝 License

MIT

## 👤 Author

Ali Askar (alihaskar)
- Email: 26202651+alihaskar@users.noreply.github.com
- GitHub: @alihaskar
