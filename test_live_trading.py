"""
Live trading test that combines gap predictor with execution engine.

This script:
1. Streams market data from Bybit
2. Calculates gap-based signals
3. Generates optimal bid/ask quotes
4. Executes orders on the exchange using the OMS/EMS
5. Tracks performance and P&L

WARNING: This places REAL ORDERS on the exchange. Use at your own risk.
"""

import os
import sys
import time
from datetime import datetime
from dotenv import load_dotenv
from rust_engine import TradingNode, ExecutionNode
from mm_engine_fast import (
    calculate_quotes_fast,
    encode_signal,
    decode_signal,
    decode_confidence
)

# Load environment variables
load_dotenv()


class LiveTradingEngine:
    """
    Complete market making engine with live execution.
    """
    
    def __init__(
        self,
        api_key: str,
        api_secret: str,
        symbol: str,
        market_type: str,
        tick_size: float,
        max_position: float,
        min_order_size: float,
        min_update_interval: float = 0.0,  # Minimum seconds between order updates (0 = no limit)
    ):
        self.symbol = symbol
        self.min_update_interval = min_update_interval
        
        # Initialize engines
        print("Initializing engines...")
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
        
        # State
        self.last_execution_time = 0
        self.last_bid_price = None
        self.last_ask_price = None
        self.execution_count = 0
        self.last_signal = None
        self.last_confidence = None
        
        # Performance tracking
        self.total_updates = 0
        self.total_executions = 0
        self.total_fills = 0
        self.start_time = time.time()
        
        # Position tracking
        self.current_position = None
        
        print("✓ Engines initialized")
        
    def on_fill(self, fill: dict):
        """
        Callback when order is filled.
        """
        self.total_fills += 1
        
        symbol = fill['symbol']
        side = fill['side']
        fill_price = fill['fill_price']
        fill_qty = fill['fill_qty']
        cum_qty = fill['cum_qty']
        avg_price = fill['avg_price']
        status = fill['order_status']
        
        print(f"\n{'='*80}")
        print(f"  🎯 FILL EXECUTED #{self.total_fills}")
        print(f"  {'='*80}")
        print(f"  Symbol: {symbol}")
        print(f"  Side: {side}")
        print(f"  Fill Price: {fill_price:.2f}")
        print(f"  Fill Qty: {fill_qty:.6f}")
        print(f"  Cumulative Qty: {cum_qty:.6f}")
        print(f"  Avg Price: {avg_price:.2f}")
        print(f"  Order Status: {status}")
        
        # Position is now included in fill data
        if 'position' in fill:
            position = fill['position']
            self.current_position = position
            net_qty = position['net_qty']
            avg_entry = position['avg_entry_price']
            realized_pnl = position['realized_pnl']
            
            position_str = "LONG" if net_qty > 0 else "SHORT" if net_qty < 0 else "FLAT"
            
            print(f"\n  📊 POSITION UPDATE:")
            print(f"  Net Position: {net_qty:+.6f} ({position_str})")
            print(f"  Avg Entry: {avg_entry:.2f}")
            print(f"  Realized P&L: {realized_pnl:+.2f} USDT")
        
        print(f"  {'='*80}\n")
        
        print("✓ Engines initialized")
    
    def should_update_orders(self, new_bid: float, new_ask: float, current_time: float) -> bool:
        """
        Decide if we should update orders.
        
        Only updates when target price actually changes - this naturally rate-limits
        without artificial delays. If you need 50 updates/sec, you'll get 50 updates/sec.
        """
        # Always update if first time
        if self.last_bid_price is None or self.last_ask_price is None:
            return True
        
        # Check if price changed (more than 0.5 tick to avoid rounding issues)
        bid_changed = abs(new_bid - self.last_bid_price) > 0.05
        ask_changed = abs(new_ask - self.last_ask_price) > 0.05
        
        if not (bid_changed or ask_changed):
            return False  # Price hasn't changed, no update needed
        
        # Price changed - check optional rate limit (0 = disabled)
        if self.min_update_interval > 0:
            if current_time - self.last_execution_time < self.min_update_interval:
                return False  # Optional throttle
        
        return True  # Update immediately
    
    def on_market_update(self, data: dict):
        """
        Main callback - receives market data, calculates quotes, executes orders.
        """
        try:
            self.total_updates += 1
            
            # Extract market data
            source = data['source']
            bid = data['bid']
            ask = data['ask']
            mid = data.get('mid_price') or (bid + ask) / 2
            gap_prob = data.get('gap_prob_up', 0.5)
            timestamp = data.get('timestamp', 0)
            
            # Only trade spot market
            if source != 'bybit_spot':
                return
            
            # Convert timestamp
            ts = datetime.fromtimestamp(timestamp / 1000)
            ts_str = ts.strftime('%H:%M:%S.%f')[:-3]
            
            # Calculate signal using gap predictor
            signal_code, conf_code = encode_signal(gap_prob)
            signal_text, signal_emoji = decode_signal(signal_code)
            confidence = decode_confidence(conf_code)
            
            # Calculate optimal quotes using fast Numba engine
            mm_bid, mm_ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(
                mid, signal_code, conf_code, tick_size=0.10
            )
            
            # Display current state
            print(f"\n[{ts_str}] Update #{self.total_updates}")
            print(f"  Market: BID {bid:.2f} | MID {mid:.2f} | ASK {ask:.2f}")
            print(f"  Signal: {signal_emoji} ({confidence}) | Gap P(up)={gap_prob:.3f}")
            print(f"  Target: BID {mm_bid:.2f} ({int(bid_edge)}t) | ASK {mm_ask:.2f} ({int(ask_edge)}t) | Spread {int(spread_ticks)}t")
            
            # Check if we should update orders
            current_time = time.time()
            if not self.should_update_orders(mm_bid, mm_ask, current_time):
                return  # Price hasn't changed, skip silently
            
            # Execute order reconciliation
            print(f"  🔄 Reconciling orders...")
            try:
                result = self.exec_node.reconcile(target_bid=mm_bid, target_ask=mm_ask)
                
                # Display results
                if 'bid' in result:
                    self._print_action("BID", result['bid'])
                if 'ask' in result:
                    self._print_action("ASK", result['ask'])
                
                # Update state
                self.last_execution_time = current_time
                self.last_bid_price = mm_bid
                self.last_ask_price = mm_ask
                self.last_signal = signal_text
                self.last_confidence = confidence
                self.execution_count += 1
                self.total_executions += 1
                
                # Print stats periodically
                if self.execution_count % 10 == 0:
                    self._print_stats()
                    
            except Exception as e:
                print(f"  ❌ ERROR during execution: {e}")
                
        except Exception as e:
            print(f"ERROR in market update callback: {e}")
            import traceback
            traceback.print_exc()
    
    def _print_action(self, side: str, action: dict):
        """Pretty print order action."""
        action_type = action.get('type')
        
        if action_type == 'submitted':
            latency = action.get('latency_ms', 0)
            print(f"    ✓ {side} SUBMITTED @ {action['price']:.2f} | ID: {action['order_id'][:8]}... | ⚡ {latency}ms")
        elif action_type == 'amended':
            latency = action.get('latency_ms', 0)
            print(f"    ↻ {side} AMENDED: {action['old_price']:.2f} → {action['new_price']:.2f} | ID: {action['order_id'][:8]}... | ⚡ {latency}ms")
        elif action_type == 'no_change':
            print(f"    = {side} UNCHANGED @ {action['price']:.2f} | ID: {action['order_id'][:8]}...")
        elif action_type == 'skipped':
            print(f"    ⊘ {side} SKIPPED: {action['reason']}")
    
    def _print_stats(self):
        """Print performance statistics."""
        elapsed = time.time() - self.start_time
        elapsed_min = elapsed / 60.0
        updates_per_min = self.total_updates / elapsed_min if elapsed_min > 0 else 0
        executions_per_min = self.total_executions / elapsed_min if elapsed_min > 0 else 0
        
        print(f"\n{'='*80}")
        print(f"  📊 PERFORMANCE STATS")
        print(f"  Runtime: {elapsed:.1f}s ({elapsed_min:.1f}m)")
        print(f"  Market Updates: {self.total_updates} ({updates_per_min:.1f}/min)")
        print(f"  Order Executions: {self.total_executions} ({executions_per_min:.1f}/min)")
        print(f"  Total Fills: {self.total_fills}")
        print(f"  Last Signal: {self.last_signal} ({self.last_confidence})")
        if self.last_bid_price and self.last_ask_price:
            print(f"  Current Quotes: BID {self.last_bid_price:.2f} | ASK {self.last_ask_price:.2f}")
        
        # Try to get current position (may fail if in async context)
        try:
            position = self.exec_node.get_position(self.symbol)
            if position:
                self.current_position = position
                net_qty = position['net_qty']
                realized_pnl = position['realized_pnl']
                position_str = "LONG" if net_qty > 0 else "SHORT" if net_qty < 0 else "FLAT"
                print(f"  Position: {net_qty:+.6f} BTC ({position_str})")
                print(f"  Realized P&L: {realized_pnl:+.2f} USDT")
        except:
            if self.current_position:
                net_qty = self.current_position['net_qty']
                realized_pnl = self.current_position['realized_pnl']
                position_str = "LONG" if net_qty > 0 else "SHORT" if net_qty < 0 else "FLAT"
                print(f"  Position: {net_qty:+.6f} BTC ({position_str}) [cached]")
                print(f"  Realized P&L: {realized_pnl:+.2f} USDT [cached]")
        
        print(f"{'='*80}\n")
    
    def start(self):
        """Start the live trading engine."""
        print("\n" + "="*80)
        print("LIVE TRADING ENGINE STARTING")
        print("="*80)
        print(f"Symbol: {self.symbol}")
        print(f"Min Update Interval: {self.min_update_interval}s")
        print("\n⚠️  WARNING: This will place REAL ORDERS on the exchange!")
        print("="*80 + "\n")
        
        # Sync existing orders
        print("Syncing existing orders from exchange...")
        try:
            self.exec_node.sync_orders()
            print("✓ Orders synced successfully\n")
        except Exception as e:
            print(f"⚠ Failed to sync orders: {e}\n")
        
        # Start fill listener
        print("Starting fill listener (private WebSocket)...")
        try:
            self.exec_node.start_fill_listener(self.on_fill)
            print("✓ Fill listener started\n")
        except Exception as e:
            print(f"⚠ Failed to start fill listener: {e}\n")
        
        # Warm up JIT compilation
        print("Warming up Numba JIT compilation...", end="", flush=True)
        for _ in range(100):
            sig, conf = encode_signal(0.5)
            calculate_quotes_fast(89000.0, sig, conf)
        print(" Done! ✓\n")
        
        print("Strategy Logic:")
        print("  🔼 UP signal   → Tight BID, Wide ASK (want to accumulate)")
        print("  🔽 DOWN signal → Wide BID, Tight ASK (want to distribute)")
        print("  ⏸ NEUTRAL     → Symmetric spread")
        print("\n" + "="*80 + "\n")
        
        print("Starting market data stream...\n")
        
        # Start streaming (blocks until interrupted)
        self.stream_node.start_stream(self.on_market_update, self.symbol)


