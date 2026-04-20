# gap-mm

A gap-probability market maker for Bybit spot/linear markets, implemented as a Rust + Python + Numba stack.

> **WARNING: This software places real orders on a live exchange. Running it risks financial loss. You are solely responsible for your trading activity.**

---

## What it does

gap-mm is a two-sided quoting bot that uses **order-book gap analysis** to skew its bid/ask spread in real time:

1. **TradingNode** (Rust): subscribes to Bybit's public WebSocket, maintains a lock-free L2 order book, and scans for gaps (empty tick ranges) on each side of the best bid/ask.
2. **Gap score**: for each update, computes `gap_prob_resistance_up` — a normalized score of ask-side gap liquidity. High value → more resistance above → contrarian SELL skew. Low value → more support below → contrarian BUY skew.
3. **Signal encoding** (Numba JIT): thresholds the gap score into `SIGNAL_UP / SIGNAL_DOWN / SIGNAL_NEUTRAL` with `CONF_HIGH / CONF_MED / CONF_LOW`.
4. **Quote calculation** (Numba JIT): places the tight side 1 tick from mid and the wide side 100 ticks from mid. Neutral → both 100 ticks (sit out).
5. **ExecutionNode** (Rust): reconciles target bid/ask with working orders via Bybit REST v5, preferring amend over cancel+replace. Tracks fills via private WebSocket.

### Architecture

```
Bybit public WS
  └─> TradingNode (Rust)
        └─> OrderBook-rs (lock-free L2 book)
              └─> gap_prob_resistance_up + market metrics
                    └─> encode_signal + calculate_quotes_fast (Numba)
                          └─> ExecutionNode.reconcile (Rust REST OMS)
                                └─> Bybit REST v5 (submit / amend)
                                └─> Bybit private WS (fills / position)
```

### Signal interpretation

| `gap_prob_resistance_up` | Signal | Quote skew |
|---|---|---|
| > 0.70 | DOWN (HIGH) | wide bid, tight ask |
| 0.50–0.70 | DOWN (MED) | wide bid, tight ask |
| ≈ 0.50 | NEUTRAL (LOW) | both wide — sit out |
| 0.30–0.50 | UP (MED) | tight bid, wide ask |
| < 0.30 | UP (HIGH) | tight bid, wide ask |

---

## Known limitations / quirks

- **No backtested edge.** The gap-probability signal is a microstructure heuristic. It has not been statistically validated. Treat this as a reference implementation, not a proven profitable strategy.
- **Bybit only.** The WS and REST code is Bybit-specific (v5 API).
- **Tick size must match your symbol.** Pass `tick_size` both in `.env` and to `TradingNode.start_stream(tick_size=...)`. Default is `0.10` (BTCUSDT spot). Other symbols need their correct tick size.
- **PostOnly orders only.** The bot never crosses the spread. If the market is too fast, orders are amended rather than re-submitted.

---

## Requirements

