"""
LiveTradingEngine: real-money market-making bot.

Combines:
- TradingNode  (Rust): Bybit public WebSocket → enriched order-book metrics
- ExecutionNode (Rust): Bybit REST OMS + private WebSocket fill listener
- gap_mm.engine (Numba): signal encoding and quote calculation

WARNING: This places REAL ORDERS on the exchange.
"""

import time
from datetime import datetime

from gap_mm.engine import (
    calculate_quotes_fast,
    decode_confidence,
    decode_signal,
    encode_signal,
)
from rust_engine import ExecutionNode, TradingNode


class LiveTradingEngine:
    """Complete market-making engine with live execution."""

    def __init__(
        self,
        api_key: str,
        api_secret: str,
        symbol: str,
        market_type: str,
        tick_size: float,
        max_position: float,
        min_order_size: float,
        min_update_interval: float = 0.0,
    ):
        self.symbol = symbol
        self.tick_size = tick_size
        self.min_update_interval = min_update_interval

        self.stream_node = TradingNode()
        self.exec_node = ExecutionNode(
            api_key=api_key,
            api_secret=api_secret,
            symbol=symbol,
            market_type=market_type,
            tick_size=tick_size,
            max_position=max_position,
            min_order_size=min_order_size,
        )

        self.last_execution_time = 0.0
        self.last_bid_price: float | None = None
        self.last_ask_price: float | None = None
        self.execution_count = 0
        self.last_signal: str | None = None
        self.last_confidence: str | None = None

        self.total_updates = 0
        self.total_executions = 0
        self.total_fills = 0
        self.start_time = time.time()
        self.current_position: dict | None = None

        print("Engines initialized")

    def on_fill(self, fill: dict) -> None:
        """Callback invoked by the private WebSocket on each execution report."""
        self.total_fills += 1

        print(f"\n{'=' * 80}")
        print(f"  FILL #{self.total_fills}")
        print(f"  {'=' * 80}")
        print(f"  Symbol:    {fill['symbol']}")
        print(f"  Side:      {fill['side']}")
        print(f"  Fill:      {fill['fill_price']:.2f} x {fill['fill_qty']:.6f}")
        print(f"  Cum qty:   {fill['cum_qty']:.6f}")
        print(f"  Avg price: {fill['avg_price']:.2f}")
        print(f"  Status:    {fill['order_status']}")

        if "position" in fill:
            pos = fill["position"]
            self.current_position = pos
            net = pos["net_qty"]
            direction = "LONG" if net > 0 else "SHORT" if net < 0 else "FLAT"
            print(f"\n  POSITION: {net:+.6f} ({direction})")
            print(f"  Avg entry:    {pos['avg_entry_price']:.2f}")
            print(f"  Realized P&L: {pos['realized_pnl']:+.2f} USDT")

        print(f"  {'=' * 80}\n")

    def _should_update(self, new_bid: float, new_ask: float, now: float) -> bool:
        """Return True only when target prices changed (and optional throttle cleared)."""
        if self.last_bid_price is None or self.last_ask_price is None:
            return True
        half_tick = self.tick_size / 2.0
        if (
            abs(new_bid - self.last_bid_price) <= half_tick
            and abs(new_ask - self.last_ask_price) <= half_tick
        ):
            return False
        return not (
            self.min_update_interval > 0
            and now - self.last_execution_time < self.min_update_interval
        )

    def on_market_update(self, data: dict) -> None:
        """Main hot-path callback: receives market data, calculates and reconciles quotes."""
        try:
            self.total_updates += 1

            if data["source"] != "bybit_spot":
                return

            bid = data["bid"]
            ask = data["ask"]
            mid = data.get("mid_price") or (bid + ask) / 2
            gap_score = data.get("gap_prob_resistance_up", 0.5)
            ts = datetime.fromtimestamp(data.get("timestamp", 0) / 1000)
            ts_str = ts.strftime("%H:%M:%S.%f")[:-3]

            signal_code, conf_code = encode_signal(gap_score)
            signal_text, _ = decode_signal(signal_code)
            confidence = decode_confidence(conf_code)

            mm_bid, mm_ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(
                mid, signal_code, conf_code, tick_size=self.tick_size
            )

            print(f"\n[{ts_str}] Update #{self.total_updates}")
            print(f"  Market: BID {bid:.2f} | MID {mid:.2f} | ASK {ask:.2f}")
            print(f"  Signal: {signal_text} ({confidence}) | gap_score={gap_score:.3f}")
            print(
                f"  Target: BID {mm_bid:.2f} ({int(bid_edge)}t) | "
                f"ASK {mm_ask:.2f} ({int(ask_edge)}t) | Spread {int(spread_ticks)}t"
            )

            now = time.time()
            if not self._should_update(mm_bid, mm_ask, now):
                return

            print("  Reconciling orders...")
            try:
                result = self.exec_node.reconcile(target_bid=mm_bid, target_ask=mm_ask)
                if "bid" in result:
                    self._print_action("BID", result["bid"])
                if "ask" in result:
                    self._print_action("ASK", result["ask"])

                self.last_execution_time = now
                self.last_bid_price = mm_bid
                self.last_ask_price = mm_ask
                self.last_signal = signal_text
                self.last_confidence = confidence
                self.execution_count += 1
                self.total_executions += 1

                if self.execution_count % 10 == 0:
                    self._print_stats()

            except Exception as exc:
                print(f"  ERROR during execution: {exc}")

        except Exception as exc:
            import traceback

            print(f"ERROR in market update callback: {exc}")
            traceback.print_exc()

    def _print_action(self, side: str, action: dict) -> None:
        t = action.get("type")
        if t == "submitted":
            print(
                f"    + {side} SUBMITTED @ {action['price']:.2f} | ID: {action['order_id'][:8]}... | {action.get('latency_ms', 0)}ms"
            )
        elif t == "amended":
            print(
                f"    ~ {side} AMENDED: {action['old_price']:.2f} -> {action['new_price']:.2f} | ID: {action['order_id'][:8]}... | {action.get('latency_ms', 0)}ms"
            )
        elif t == "no_change":
            print(
                f"    = {side} UNCHANGED @ {action['price']:.2f} | ID: {action['order_id'][:8]}..."
            )
        elif t == "skipped":
            print(f"    - {side} SKIPPED: {action['reason']}")

    def _print_stats(self) -> None:
        elapsed = time.time() - self.start_time
        max(elapsed / 60.0, 1e-9)
        print(f"\n{'=' * 80}")
        print(
            f"  STATS  runtime={elapsed:.1f}s  updates={self.total_updates}  executions={self.total_executions}  fills={self.total_fills}"
        )
        print(f"  Last signal: {self.last_signal} ({self.last_confidence})")
        if self.last_bid_price and self.last_ask_price:
            print(
                f"  Current quotes: BID {self.last_bid_price:.2f} | ASK {self.last_ask_price:.2f}"
            )
        if self.current_position:
            net = self.current_position["net_qty"]
            direction = "LONG" if net > 0 else "SHORT" if net < 0 else "FLAT"
            print(
                f"  Position: {net:+.6f} ({direction})  P&L: {self.current_position['realized_pnl']:+.2f} USDT"
            )
        print(f"{'=' * 80}\n")

    def start(self) -> None:
        """Start the live trading engine (blocks until interrupted)."""
        print("\n" + "=" * 80)
        print("GAP-MM LIVE TRADING ENGINE")
        print("=" * 80)
        print(f"Symbol:          {self.symbol}")
        print(f"Tick size:       {self.tick_size}")
        print(f"Update interval: {self.min_update_interval}s")
        print("\nWARNING: This will place REAL ORDERS on the exchange!")
        print("=" * 80 + "\n")

        print("Syncing existing orders...")
        try:
            self.exec_node.sync_orders()
            print("Orders synced\n")
        except Exception as exc:
            print(f"Failed to sync orders: {exc}\n")

        print("Starting fill listener...")
        try:
            self.exec_node.start_fill_listener(self.on_fill)
            print("Fill listener started\n")
        except Exception as exc:
            print(f"Failed to start fill listener: {exc}\n")

        print("Warming up Numba JIT...", end="", flush=True)
        for _ in range(100):
            sig, conf = encode_signal(0.5)
            calculate_quotes_fast(89000.0, sig, conf, tick_size=self.tick_size)
        print(" done\n")

        print("Strategy: contrarian gap-probability market maker")
        print("  BUY signal  -> tight bid (1t), wide ask (100t)")
        print("  SELL signal -> wide bid (100t), tight ask (1t)")
        print("  NEUTRAL     -> both wide (100t)")
        print("\n" + "=" * 80 + "\n")

        self.stream_node.start_stream(self.on_market_update, self.symbol)
