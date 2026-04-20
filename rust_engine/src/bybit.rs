use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use orderbook_rs::{OrderBook, OrderId, Side, TimeInForce};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const BYBIT_SPOT_WS: &str = "wss://stream.bybit.com/v5/public/spot";
const BYBIT_LINEAR_WS: &str = "wss://stream.bybit.com/v5/public/linear";
const PRICE_SCALE: f64 = 1e8;

#[derive(Debug, Clone)]
pub enum BybitMessage {
    Snapshot {
        symbol: String,
        source: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        #[allow(dead_code)]
        update_id: u64,
        timestamp: u64,
    },
    Delta {
        symbol: String,
        source: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        #[allow(dead_code)]
        update_id: u64,
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    pub symbol: String,
    pub source: String,
    pub bid: f64,
    pub ask: f64,
    pub spread: f64,
    pub mid_price: Option<f64>,
    pub spread_bps: Option<f64>,
    pub imbalance: f64,
    pub bid_depth_5: u64,
    pub ask_depth_5: u64,
    pub timestamp: u64,
    /// Normalized ask-side gap-liquidity score in [0, 1].
    ///
    /// Computed as:
    ///   score_ask / (score_ask + score_bid)
    /// where score_X = liquidity_beyond_gap_X / (gap_distance_X + ε).
    ///
    /// Interpretation (contrarian):
    ///   > 0.5 → more resistance on ask side → price likely goes down
    ///   < 0.5 → more support on bid side   → price likely goes up
    ///   ≈ 0.5 → balanced
    pub gap_prob_resistance_up: f64,
    pub gap_distance_up: u64,
    pub gap_distance_dn: u64,
    pub liquidity_up: u64,
    pub liquidity_dn: u64,
}

#[derive(Debug, Deserialize)]
struct OrderBookResponse {
    #[allow(dead_code)]
    topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    ts: u64,
    data: OrderBookData,
}

#[derive(Debug, Deserialize)]
struct OrderBookData {
    s: String,
    b: Vec<Vec<String>>,
    a: Vec<Vec<String>>,
    u: u64,
    #[allow(dead_code)]
    seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LevelKey {
    side: u8,
    price: u128,
}

pub struct OrderBookState {
    book: OrderBook<()>,
    levels: HashMap<LevelKey, OrderId>,
    prev_best_bid: Option<u128>,
    prev_best_ask: Option<u128>,
    /// Tick size in scaled integer units (price_f64 * PRICE_SCALE).
    tick_size_scaled: u128,
}

impl OrderBookState {
    pub fn new(symbol: &str, tick_size: f64) -> Self {
        let tick_size_scaled = (tick_size * PRICE_SCALE).round() as u128;
        Self {
            book: OrderBook::<()>::new(symbol),
            levels: HashMap::new(),
            prev_best_bid: None,
            prev_best_ask: None,
            tick_size_scaled,
        }
    }

    pub fn reset(&mut self, symbol: &str) {
        let tick_size_scaled = self.tick_size_scaled;
        *self = Self {
            book: OrderBook::<()>::new(symbol),
            levels: HashMap::new(),
            prev_best_bid: None,
            prev_best_ask: None,
            tick_size_scaled,
        };
    }

    pub fn apply_levels(&mut self, side: Side, levels: &[(f64, f64)]) {
        for (price_f64, qty_f64) in levels {
            let Some(price) = parse_price(*price_f64) else {
                continue;
            };
            let Some(quantity) = parse_quantity(*qty_f64) else {
                continue;
            };

            let key = LevelKey {
                side: match side {
                    Side::Buy => 0,
                    Side::Sell => 1,
                },
                price,
            };

            if quantity == 0 {
                if let Some(order_id) = self.levels.remove(&key) {
                    let _ = self.book.cancel_order(order_id);
                }
                continue;
            }

            let order_id = self
                .levels
                .get(&key)
                .cloned()
                .unwrap_or_else(OrderId::new_uuid);

            let _ = self.book.cancel_order(order_id);

            if self
                .book
                .add_limit_order(order_id, price, quantity, side, TimeInForce::Gtc, None)
                .is_ok()
            {
                self.levels.insert(key, order_id);
            }
        }
    }

