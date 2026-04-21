"""
Unit tests for gap_mm.engine.calculate_pnl_fast and check_signal_correct.
"""

import pytest

from gap_mm.engine import (
    SIGNAL_DOWN,
    SIGNAL_NEUTRAL,
    SIGNAL_UP,
    calculate_pnl_fast,
    check_signal_correct,
)


@pytest.fixture(scope="session", autouse=True)
def warmup_jit():
    calculate_pnl_fast(100.0, 101.0, 1, 1.0)
    check_signal_correct(SIGNAL_UP, 1.0)


class TestCalculatePnlFast:
    """P&L calculation for long and short positions."""

    def test_long_profit(self):
        pnl, pnl_bps = calculate_pnl_fast(100.0, 101.0, 1, 1.0)
        assert pnl == pytest.approx(1.0)
        assert pnl_bps == pytest.approx(100.0)

    def test_long_loss(self):
        pnl, pnl_bps = calculate_pnl_fast(100.0, 99.0, 1, 1.0)
        assert pnl == pytest.approx(-1.0)
        assert pnl_bps == pytest.approx(-100.0)

    def test_short_profit(self):
        pnl, pnl_bps = calculate_pnl_fast(100.0, 99.0, -1, 1.0)
        assert pnl == pytest.approx(1.0)
        assert pnl_bps == pytest.approx(100.0)

    def test_short_loss(self):
        pnl, pnl_bps = calculate_pnl_fast(100.0, 101.0, -1, 1.0)
        assert pnl == pytest.approx(-1.0)
        assert pnl_bps == pytest.approx(-100.0)

    def test_zero_price_change(self):
        pnl, pnl_bps = calculate_pnl_fast(100.0, 100.0, 1, 1.0)
        assert pnl == pytest.approx(0.0)
        assert pnl_bps == pytest.approx(0.0)

    def test_quantity_scales_pnl(self):
        pnl_1, _ = calculate_pnl_fast(100.0, 101.0, 1, 1.0)
        pnl_5, _ = calculate_pnl_fast(100.0, 101.0, 1, 5.0)
        assert pnl_5 == pytest.approx(pnl_1 * 5)

    def test_bps_calculation(self):
        # 1% price change on a long → 100 bps
        pnl, pnl_bps = calculate_pnl_fast(1000.0, 1010.0, 1, 1.0)
        assert pnl_bps == pytest.approx(100.0, rel=1e-6)


class TestCheckSignalCorrect:
    """Signal correctness evaluation."""

    def test_up_signal_with_positive_change_is_correct(self):
        assert check_signal_correct(SIGNAL_UP, 0.01) == 1

    def test_up_signal_with_negative_change_is_wrong(self):
        assert check_signal_correct(SIGNAL_UP, -0.01) == -1

    def test_down_signal_with_negative_change_is_correct(self):
        assert check_signal_correct(SIGNAL_DOWN, -0.01) == 1

    def test_down_signal_with_positive_change_is_wrong(self):
        assert check_signal_correct(SIGNAL_DOWN, 0.01) == -1

    def test_neutral_signal_always_zero(self):
        assert check_signal_correct(SIGNAL_NEUTRAL, 0.01) == 0
        assert check_signal_correct(SIGNAL_NEUTRAL, -0.01) == 0
        assert check_signal_correct(SIGNAL_NEUTRAL, 0.0) == 0

    def test_up_signal_with_zero_change_is_wrong(self):
        # price_change == 0 does not match > 0 condition → wrong
        assert check_signal_correct(SIGNAL_UP, 0.0) == -1

    def test_down_signal_with_zero_change_is_wrong(self):
        assert check_signal_correct(SIGNAL_DOWN, 0.0) == -1
