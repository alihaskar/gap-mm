mod bybit;

use anyhow::Result;
use bybit::{start_bybit_streams, process_orderbook_updates};
use rustls::crypto::ring;


#[tokio::main]
async fn main() -> Result<()> {
    // Install ring crypto provider for rustls
    let _ = ring::default_provider().install_default();

    let rx = start_bybit_streams("BTCUSDT").await?;

    process_orderbook_updates(rx, |update| {
        println!(
            "[{}] {} | Best Bid: {:.2} | Best Ask: {:.2} | Spread: {:.2}",
            update.source, update.symbol, update.bid, update.ask, update.spread
        );
    })
    .await;

    Ok(())
}