    pub fn get_best_bid_ask(&self) -> Option<(f64, f64)> {
        let best_bid = self.book.best_bid()?;
        let best_ask = self.book.best_ask()?;
        Some((
            (best_bid as f64) / PRICE_SCALE,
            (best_ask as f64) / PRICE_SCALE,
        ))
    }

    pub fn get_enriched_update(
        &self,
        symbol: &str,
        source: &str,
        timestamp: u64,
    ) -> Option<MarketUpdate> {
        let best_bid = self.book.best_bid()?;
        let best_ask = self.book.best_ask()?;

        let mid_price = self.book.mid_price();
        let spread_bps = self.book.spread_bps(None);
        let imbalance = self.book.order_book_imbalance(5);

        let bid_depth_5 = self.book.total_depth_at_levels(5, Side::Buy);
        let ask_depth_5 = self.book.total_depth_at_levels(5, Side::Sell);

        let (gap_prob_resistance_up, gap_dist_up, gap_dist_dn, liq_up, liq_dn) =
            self.calculate_gap_probability(best_bid, best_ask);

        Some(MarketUpdate {
            symbol: symbol.to_string(),
            source: source.to_string(),
            bid: (best_bid as f64) / PRICE_SCALE,
            ask: (best_ask as f64) / PRICE_SCALE,
            spread: ((best_ask - best_bid) as f64) / PRICE_SCALE,
            mid_price: mid_price.map(|p| p / PRICE_SCALE),
            spread_bps: spread_bps.map(|bps| bps / PRICE_SCALE),
            imbalance,
            bid_depth_5: (bid_depth_5 as f64 / PRICE_SCALE) as u64,
            ask_depth_5: (ask_depth_5 as f64 / PRICE_SCALE) as u64,
            timestamp,
            gap_prob_resistance_up,
            gap_distance_up: gap_dist_up,
            gap_distance_dn: gap_dist_dn,
            liquidity_up: liq_up,
            liquidity_dn: liq_dn,
        })
    }

    /// Compute the ask-side gap-liquidity score and raw gap metrics.
    ///
    /// Scans up to `MAX_SCAN_TICKS` ticks from the best bid/ask in each
    /// direction to find the first empty stretch ("gap"), then sums the
    /// liquidity in the next `LIQUIDITY_LEVELS` levels beyond the gap.
    ///
    /// Returns `(gap_prob_resistance_up, gap_dist_up, gap_dist_dn, liq_up, liq_dn)`.
    fn calculate_gap_probability(
        &self,
        best_bid: u128,
        best_ask: u128,
    ) -> (f64, u64, u64, u64, u64) {
        const MAX_SCAN_TICKS: u64 = 100;
        const LIQUIDITY_LEVELS: usize = 5;
        const EPSILON: f64 = 1.0;

        let tick = self.tick_size_scaled;
        let asks = self.book.get_asks();
        let bids = self.book.get_bids();

        // Scan upward from best ask
        let mut gap_up = 0u64;
        let mut price_up = best_ask;
        let mut found_up = false;
        for _ in 0..MAX_SCAN_TICKS {
            price_up += tick;
            if let Some(level) = asks.get(&price_up) {
                if level.total_quantity() > 0 {
                    found_up = true;
                    break;
                }
            }
            gap_up += 1;
        }

        // Scan downward from best bid
        let mut gap_dn = 0u64;
        let mut price_dn = best_bid;
        let mut found_dn = false;
        for _ in 0..MAX_SCAN_TICKS {
            if price_dn < tick {
                break;
            }
            price_dn -= tick;
            if let Some(level) = bids.get(&price_dn) {
                if level.total_quantity() > 0 {
                    found_dn = true;
                    break;
                }
            }
            gap_dn += 1;
        }

        // Aggregate liquidity beyond each gap
        let mut liquidity_up = 0u64;
        if found_up {
            let mut p = price_up;
            for _ in 0..LIQUIDITY_LEVELS {
                if let Some(level) = asks.get(&p) {
                    liquidity_up += level.total_quantity();
                }
                p += tick;
            }
        }

        let mut liquidity_dn = 0u64;
        if found_dn {
            let mut p = price_dn;
            for _ in 0..LIQUIDITY_LEVELS {
                if let Some(level) = bids.get(&p) {
                    liquidity_dn += level.total_quantity();
                }
                if p < tick {
                    break;
                }
                p -= tick;
            }
        }

        let v_up = liquidity_up as f64;
        let v_dn = liquidity_dn as f64;
        let score_up = v_up / (gap_up as f64 + EPSILON);
        let score_dn = v_dn / (gap_dn as f64 + EPSILON);

        let gap_prob_resistance_up = if score_up + score_dn > 0.0 {
            score_up / (score_up + score_dn)
        } else {
            0.5
        };

        (gap_prob_resistance_up, gap_up, gap_dn, liquidity_up, liquidity_dn)
    }

