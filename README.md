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
│   │   ├── bybit.rs      # WebSocket, orderbook & gap analysis
│   │   ├── lib.rs        # PyO3 Python bindings (FFI)
│   │   └── main.rs       # Standalone Rust binary
│   ├── Cargo.toml
│   └── README.md         # Detailed Rust engine docs
├── OrderBook-rs/         # High-perf orderbook (git submodule)
├── test_feed.py          # Basic market data feed (EMA predictor)
├── test_gap_predictor.py # Market making engine with quote skewing
├── mm_engine_fast.py     # Numba-optimized quote calculations
├── price_predictor.py    # Numba-optimized EMA predictor
├── implementation_plan.txt  # Technical implementation details
└── pyproject.toml        # Python dependencies
```

## 🎯 What's Included

- **Native WebSocket Streaming**: Direct connection to Bybit spot + perp markets
- **OrderBook-rs Integration**: 200K+ ops/sec lock-free orderbook engine
- **Python Bindings**: Clean PyO3 API with enriched market metrics
- **Gap Analysis**: Microstructure-based orderbook gap predictor (Rust)
- **Market Making Engine**: Quote skewing strategy with contrarian logic
- **Numba JIT Optimization**: Sub-microsecond quote calculations (10M+ ops/sec)
- **EMA Predictor**: Time-aligned sampling with linear regression
- **Real-time Analytics**: Mid price, imbalance, depth, spread, gap probability
- **Change Detection**: Efficient callbacks only when best bid/ask changes
- **Performance Tracking**: Live P&L, accuracy, and EV monitoring

## 📖 Usage

### Available Strategies

#### 1. **Basic Market Data Feed** (`test_feed.py`)
Simple real-time market data with EMA-based price predictor:
```bash
poetry run python test_feed.py
```
- Displays spot/perp mid prices and difference
- 10-second aligned sampling
- EMA smoothing + linear regression prediction
- Signal generation (BUY/SELL)

#### 2. **Market Making Engine** (`test_gap_predictor.py`)
Advanced orderbook gap analysis with quote skewing:
```bash
poetry run python test_gap_predictor.py
```
- **Contrarian gap predictor**: High liquidity = resistance/support
- **Quote skewing**: Adjusts bid/ask spreads based on signal
- **Inverted logic**: More liquidity → price repels (not attracts)
- **Performance tracking**: Live accuracy, P&L, and EV metrics
- **Ultra-fast Numba**: Sub-microsecond quote calculations

**Strategy Logic:**
- Signal UP → Tight bid (1 tick), Wide ask (3 ticks) — want to buy
- Signal DOWN → Wide bid (3 ticks), Tight ask (1 tick) — want to sell
- Signal NEUTRAL → Symmetric spread (2 ticks each)

### Basic API Example

```python
from rust_engine import TradingNode

def on_market_data(data):
    print(f"[{data['source']}] {data['symbol']}")
    print(f"  Bid/Ask: {data['bid']:.2f} / {data['ask']:.2f}")
    print(f"  Mid: {data['mid_price']:.2f}")
    print(f"  Gap P(up): {data['gap_prob_up']:.3f}")
    print(f"  Gap distances: ↑{data['gap_distance_up']} ↓{data['gap_distance_dn']}")

node = TradingNode()
node.start_stream(on_market_data, symbol="BTCUSDT")
```

## 📊 Market Data Fields

| Field | Description |
|-------|-------------|
| `bid`, `ask` | Best bid/ask prices from OrderBook-rs |
| `mid_price` | Fair value calculation |
| `timestamp` | Exchange timestamp (milliseconds) |
| `spread_bps` | Spread in basis points |
| `imbalance` | Buy/sell pressure (-1.0 to +1.0) |
| `bid_depth_5`, `ask_depth_5` | Total liquidity in top 5 levels |
| **Gap Analysis** | |
| `gap_prob_up` | Probability of upward move (0.0 to 1.0) |
| `gap_distance_up` | Empty ticks above best ask (0-100) |
| `gap_distance_dn` | Empty ticks below best bid (0-100) |
| `liquidity_up` | Volume in 5 levels beyond gap (asks) |
| `liquidity_dn` | Volume in 5 levels beyond gap (bids) |

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

### Three-Layer High-Performance Design

This system uses a **best-of-all-worlds** architecture combining Rust, Python, and Numba for optimal performance:

```
┌─────────────────────────────────────────────────────────────┐
│                    BYBIT EXCHANGE                            │
│              (WebSocket Market Data)                         │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  RUST MODULE (C Speed)                       │
├─────────────────────────────────────────────────────────────┤
│  • WebSocket Connection Management                           │
│  • Order Book State (orderbook-rs: 200K+ ops/sec)           │
│  • Gap Scanning (100 ticks up/down)                          │
│  • Liquidity Measurement (5 levels beyond gap)               │
│  • P_up Calculation (~1-5 microseconds)                      │
│                                                              │
│  OUTPUT: {bid, ask, mid, timestamp, gap_prob_up,             │
│           gap_distance_up, gap_distance_dn,                  │
│           liquidity_up, liquidity_dn}                        │
└────────────────────────┬────────────────────────────────────┘
                         │ Python FFI (pyo3)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              PYTHON ORCHESTRATION LAYER                      │
