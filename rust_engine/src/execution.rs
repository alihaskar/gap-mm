use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::private_ws::FillEvent;

type HmacSha256 = Hmac<Sha256>;

const BYBIT_API_BASE: &str = "https://api.bybit.com";
const RECV_WINDOW: u64 = 5000;

// ============================================================================
// Order Types & State Management
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    fn as_str(&self) -> &str {
        match self {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order submitted locally but not confirmed
    Pending,
    /// Order confirmed by exchange (NEW)
    New,
    /// Order partially filled
    PartiallyFilled,
    /// Order fully filled
    Filled,
    /// Order cancelled
    Cancelled,
    /// Order rejected by exchange
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Exchange order ID (None if not yet confirmed)
    pub order_id: Option<String>,
    /// Client order ID (our internal tracking)
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub filled_qty: f64,
    pub status: OrderStatus,
    /// Timestamp when order was created locally
    pub created_at: u64,
    /// Timestamp of last update (from REST or WS)
    pub updated_at: u64,
}

/// Dual-state architecture
#[derive(Debug)]
pub struct OrderState {
    /// Internal optimistic state (what we just did)
    internal: RwLock<HashMap<String, Order>>,
    /// Exchange confirmed state (REST + WS truth)
    exchange: RwLock<HashMap<String, Order>>,
}

impl OrderState {
    pub fn new() -> Self {
        Self {
            internal: RwLock::new(HashMap::new()),
            exchange: RwLock::new(HashMap::new()),
        }
    }

    /// Add order to internal state (optimistic)
    pub async fn add_internal(&self, order: Order) {
        let mut internal = self.internal.write().await;
        internal.insert(order.client_order_id.clone(), order);
    }

    /// Update exchange state (confirmed by REST or WS)
    pub async fn update_exchange(&self, order: Order) {
        let mut exchange = self.exchange.write().await;
        let mut internal = self.internal.write().await;
        
        // Update exchange state
        exchange.insert(order.client_order_id.clone(), order.clone());
        
        // Sync internal state
        internal.insert(order.client_order_id.clone(), order);
    }

    /// Get active orders (from exchange state)
    pub async fn get_active_orders(&self, symbol: &str, side: Option<OrderSide>) -> Vec<Order> {
        let exchange = self.exchange.read().await;
        exchange
            .values()
            .filter(|o| {
                o.symbol == symbol
                    && matches!(o.status, OrderStatus::New | OrderStatus::PartiallyFilled)
                    && side.map_or(true, |s| o.side == s)
            })
            .cloned()
            .collect()
    }

    /// Check if order exists in internal state (prevent double submission)
    pub async fn has_pending_order(&self, symbol: &str, side: OrderSide) -> bool {
        let internal = self.internal.read().await;
        internal.values().any(|o| {
            o.symbol == symbol
                && o.side == side
                && matches!(o.status, OrderStatus::Pending | OrderStatus::New)
        })
    }

    /// Remove order from both states (for cleanup after errors)
    pub async fn remove_order(&self, side: OrderSide) {
        let mut internal = self.internal.write().await;
        let mut exchange = self.exchange.write().await;
        
        // Remove all orders for this side
        internal.retain(|_, o| o.side != side);
        exchange.retain(|_, o| o.side != side);
    }
}

// ============================================================================
// Position Tracking
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub net_qty: f64,  // Positive = long, negative = short, 0 = flat
    pub avg_entry_price: f64,
    pub total_buy_qty: f64,
    pub total_sell_qty: f64,
    pub total_buy_value: f64,
    pub total_sell_value: f64,
    pub realized_pnl: f64,
    pub last_update: u64,
}

#[derive(Debug)]
pub struct PositionState {
    positions: RwLock<HashMap<String, Position>>,
}