    pub fn update_and_check_change(&mut self) -> bool {
        let best_bid = self.book.best_bid();
        let best_ask = self.book.best_ask();
        let changed = self.prev_best_bid != best_bid || self.prev_best_ask != best_ask;
        self.prev_best_bid = best_bid;
        self.prev_best_ask = best_ask;
        changed
    }
}

pub async fn start_bybit_streams(symbol: &str) -> Result<mpsc::Receiver<BybitMessage>> {
    let (tx, rx) = mpsc::channel(1000);

    let spot_tx = tx.clone();
    let symbol_spot = symbol.to_string();
    tokio::spawn(async move {
        loop {
            match connect_ws_with_reconnect(BYBIT_SPOT_WS, &symbol_spot, "bybit_spot", spot_tx.clone()).await {
                Ok(_) => eprintln!("[bybit_spot] Connection ended normally"),
                Err(e) => eprintln!("[bybit_spot] Connection error: {}", e),
            }
            eprintln!("[bybit_spot] Reconnecting in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    let linear_tx = tx.clone();
    let symbol_linear = symbol.to_string();
    tokio::spawn(async move {
        loop {
            match connect_ws_with_reconnect(BYBIT_LINEAR_WS, &symbol_linear, "bybit_linear_perp", linear_tx.clone()).await {
                Ok(_) => eprintln!("[bybit_linear_perp] Connection ended normally"),
                Err(e) => eprintln!("[bybit_linear_perp] Connection error: {}", e),
            }
            eprintln!("[bybit_linear_perp] Reconnecting in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    drop(tx);
    Ok(rx)
}

pub async fn process_orderbook_updates(
    mut rx: mpsc::Receiver<BybitMessage>,
    tick_size: f64,
    mut callback: impl FnMut(MarketUpdate) + Send + 'static,
) {
    let mut books: HashMap<String, OrderBookState> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        let (symbol, source, bids, asks, is_snapshot, timestamp) = match msg {
            BybitMessage::Snapshot { symbol, source, bids, asks, timestamp, .. } => {
                (symbol, source, bids, asks, true, timestamp)
            }
            BybitMessage::Delta { symbol, source, bids, asks, timestamp, .. } => {
                (symbol, source, bids, asks, false, timestamp)
            }
        };

        let key = format!("{}:{}", source, symbol);
        let state = books
            .entry(key.clone())
            .or_insert_with(|| OrderBookState::new(&symbol, tick_size));

        if is_snapshot {
            state.reset(&symbol);
        }

        state.apply_levels(Side::Buy, &bids);
        state.apply_levels(Side::Sell, &asks);

        if let Some(update) = state.get_enriched_update(&symbol, &source, timestamp) {
            callback(update);
        }
    }
}

async fn connect_ws_with_reconnect(
    ws_url: &str,
    symbol: &str,
    source: &str,
    tx: mpsc::Sender<BybitMessage>,
) -> Result<()> {
    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("[{}] Connected to {}", source, ws_url);

    let (write, mut read) = ws_stream.split();
    let write = std::sync::Arc::new(tokio::sync::Mutex::new(write));

    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": [format!("orderbook.200.{}", symbol)]
    });

    {
        let mut w = write.lock().await;
        w.send(Message::Text(subscribe_msg.to_string().into())).await?;
    }
    println!("[{}] Subscribed to orderbook.200.{}", source, symbol);

    let write_ping = write.clone();
    let source_ping = source.to_string();
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(20));
        loop {
            interval.tick().await;
            let ping_msg = serde_json::json!({"op": "ping"});
            let mut w = write_ping.lock().await;
            if w.send(Message::Text(ping_msg.to_string().into())).await.is_err() {
                eprintln!("[{}] Failed to send ping", source_ping);
                break;
            }
        }
    });

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = process_ws_message(&text, source, &tx).await {
                    eprintln!("[{}] Error processing message: {}", source, e);
                }
            }
            Ok(Message::Ping(data)) => {
                let mut w = write.lock().await;
                w.send(Message::Pong(data)).await?;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                println!("[{}] WebSocket closed by server", source);
                break;
            }
            Err(e) => {
                eprintln!("[{}] WebSocket error: {}", source, e);
                break;
            }
            _ => {}
        }
    }

