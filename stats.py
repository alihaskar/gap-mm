"""
Fetch and analyze trade statistics from Bybit for the last 24 hours.
Calculates volume and weighted average prices for buys/sells.
"""

import os
import time
import hmac
import hashlib
import csv
from datetime import datetime, timedelta
from dotenv import load_dotenv
import requests

load_dotenv()

BASE_URL = "https://api.bybit.com"


def generate_signature(api_secret: str, params: str) -> str:
    """Generate HMAC SHA256 signature."""
    return hmac.new(
        api_secret.encode('utf-8'),
        params.encode('utf-8'),
        hashlib.sha256
    ).hexdigest()


def get_trades_last_24h(api_key: str, api_secret: str, symbol: str) -> list:
    """Fetch all trades for the last 24 hours."""
    end_time = datetime.now()
    start_time = end_time - timedelta(hours=24)
    
    # Convert to milliseconds
    start_ms = int(start_time.timestamp() * 1000)
    end_ms = int(end_time.timestamp() * 1000)
    
    all_trades = []
    
    print(f"Fetching trades from {start_time} to {end_time}...")
    
    endpoint = "/v5/execution/list"
    
    # Bybit returns max 100 trades per request, need to paginate
    while True:
        timestamp = str(int(time.time() * 1000))
        
        params = f"category=spot&symbol={symbol}&startTime={start_ms}&endTime={end_ms}&limit=100"
        param_str = f"{timestamp}{api_key}5000{params}"
        signature = generate_signature(api_secret, param_str)
        
        headers = {
            "X-BAPI-API-KEY": api_key,
            "X-BAPI-SIGN": signature,
            "X-BAPI-SIGN-TYPE": "2",
            "X-BAPI-TIMESTAMP": timestamp,
            "X-BAPI-RECV-WINDOW": "5000"
        }
        
        url = f"{BASE_URL}{endpoint}?{params}"
        response = requests.get(url, headers=headers)
        data = response.json()
        
        if data['retCode'] != 0:
            print(f"Error: {data['retMsg']}")
            break
            
        trades = data['result']['list']
        
        if not trades:
            break
            
        all_trades.extend(trades)
        print(f"Fetched {len(trades)} trades (total: {len(all_trades)})...")
        
        # Get older trades
        oldest_time = int(trades[-1]['execTime'])
        if oldest_time <= start_ms:
            break
            
        end_ms = oldest_time - 1
    
    return all_trades


