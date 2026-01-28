"""
Test script for market making engine with quote skewing.

This demonstrates a market making strategy that uses orderbook gap analysis
to skew bid/ask quotes. The engine adjusts spreads based on predicted
short-term directional bias while always maintaining two-sided quotes.
"""

import sys
from datetime import datetime
from rust_engine import TradingNode
from mm_engine_fast import (
    calculate_quotes_fast, 
    encode_signal,
    decode_signal, 
    decode_confidence
)


# State to track spot and perp data
market_state = {
    'bybit_spot': None,
    'bybit_linear_perp': None
}

# Performance tracking
class PerformanceTracker:
    def __init__(self):
        self.last_signal = None
        self.last_price = None
        self.last_timestamp = None
        self.correct = 0
        self.wrong = 0
        self.neutral = 0
        self.total_bps = 0.0
        self.last_stats_print = None
        
    def record_prediction(self, signal, price, timestamp, gap_prob):
        """Record a prediction and check previous prediction outcome."""
        # Check if we have a previous prediction to evaluate
        if self.last_signal is not None and self.last_price is not None:
            price_change = price - self.last_price
            price_change_bps = (price_change / self.last_price) * 10000
            
            # Determine if prediction was correct
            if self.last_signal == "UP":
                if price_change > 0:
                    self.correct += 1
                    self.total_bps += abs(price_change_bps)
                elif price_change < 0:
                    self.wrong += 1
                    self.total_bps -= abs(price_change_bps)
                else:
                    self.neutral += 1
            elif self.last_signal == "DOWN":
                if price_change < 0:
                    self.correct += 1
                    self.total_bps += abs(price_change_bps)
                elif price_change > 0:
                    self.wrong += 1
                    self.total_bps -= abs(price_change_bps)
                else:
                    self.neutral += 1
            else:  # NEUTRAL
                self.neutral += 1
        
        # Store current prediction
        self.last_signal = signal
        self.last_price = price
        self.last_timestamp = timestamp
    
    def should_print_stats(self, current_time):
        """Check if we should print stats (every 10 seconds)."""
        if self.last_stats_print is None:
            self.last_stats_print = current_time
            return False
        
        elapsed = (current_time - self.last_stats_print).total_seconds()
        if elapsed >= 10:
            self.last_stats_print = current_time
            return True
        return False
    
    def get_stats(self):
        """Get current statistics."""
        total = self.correct + self.wrong + self.neutral
        if total == 0:
            return None
        
        accuracy = (self.correct / (self.correct + self.wrong) * 100) if (self.correct + self.wrong) > 0 else 0
        
        # Calculate time elapsed in minutes
        # For simplicity, estimate based on number of predictions (rough approximation)
        estimated_minutes = total / 60.0 if total > 0 else 1.0  # Assume ~1 update per second
        
        ev_bps_per_min = self.total_bps / estimated_minutes if estimated_minutes > 0 else 0
        
        return {
            'total': total,
            'correct': self.correct,
            'wrong': self.wrong,
            'neutral': self.neutral,
            'accuracy': accuracy,
            'total_bps': self.total_bps,
            'ev_bps_per_min': ev_bps_per_min
        }

tracker = PerformanceTracker()


# Quote calculation now handled by fast Numba module (mm_engine_fast.py)


