# Volume Hedge Trading System

High-performance Rust orderbook engine with Python bindings for Bybit spot and perpetual futures markets.

## 🚀 Quick Start

### Prerequisites

- Rust 1.75.0+ (tested on 1.89.0)
- Python 3.12+
- Poetry
- Git

### Setup

1. **Clone this repository with submodules**
   ```bash
   git clone --recursive <your-repo-url>
   cd volume_hedge
   ```
   
   Or if already cloned:
   ```bash
   git clone <your-repo-url>
   cd volume_hedge
   git submodule update --init --recursive
   ```
   
   > **Note:** OrderBook-rs is included as a git submodule tracking the upstream repository.

3. **Install Python dependencies**
   ```bash
   poetry install
   ```

4. **Build Rust engine**
   ```bash
   cd rust_engine
   poetry run maturin develop --release
   cd ..
   ```

5. **Test the system**
   ```bash
   poetry run python test_feed.py
   ```

## 📁 Project Structure

```
volume_hedge/
├── rust_engine/          # Rust WebSocket engine + Python bindings
│   ├── src/
│   │   ├── bybit.rs      # WebSocket & orderbook logic
│   │   ├── lib.rs        # PyO3 Python bindings
│   │   └── main.rs       # Standalone Rust binary
│   ├── Cargo.toml
│   └── README.md         # Detailed Rust engine docs
├── OrderBook-rs/         # External dependency (git submodule)
├── test_feed.py          # Example Python integration
├── implementation_plan.txt  # Technical implementation details
└── pyproject.toml        # Python dependencies
```

## 🎯 What's Included

- **Native WebSocket Streaming**: Direct connection to Bybit spot + perp markets
- **OrderBook-rs Integration**: 200K+ ops/sec lock-free orderbook engine
- **Python Bindings**: Clean PyO3 API with enriched market metrics
- **Real-time Analytics**: Mid price, imbalance, depth, spread calculations
- **Change Detection**: Efficient callbacks only when best bid/ask changes

## 📖 Usage

See [rust_engine/README.md](rust_engine/README.md) for detailed API documentation.

### Basic Example

```python
from rust_engine import TradingNode

def on_market_data(data):
    print(f"[{data['source']}] {data['symbol']}")
    print(f"  Bid/Ask: {data['bid']:.2f} / {data['ask']:.2f}")
    print(f"  Mid: {data['mid_price']:.2f}")
    print(f"  Imbalance: {data['imbalance']:+.3f}")

node = TradingNode()
node.start_stream(on_market_data, symbol="BTCUSDT")
```

## 📊 Market Data Fields

| Field | Description |
|-------|-------------|
| `bid`, `ask` | Best bid/ask prices from OrderBook-rs |
| `mid_price` | Fair value calculation |
| `spread_bps` | Spread in basis points |
| `imbalance` | Buy/sell pressure (-1.0 to +1.0) |
| `bid_depth_5`, `ask_depth_5` | Total liquidity in top 5 levels |

## 🔧 Development

### Modify Rust Code
```bash
cd rust_engine
# Edit src/bybit.rs or src/lib.rs
poetry run maturin develop --release
cd ..
poetry run python test_feed.py
```

### Modify Python Code
```bash
# No rebuild needed!
poetry run python your_strategy.py
```

## 📚 Documentation

- [Implementation Plan](implementation_plan.txt) - Full technical details
- [Rust Engine README](rust_engine/README.md) - API documentation
- [OrderBook-rs User Guide](OrderBook-rs/doc/USER_GUIDE.md) - OrderBook features

## 🏗 Architecture

```
Bybit WebSocket → Parse JSON → OrderBook-rs State → Python Callback
                                (200K ops/sec)
```

**Key Design:**
- Native WebSocket (no external exchange libraries)
- Price levels mapped to synthetic OrderIDs
- OrderBook-rs handles NBBO calculation with O(1) caching
- Callbacks fire only on best bid/ask changes

## 🎯 Status

✅ **PRODUCTION READY**
- Phase 1: Data Ingestion (Rust WebSocket) - Complete
- Phase 2: OrderBook Integration - Complete  
- Phase 3: Python Bindings - Complete
- Phase 4: Integration Testing - Complete

## 📈 Performance

- WebSocket latency: <1ms
- OrderBook updates: 200K+ ops/sec
- Python callback overhead: <1ms
- Memory: ~1.2MB for 10K orders

## 🤝 Contributing

This is a personal trading system. External dependencies:
- [OrderBook-rs](https://github.com/joaquinbejar/OrderBook-rs) by @joaquinbejar

## 📝 License

MIT

## 👤 Author

Ali Askar (@alihaskar)
- Email: 26202651+alihaskar@users.noreply.github.com
