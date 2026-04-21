use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

type HmacSha256 = Hmac<Sha256>;

const BYBIT_PRIVATE_WS: &str = "wss://stream.bybit.com/v5/private";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub category: String,
    pub symbol: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
    pub side: String,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "orderPrice")]
    pub order_price: String,
    #[serde(rename = "orderQty")]
    pub order_qty: String,
    #[serde(rename = "execPrice")]
    pub exec_price: String,
    #[serde(rename = "execQty")]
    pub exec_qty: String,
    #[serde(rename = "leavesQty")]
    pub leaves_qty: String,
    #[serde(rename = "execTime")]
    pub exec_time: String,
    #[serde(rename = "isMaker")]
    pub is_maker: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FillEvent {
    pub symbol: String,
    pub order_id: String,
    pub client_order_id: String,
    pub side: String, // "Buy" or "Sell"
    pub fill_price: f64,
    pub fill_qty: f64,
    pub cum_qty: f64,
    pub avg_price: f64,
    pub order_status: String,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PrivateMessage {
    topic: String,
    #[serde(rename = "creationTime")]
    creation_time: Option<u64>,
    data: Vec<ExecutionReport>,
}

#[derive(Debug, Serialize)]
struct AuthRequest {
    req_id: String,
    op: String,
    args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SubscribeRequest {
    req_id: String,
    op: String,
    args: Vec<String>,
}

pub async fn start_private_stream(
    api_key: &str,
    api_secret: &str,
) -> Result<mpsc::UnboundedReceiver<FillEvent>> {
    let (tx, rx) = mpsc::unbounded_channel();

    let api_key = api_key.to_string();
    let api_secret = api_secret.to_string();

    tokio::spawn(async move {
        if let Err(e) = run_private_stream(&api_key, &api_secret, tx).await {
            eprintln!("Private stream error: {}", e);
        }
    });

    Ok(rx)
}

async fn run_private_stream(
    api_key: &str,
    api_secret: &str,
    tx: mpsc::UnboundedSender<FillEvent>,
) -> Result<()> {
    loop {
        match connect_and_stream(api_key, api_secret, &tx).await {
            Ok(_) => {
                eprintln!("Private stream ended normally");
            }
            Err(e) => {
                eprintln!("Private stream error: {}, reconnecting in 5s...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn connect_and_stream(
    api_key: &str,
    api_secret: &str,
    tx: &mpsc::UnboundedSender<FillEvent>,
) -> Result<()> {
    eprintln!("Connecting to Bybit private WebSocket...");

    let (ws_stream, _) = connect_async(BYBIT_PRIVATE_WS).await?;
    let (mut write, mut read) = ws_stream.split();

    eprintln!("✓ Connected to private WebSocket");

    // Authenticate
    let expires = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64 + 10000;

    let sign_str = format!("GET/realtime{}", expires);
    let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())?;
    mac.update(sign_str.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let auth_msg = AuthRequest {
        req_id: "auth".to_string(),
        op: "auth".to_string(),
        args: vec![api_key.to_string(), expires.to_string(), signature],
    };

    let auth_json = serde_json::to_string(&auth_msg)?;
    write.send(Message::Text(auth_json.into())).await?;

    eprintln!("Sent authentication request...");

    // Wait for auth response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        eprintln!("Auth response: {}", text);
        if text.contains("\"success\":true") {
            eprintln!("✓ Authentication successful");
        } else {
            return Err(anyhow::anyhow!("Authentication failed: {}", text));
        }
    }

    // Subscribe to execution reports
    let subscribe_msg = SubscribeRequest {
        req_id: "sub_execution".to_string(),
        op: "subscribe".to_string(),
        args: vec!["execution".to_string()],
    };

    let sub_json = serde_json::to_string(&subscribe_msg)?;
    write.send(Message::Text(sub_json.into())).await?;

    eprintln!("Subscribed to execution reports");
    eprintln!("Listening for fills...\n");

    // Process messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                eprintln!("📨 Private WS Message: {}", text);

                if text.contains("\"op\":\"subscribe\"") {
                    eprintln!("   → Subscription confirmation");
                    continue; // Skip subscription confirmation
                }

                if text.contains("\"topic\":\"execution\"") {
                    eprintln!("   → Execution report detected!");
                    if let Ok(private_msg) = serde_json::from_str::<PrivateMessage>(&text) {
                        eprintln!("   → Parsed {} execution reports", private_msg.data.len());
                        for report in private_msg.data {
                            if let Some(fill_event) = process_execution_report(report) {
                                if tx.send(fill_event).is_err() {
                                    eprintln!("Failed to send fill event (channel closed)");
                                    return Ok(());
                                }
                            }
                        }
                    } else {
                        eprintln!("   → Failed to parse execution message");
                    }
                } else {
                    eprintln!("   → Not an execution report");
                }
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await?;
            }
            Ok(Message::Close(_)) => {
                eprintln!("WebSocket closed by server");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_execution_report(report: ExecutionReport) -> Option<FillEvent> {
    // Parse numeric fields
    let fill_qty: f64 = report.exec_qty.parse().unwrap_or(0.0);
    let fill_price: f64 = report.exec_price.parse().unwrap_or(0.0);
    let order_qty: f64 = report.order_qty.parse().unwrap_or(0.0);
    let leaves_qty: f64 = report.leaves_qty.parse().unwrap_or(0.0);
    let cum_qty = order_qty - leaves_qty; // Calculate cumulative filled
    let timestamp: u64 = report.exec_time.parse().ok()?;

    // Determine order status based on leaves_qty
    let order_status = if leaves_qty == 0.0 && cum_qty > 0.0 {
        "Filled"
    } else if leaves_qty > 0.0 && cum_qty > 0.0 {
        "PartiallyFilled"
    } else {
        "New"
    };

    // Log ALL execution reports (for debugging)
    eprintln!(
        "📡 Execution Report: {} | {} | {} | Status: {} | Fill: {:.6} | Cum: {:.6} | Maker: {}",
        report.symbol,
        &report.order_id[..report.order_id.len().min(10)],
        report.side,
        order_status,
        fill_qty,
        cum_qty,
        report.is_maker
    );

    // Only process if there was an actual fill
    if fill_qty > 0.0 {
        Some(FillEvent {
            symbol: report.symbol,
            order_id: report.order_id,
            client_order_id: report.order_link_id,
            side: report.side,
            fill_price,
            fill_qty,
            cum_qty,
            avg_price: fill_price, // For single fills, avg = fill price
            order_status: order_status.to_string(),
            timestamp,
        })
    } else {
        // Status update but no fill
        eprintln!("   → Status update only (no fill)");
        None
    }
}
