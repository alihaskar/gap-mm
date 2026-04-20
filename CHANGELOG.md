# Changelog

All notable changes to gap-mm are documented here.

## [0.1.0] — 2026-04-20

Initial public release.

### Added

- `TradingNode` (Rust/PyO3): Bybit public WebSocket streaming for spot and linear perp markets, L2 order book via OrderBook-rs, gap-probability calculation.
- `ExecutionNode` (Rust/PyO3): Bybit REST v5 OMS with dual-state order tracking, amend-preferred reconciliation, private WebSocket fill listener, position tracking with realized P&L.
- `gap_mm.engine` (Python/Numba): JIT-compiled `encode_signal`, `calculate_quotes_fast`, `calculate_pnl_fast`, `check_signal_correct`.
- `gap_mm.live.LiveTradingEngine`: full market-making bot combining both engines.
- `python -m gap_mm` CLI entrypoint.
- `examples/minimal_stream.py`: safe streaming demo (no orders).
- Unit tests: `encode_signal`, `calculate_quotes_fast`, `calculate_pnl_fast`, `check_signal_correct`.
- Integration tests: reconcile with mocked Bybit REST, gap-signal pipeline.
- Rust tests: `calculate_gap_probability` with synthetic order books, tick-size parametrization.
- GitHub Actions CI: Python 3.12/3.13, ubuntu/macos/windows, pytest + cargo test + ruff + clippy.

### Fixed

- `gap_prob_resistance_up` (formerly `gap_prob_up`): renamed to accurately reflect contrarian semantics — high value means resistance above, not probability of upward move.
- Tick size in `calculate_gap_probability` is now parametric (`tick_size` field on `OrderBookState`, threaded through `start_stream`). Previously hardcoded to 0.10, breaking non-BTC symbols.
- Removed misleading comment in `encode_signal` that contradicted the implemented contrarian mapping.
- Removed stale copy-paste `print("✓ Engines initialized")` inside `on_fill` callback.
