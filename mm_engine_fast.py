"""
Ultra-fast market making quote calculator using Numba JIT compilation.

All core calculations are compiled to machine code for minimal latency.
"""

from numba import jit
import numpy as np


# Signal encoding for numba (nopython mode doesn't support strings)
SIGNAL_UP = 1
SIGNAL_DOWN = -1
SIGNAL_NEUTRAL = 0

# Confidence encoding
CONF_HIGH = 2
CONF_MED = 1
CONF_LOW = 0


@jit(nopython=True)
def calculate_quotes_fast(mid_price, signal, confidence, tick_size=0.10):
    """
    Ultra-fast quote calculation using quote skewing strategy.
    
    Args:
        mid_price: Current mid price (float)
        signal: Direction signal (1=UP, -1=DOWN, 0=NEUTRAL)
        confidence: Confidence level (2=HIGH, 1=MED, 0=LOW)
        tick_size: Minimum price increment (default 0.10)
    
    Returns:
        (bid_price, ask_price, bid_edge_ticks, ask_edge_ticks, spread_ticks)
    """
    
    # Determine bid and ask edges based on signal and confidence
    if confidence == CONF_HIGH:  # HIGH confidence
        if signal == SIGNAL_UP:  # Want to buy
            bid_edge = 1.0  # aggressive
            ask_edge = 100.0  # defensive (wide)
        elif signal == SIGNAL_DOWN:  # Want to sell
            bid_edge = 100.0  # defensive (wide)
            ask_edge = 1.0  # aggressive
        else:  # NEUTRAL
            bid_edge = 100.0  # wide
            ask_edge = 100.0  # wide
            
    elif confidence == CONF_MED:  # MED confidence
        if signal == SIGNAL_UP:
            bid_edge = 1.0
            ask_edge = 100.0  # defensive (wide)
        elif signal == SIGNAL_DOWN:
            bid_edge = 100.0  # defensive (wide)
            ask_edge = 1.0
        else:  # NEUTRAL
            bid_edge = 100.0  # wide
            ask_edge = 100.0  # wide
            
    else:  # LOW confidence
        bid_edge = 100.0  # wide
        ask_edge = 100.0  # wide
    
    # Calculate raw prices
    bid_price = mid_price - (bid_edge * tick_size)
    ask_price = mid_price + (ask_edge * tick_size)
    
    # Round to nearest tick (critical for exchange compatibility)
    bid_price = round(bid_price / tick_size) * tick_size
    ask_price = round(ask_price / tick_size) * tick_size
    
    # Calculate spread
    spread_ticks = (ask_price - bid_price) / tick_size
    
    return bid_price, ask_price, bid_edge, ask_edge, spread_ticks


@jit(nopython=True)
def encode_signal(gap_prob):
    """
    Fast signal encoding from gap probability.
    
    Args:
        gap_prob: Probability of upward move (0.0 to 1.0)
    
    Returns:
        (signal, confidence)
        signal: 1=UP, -1=DOWN, 0=NEUTRAL
        confidence: 2=HIGH, 1=MED, 0=LOW
    """
    
    # CORRECT LOGIC: high P_up means price going up → UP
    if gap_prob > 0.500001:
        signal = SIGNAL_DOWN ##SIGNAL_UP
        confidence = CONF_HIGH if gap_prob > 0.7 else CONF_MED
    elif gap_prob < 0.499999:
        signal = SIGNAL_UP
        confidence = CONF_HIGH if gap_prob < 0.3 else CONF_MED
    else:
        signal = SIGNAL_NEUTRAL
        confidence = CONF_LOW
    
    return signal, confidence


@jit(nopython=True)
def calculate_pnl_fast(entry_price, exit_price, side, quantity):
    """
    Fast P&L calculation.
    
    Args:
        entry_price: Entry price
        exit_price: Exit price
        side: 1 for long, -1 for short
        quantity: Position size
    
    Returns:
        (pnl, pnl_bps)
    """
    price_change = exit_price - entry_price
    pnl = price_change * side * quantity
    pnl_bps = (price_change / entry_price) * 10000.0 * side
    
    return pnl, pnl_bps