impl PositionState {
    pub fn new() -> Self {
        Self {
            positions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_position(&self, symbol: &str) -> Option<Position> {
        let positions = self.positions.read().await;
        positions.get(symbol).cloned()
    }

    pub async fn update_position(&self, position: Position) {
        let mut positions = self.positions.write().await;
        positions.insert(position.symbol.clone(), position);
    }

    /// Process a fill event and update position
    pub async fn process_fill(&self, fill: &FillEvent) {
        let mut positions = self.positions.write().await;
        
        let position = positions.entry(fill.symbol.clone()).or_insert_with(|| Position {
            symbol: fill.symbol.clone(),
            net_qty: 0.0,
            avg_entry_price: 0.0,
            total_buy_qty: 0.0,
            total_sell_qty: 0.0,
            total_buy_value: 0.0,
            total_sell_value: 0.0,
            realized_pnl: 0.0,
            last_update: fill.timestamp,
        });

        let fill_value = fill.fill_price * fill.fill_qty;

        if fill.side == "Buy" {
            // Buying
            if position.net_qty < 0.0 {
                // Closing short position - realize P&L
                let closing_qty = fill.fill_qty.min(position.net_qty.abs());
                let realized_pnl = (position.avg_entry_price - fill.fill_price) * closing_qty;
                position.realized_pnl += realized_pnl;
                
                if fill.fill_qty > closing_qty {
                    // Flip to long
                    let remaining_qty = fill.fill_qty - closing_qty;
                    position.net_qty = remaining_qty;
                    position.avg_entry_price = fill.fill_price;
                } else {
                    position.net_qty += fill.fill_qty;
                }
            } else {
                // Adding to long or opening long
                let total_value = position.avg_entry_price * position.net_qty + fill_value;
                position.net_qty += fill.fill_qty;
                position.avg_entry_price = total_value / position.net_qty;
            }
            
            position.total_buy_qty += fill.fill_qty;
            position.total_buy_value += fill_value;
            
        } else {
            // Selling
            if position.net_qty > 0.0 {
                // Closing long position - realize P&L
                let closing_qty = fill.fill_qty.min(position.net_qty);
                let realized_pnl = (fill.fill_price - position.avg_entry_price) * closing_qty;
                position.realized_pnl += realized_pnl;
                
                if fill.fill_qty > closing_qty {
                    // Flip to short
                    let remaining_qty = fill.fill_qty - closing_qty;
                    position.net_qty = -remaining_qty;
                    position.avg_entry_price = fill.fill_price;
                } else {
                    position.net_qty -= fill.fill_qty;
                }
            } else {
                // Adding to short or opening short
                let total_value = position.avg_entry_price * position.net_qty.abs() + fill_value;
                position.net_qty -= fill.fill_qty;
                position.avg_entry_price = total_value / position.net_qty.abs();
            }
            
            position.total_sell_qty += fill.fill_qty;
            position.total_sell_value += fill_value;
        }

        position.last_update = fill.timestamp;
    }
}

// ============================================================================
// Bybit API Authentication
// ============================================================================

pub struct BybitAuth {
    api_key: String,
    api_secret: String,
    client: reqwest::Client,
    market_type: String,  // "spot" or "linear"
}

impl BybitAuth {
    pub fn new(api_key: String, api_secret: String, market_type: String) -> Self {
        let client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap();

        Self {
            api_key,
            api_secret,
            client,
            market_type,
        }
    }

    /// Generate HMAC-SHA256 signature for Bybit V5 API
    fn sign(&self, timestamp: u64, params: &str) -> String {
        let sign_str = format!("{}{}{}", timestamp, &self.api_key, RECV_WINDOW);
        let sign_str = if params.is_empty() {
            sign_str
        } else {
            format!("{}{}", sign_str, params)
        };

        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sign_str.as_bytes());
        
        hex::encode(mac.finalize().into_bytes())
    }

    /// Submit new order
    pub async fn submit_order(
        &self,
        symbol: &str,
        side: OrderSide,
        price: f64,
        quantity: f64,
        client_order_id: &str,
    ) -> Result<SubmitOrderResponse> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        // Format price to 1 decimal place for BTCUSDT (tick_size=0.10)
        let price_str = format!("{:.1}", price);
        let qty_str = format!("{:.5}", quantity);  // 5 decimals for BTC qty (supports down to 0.00001 BTC)

        let body = serde_json::json!({
            "category": &self.market_type,  // "spot" or "linear"
            "symbol": symbol,
            "side": side.as_str(),
            "orderType": "Limit",
            "qty": qty_str,
            "price": price_str,
            "timeInForce": "PostOnly",
            "orderLinkId": client_order_id,
        });

        let body_str = serde_json::to_string(&body)?;
        let signature = self.sign(timestamp, &body_str);

        let response = self
            .client
            .post(format!("{}/v5/order/create", BYBIT_API_BASE))
            .header("X-BAPI-API-KEY", &self.api_key)
            .header("X-BAPI-TIMESTAMP", timestamp.to_string())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-RECV-WINDOW", RECV_WINDOW.to_string())
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let result: BybitResponse<SubmitOrderResponse> = serde_json::from_str(&text)
            .map_err(|e| anyhow!("Failed to parse submit response: {} | Response: {}", e, text))?;
        
        if result.ret_code != 0 {
            return Err(anyhow!("Bybit error {}: {} | Response: {}", result.ret_code, result.ret_msg, text));
        }

        result.result.ok_or_else(|| anyhow!("No result in response: {}", text))
    }

