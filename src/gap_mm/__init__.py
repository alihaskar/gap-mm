"""
gap-mm: gap-probability market maker for Bybit.

Core modules:
    engine  - Numba JIT signal encoding and quote calculation
    live    - LiveTradingEngine: the live market-making bot
"""

from gap_mm.engine import (
    CONF_HIGH,
    CONF_LOW,
    CONF_MED,
    SIGNAL_DOWN,
    SIGNAL_NEUTRAL,
    SIGNAL_UP,
    calculate_pnl_fast,
    calculate_quotes_fast,
    check_signal_correct,
    decode_confidence,
    decode_signal,
    encode_signal,
)
from gap_mm.live import LiveTradingEngine

__all__ = [
    "LiveTradingEngine",
    "encode_signal",
    "calculate_quotes_fast",
    "calculate_pnl_fast",
    "check_signal_correct",
    "decode_signal",
    "decode_confidence",
    "SIGNAL_UP",
    "SIGNAL_DOWN",
    "SIGNAL_NEUTRAL",
    "CONF_HIGH",
    "CONF_MED",
    "CONF_LOW",
]