def export_trades_to_csv(trades: list, filename: str = "trades_export.csv"):
    """Export all trade data to CSV."""
    if not trades:
        print("No trades to export")
        return
    
    # Get all unique keys from trades
    all_keys = set()
    for trade in trades:
        all_keys.update(trade.keys())
    
    fieldnames = sorted(list(all_keys))
    
    with open(filename, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(trades)
    
    print(f"✓ Exported {len(trades)} trades to {filename}")


def get_klines(api_key: str, api_secret: str, symbol: str, interval: str = "1") -> list:
    """Fetch klines (candles) for the last 24 hours."""
    end_time = datetime.now()
    start_time = end_time - timedelta(hours=24)
    
    # Convert to milliseconds
    start_ms = int(start_time.timestamp() * 1000)
    end_ms = int(end_time.timestamp() * 1000)
    
    all_klines = []
    
    print(f"Fetching {interval}-minute klines from {start_time} to {end_time}...")
    
    endpoint = "/v5/market/kline"
    
    # Bybit returns max 1000 klines per request
    current_start = start_ms
    
    while current_start < end_ms:
        timestamp = str(int(time.time() * 1000))
        
        params = f"category=spot&symbol={symbol}&interval={interval}&start={current_start}&end={end_ms}&limit=1000"
        param_str = f"{timestamp}{api_key}5000{params}"
        signature = generate_signature(api_secret, param_str)
        
        headers = {
            "X-BAPI-API-KEY": api_key,
            "X-BAPI-SIGN": signature,
            "X-BAPI-SIGN-TYPE": "2",
            "X-BAPI-TIMESTAMP": timestamp,
            "X-BAPI-RECV-WINDOW": "5000"
        }
        
        url = f"{BASE_URL}{endpoint}?{params}"
        response = requests.get(url, headers=headers)
        data = response.json()
        
        if data['retCode'] != 0:
            print(f"Error fetching klines: {data['retMsg']}")
            break
        
        klines = data['result']['list']
        
        if not klines:
            break
        
        all_klines.extend(klines)
        print(f"Fetched {len(klines)} klines (total: {len(all_klines)})...")
        
        # Get next batch (klines are sorted newest first)
        oldest_time = int(klines[-1][0])
        if oldest_time <= start_ms:
            break
        
        current_start = oldest_time + 60000  # Move forward 1 minute
    
    # Sort by timestamp ascending
    all_klines.sort(key=lambda x: int(x[0]))
    
    return all_klines


def export_klines_to_csv(klines: list, filename: str = "klines_export.csv"):
    """Export klines to CSV."""
    if not klines:
        print("No klines to export")
        return
    
    fieldnames = ['timestamp', 'open', 'high', 'low', 'close', 'volume', 'turnover']
    
    with open(filename, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        
        for kline in klines:
            # Convert timestamp to readable format
            ts = int(kline[0])
            dt = datetime.fromtimestamp(ts / 1000)
            
            writer.writerow({
                'timestamp': dt.strftime('%Y-%m-%d %H:%M:%S'),
                'open': kline[1],
                'high': kline[2],
                'low': kline[3],
                'close': kline[4],
                'volume': kline[5],
                'turnover': kline[6]
            })
    
    print(f"✓ Exported {len(klines)} klines to {filename}")


def calculate_stats(trades: list) -> dict:
    """Calculate volume and weighted average prices."""
    if not trades:
        return {}
    
    total_volume_usdt = 0.0
    buy_volume_usdt = 0.0
    sell_volume_usdt = 0.0
    buy_volume_btc = 0.0
    sell_volume_btc = 0.0
    buy_value = 0.0
    sell_value = 0.0
    total_fees = 0.0
    maker_rebates = 0.0
    taker_fees = 0.0
    
    # Get actual time span from trades
    timestamps = [int(trade['execTime']) for trade in trades]
    first_trade_ms = min(timestamps)
    last_trade_ms = max(timestamps)
    time_span_hours = (last_trade_ms - first_trade_ms) / (1000 * 60 * 60)
    
    # Prevent division by zero for very short time spans
    if time_span_hours < 0.01:
        time_span_hours = 0.01
    
    # Group trades by hour for profit breakdown
    hourly_stats = {}
    
    for trade in trades:
        qty = float(trade['execQty'])
        price = float(trade['execPrice'])
        side = trade['side']  # 'Buy' or 'Sell'
        value = qty * price
        trade_time_ms = int(trade['execTime'])
        
        # Extract fee (negative = rebate, positive = paid)
        fee = float(trade.get('execFee', 0))
        is_maker = trade.get('isMaker', '0') == '1'
        
        total_fees += fee
        if is_maker:
            maker_rebates += fee
        else:
            taker_fees += fee
        
        # Calculate hour bucket (hours since first trade)
        hour_bucket = int((trade_time_ms - first_trade_ms) / (1000 * 60 * 60))
        
        if hour_bucket not in hourly_stats:
            hourly_stats[hour_bucket] = {
                'buy_value': 0.0,
                'sell_value': 0.0,
                'trades': 0,
                'volume': 0.0,
                'fees': 0.0
            }
        
        hourly_stats[hour_bucket]['trades'] += 1
        hourly_stats[hour_bucket]['volume'] += value
        hourly_stats[hour_bucket]['fees'] += fee
        
        total_volume_usdt += value
        
        if side == 'Buy':
            buy_volume_btc += qty
            buy_volume_usdt += value
            buy_value += value
            hourly_stats[hour_bucket]['buy_value'] += value
        else:
            sell_volume_btc += qty
            sell_volume_usdt += value
            sell_value += value
            hourly_stats[hour_bucket]['sell_value'] += value
    
    # Calculate profit for each hour
    hourly_profit = {}
    for hour, stats in hourly_stats.items():
        hourly_profit[hour] = {
            'profit': stats['sell_value'] - stats['buy_value'],
            'fees': stats['fees'],
            'net_profit': (stats['sell_value'] - stats['buy_value']) + stats['fees'],
            'trades': stats['trades'],
            'volume': stats['volume']
        }
    
    # Calculate weighted averages
    avg_buy_price = buy_value / buy_volume_btc if buy_volume_btc > 0 else 0
    avg_sell_price = sell_value / sell_volume_btc if sell_volume_btc > 0 else 0
    
    # Total realized profit/loss
    total_profit = sell_value - buy_value
    total_net_profit = total_profit + total_fees  # fees are negative for rebates
    
    # Calculate hourly averages based on actual time span
    total_trades = len(trades)
    avg_volume_per_hour = total_volume_usdt / time_span_hours
    avg_trades_per_hour = total_trades / time_span_hours
    avg_trade_size = total_volume_usdt / total_trades if total_trades > 0 else 0
    avg_profit_per_hour = total_profit / time_span_hours
    avg_net_profit_per_hour = total_net_profit / time_span_hours
    avg_fees_per_hour = total_fees / time_span_hours
    
    return {
        'total_trades': total_trades,
        'total_volume_usdt': total_volume_usdt,
        'buy_volume_usdt': buy_volume_usdt,
        'sell_volume_usdt': sell_volume_usdt,
        'avg_buy_price': avg_buy_price,
        'avg_sell_price': avg_sell_price,
        'avg_volume_per_hour': avg_volume_per_hour,
        'avg_trades_per_hour': avg_trades_per_hour,
        'avg_trade_size': avg_trade_size,
        'avg_profit_per_hour': avg_profit_per_hour,
        'avg_net_profit_per_hour': avg_net_profit_per_hour,
        'avg_fees_per_hour': avg_fees_per_hour,
        'total_profit': total_profit,
        'total_fees': total_fees,
        'total_net_profit': total_net_profit,
        'maker_rebates': maker_rebates,
        'taker_fees': taker_fees,
        'time_span_hours': time_span_hours,
        'first_trade_time': datetime.fromtimestamp(first_trade_ms / 1000),
        'last_trade_time': datetime.fromtimestamp(last_trade_ms / 1000),
        'hourly_profit': hourly_profit,
        'first_trade_ms': first_trade_ms,
    }


def main():
    # Load credentials
    api_key = os.getenv("BYBIT_API_KEY")
    api_secret = os.getenv("BYBIT_API_SECRET")
    symbol = os.getenv("TRADING_SYMBOL", "BTCUSDT")
    
    if not api_key or not api_secret:
        print("ERROR: API credentials not found in .env file")
        return
    
    print("="*70)
    print(f"TRADE STATISTICS - {symbol}")
    print("="*70)
    
    # Fetch trades
    trades = get_trades_last_24h(api_key, api_secret, symbol)
    
    if not trades:
        print("\nNo trades found in the last 24 hours")
        return
    
    # Export trades to CSV
    print("\n" + "="*70)
    export_trades_to_csv(trades, "trades_export.csv")
    
    # Fetch and export klines
    print("\n" + "="*70)
    klines = get_klines(api_key, api_secret, symbol, interval="1")
    export_klines_to_csv(klines, "klines_1min_export.csv")
    
    # Calculate stats
    stats = calculate_stats(trades)
    
    # Print results
    print("\n" + "="*70)
    print("RESULTS")
    print("="*70)
    print(f"Time Span:           {stats['time_span_hours']:.2f} hours")
    print(f"First Trade:         {stats['first_trade_time']}")
    print(f"Last Trade:          {stats['last_trade_time']}")
    print(f"\nTotal Trades:        {stats['total_trades']:,}")
    print(f"Total Volume:        ${stats['total_volume_usdt']:,.2f} USDT")
    print(f"\nP&L BREAKDOWN:")
    print(f"  Trading P&L:       ${stats['total_profit']:+,.2f} USDT")
    print(f"  Maker Rebates:     ${abs(stats['maker_rebates']):,.2f} USDT (earned)")
    print(f"  Taker Fees:        ${abs(stats['taker_fees']):,.2f} USDT (paid)")
    print(f"  Net Fees:          ${-stats['total_fees']:+,.2f} USDT")
    print(f"  Total Net P&L:     ${stats['total_net_profit']:+,.2f} USDT")
    print(f"\nHOURLY AVERAGES:")
    print(f"  Trades per Hour:   {stats['avg_trades_per_hour']:,.1f}")
    print(f"  Volume per Hour:   ${stats['avg_volume_per_hour']:,.2f} USDT")
    print(f"  Profit per Hour:   ${stats['avg_profit_per_hour']:+,.2f} USDT")
    print(f"  Net Fees per Hour: ${-stats['avg_fees_per_hour']:+,.2f} USDT")
    print(f"  Net P&L per Hour:  ${stats['avg_net_profit_per_hour']:+,.2f} USDT")
    print(f"  Avg Trade Size:    ${stats['avg_trade_size']:,.2f} USDT")
    print(f"\nBUY SIDE:")
    print(f"  Volume:            ${stats['buy_volume_usdt']:,.2f} USDT")
    print(f"  Weighted Avg Price: ${stats['avg_buy_price']:,.2f}")
    print(f"\nSELL SIDE:")
    print(f"  Volume:            ${stats['sell_volume_usdt']:,.2f} USDT")
    print(f"  Weighted Avg Price: ${stats['avg_sell_price']:,.2f}")
    print(f"\nNet Volume:          ${stats['buy_volume_usdt'] - stats['sell_volume_usdt']:+,.2f} USDT")
    print(f"Price Difference:    ${stats['avg_sell_price'] - stats['avg_buy_price']:+,.2f}")
    
    # Hourly profit breakdown
    print(f"\n" + "="*70)
    print("HOURLY PROFIT BREAKDOWN")
    print("="*70)
    
    first_trade_time = stats['first_trade_time']
    for hour in sorted(stats['hourly_profit'].keys()):
        hour_data = stats['hourly_profit'][hour]
        hour_time = first_trade_time + timedelta(hours=hour)
        profit = hour_data['profit']
        fees = -hour_data['fees']  # Negate to show rebates as positive
        net_profit = hour_data['net_profit']
        trades = hour_data['trades']
        volume = hour_data['volume']
        
        print(f"Hour {hour:2d} ({hour_time.strftime('%H:%M')}): "
              f"P&L: ${profit:+8,.2f} | "
              f"Fees: ${fees:+7,.2f} | "
              f"Net: ${net_profit:+8,.2f} | "
              f"{trades:4,} trades")
    
    print("="*70)


if __name__ == "__main__":
    main()
