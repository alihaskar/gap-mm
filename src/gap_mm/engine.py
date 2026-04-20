"""
Ultra-fast market-making quote calculator using Numba JIT compilation.

Signal interpretation (contrarian gap analysis)
-----------------------------------------------
``gap_prob_resistance_up`` is the normalized score of ask-side gap liquidity:

    gap_prob_resistance_up = V_ask / (V_ask + V_bid)

where V_ask and V_bid are the sums of liquidity in the first K levels beyond
the first empty gap on each side, divided by that gap's distance in ticks.

Contrarian reading:
- High value (> 0.5): heavier liquidity/resistance on the ask side → price
  is more likely to be rejected upward → SELL / tighten ask.
- Low value (< 0.5): heavier liquidity/support on the bid side → price is
  more likely to be rejected downward → BUY / tighten bid.
- Near 0.5: balanced book → sit out with wide symmetric quotes.

All core calculations are compiled to machine code for minimal latency.
"""

from numba import jit
import numpy as np

# Signal direction constants (numba nopython mode does not support strings)
SIGNAL_UP = 1
SIGNAL_DOWN = -1
SIGNAL_NEUTRAL = 0

# Confidence level constants
CONF_HIGH = 2
CONF_MED = 1
CONF_LOW = 0


@jit(nopython=True)
def encode_signal(gap_prob_resistance_up: float):
    """
    Encode a gap-resistance score into a directional signal and confidence.

    Parameters
    ----------
    gap_prob_resistance_up:
        Normalized ask-side liquidity score in [0, 1] produced by the Rust
        engine.  Values > 0.5 indicate more resistance on the ask side;
        values < 0.5 indicate more support on the bid side.

    Returns
    -------
    (signal, confidence)
        signal:     SIGNAL_UP=1, SIGNAL_DOWN=-1, SIGNAL_NEUTRAL=0
        confidence: CONF_HIGH=2, CONF_MED=1, CONF_LOW=0
    """
    if gap_prob_resistance_up >= 0.500001:
        # Heavy ask-side resistance → contrarian SELL
        signal = SIGNAL_DOWN
        confidence = CONF_HIGH if gap_prob_resistance_up > 0.7 else CONF_MED
    elif gap_prob_resistance_up <= 0.499999:
        # Heavy bid-side support → contrarian BUY
        signal = SIGNAL_UP
        confidence = CONF_HIGH if gap_prob_resistance_up < 0.3 else CONF_MED
    else:
        signal = SIGNAL_NEUTRAL
        confidence = CONF_LOW

    return signal, confidence


@jit(nopython=True)
def calculate_quotes_fast(
    mid_price: float,
    signal: int,
    confidence: int,
    tick_size: float = 0.10,
):
    """
    Calculate bid/ask quote prices using asymmetric spread skewing.

    HIGH/MED confidence: one side 1 tick from mid (aggressive),
    opposite side 100 ticks from mid (defensive / inventory management).
    LOW confidence: both sides 100 ticks from mid (sit out).

    Parameters
    ----------
    mid_price:  Current mid price.
    signal:     Directional bias (SIGNAL_UP / SIGNAL_DOWN / SIGNAL_NEUTRAL).
    confidence: Confidence level (CONF_HIGH / CONF_MED / CONF_LOW).
    tick_size:  Minimum price increment (must match ``TICK_SIZE`` in .env).

    Returns
    -------
    (bid_price, ask_price, bid_edge_ticks, ask_edge_ticks, spread_ticks)
    """
    if confidence == CONF_HIGH or confidence == CONF_MED:
        if signal == SIGNAL_UP:
            bid_edge = 1.0
            ask_edge = 100.0
        elif signal == SIGNAL_DOWN:
            bid_edge = 100.0
            ask_edge = 1.0
        else:
            bid_edge = 100.0
            ask_edge = 100.0
    else:
        bid_edge = 100.0
        ask_edge = 100.0

    bid_price = round((mid_price - bid_edge * tick_size) / tick_size) * tick_size
    ask_price = round((mid_price + ask_edge * tick_size) / tick_size) * tick_size

    spread_ticks = (ask_price - bid_price) / tick_size

    return bid_price, ask_price, bid_edge, ask_edge, spread_ticks


@jit(nopython=True)
def calculate_pnl_fast(
    entry_price: float,
    exit_price: float,
    side: int,
    quantity: float,
):
    """
    Calculate P&L for a closed position.

    Parameters
    ----------
    entry_price: Entry price.
    exit_price:  Exit price.
    side:        1 for long, -1 for short.
    quantity:    Position size (positive).

    Returns
    -------
    (pnl, pnl_bps)
    """
    price_change = exit_price - entry_price
    pnl = price_change * side * quantity
    pnl_bps = (price_change / entry_price) * 10000.0 * side
    return pnl, pnl_bps


@jit(nopython=True)
def check_signal_correct(signal: int, price_change: float) -> int:
    """
    Check whether a signal prediction was correct.

    Returns
    -------
    1 if correct, -1 if wrong, 0 if neutral signal.
    """
    if signal == SIGNAL_NEUTRAL:
        return 0
    if signal == SIGNAL_UP and price_change > 0:
        return 1
    if signal == SIGNAL_DOWN and price_change < 0:
        return 1
    return -1


@jit(nopython=True)
def calculate_statistics(
    correct_count: int,
    wrong_count: int,
    neutral_count: int,
    total_bps: float,
    time_minutes: float,
):
    """
    Compute accuracy and expected-value metrics.

    Returns
    -------
    (accuracy_pct, ev_bps_per_min)
    """
    total_predictions = correct_count + wrong_count
    accuracy = (correct_count / total_predictions * 100.0) if total_predictions > 0 else 0.0
    ev_bps_per_min = (total_bps / time_minutes) if time_minutes > 0 else 0.0
    return accuracy, ev_bps_per_min


# ── Python-layer helpers (not JIT-compiled) ──────────────────────────────────

def decode_signal(signal_code: int) -> tuple[str, str]:
    """Return (text, emoji_text) for a signal code."""
    if signal_code == SIGNAL_UP:
        return "UP", "UP"
    if signal_code == SIGNAL_DOWN:
        return "DOWN", "DOWN"
    return "NEUTRAL", "NEUTRAL"


def decode_confidence(conf_code: int) -> str:
    """Return text label for a confidence code."""
    if conf_code == CONF_HIGH:
        return "HIGH"
    if conf_code == CONF_MED:
        return "MED"
    return "LOW"
