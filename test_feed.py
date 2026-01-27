"""
Test script for Rust engine Python bindings.

This demonstrates Phase 3: Python Binding & Event Filtering
"""

import sys
from rust_engine import TradingNode


def on_market_data(data):
    """
    Callback function called when best bid/ask changes.
    
    Args:
        data: Dictionary with enriched orderbook metrics
    """
    try:
        mid = data['mid_price'] if data['mid_price'] else 0.0
        spread_bps = data['spread_bps'] if data['spread_bps'] else 0.0
        imbalance = data['imbalance']
        
        msg = f"[{data['source']}] {data['symbol']} | " \
              f"Bid: {data['bid']:.2f} | Ask: {data['ask']:.2f} | " \
              f"Mid: {mid:.2f} | Spread: {spread_bps:.4f} bps | " \
              f"Imbalance: {imbalance:+.3f} | " \
              f"Depth(5): {data['bid_depth_5']}/{data['ask_depth_5']}"
        print(msg, flush=True)
    except Exception as e:
        print(f"ERROR in callback: {e}", flush=True)
        print(f"Data: {data}", flush=True)


def main():
    print("Starting TradingNode...")
    print("This will stream BTCUSDT orderbook from Bybit spot and linear perp")
    print("Callback will be triggered only when best bid/ask changes\n")
    
    node = TradingNode()
    
    try:
        # Start streaming with callback
        # This will block until interrupted
        node.start_stream(on_market_data, symbol="BTCUSDT")
    except KeyboardInterrupt:
        print("\nStopping stream...")
        node.stop()
        print("Stream stopped.")


if __name__ == "__main__":
    main()
