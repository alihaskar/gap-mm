mod bybit;

use anyhow::Result;
use bybit::{process_orderbook_updates, start_bybit_streams, MarketUpdate};
use rustls::crypto::ring;

#[tokio::main]
async fn main() -> Result<()> {
    // Install ring crypto provider for rustls
    let _ = ring::default_provider().install_default();

    let rx = start_bybit_streams("BTCUSDT").await?;

    process_orderbook_updates(rx, 0.1, |update: MarketUpdate| {
        println!(
            "[{}] {} | Best Bid: {:.2} | Best Ask: {:.2} | Spread: {:.2}",
            update.source, update.symbol, update.bid, update.ask, update.spread
        );
    })
    .await;

    Ok(())
}
