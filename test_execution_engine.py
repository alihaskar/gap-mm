"""
Example demonstrating both engines running simultaneously:
1. TradingNode - Data streaming engine (market data)
2. ExecutionNode - Execution engine (order management)

Both engines communicate with Python and can run in parallel.
"""

import os
import time
from dotenv import load_dotenv
from rust_engine import TradingNode, ExecutionNode
from mm_engine_fast import encode_signal, calculate_quotes_fast

# Load environment variables from .env file
load_dotenv()


class DualEngineBot:
    """Bot that uses both streaming and execution engines."""
    
    def __init__(
        self, 
        api_key: str, 
        api_secret: str, 
        symbol: str = "BTCUSDT",
        market_type: str = "spot",
        tick_size: float = 0.10,
        max_position: float = 0.01,
        min_order_size: float = 0.001,
    ):
        self.symbol = symbol
        
        # Initialize data streaming engine
        self.stream_node = TradingNode()
        
        # Initialize execution engine
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
        self.latest_data = None
        self.order_count = 0
        
    def on_market_update(self, data: dict):
        """
        Callback for market data updates from TradingNode.
        This is called by the Rust streaming engine.
        """
        self.latest_data = data
        
        # Extract key data
        mid_price = data['mid_price']
        gap_prob = data['gap_prob_up']
        
        # Calculate target quotes using the fast engine
        signal, confidence = encode_signal(gap_prob)
        bid, ask, bid_edge, ask_edge, spread = calculate_quotes_fast(
            mid_price, signal, confidence, tick_size=0.10
        )
        
        # Print current state
        print(f"\n{'='*70}")
        print(f"Mid: {mid_price:.2f} | Gap P(up): {gap_prob:.3f}")
        print(f"Target BID: {bid:.2f} ({int(bid_edge)} ticks)")
        print(f"Target ASK: {ask:.2f} ({int(ask_edge)} ticks)")
        
        # Reconcile orders with execution engine
        try:
            result = self.exec_node.reconcile(target_bid=bid, target_ask=ask)
            
            # Display results
            if 'bid' in result:
                self._print_action("BID", result['bid'])
                
            if 'ask' in result:
                self._print_action("ASK", result['ask'])
                
            self.order_count += 1
            
        except Exception as e:
            print(f"ERROR in reconciliation: {e}")
    
    def _print_action(self, side: str, action: dict):
        """Pretty print order action."""
        action_type = action['type']
        
        if action_type == 'submitted':
            print(f"✓ {side} SUBMITTED: {action['price']:.2f} | ID: {action['order_id']}")
        elif action_type == 'amended':
            print(f"↻ {side} AMENDED: {action['old_price']:.2f} → {action['new_price']:.2f} | ID: {action['order_id']}")
        elif action_type == 'no_change':
            print(f"= {side} NO CHANGE: {action['price']:.2f} | ID: {action['order_id']}")
        elif action_type == 'skipped':
            print(f"⊘ {side} SKIPPED: {action['reason']}")
    
    def start(self):
        """Start both engines."""
        print("="*70)
        print("DUAL ENGINE BOT STARTING")
        print("="*70)
        print(f"Symbol: {self.symbol}")
        print("Engines:")
        print("  1. TradingNode (Streaming)")
        print("  2. ExecutionNode (Order Management)")
        print("="*70)
        
        # Sync existing orders from exchange
        print("\nSyncing existing orders from exchange...")
        try:
            self.exec_node.sync_orders()
            print("✓ Orders synced successfully")
        except Exception as e:
            print(f"⚠ Failed to sync orders: {e}")
        
        # Start streaming (this blocks)
        print("\nStarting market data stream...")
        self.stream_node.start_stream(self.on_market_update, self.symbol)


def main():
    """Example usage."""
    
    # Load API credentials from environment
    API_KEY = os.getenv("BYBIT_API_KEY")
    API_SECRET = os.getenv("BYBIT_API_SECRET")
    
    # Load trading configuration
    SYMBOL = os.getenv("TRADING_SYMBOL", "BTCUSDT")
    MARKET_TYPE = os.getenv("MARKET_TYPE", "spot")
    TICK_SIZE = float(os.getenv("TICK_SIZE", "0.10"))
    MAX_POSITION = float(os.getenv("MAX_POSITION_SIZE", "0.01"))
    MIN_ORDER_SIZE = float(os.getenv("MIN_ORDER_SIZE", "0.001"))
    
    if not API_KEY or not API_SECRET:
        print("="*70)
        print("WARNING: API credentials not found in .env file!")
        print("="*70)
        print("\nPlease create a .env file with:")
        print("  BYBIT_API_KEY=your_api_key_here")
        print("  BYBIT_API_SECRET=your_api_secret_here")
        print("  TRADING_SYMBOL=BTCUSDT")
        print("  MARKET_TYPE=spot")
        print("\nRunning in DEMO mode (streaming only, no orders)...")
        print("="*70)
        
        # Demo mode - just streaming
        stream = TradingNode()
        
        def demo_callback(data):
            mid = data['mid_price']
            gap_prob = data['gap_prob_up']
            
            signal, conf = encode_signal(gap_prob)
            bid, ask, _, _, _ = calculate_quotes_fast(mid, signal, conf, tick_size=0.10)
            
            print(f"Mid: {mid:.2f} | Gap: {gap_prob:.3f} | "
                  f"Would place BID: {bid:.2f}, ASK: {ask:.2f}")
        
        print("\nStarting demo stream (Ctrl+C to exit)...")
        stream.start_stream(demo_callback, SYMBOL)
        return
    
    # Real mode with both engines
    bot = DualEngineBot(
        api_key=API_KEY,
        api_secret=API_SECRET,
        symbol=SYMBOL,
        market_type=MARKET_TYPE,
        tick_size=TICK_SIZE,
        max_position=MAX_POSITION,
        min_order_size=MIN_ORDER_SIZE,
    )
    
    try:
        bot.start()
    except KeyboardInterrupt:
        print("\n\n" + "="*70)
        print("Bot stopped by user")
        print(f"Total reconciliations: {bot.order_count}")
        print("="*70)


if __name__ == "__main__":
    main()
