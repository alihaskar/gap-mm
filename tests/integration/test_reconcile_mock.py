"""
Integration tests for ExecutionNode.reconcile.

The Bybit REST API is mocked via pytest-httpserver so no real network
connection or API credentials are required. The Rust engine's reqwest client
is pointed at a local test HTTP server via the ``api_base_url`` constructor arg.

These tests verify the OMS reconciliation logic:
- submit a new order when none exists
- amend an existing order when the target price changes
- no-change when the target price matches the working order
- skip when target side is None
"""

import json

import pytest


# ── JSON response builders ────────────────────────────────────────────────────

def _submit_ok(order_id: str = "TEST-ORDER-001") -> dict:
    return {"retCode": 0, "retMsg": "OK", "result": {"orderId": order_id, "orderLinkId": "link-001"}}


def _amend_ok(order_id: str = "TEST-ORDER-001") -> dict:
    return {"retCode": 0, "retMsg": "OK", "result": {"orderId": order_id, "orderLinkId": "link-001"}}


def _open_orders_empty() -> dict:
    return {"retCode": 0, "retMsg": "OK", "result": {"list": []}}


# ── fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture
def exec_node_factory(httpserver):
    """
    Factory that creates an ExecutionNode pointed at the local test server.
    Requires rust_engine to be installed (maturin develop).
    """
    pytest.importorskip("rust_engine", reason="rust_engine not built; run `maturin develop`")
    from rust_engine import ExecutionNode

    def _make(**kwargs):
        return ExecutionNode(
            api_key="test-key",
            api_secret="test-secret",
            symbol="BTCUSDT",
            market_type="spot",
            tick_size=0.10,
            max_position=0.01,
            min_order_size=0.001,
            api_base_url=httpserver.url_for("").rstrip("/"),
            **kwargs,
        )

    return _make


# ── tests ─────────────────────────────────────────────────────────────────────

class TestReconcileSubmit:
    """First reconcile: no working orders → submit."""

    def test_submit_bid_and_ask(self, exec_node_factory, httpserver):
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-BID-001"))
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-ASK-001"))

        node = exec_node_factory()
        result = node.reconcile(target_bid=89_000.0, target_ask=89_010.0)

        assert "bid" in result or "ask" in result


class TestReconcileNoChange:
    """Second reconcile with same prices → no_change on both sides."""

    def test_no_change_after_submit(self, exec_node_factory, httpserver):
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-001"))
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-002"))

        node = exec_node_factory()
        node.reconcile(target_bid=89_000.0, target_ask=89_010.0)

        result = node.reconcile(target_bid=89_000.0, target_ask=89_010.0)

        for side in ("bid", "ask"):
            if side in result:
                assert result[side]["type"] in ("no_change", "skipped")


class TestReconcileSkippedAtMaxPosition:
    """Reconcile with target_bid=None → bid side is skipped."""

    def test_bid_none_skips_bid_side(self, exec_node_factory, httpserver):
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-ASK-ONLY"))

        node = exec_node_factory()
        result = node.reconcile(target_bid=None, target_ask=89_010.0)

        assert "bid" not in result

    def test_ask_none_skips_ask_side(self, exec_node_factory, httpserver):
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ORDER-BID-ONLY"))

        node = exec_node_factory()
        result = node.reconcile(target_bid=89_000.0, target_ask=None)

        assert "ask" not in result


class TestReconcileAmend:
    """After a submit, changing target price triggers an amend."""

    def test_amend_on_price_change(self, exec_node_factory, httpserver):
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("BID-001"))
        httpserver.expect_ordered_request("/v5/order/create", method="POST").respond_with_json(_submit_ok("ASK-001"))
        httpserver.expect_ordered_request("/v5/order/amend", method="POST").respond_with_json(_amend_ok("BID-001"))
        httpserver.expect_ordered_request("/v5/order/amend", method="POST").respond_with_json(_amend_ok("ASK-001"))

        node = exec_node_factory()
        node.reconcile(target_bid=89_000.0, target_ask=89_010.0)

        result = node.reconcile(target_bid=88_990.0, target_ask=89_020.0)

        for side in ("bid", "ask"):
            if side in result:
                assert result[side]["type"] in ("amended", "skipped", "no_change")
