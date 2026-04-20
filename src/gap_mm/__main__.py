"""
CLI entry point.

Usage:
    python -m gap_mm
    poetry run python -m gap_mm

All configuration is read from the .env file (or environment variables).
See .env.example for a full list of options.

WARNING: This places REAL ORDERS on the exchange.
"""

import os
import sys

from dotenv import load_dotenv

from gap_mm.live import LiveTradingEngine


def main() -> None:
    load_dotenv()

    api_key = os.getenv("BYBIT_API_KEY")
    api_secret = os.getenv("BYBIT_API_SECRET")

    if not api_key or not api_secret:
        print("=" * 70)
        print("ERROR: Missing API credentials.")
        print("Copy .env.example to .env and fill in BYBIT_API_KEY / BYBIT_API_SECRET.")
        print("=" * 70)
        sys.exit(1)

    symbol = os.getenv("TRADING_SYMBOL", "BTCUSDT")
    market_type = os.getenv("MARKET_TYPE", "spot")
    tick_size = float(os.getenv("TICK_SIZE", "0.10"))
    max_position = float(os.getenv("MAX_POSITION_SIZE", "0.01"))
    min_order_size = float(os.getenv("MIN_ORDER_SIZE", "0.001"))
    min_update_interval = float(os.getenv("MIN_UPDATE_INTERVAL", "0.0"))

    print("\n" + "=" * 70)
    print("LIVE TRADING MODE")
    print("=" * 70)
    print(f"  Symbol:         {symbol}")
    print(f"  Market:         {market_type}")
    print(f"  Max position:   {max_position} {symbol[:3]}")
    print(f"  Min order size: {min_order_size} {symbol[:3]}")
    print(f"  Tick size:      {tick_size}")
    print(f"  Update interval:{min_update_interval}s")
    print("\nThis will place REAL ORDERS with REAL MONEY on Bybit.")
    print("=" * 70 + "\n")

    response = input("Type YES to confirm: ")
    if response.strip().upper() != "YES":
        print("Aborted.")
        sys.exit(0)

    engine = LiveTradingEngine(
        api_key=api_key,
        api_secret=api_secret,
        symbol=symbol,
        market_type=market_type,
        tick_size=tick_size,
        max_position=max_position,
        min_order_size=min_order_size,
        min_update_interval=min_update_interval,
    )

    try:
        engine.start()
    except KeyboardInterrupt:
        print("\n\n" + "=" * 70)
        print("Engine stopped (Ctrl+C)")
        engine._print_stats()
        print("=" * 70)


if __name__ == "__main__":
    main()