@jit(nopython=True)
def check_signal_correct(signal, price_change):
    """
    Fast check if signal prediction was correct.
    
    Args:
        signal: 1=UP, -1=DOWN, 0=NEUTRAL
        price_change: Actual price change
    
    Returns:
        1 if correct, -1 if wrong, 0 if neutral
    """
    if signal == SIGNAL_NEUTRAL:
        return 0
    
    if signal == SIGNAL_UP and price_change > 0:
        return 1
    elif signal == SIGNAL_DOWN and price_change < 0:
        return 1
    else:
        return -1


@jit(nopython=True)
def calculate_statistics(correct_count, wrong_count, neutral_count, total_bps, time_minutes):
    """
    Fast statistics calculation.
    
    Returns:
        (accuracy, ev_bps_per_min)
    """
    total_predictions = correct_count + wrong_count
    
    if total_predictions > 0:
        accuracy = (correct_count / total_predictions) * 100.0
    else:
        accuracy = 0.0
    
    if time_minutes > 0:
        ev_bps_per_min = total_bps / time_minutes
    else:
        ev_bps_per_min = 0.0
    
    return accuracy, ev_bps_per_min


# Helper functions for signal decoding (used in Python layer only)
def decode_signal(signal_code):
    """Decode signal code to string (Python layer)."""
    if signal_code == SIGNAL_UP:
        return "UP", "🔼 UP"
    elif signal_code == SIGNAL_DOWN:
        return "DOWN", "🔽 DOWN"
    else:
        return "NEUTRAL", "⏸ NEUTRAL"


def decode_confidence(conf_code):
    """Decode confidence code to string (Python layer)."""
    if conf_code == CONF_HIGH:
        return "HIGH"
    elif conf_code == CONF_MED:
        return "MED"
    else:
        return "LOW"


if __name__ == "__main__":
    # Warm up JIT compilation
    print("Warming up JIT compilation...")
    
    for _ in range(100):
        signal, conf = encode_signal(0.75)
        bid, ask, bid_edge, ask_edge, spread = calculate_quotes_fast(89000.0, signal, conf)
    
    print("JIT compilation complete!")
    
    # Benchmark
    import time
    
    iterations = 1000000
    start = time.perf_counter()
    
    for i in range(iterations):
        gap_prob = 0.3 + (i % 100) / 100.0  # vary between 0.3 and 1.3
        signal, conf = encode_signal(gap_prob)
        bid, ask, bid_edge, ask_edge, spread = calculate_quotes_fast(89000.0 + i % 100, signal, conf)
    
    elapsed = time.perf_counter() - start
    ns_per_calc = (elapsed / iterations) * 1e9
    
    print(f"\nBenchmark Results:")
    print(f"  Iterations: {iterations:,}")
    print(f"  Total time: {elapsed:.3f} seconds")
    print(f"  Time per calculation: {ns_per_calc:.1f} nanoseconds")
    print(f"  Throughput: {iterations/elapsed:,.0f} calculations/second")
    
    # Example usage
    print("\n" + "="*60)
    print("Example Usage:")
    print("="*60)
    
    gap_prob = 0.25
    mid = 89050.0
    
    signal, conf = encode_signal(gap_prob)
    bid, ask, bid_edge, ask_edge, spread = calculate_quotes_fast(mid, signal, conf)
    
    signal_text, signal_emoji = decode_signal(signal)
    conf_text = decode_confidence(conf)
    
    print(f"\nInput:")
    print(f"  Mid Price: {mid:.2f}")
    print(f"  Gap P(up): {gap_prob:.3f}")
    
    print(f"\nOutput:")
    print(f"  Signal: {signal_emoji} ({conf_text})")
    print(f"  BID: {bid:.2f} ({int(bid_edge)} ticks)")
    print(f"  ASK: {ask:.2f} ({int(ask_edge)} ticks)")
    print(f"  SPREAD: {int(spread)} ticks")