- Rust 1.75.0+
- Python 3.12+
- [Poetry](https://python-poetry.org/)
- Git (with submodule support)

---

## Installation

```bash
# 1. Clone
git clone https://github.com/alihaskar/gap-mm
cd gap-mm

# 2. Install Python dependencies
poetry install --with dev

# 3. Build the Rust extension
cd rust_engine
poetry run maturin develop --release
cd ..

# 4. Configure
cp .env.example .env
# Edit .env — add your Bybit API key and secret
```

---

## Configuration

All settings live in `.env` (copy from `.env.example`):

| Variable | Default | Description |
|---|---|---|
| `BYBIT_API_KEY` | — | Bybit API key |
| `BYBIT_API_SECRET` | — | Bybit API secret |
| `TRADING_SYMBOL` | `BTCUSDT` | Trading pair |
| `MARKET_TYPE` | `spot` | `spot` or `linear` |
| `TICK_SIZE` | `0.10` | Minimum price increment for the symbol |
| `MAX_POSITION_SIZE` | `0.01` | Maximum net position in base currency |
| `MIN_ORDER_SIZE` | `0.001` | Minimum order size per quote |
| `MIN_UPDATE_INTERVAL` | `0.0` | Seconds between order updates (0 = unlimited) |

---

## Usage

### Run the live bot

```bash
poetry run python -m gap_mm
```

The bot will ask for `YES` confirmation before placing any orders.

### Stream only (no orders)

```bash
poetry run python examples/minimal_stream.py
```

### Python API

```python
from rust_engine import TradingNode, ExecutionNode
from gap_mm.engine import encode_signal, calculate_quotes_fast

stream = TradingNode()
executor = ExecutionNode(
    api_key="...", api_secret="...",
    symbol="BTCUSDT", market_type="spot",
    tick_size=0.10, max_position=0.01, min_order_size=0.001,
)

def on_update(data):
    mid = data["mid_price"]
    gap_score = data["gap_prob_resistance_up"]
    signal, conf = encode_signal(gap_score)
    bid, ask, _, _, _ = calculate_quotes_fast(mid, signal, conf, tick_size=0.10)
    result = executor.reconcile(target_bid=bid, target_ask=ask)

stream.start_stream(on_update, symbol="BTCUSDT", tick_size=0.10)
```

---

## Market data fields

Each callback receives a dict with:

| Field | Description |
|---|---|
| `bid`, `ask` | Best bid/ask |
| `mid_price` | Mid price |
| `spread`, `spread_bps` | Absolute spread / spread in basis points |
| `imbalance` | Order-book imbalance (−1 to +1) |
| `bid_depth_5`, `ask_depth_5` | Quantity in top 5 levels |
| `timestamp` | Exchange timestamp (ms) |
| `gap_prob_resistance_up` | Normalized ask-side gap score (0–1) |
| `gap_distance_up` | Empty ticks above best ask (0–100) |
| `gap_distance_dn` | Empty ticks below best bid (0–100) |
| `liquidity_up` | Volume in 5 levels beyond ask gap |
| `liquidity_dn` | Volume in 5 levels beyond bid gap |

---

## Performance

Measured on an AMD Ryzen 9 / Windows 11 machine against a localhost mock HTTP server
(loopback RTT ≈ 50 µs). Run the benchmark yourself with:

```bash
poetry run python tests/benchmarks/bench_latency.py
```

| Segment | p50 | p99 | What it covers |
|---|---|---|---|
| Numba signal path | **200 ns** | 300 ns | `encode_signal` + `calculate_quotes_fast` |
| Python tick dispatch | **300 ns** | 500 ns | dict unpack + Numba calls + price-change guard |
| `reconcile()` roundtrip | **647 µs** | 901 µs | Python→Rust FFI + reqwest HTTP + JSON parse |
| Full pipeline | **649 µs** | 901 µs | tick arrival → order on the wire |

**Key takeaway:** The signal math is ~300 ns — effectively free. The bottleneck is the REST
round-trip (~650 µs to localhost, +1–5 ms for a co-located exchange). Moving to WebSocket
order placement would collapse this to the ~300 ns range.

---

## Running tests

```bash
# Python unit + integration tests
poetry run pytest tests/ -v

# Rust unit tests
cd rust_engine && cargo test

# Latency benchmarks (prints report, no pass/fail)
poetry run python tests/benchmarks/bench_latency.py
```

---

## Project structure

```
gap-mm/
├── src/
│   └── gap_mm/
│       ├── __init__.py           # public API
│       ├── engine.py             # Numba JIT: signal encoding, quote calc, P&L
│       ├── live.py               # LiveTradingEngine
│       └── __main__.py           # CLI entrypoint: python -m gap_mm
├── rust_engine/
│   └── src/
│       ├── bybit.rs              # WS streaming, OrderBookState, gap analysis
│       ├── execution.rs          # OMS/EMS: order state, reconciliation, position
│       ├── private_ws.rs         # Private WS: fills
│       ├── lib.rs                # PyO3 bindings
│       └── main.rs               # Standalone binary (optional)
├── tests/
│   ├── unit/                     # Pure Python, no network
│   ├── integration/              # Requires rust_engine build; REST mocked
│   └── benchmarks/               # Latency benchmark script
├── examples/
│   └── minimal_stream.py         # Stream only, no orders
├── .env.example
├── pyproject.toml
├── LICENSE                       # MIT
└── CONTRIBUTING.md
```

---

## Attribution

The order-book engine uses [orderbook-rs](https://github.com/joaquinbejar/OrderBook-rs)
by Joaquín Béjar García (MIT license), pulled from crates.io.

---

## License

MIT. See [LICENSE](LICENSE).

## Author

Ali Askar ([@alihaskar](https://github.com/alihaskar))