    /// Amend existing order
    pub async fn amend_order(
        &self,
        symbol: &str,
        order_id: &str,
        new_price: f64,
        new_quantity: Option<f64>,
    ) -> Result<AmendOrderResponse> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        // Format price to 1 decimal place for BTCUSDT (tick_size=0.10)
        let price_str = format!("{:.1}", new_price);

        let mut body = serde_json::json!({
            "category": &self.market_type,
            "symbol": symbol,
            "orderId": order_id,
            "price": price_str,
        });

        if let Some(qty) = new_quantity {
            let qty_str = format!("{:.5}", qty);  // 5 decimals for BTC qty
            body["qty"] = serde_json::json!(qty_str);
        }

        let body_str = serde_json::to_string(&body)?;
        let signature = self.sign(timestamp, &body_str);

        let response = self
            .client
            .post(format!("{}/v5/order/amend", BYBIT_API_BASE))
            .header("X-BAPI-API-KEY", &self.api_key)
            .header("X-BAPI-TIMESTAMP", timestamp.to_string())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-RECV-WINDOW", RECV_WINDOW.to_string())
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let result: BybitResponse<AmendOrderResponse> = serde_json::from_str(&text)
            .map_err(|e| anyhow!("Failed to parse amend response: {} | Response: {}", e, text))?;
        
        if result.ret_code != 0 {
            return Err(anyhow!("Bybit error {}: {} | Response: {}", result.ret_code, result.ret_msg, text));
        }

        result.result.ok_or_else(|| anyhow!("No result in response: {}", text))
    }

    /// Cancel order
    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<CancelOrderResponse> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        let body = serde_json::json!({
            "category": &self.market_type,
            "symbol": symbol,
            "orderId": order_id,
        });

        let body_str = serde_json::to_string(&body)?;
        let signature = self.sign(timestamp, &body_str);

        let response = self
            .client
            .post(format!("{}/v5/order/cancel", BYBIT_API_BASE))
            .header("X-BAPI-API-KEY", &self.api_key)
            .header("X-BAPI-TIMESTAMP", timestamp.to_string())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-RECV-WINDOW", RECV_WINDOW.to_string())
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let result: BybitResponse<CancelOrderResponse> = serde_json::from_str(&text)?;
        
        if result.ret_code != 0 {
            return Err(anyhow!("Bybit error {}: {}", result.ret_code, result.ret_msg));
        }

        result.result.ok_or_else(|| anyhow!("No result in response"))
    }

    /// Get open orders
    pub async fn get_open_orders(&self, symbol: &str) -> Result<Vec<OrderInfo>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        let params = format!("category={}&symbol={}", self.market_type, symbol);
        let signature = self.sign(timestamp, "");

        let response = self
            .client
            .get(format!("{}/v5/order/realtime?{}", BYBIT_API_BASE, params))
            .header("X-BAPI-API-KEY", &self.api_key)
            .header("X-BAPI-TIMESTAMP", timestamp.to_string())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-RECV-WINDOW", RECV_WINDOW.to_string())
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let result: BybitResponse<OpenOrdersResponse> = serde_json::from_str(&text)?;
        
        if result.ret_code != 0 {
            return Err(anyhow!("Bybit error {}: {}", result.ret_code, result.ret_msg));
        }

        Ok(result.result.map(|r| r.list).unwrap_or_default())
    }
}

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct BybitResponse<T> {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitOrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AmendOrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
}

