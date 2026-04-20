"""
Integration test: gap-probability score → encode_signal pipeline.

This test does NOT require network or API keys.
It verifies that the Python-layer signal encoding is internally consistent
with specific gap score inputs (i.e., the pipeline works end-to-end without
the Rust WebSocket layer).

For Rust-layer gap calculation tests, see rust_engine/src/bybit.rs #[cfg(test)].
"""

import pytest
from gap_mm.engine import (
    encode_signal,
    calculate_quotes_fast,
    SIGNAL_UP,
    SIGNAL_DOWN,
    SIGNAL_NEUTRAL,
    CONF_HIGH,
    CONF_MED,
    CONF_LOW,
)

TICK = 0.10
MID = 90_000.0


@pytest.fixture(scope="session", autouse=True)
def warmup():
    encode_signal(0.5)
    calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)


class TestGapSignalPipelineEndToEnd:
    """
    Simulate the full hot path:
        gap_score -> encode_signal -> calculate_quotes_fast

    These mirror real observations from a live BTCUSDT order book.
    """

    def test_heavy_ask_pipeline(self):
        """High gap score → SELL signal → tight ask, wide bid."""
        gap_score = 0.82  # lots of ask-side liquidity beyond gap
        signal, conf = encode_signal(gap_score)
        assert signal == SIGNAL_DOWN
        assert conf == CONF_HIGH

        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, signal, conf, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 1.0
        assert ask > bid

    def test_heavy_bid_pipeline(self):
        """Low gap score → BUY signal → tight bid, wide ask."""
        gap_score = 0.18  # lots of bid-side liquidity beyond gap
        signal, conf = encode_signal(gap_score)
        assert signal == SIGNAL_UP
        assert conf == CONF_HIGH

        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, signal, conf, TICK)
        assert bid_edge == 1.0
        assert ask_edge == 100.0

    def test_balanced_book_pipeline(self):
        """Score ≈ 0.5 → NEUTRAL / sit out."""
        gap_score = 0.5
        signal, conf = encode_signal(gap_score)
        assert signal == SIGNAL_NEUTRAL
        assert conf == CONF_LOW

        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, signal, conf, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 100.0

    def test_med_confidence_ask_heavy(self):
        gap_score = 0.62
        signal, conf = encode_signal(gap_score)
        assert signal == SIGNAL_DOWN
        assert conf == CONF_MED

        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, signal, conf, TICK)
        assert ask_edge == 1.0
        assert bid_edge == 100.0

    def test_quotes_never_cross(self):
        """bid < ask for any signal."""
        for score in [0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0]:
            signal, conf = encode_signal(score)
            bid, ask, _, _, _ = calculate_quotes_fast(MID, signal, conf, TICK)
            assert bid < ask, f"bid {bid} >= ask {ask} for score={score}"

    def test_spread_consistent_with_edges(self):
        gap_score = 0.9
        signal, conf = encode_signal(gap_score)
        bid, ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(MID, signal, conf, TICK)
        expected_spread = (ask - bid) / TICK
        assert abs(spread_ticks - expected_spread) < 1e-6
