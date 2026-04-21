"""
Unit tests for gap_mm.engine.encode_signal.

All tests run without network or API keys.
Numba JIT is warmed up once per session via the autouse fixture.
"""

import pytest

from gap_mm.engine import (
    CONF_HIGH,
    CONF_LOW,
    CONF_MED,
    SIGNAL_DOWN,
    SIGNAL_NEUTRAL,
    SIGNAL_UP,
    encode_signal,
)


@pytest.fixture(scope="session", autouse=True)
def warmup_jit():
    """Trigger JIT compilation before any test runs."""
    encode_signal(0.5)
    encode_signal(0.9)
    encode_signal(0.1)


class TestEncodeSignalDirection:
    """Signal direction: contrarian mapping of ask-side liquidity score."""

    def test_high_ask_score_gives_sell(self):
        # Heavy resistance above → expect price to go down → SELL
        signal, _ = encode_signal(0.9)
        assert signal == SIGNAL_DOWN

    def test_low_ask_score_gives_buy(self):
        # Heavy support below → expect price to go up → BUY
        signal, _ = encode_signal(0.1)
        assert signal == SIGNAL_UP

    def test_neutral_score_gives_neutral(self):
        signal, _ = encode_signal(0.5)
        assert signal == SIGNAL_NEUTRAL

    def test_just_above_midpoint_gives_sell(self):
        signal, _ = encode_signal(0.501)
        assert signal == SIGNAL_DOWN

    def test_just_below_midpoint_gives_buy(self):
        signal, _ = encode_signal(0.499)
        assert signal == SIGNAL_UP


class TestEncodeSignalConfidence:
    """Confidence levels based on distance from 0.5."""

    def test_score_0_9_is_high_confidence(self):
        _, conf = encode_signal(0.9)
        assert conf == CONF_HIGH

    def test_score_0_1_is_high_confidence(self):
        _, conf = encode_signal(0.1)
        assert conf == CONF_HIGH

    def test_score_0_65_is_med_confidence(self):
        _, conf = encode_signal(0.65)
        assert conf == CONF_MED

    def test_score_0_35_is_med_confidence(self):
        _, conf = encode_signal(0.35)
        assert conf == CONF_MED

    def test_score_0_5_is_low_confidence(self):
        _, conf = encode_signal(0.5)
        assert conf == CONF_LOW

    def test_boundary_just_above_70_is_high(self):
        _, conf = encode_signal(0.701)
        assert conf == CONF_HIGH

    def test_boundary_just_below_70_is_med(self):
        _, conf = encode_signal(0.699)
        assert conf == CONF_MED

    def test_boundary_just_below_30_is_high(self):
        _, conf = encode_signal(0.299)
        assert conf == CONF_HIGH

    def test_boundary_just_above_30_is_med(self):
        _, conf = encode_signal(0.301)
        assert conf == CONF_MED


class TestEncodeSignalBoundaryValues:
    """Test the exact threshold values."""

    def test_exactly_0_500001_is_sell(self):
        signal, _ = encode_signal(0.500001)
        assert signal == SIGNAL_DOWN

    def test_exactly_0_499999_is_buy(self):
        signal, _ = encode_signal(0.499999)
        assert signal == SIGNAL_UP

    def test_score_0_0_is_buy_high(self):
        signal, conf = encode_signal(0.0)
        assert signal == SIGNAL_UP
        assert conf == CONF_HIGH

    def test_score_1_0_is_sell_high(self):
        signal, conf = encode_signal(1.0)
        assert signal == SIGNAL_DOWN
        assert conf == CONF_HIGH