#[derive(Debug, Deserialize)]
struct OpenOrdersResponse {
    list: Vec<OrderInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrderInfo {
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
    pub symbol: String,
    pub side: String,
    pub price: String,
    pub qty: String,
    #[serde(rename = "cumExecQty")]
    pub cum_exec_qty: String,
    #[serde(rename = "orderStatus")]
    pub order_status: String,
    #[serde(rename = "createdTime")]
    pub created_time: String,
    #[serde(rename = "updatedTime")]
    pub updated_time: String,
}

// ============================================================================
// Execution Engine (OMS/EMS)
// ============================================================================

pub struct ExecutionEngine {
    pub auth: Arc<BybitAuth>,
    pub order_state: Arc<OrderState>,
    pub position_state: Arc<PositionState>,
    symbol: String,
    tick_size: f64,
    max_position: f64,
    min_order_size: f64,  // Minimum order size for the symbol
}

impl ExecutionEngine {
    pub fn new(
        api_key: String,
        api_secret: String,
        symbol: String,
        market_type: String,
        tick_size: f64,
        max_position: f64,
        min_order_size: f64,
    ) -> Self {
        Self {
            auth: Arc::new(BybitAuth::new(api_key, api_secret, market_type)),
            order_state: Arc::new(OrderState::new()),
            position_state: Arc::new(PositionState::new()),
            symbol,
            tick_size,
            max_position,
            min_order_size,
        }
    }