def on_market_data(data):
    """
    Callback function called when best bid/ask changes.
    
    Args:
        data: Dictionary with enriched orderbook metrics including gap analysis
    """
    try:
        source = data['source']
        bid = data['bid']
        ask = data['ask']
        mid = (bid + ask) / 2 if bid and ask else 0.0
        
        # Update state for this market
        market_state[source] = {
            'bid': bid,
            'ask': ask,
            'mid': mid,
            'gap_prob_up': data.get('gap_prob_up', 0.5),
            'gap_distance_up': data.get('gap_distance_up', 0),
            'gap_distance_dn': data.get('gap_distance_dn', 0),
            'liquidity_up': data.get('liquidity_up', 0),
            'liquidity_dn': data.get('liquidity_dn', 0),
        }
        
        # Only print when we have both markets
        spot = market_state['bybit_spot']
        perp = market_state['bybit_linear_perp']
        
        if spot and perp:
            # Convert timestamp
            ts = datetime.fromtimestamp(data['timestamp'] / 1000)
            ts_str = ts.strftime('%H:%M:%S.%f')[:-3]
            
            # Spot-Perp difference
            diff = spot['mid'] - perp['mid']
            
            # Gap analysis for spot
            gap_prob = spot['gap_prob_up']
            gap_up = spot['gap_distance_up']
            gap_dn = spot['gap_distance_dn']
            liq_up = spot['liquidity_up']
            liq_dn = spot['liquidity_dn']
            
            # Use FAST Numba-compiled signal encoding and quote calculation
            signal_code, conf_code = encode_signal(gap_prob)
            signal_text, gap_signal = decode_signal(signal_code)
            confidence = decode_confidence(conf_code)
            
            # Record prediction and check previous outcome
            tracker.record_prediction(signal_text, spot['mid'], ts, gap_prob)
            
            # Calculate market making quotes with ULTRA-FAST Numba
            mm_bid, mm_ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(
                spot['mid'], signal_code, conf_code
            )
            
            print(f"[{ts_str}]", flush=True)
            print(f"  SPOT:  {spot['mid']:>9.2f}  |  PERP: {perp['mid']:>9.2f}  |  DIFF: {diff:>+7.2f}", flush=True)
            print(f"  GAP SIGNAL: {gap_signal} ({confidence}) | P(up) = {gap_prob:.3f}", flush=True)
            print(f"  Gaps:  ↑{gap_up:>3} ticks  ↓{gap_dn:>3} ticks", flush=True)
            print(f"  Liquidity beyond gap:  ↑{liq_up:>8}  ↓{liq_dn:>8}", flush=True)
            print(f"", flush=True)
            print(f"  💰 MM QUOTES:", flush=True)
            print(f"     BID: {mm_bid:>9.2f}  ({int(bid_edge)} ticks from mid)", flush=True)
            print(f"     ASK: {mm_ask:>9.2f}  ({int(ask_edge)} ticks from mid)", flush=True)
            print(f"     SPREAD: {int(spread_ticks)} ticks  |  Edge: {(mm_ask - mm_bid):.2f}", flush=True)
            
            # Print stats every 10 seconds
            if tracker.should_print_stats(ts):
                stats = tracker.get_stats()
                if stats:
                    print(f"\n{'='*80}", flush=True)
                    print(f"  📊 PERFORMANCE STATS", flush=True)
                    print(f"  Total Predictions: {stats['total']}", flush=True)
                    print(f"  Correct: {stats['correct']} | Wrong: {stats['wrong']} | Neutral: {stats['neutral']}", flush=True)
                    print(f"  Accuracy: {stats['accuracy']:.1f}%", flush=True)
                    print(f"  Total P&L: {stats['total_bps']:+.2f} bps", flush=True)
                    print(f"  Expected Value: {stats['ev_bps_per_min']:+.2f} bps/min", flush=True)
                    print(f"{'='*80}\n", flush=True)
            
            print("-" * 80, flush=True)
            
    except Exception as e:
        print(f"ERROR in callback: {e}", flush=True)
        print(f"Data: {data}", flush=True)


def main():
    print("=" * 80)
    print("MARKET MAKING ENGINE - BTCUSDT (Quote Skewing)")
    print("=" * 80)
    print("\n⚡ Using NUMBA JIT-compiled fast quote engine")
    print("   Warming up JIT compilation...", end="", flush=True)
    
    # Warm up JIT compilation
    for _ in range(100):
        sig, conf = encode_signal(0.5)
        calculate_quotes_fast(89000.0, sig, conf)
    print(" Done! ✓")
    
    print("\nThis engine uses CONTRARIAN orderbook gap analysis for quote skewing.")
    print("\nGap Predictor Logic:")
    print("  🔼 UP       : P(up) < 0.4  - Liquidity below = support → price UP")
    print("  🔽 DOWN     : P(up) > 0.6  - Liquidity above = resistance → price DOWN")
    print("  ⏸ NEUTRAL  : P(up) ≈ 0.5  - No directional bias")
    print("\nMarket Making Strategy (Quote Skewing):")
    print("  Signal UP (HIGH)   → Tight BID (1t), Wide ASK (3t)  - Want to buy")
    print("  Signal DOWN (HIGH) → Wide BID (3t), Tight ASK (1t) - Want to sell")
    print("  Signal NEUTRAL     → Symmetric spread (2t each)")
    print("\nYou capture spread on both sides while naturally accumulating in the")
    print("predicted direction. No naked directional bets.")
    print("\n" + "=" * 80 + "\n")
    
    node = TradingNode()
    
    try:
        # Start streaming with callback
        # This will block until interrupted
        node.start_stream(on_market_data, symbol="BTCUSDT")
    except KeyboardInterrupt:
        print("\n\nStopping stream...")
        node.stop()
        print("Stream stopped.")


if __name__ == "__main__":
    main()
