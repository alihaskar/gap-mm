"""
Test script for Rust engine Python bindings.

This demonstrates Phase 3: Python Binding & Event Filtering
"""

import sys
from datetime import datetime
from rust_engine import TradingNode
from price_predictor import PricePredictor


# State to track spot and perp data
market_state = {
    'bybit_spot': None,
    'bybit_linear_perp': None
}

# Price predictor for spot mid
predictor = PricePredictor(sample_interval=10, ema_period=4, queue_size=11)


def on_market_data(data):
    """
    Callback function called when best bid/ask changes.
    
    Args:
        data: Dictionary with enriched orderbook metrics
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
            'mid': mid
        }
        
        # Print on every update, showing both markets
        spot = market_state['bybit_spot']
        perp = market_state['bybit_linear_perp']
        
        if spot and perp:
            # Convert timestamp from milliseconds to datetime
            ts = datetime.fromtimestamp(data['timestamp'] / 1000)
            ts_str = ts.strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
            
            diff = spot['mid'] - perp['mid']
            print(f"[{ts_str}] SPOT: {spot['mid']:.2f} | PERP: {perp['mid']:.2f} | DIFF: {diff:+.2f}", flush=True)
            
            # Add spot mid to predictor
            signal_info = predictor.add_price(spot['mid'], ts)
            if signal_info:
                updated_marker = " ***NEW***" if signal_info.get('updated', False) else ""
                print(f">>> SIGNAL: {signal_info['signal']}{updated_marker} | Fair: {signal_info['fair_price']:.2f} | Close: {signal_info['close_price']:.2f} | Residual: {signal_info['residual']:+.2f}", flush=True)
            
            print("-" * 80, flush=True)
            
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