    /// Core reconciliation logic: compare target prices with existing orders
    pub async fn reconcile_orders(
        &self,
        target_bid: Option<f64>,
        target_ask: Option<f64>,
    ) -> Result<ReconcileResult> {
        let mut result = ReconcileResult::default();

        // Process bid side - don't fail entire reconcile if one side has insufficient balance
        if let Some(bid_price) = target_bid {
            match self.reconcile_side(OrderSide::Buy, bid_price).await {
                Ok(action) => result.bid_action = Some(action),
                Err(e) => {
                    // Log but continue - other side might work (e.g., insufficient USDT but have BTC)
                    eprintln!("BID reconcile failed: {}", e);
                }
            }
        }

        // Process ask side - independent of bid side
        if let Some(ask_price) = target_ask {
            match self.reconcile_side(OrderSide::Sell, ask_price).await {
                Ok(action) => result.ask_action = Some(action),
                Err(e) => {
                    // Log but continue - other side might work (e.g., insufficient BTC but have USDT)
                    eprintln!("ASK reconcile failed: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Reconcile a single side (bid or ask)
    async fn reconcile_side(&self, side: OrderSide, target_price: f64) -> Result<OrderAction> {
        // Round to tick size
        let target_price = self.round_to_tick(target_price);

        // Check position limits - be smart about which side to block
        let position = self.position_state.get_position(&self.symbol).await;
        let current_position = position.as_ref().map(|p| p.net_qty).unwrap_or(0.0);

        // If at max LONG position, only allow SELL (to reduce position)
        if current_position >= self.max_position && side == OrderSide::Buy {
            return Ok(OrderAction::Skipped {
                reason: "Max LONG position - only SELLs allowed".to_string(),
            });
        }

        // If at max SHORT position, only allow BUY (to reduce position)
        if current_position <= -self.max_position && side == OrderSide::Sell {
            return Ok(OrderAction::Skipped {
                reason: "Max SHORT position - only BUYs allowed".to_string(),
            });
        }

        // Get active orders for this side
        let active_orders = self.order_state.get_active_orders(&self.symbol, Some(side)).await;

        match active_orders.first() {
            None => {
                // No order exists -> Submit new
                if self.order_state.has_pending_order(&self.symbol, side).await {
                    return Ok(OrderAction::Skipped {
                        reason: "Order already pending".to_string(),
                    });
                }

                let client_order_id = format!("{}_{}_{}",
                    self.symbol,
                    side.as_str(),
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
                );

                let order = Order {
                    order_id: None,
                    client_order_id: client_order_id.clone(),
                    symbol: self.symbol.clone(),
                    side,
                    price: target_price,
                    quantity: self.min_order_size,
                    filled_qty: 0.0,
                    status: OrderStatus::Pending,
                    created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                    updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                };

                // Submit to exchange FIRST - track latency
                let start = std::time::Instant::now();
                let response = match self.auth.submit_order(
                    &self.symbol,
                    side,
                    target_price,
                    self.min_order_size,
                    &client_order_id,
                ).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        // Submission failed - don't pollute internal state
                        return Err(e);
                    }
                };
                let latency_ms = start.elapsed().as_millis() as u64;

                // Success - add to both states
                let mut confirmed_order = order;
                confirmed_order.order_id = Some(response.order_id.clone());
                confirmed_order.status = OrderStatus::New;
                confirmed_order.updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                
                self.order_state.add_internal(confirmed_order.clone()).await;
                self.order_state.update_exchange(confirmed_order).await;

                Ok(OrderAction::Submitted {
                    order_id: response.order_id,
                    price: target_price,
                    latency_ms,
                })
            }
            Some(existing_order) => {
                // Order exists -> Check if price matches
                if (existing_order.price - target_price).abs() < self.tick_size / 2.0 {
                    // Price matches -> Do nothing
                    Ok(OrderAction::NoChange {
                        order_id: existing_order.order_id.clone().unwrap_or_default(),
                        price: existing_order.price,
                    })
                } else {
                    // Price changed -> Amend order with latency tracking
                    let order_id = existing_order.order_id.as_ref()
                        .ok_or_else(|| anyhow!("Order has no exchange ID"))?;

                    let start = std::time::Instant::now();
                    let response = self.auth.amend_order(
                        &self.symbol,
                        order_id,
                        target_price,
                        None,
                    ).await;
                    let latency_ms = start.elapsed().as_millis() as u64;

                    // Handle amend response
                    match response {
                        Ok(resp) => {
                            // Update exchange state
                            let mut updated_order = existing_order.clone();
                            updated_order.price = target_price;
                            updated_order.updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                            self.order_state.update_exchange(updated_order).await;

                            Ok(OrderAction::Amended {
                                order_id: resp.order_id,
                                old_price: existing_order.price,
                                new_price: target_price,
                                latency_ms,
                            })
                        },
                        Err(e) => {
                            // If order doesn't exist on exchange, remove from internal and retry
                            let err_msg = e.to_string();
                            if err_msg.contains("170213") || err_msg.contains("Order does not exist") {
                                eprintln!("Order {} doesn't exist on exchange, clearing and resubmitting", order_id);
                                self.order_state.remove_order(side).await;
                                // Return error to trigger resubmit on next reconcile
                                Err(e)
                            } else {
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Round price to tick size
    fn round_to_tick(&self, price: f64) -> f64 {
        (price / self.tick_size).round() * self.tick_size
    }

    /// Sync orders from exchange (for recovery/startup)
    pub async fn sync_orders(&self) -> Result<()> {
        let orders = self.auth.get_open_orders(&self.symbol).await?;

        for order_info in orders {
            let side = match order_info.side.as_str() {
                "Buy" => OrderSide::Buy,
                "Sell" => OrderSide::Sell,
                _ => continue,
            };

            let status = match order_info.order_status.as_str() {
                "New" => OrderStatus::New,
                "PartiallyFilled" => OrderStatus::PartiallyFilled,
                "Filled" => OrderStatus::Filled,
                "Cancelled" => OrderStatus::Cancelled,
                "Rejected" => OrderStatus::Rejected,
                _ => OrderStatus::New,
            };

            let order = Order {
                order_id: Some(order_info.order_id),
                client_order_id: order_info.order_link_id,
                symbol: order_info.symbol,
                side,
                price: order_info.price.parse().unwrap_or(0.0),
                quantity: order_info.qty.parse().unwrap_or(0.0),
                filled_qty: order_info.cum_exec_qty.parse().unwrap_or(0.0),
                status,
                created_at: order_info.created_time.parse().unwrap_or(0),
                updated_at: order_info.updated_time.parse().unwrap_or(0),
            };

            self.order_state.update_exchange(order).await;
        }

        Ok(())
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Default)]
pub struct ReconcileResult {
    pub bid_action: Option<OrderAction>,
    pub ask_action: Option<OrderAction>,
}

#[derive(Debug, Clone)]
pub enum OrderAction {
    Submitted { order_id: String, price: f64, latency_ms: u64 },
    Amended { order_id: String, old_price: f64, new_price: f64, latency_ms: u64 },
    NoChange { order_id: String, price: f64 },
    Skipped { reason: String },
}