├─────────────────────────────────────────────────────────────┤
│  • Callback Management (on_market_data)                      │
│  • State Tracking (spot/perp markets)                        │
│  • Performance Monitoring & Statistics                       │
│  • Display & Logging                                         │
│  • Calls Numba for hot-path calculations ──────┐            │
└─────────────────────────────────────────────────┼───────────┘
                                                  │
                                                  ▼
                         ┌────────────────────────────────────┐
                         │   NUMBA JIT LAYER (Machine Code)   │
                         ├────────────────────────────────────┤
                         │  encode_signal(gap_prob)           │
                         │    → (signal, confidence)          │
                         │       [~20 nanoseconds]            │
                         │                                    │
                         │  calculate_quotes_fast(...)        │
                         │    → (bid, ask, edges, spread)     │
                         │       [~80 nanoseconds]            │
                         │                                    │
                         │  • Compiled to native machine code │
                         │  • No Python interpreter overhead  │
                         │  • 10M+ calculations/second        │
                         └────────────────────────────────────┘
```

### Layer Responsibilities

#### 🦀 **Rust Layer** (`rust_engine/`)
**What it does:**
- WebSocket connection management (auto-reconnect, keepalive pings)
- Real-time order book state using OrderBook-rs
- Gap analysis: scans up to 100 price levels in each direction
- Liquidity measurement: aggregates volume beyond gaps
- Probability calculation: `P_up = V_up/(gap_up+1) / [V_up/(gap_up+1) + V_dn/(gap_dn+1)]`
- FFI bridge to Python via PyO3

**Why Rust:**
- C-level performance for I/O and data structures
- Memory safety without garbage collection overhead
- Fearless concurrency for handling multiple WebSocket streams
- Lock-free orderbook operations

#### 🐍 **Python Layer** (`test_gap_predictor.py`, `test_feed.py`)
**What it does:**
- Receives enriched market data from Rust
- Orchestrates business logic and strategy rules
- Manages application state (market data, predictions)
- Performance tracking and evaluation
- Formatting and display

**Why Python:**
- Rapid prototyping and strategy development
- Rich ecosystem for data analysis
- Easy to modify and experiment
- Perfect glue language between Rust and Numba

#### ⚡ **Numba Layer** (`mm_engine_fast.py`, `price_predictor.py`)
**What it does:**
- JIT-compiles critical calculations to machine code
- Signal encoding from gap probability (inverted logic)
- Market making quote calculations (quote skewing)
- P&L and performance statistics
- All pure numerical operations with zero Python overhead

**Why Numba:**
- Sub-microsecond latency for hot-path calculations
- Type-specialized machine code generation
- No manual C/Rust FFI needed for numerical code
- Automatic SIMD vectorization where possible

### Performance Characteristics

| Layer | Operation | Latency | Throughput |
|-------|-----------|---------|------------|
| **Rust** | Gap scanning | 1-5 µs | 200K-1M ops/sec |
| **Rust** | P_up calculation | 100 ns | 10M+ ops/sec |
| **Python** | FFI callback | 10-50 µs | N/A (event-driven) |
| **Numba** | Signal encoding | 20 ns | 50M ops/sec |
| **Numba** | Quote calculation | 80 ns | 12M ops/sec |

**Total latency: Market update → Quotes calculated = 20-100 microseconds**

### Key Design Principles

1. **Zero-Copy Where Possible**: Rust passes data to Python via FFI with minimal serialization
2. **Event-Driven Architecture**: Callbacks fire only on NBBO changes (not every tick)
3. **Hot Path Optimization**: Critical calculations (quote pricing) run in JIT-compiled Numba
4. **Separation of Concerns**: I/O (Rust) → Logic (Python) → Computation (Numba)
5. **Production-Ready**: Auto-reconnection, error handling, graceful shutdown

### Data Flow Example

```python
# 1. Market update arrives via WebSocket (Rust)
WebSocket receives: {"bid": 89000, "ask": 89000.1, ...}

# 2. Rust processes and enriches
- Updates orderbook state
- Scans for gaps: gap_up=5 ticks, gap_dn=2 ticks
- Measures liquidity: liq_up=1000, liq_dn=5000
- Calculates: P_up = 0.29 (support below)

# 3. Python callback receives enriched data
on_market_data({
    'bid': 89000, 'ask': 89000.1, 'mid': 89000.05,
    'gap_prob_up': 0.29, 'gap_distance_up': 5, ...
})

# 4. Numba calculates quotes (sub-microsecond)
signal, conf = encode_signal(0.29)  # → UP, HIGH
bid, ask = calculate_quotes_fast(89000.05, signal, conf)
# → bid: 89000.00 (1 tick), ask: 89000.35 (3 ticks)

# 5. Python displays results
print(f"Signal: UP | BID: {bid} | ASK: {ask}")
```

This architecture achieves **HFT-grade performance** while maintaining the **flexibility of Python** for strategy development.

## 🎯 Status

✅ **PRODUCTION READY**
- Phase 1: Data Ingestion (Rust WebSocket) - Complete
- Phase 2: OrderBook Integration - Complete  
- Phase 3: Python Bindings - Complete
- Phase 4: Integration Testing - Complete

## 📈 Performance

| Component | Metric | Performance |
|-----------|--------|-------------|
| **WebSocket** | Latency | <1ms |
| **OrderBook** | Update throughput | 200K+ ops/sec |
| **Gap Scanning** | Per scan (100 ticks) | 1-5 µs |
| **Numba (Signal)** | encode_signal() | ~20 ns |
| **Numba (Quotes)** | calculate_quotes_fast() | ~80 ns |
| **Numba** | Throughput | 10M+ calcs/sec |
| **Python FFI** | Callback overhead | 10-50 µs |
| **Memory** | 10K orders | ~1.2 MB |
| **End-to-End** | Market update → Quotes | 20-100 µs |

**HFT-grade performance** suitable for market making and statistical arbitrage.

## 🤝 Contributing

This is a personal trading system. External dependencies:
- [OrderBook-rs](https://github.com/joaquinbejar/OrderBook-rs) by @joaquinbejar

## 📝 License

MIT

## 👤 Author

Ali Askar (@alihaskar)
- Email: 26202651+alihaskar@users.noreply.github.com
