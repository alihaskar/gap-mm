"""
Minimal example: stream Bybit order-book metrics and print them.

No orders are placed. Safe to run without API credentials.

Usage:
    poetry run python examples/minimal_stream.py
"""

from rust_engine import TradingNode
from gap_mm.engine import encode_signal, calculate_quotes_fast, decode_signal, decode_confidence


def on_market_data(data: dict) -> None:
    if data["source"] != "bybit_spot":
        return

    bid = data["bid"]
    ask = data["ask"]
    mid = data.get("mid_price") or (bid + ask) / 2
    gap_score = data.get("gap_prob_resistance_up", 0.5)

    signal_code, conf_code = encode_signal(gap_score)
    signal_text, _ = decode_signal(signal_code)
    confidence = decode_confidence(conf_code)

    mm_bid, mm_ask, bid_edge, ask_edge, _ = calculate_quotes_fast(mid, signal_code, conf_code)

    print(
        f"mid={mid:.2f}  gap_score={gap_score:.3f}  "
        f"signal={signal_text}({confidence})  "
        f"bid={mm_bid:.2f}({int(bid_edge)}t)  ask={mm_ask:.2f}({int(ask_edge)}t)"
    )


if __name__ == "__main__":
    node = TradingNode()
    try:
        node.start_stream(on_market_data, symbol="BTCUSDT")
    except KeyboardInterrupt:
        node.stop()
        print("\nStopped.")