    ping_task.abort();
    Ok(())
}

async fn process_ws_message(
    text: &str,
    source: &str,
    tx: &mpsc::Sender<BybitMessage>,
) -> Result<()> {
    if text.contains("\"op\":\"pong\"") || text.contains("\"success\":true") {
        return Ok(());
    }

    let response: OrderBookResponse = serde_json::from_str(text)?;
    let data = response.data;

    let bids = parse_level_strings(&data.b);
    let asks = parse_level_strings(&data.a);

    let msg = match response.msg_type.as_str() {
        "snapshot" => BybitMessage::Snapshot {
            symbol: data.s,
            source: source.to_string(),
            bids,
            asks,
            update_id: data.u,
            timestamp: response.ts,
        },
        "delta" => BybitMessage::Delta {
            symbol: data.s,
            source: source.to_string(),
            bids,
            asks,
            update_id: data.u,
            timestamp: response.ts,
        },
        _ => return Ok(()),
    };

    tx.send(msg).await.ok();
    Ok(())
}

fn parse_level_strings(levels: &[Vec<String>]) -> Vec<(f64, f64)> {
    levels
        .iter()
        .filter_map(|level| {
            if level.len() >= 2 {
                let price = level[0].parse::<f64>().ok()?;
                let qty = level[1].parse::<f64>().ok()?;
                Some((price, qty))
            } else {
                None
            }
        })
        .collect()
}

fn parse_price(price: f64) -> Option<u128> {
    if price <= 0.0 {
        return None;
    }
    Some((price * PRICE_SCALE).round() as u128)
}

fn parse_quantity(quantity: f64) -> Option<u64> {
    if quantity < 0.0 {
        return None;
    }
    Some((quantity * PRICE_SCALE).round() as u64)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_book(symbol: &str, tick_size: f64) -> OrderBookState {
        OrderBookState::new(symbol, tick_size)
    }

    /// Add a single ask or bid level to the state.
    fn add_ask(state: &mut OrderBookState, price: f64, qty: f64) {
        state.apply_levels(Side::Sell, &[(price, qty)]);
    }

    fn add_bid(state: &mut OrderBookState, price: f64, qty: f64) {
        state.apply_levels(Side::Buy, &[(price, qty)]);
    }

    #[test]
    fn test_gap_probability_balanced() {
        let tick = 0.10;
        let mut state = make_book("BTCUSDT", tick);

        // Best bid/ask
        add_bid(&mut state, 100.00, 1.0);
        add_ask(&mut state, 100.10, 1.0);

        // Symmetrical gaps: 5 empty ticks then same liquidity on both sides
        add_ask(&mut state, 100.60, 500.0); // 5 ticks gap up
        add_bid(&mut state, 99.50, 500.0);  // 5 ticks gap down

        let bid = state.book.best_bid().unwrap();
        let ask = state.book.best_ask().unwrap();
        let (score, gap_up, gap_dn, liq_up, liq_dn) = state.calculate_gap_probability(bid, ask);

        assert_eq!(gap_up, 5, "should detect 5-tick gap up");
        assert_eq!(gap_dn, 5, "should detect 5-tick gap down");
        assert!(liq_up > 0);
        assert!(liq_dn > 0);
        // Balanced → score ≈ 0.5
        assert!((score - 0.5).abs() < 0.01, "balanced book score should be ~0.5, got {}", score);
    }

    #[test]
    fn test_gap_probability_ask_heavy() {
        let tick = 0.10;
        let mut state = make_book("BTCUSDT", tick);

        add_bid(&mut state, 100.00, 1.0);
        add_ask(&mut state, 100.10, 1.0);

        // Much more liquidity on the ask side beyond the gap
        add_ask(&mut state, 100.60, 10_000.0); // 5 ticks gap, heavy resistance
        add_bid(&mut state, 99.50, 100.0);      // 5 ticks gap, light support

        let bid = state.book.best_bid().unwrap();
        let ask = state.book.best_ask().unwrap();
        let (score, _, _, _, _) = state.calculate_gap_probability(bid, ask);

        assert!(score > 0.5, "ask-heavy book should score > 0.5, got {}", score);
    }

    #[test]
    fn test_gap_probability_bid_heavy() {
        let tick = 0.10;
        let mut state = make_book("BTCUSDT", tick);

        add_bid(&mut state, 100.00, 1.0);
        add_ask(&mut state, 100.10, 1.0);

        add_ask(&mut state, 100.60, 100.0);     // light resistance
        add_bid(&mut state, 99.50, 10_000.0);   // heavy support

        let bid = state.book.best_bid().unwrap();
        let ask = state.book.best_ask().unwrap();
        let (score, _, _, _, _) = state.calculate_gap_probability(bid, ask);

        assert!(score < 0.5, "bid-heavy book should score < 0.5, got {}", score);
    }

    #[test]
    fn test_non_default_tick_size() {
        // Parametrize with a different tick size (e.g. ETH: 0.01)
        let tick = 0.01;
        let mut state = make_book("ETHUSDT", tick);

        add_bid(&mut state, 3000.00, 1.0);
        add_ask(&mut state, 3000.01, 1.0);

        // 10 tick gap up, 0 tick gap down (next level immediately)
        add_ask(&mut state, 3000.11, 500.0);
        add_bid(&mut state, 2999.99, 500.0);

        let bid = state.book.best_bid().unwrap();
        let ask = state.book.best_ask().unwrap();
        let (_, gap_up, gap_dn, _, _) = state.calculate_gap_probability(bid, ask);

        assert_eq!(gap_up, 10, "should detect 10-tick gap with 0.01 tick size");
        assert_eq!(gap_dn, 0, "should detect 0-tick gap (immediate liquidity)");
    }

    #[test]
    fn test_no_liquidity_returns_neutral() {
        let tick = 0.10;
        let mut state = make_book("BTCUSDT", tick);

        add_bid(&mut state, 100.00, 1.0);
        add_ask(&mut state, 100.10, 1.0);

        let bid = state.book.best_bid().unwrap();
        let ask = state.book.best_ask().unwrap();
        let (score, _, _, liq_up, liq_dn) = state.calculate_gap_probability(bid, ask);

        assert_eq!(liq_up, 0);
        assert_eq!(liq_dn, 0);
        assert_eq!(score, 0.5, "no gap liquidity should return neutral 0.5");
    }
}
