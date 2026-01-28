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
}

impl OrderBookState {
    pub fn new(symbol: &str) -> Self {
        Self {
            book: OrderBook::<()>::new(symbol),
            levels: HashMap::new(),
            prev_best_bid: None,
            prev_best_ask: None,
        }
    }

    pub fn reset(&mut self, symbol: &str) {
        *self = Self::new(symbol);
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

            // Remove if qty is zero
            if quantity == 0 {
                if let Some(order_id) = self.levels.remove(&key) {
                    let _ = self.book.cancel_order(order_id);
                }
                continue;
            }

            // Cancel existing order at this price level
            let order_id = self
                .levels
                .get(&key)
                .cloned()
                .unwrap_or_else(OrderId::new_uuid);

            let _ = self.book.cancel_order(order_id);

            // Add new order
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

    pub fn get_enriched_update(&self, symbol: &str, source: &str, timestamp: u64) -> Option<MarketUpdate> {
        let best_bid = self.book.best_bid()?;
        let best_ask = self.book.best_ask()?;

        // Use OrderBook-rs built-in metrics
        let mid_price = self.book.mid_price();
        let spread_bps = self.book.spread_bps(None);
        let imbalance = self.book.order_book_imbalance(5);
        
        // Get depth at top 5 levels
        let bid_depth_5 = self.book.total_depth_at_levels(5, Side::Buy);
        let ask_depth_5 = self.book.total_depth_at_levels(5, Side::Sell);

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
        })
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

pub async fn start_bybit_streams(
    symbol: &str,
) -> Result<mpsc::Receiver<BybitMessage>> {
    let (tx, rx) = mpsc::channel(1000);

    // Spawn spot connection with auto-reconnect
    let spot_tx = tx.clone();
    let symbol_spot = symbol.to_string();
    tokio::spawn(async move {
        loop {
            match connect_ws_with_reconnect(BYBIT_SPOT_WS, &symbol_spot, "bybit_spot", spot_tx.clone()).await {
                Ok(_) => {
                    eprintln!("[bybit_spot] Connection ended normally");
                }
                Err(e) => {
                    eprintln!("[bybit_spot] Connection error: {}", e);
                }
            }
            eprintln!("[bybit_spot] Reconnecting in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // Spawn linear perp connection with auto-reconnect
    let linear_tx = tx.clone();
    let symbol_linear = symbol.to_string();
    tokio::spawn(async move {
        loop {
            match connect_ws_with_reconnect(BYBIT_LINEAR_WS, &symbol_linear, "bybit_linear_perp", linear_tx.clone()).await {
                Ok(_) => {
                    eprintln!("[bybit_linear_perp] Connection ended normally");
                }
                Err(e) => {
                    eprintln!("[bybit_linear_perp] Connection error: {}", e);
                }
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
    mut callback: impl FnMut(MarketUpdate) + Send + 'static,
) {
    let mut books: HashMap<String, OrderBookState> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        let (symbol, source, bids, asks, is_snapshot, timestamp) = match msg {
            BybitMessage::Snapshot {
                symbol,
                source,
                bids,
                asks,
                timestamp,
                ..
            } => (symbol, source, bids, asks, true, timestamp),
            BybitMessage::Delta {
                symbol,
                source,
                bids,
                asks,
                timestamp,
                ..
            } => (symbol, source, bids, asks, false, timestamp),
        };

        let key = format!("{}:{}", source, symbol);
        let state = books
            .entry(key.clone())
            .or_insert_with(|| OrderBookState::new(&symbol));

        if is_snapshot {
            state.reset(&symbol);
        }

        // Apply bids and asks
        state.apply_levels(Side::Buy, &bids);
        state.apply_levels(Side::Sell, &asks);

        // Check if best changed
        let changed = state.update_and_check_change();

        if changed {
            if let Some(update) = state.get_enriched_update(&symbol, &source, timestamp) {
                callback(update);
            }
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

    // Subscribe to orderbook
    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": [format!("orderbook.200.{}", symbol)]
    });

    {
        let mut w = write.lock().await;
        w.send(Message::Text(subscribe_msg.to_string().into())).await?;
    }
    println!("[{}] Subscribed to orderbook.200.{}", source, symbol);

    // Spawn ping task to keep connection alive (every 20 seconds)
    let write_ping = write.clone();
    let source_ping = source.to_string();
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(20));
        loop {
            interval.tick().await;
            let ping_msg = serde_json::json!({"op": "ping"});
            let mut w = write_ping.lock().await;
            if w.send(Message::Text(ping_msg.to_string().into())).await.is_err() {
                eprintln!("[{}] Failed to send ping, connection likely closed", source_ping);
                break;
            }
        }
    });

    // Handle messages
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
            Ok(Message::Pong(_)) => {
                // Received pong response
            }
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

    // Clean up ping task
    ping_task.abort();

    Ok(())
}

async fn process_ws_message(
    text: &str,
    source: &str,
    tx: &mpsc::Sender<BybitMessage>,
) -> Result<()> {
    // Skip ping/pong and success messages
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