def main():
    """Main entry point."""
    
    # Load configuration from environment
    API_KEY = os.getenv("BYBIT_API_KEY")
    API_SECRET = os.getenv("BYBIT_API_SECRET")
    SYMBOL = os.getenv("TRADING_SYMBOL", "BTCUSDT")
    MARKET_TYPE = os.getenv("MARKET_TYPE", "spot")
    TICK_SIZE = float(os.getenv("TICK_SIZE", "0.10"))
    MAX_POSITION = float(os.getenv("MAX_POSITION_SIZE", "0.01"))
    MIN_ORDER_SIZE = float(os.getenv("MIN_ORDER_SIZE", "0.001"))
    MIN_UPDATE_INTERVAL = float(os.getenv("MIN_UPDATE_INTERVAL", "0.0"))
    
    # Validate credentials
    if not API_KEY or not API_SECRET:
        print("="*80)
        print("ERROR: Missing API credentials!")
        print("="*80)
        print("\nPlease configure your .env file with:")
        print("  BYBIT_API_KEY=your_api_key_here")
        print("  BYBIT_API_SECRET=your_api_secret_here")
        print("\nSee .env.example for full configuration options.")
        print("="*80)
        sys.exit(1)
    
    # Confirm live trading
    print("\n" + "="*80)
    print("⚠️  LIVE TRADING MODE")
    print("="*80)
    print(f"\nConfiguration:")
    print(f"  Symbol: {SYMBOL}")
    print(f"  Market: {MARKET_TYPE}")
    print(f"  Max Position: {MAX_POSITION} {SYMBOL[:3]}")
    print(f"  Min Order Size: {MIN_ORDER_SIZE} {SYMBOL[:3]}")
    print(f"  Tick Size: {TICK_SIZE}")
    print(f"  Update Interval: {MIN_UPDATE_INTERVAL}s")
    print("\nThis will place REAL ORDERS with REAL MONEY on Bybit.")
    print("="*80 + "\n")
    
    response = input("Type 'YES' to confirm and start live trading: ")
    if response.strip().upper() != "YES":
        print("\nAborted by user.")
        sys.exit(0)
    
    # Initialize and start engine
    engine = LiveTradingEngine(
        api_key=API_KEY,
        api_secret=API_SECRET,
        symbol=SYMBOL,
        market_type=MARKET_TYPE,
        tick_size=TICK_SIZE,
        max_position=MAX_POSITION,
        min_order_size=MIN_ORDER_SIZE,
        min_update_interval=MIN_UPDATE_INTERVAL,
    )
    
    try:
        engine.start()
    except KeyboardInterrupt:
        print("\n\n" + "="*80)
        print("Engine stopped by user (Ctrl+C)")
        engine._print_stats()
        print("="*80)


if __name__ == "__main__":
    main()
