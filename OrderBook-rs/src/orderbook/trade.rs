/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 2/10/25
******************************************************************************/
use pricelevel::MatchResult;
use std::sync::Arc;

/// Enhanced trade result that includes symbol information
#[derive(Debug, Clone)]
pub struct TradeResult {
    /// The symbol this trade result belongs to
    pub symbol: String,
    /// The underlying match result from the pricelevel crate
    pub match_result: MatchResult,
}

impl TradeResult {
    /// Create a new TradeResult
    pub fn new(symbol: String, match_result: MatchResult) -> Self {
        Self {
            symbol,
            match_result,
        }
    }
}

/// Trade listener specification using Arc for shared ownership
pub type TradeListener = Arc<dyn Fn(&TradeResult) + Send + Sync>;

/// A trade event that includes additional metadata for processing
#[derive(Debug, Clone)]
pub struct TradeEvent {
    /// The trading symbol for this event
    pub symbol: String,
    /// The trade result containing match details
    pub trade_result: TradeResult,
    /// Unix timestamp in milliseconds when the trade occurred
    pub timestamp: u64,
}

/// Structure to store trade information for later display
#[derive(Debug, Clone)]
pub struct TradeInfo {
    /// The trading symbol
    pub symbol: String,
    /// The order identifier as a string
    pub order_id: String,
    /// Total quantity executed in this trade
    pub executed_quantity: u64,
    /// Remaining quantity not yet filled
    pub remaining_quantity: u64,
    /// Whether the order was completely filled
    pub is_complete: bool,
    /// Number of individual transactions that occurred
    pub transaction_count: usize,
    /// Detailed information about each transaction
    pub transactions: Vec<TransactionInfo>,
}

/// Information about a single transaction within a trade
#[derive(Debug, Clone)]
pub struct TransactionInfo {
    /// The price at which the transaction occurred
    pub price: u128,
    /// The quantity traded in this transaction
    pub quantity: u64,
    /// Unique identifier for this transaction
    pub transaction_id: String,
    /// Order ID of the maker (passive) side
    pub maker_order_id: String,
    /// Order ID of the taker (aggressive) side
    pub taker_order_id: String,
}
