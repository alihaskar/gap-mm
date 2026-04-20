"""
Unit tests for gap_mm.engine.calculate_quotes_fast.
"""

import pytest
from gap_mm.engine import (
    calculate_quotes_fast,
    encode_signal,
    SIGNAL_UP,
    SIGNAL_DOWN,
    SIGNAL_NEUTRAL,
    CONF_HIGH,
    CONF_MED,
    CONF_LOW,
)

MID = 90_000.0
TICK = 0.10


@pytest.fixture(scope="session", autouse=True)
def warmup_jit():
    calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)


class TestQuoteEdges:
    """Bid/ask edge ticks for each signal/confidence combination."""

    def test_high_up_tight_bid_wide_ask(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)
        assert bid_edge == 1.0
        assert ask_edge == 100.0

    def test_high_down_wide_bid_tight_ask(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_DOWN, CONF_HIGH, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 1.0

    def test_med_up_tight_bid_wide_ask(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_UP, CONF_MED, TICK)
        assert bid_edge == 1.0
        assert ask_edge == 100.0

    def test_med_down_wide_bid_tight_ask(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_DOWN, CONF_MED, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 1.0

    def test_low_confidence_both_wide(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_NEUTRAL, CONF_LOW, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 100.0

    def test_high_neutral_both_wide(self):
        bid, ask, bid_edge, ask_edge, _ = calculate_quotes_fast(MID, SIGNAL_NEUTRAL, CONF_HIGH, TICK)
        assert bid_edge == 100.0
        assert ask_edge == 100.0


class TestQuotePrices:
    """Bid/ask prices are computed correctly from mid and edges."""

    def test_bid_price_1_tick_below_mid(self):
        bid, _, _, _, _ = calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)
        expected = round((MID - 1 * TICK) / TICK) * TICK
        assert abs(bid - expected) < 1e-9

    def test_ask_price_1_tick_above_mid(self):
        _, ask, _, _, _ = calculate_quotes_fast(MID, SIGNAL_DOWN, CONF_HIGH, TICK)
        expected = round((MID + 1 * TICK) / TICK) * TICK
        assert abs(ask - expected) < 1e-9

    def test_bid_price_100_ticks_below_mid(self):
        bid, _, _, _, _ = calculate_quotes_fast(MID, SIGNAL_DOWN, CONF_HIGH, TICK)
        expected = round((MID - 100 * TICK) / TICK) * TICK
        assert abs(bid - expected) < 1e-9

    def test_ask_price_100_ticks_above_mid(self):
        _, ask, _, _, _ = calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)
        expected = round((MID + 100 * TICK) / TICK) * TICK
        assert abs(ask - expected) < 1e-9


class TestSpreadTicks:
    """spread_ticks == (ask - bid) / tick_size."""

    def test_spread_ticks_high_up(self):
        bid, ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(MID, SIGNAL_UP, CONF_HIGH, TICK)
        expected_spread = (ask - bid) / TICK
        assert abs(spread_ticks - expected_spread) < 1e-6

    def test_spread_ticks_low(self):
        bid, ask, bid_edge, ask_edge, spread_ticks = calculate_quotes_fast(MID, SIGNAL_NEUTRAL, CONF_LOW, TICK)
        assert spread_ticks == pytest.approx(200.0, abs=1e-6)


class TestTickRounding:
    """Prices are always multiples of tick_size."""

    @pytest.mark.parametrize("mid", [89_000.05, 89_100.12, 89_999.99])
    def test_bid_is_on_tick_grid(self, mid):
        bid, _, _, _, _ = calculate_quotes_fast(mid, SIGNAL_UP, CONF_HIGH, TICK)
        remainder = round(bid / TICK) * TICK
        assert abs(bid - remainder) < 1e-9

    @pytest.mark.parametrize("mid", [89_000.05, 89_100.12, 89_999.99])
    def test_ask_is_on_tick_grid(self, mid):
        _, ask, _, _, _ = calculate_quotes_fast(mid, SIGNAL_DOWN, CONF_HIGH, TICK)
        remainder = round(ask / TICK) * TICK
        assert abs(ask - remainder) < 1e-9

    def test_non_default_tick_size(self):
        tick = 0.01
        mid = 3000.005
        bid, ask, _, _, _ = calculate_quotes_fast(mid, SIGNAL_UP, CONF_HIGH, tick)
        assert abs(bid - round(bid / tick) * tick) < 1e-9
        assert abs(ask - round(ask / tick) * tick) < 1e-9
