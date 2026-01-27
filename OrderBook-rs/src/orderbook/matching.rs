//! Contains the core matching engine logic for the order book.

use crate::orderbook::book_change_event::PriceLevelChangedEvent;
use crate::orderbook::pool::MatchingPool;
use crate::{OrderBook, OrderBookError};
use pricelevel::{MatchResult, OrderId, Side};
use std::sync::atomic::Ordering;

impl<T> OrderBook<T>
where
    T: Clone + Send + Sync + Default + 'static,
{
    /// Highly optimized internal matching function
    ///
    /// # Performance Optimization
    /// Uses SkipMap which maintains prices in sorted order automatically.
    /// This eliminates O(N log N) sorting overhead, reducing time complexity
    /// from O(N log N) to O(M log N), where:
    /// - N = total number of price levels
    /// - M = number of price levels actually matched (typically << N)
    ///
    /// In the happy case (single price level fill), complexity is O(log N).
    pub fn match_order(
        &self,
        order_id: OrderId,
        side: Side,
        quantity: u64,
        limit_price: Option<u128>,
    ) -> Result<MatchResult, OrderBookError> {
        self.cache.invalidate();
        let mut match_result = MatchResult::new(order_id, quantity);
        let mut remaining_quantity = quantity;

        // Choose the appropriate side for matching
        let match_side = match side {
            Side::Buy => &self.asks,
            Side::Sell => &self.bids,
        };

        // Early exit if the opposite side is empty
        if match_side.is_empty() {
            if limit_price.is_none() {
                return Err(OrderBookError::InsufficientLiquidity {
                    side,
                    requested: quantity,
                    available: 0,
                });
            }
            match_result.remaining_quantity = remaining_quantity;
            return Ok(match_result);
        }

        // Use static memory pool for better performance
        thread_local! {
            static MATCHING_POOL: MatchingPool = MatchingPool::new();
        }

        // Get reusable vectors from pool
        let (mut filled_orders, mut empty_price_levels) = MATCHING_POOL.with(|pool| {
            let filled = pool.get_filled_orders_vec();
            let empty = pool.get_price_vec();
            (filled, empty)
        });

        // Iterate through prices in optimal order (already sorted by SkipMap)
        // For buy orders: iterate asks in ascending order (best ask first)
        // For sell orders: iterate bids in descending order (best bid first)
        let price_iter: Box<dyn Iterator<Item = _>> = match side {
            Side::Buy => Box::new(match_side.iter()),
            Side::Sell => Box::new(match_side.iter().rev()),
        };

        // Process each price level
        for entry in price_iter {
            let price = *entry.key();
            // Check price limit constraint early
            if let Some(limit) = limit_price {
                match side {
                    Side::Buy if price > limit => break,
                    Side::Sell if price < limit => break,
                    _ => {}
                }
            }

            // Get price level value from the entry
            let price_level = entry.value();

            // Perform the match at this price level
            let price_level_match = price_level.match_order(
                remaining_quantity,
                order_id,
                &self.transaction_id_generator,
            );

            // Process transactions if any occurred
            if !price_level_match.transactions.as_vec().is_empty() {
                // Update last trade price atomically
                self.last_trade_price.store(price);
                self.has_traded.store(true, Ordering::Relaxed);

                // Add transactions to result
                for transaction in price_level_match.transactions.as_vec() {
                    match_result.add_transaction(*transaction);
                }

                // notify price level changes
                if let Some(ref listener) = self.price_level_changed_listener {
                    listener(PriceLevelChangedEvent {
                        side: side.opposite(),
                        price: price_level.price(),
                        quantity: price_level.visible_quantity(),
                    });
                }
            }

            // Collect filled orders for batch removal
            for &filled_order_id in &price_level_match.filled_order_ids {
                match_result.add_filled_order_id(filled_order_id);
                filled_orders.push(filled_order_id);
            }

            // Update remaining quantity
            remaining_quantity = price_level_match.remaining_quantity;

            // Check if price level is empty and mark for removal
            if price_level.order_count() == 0 {
                empty_price_levels.push(price);
            }

            // Early exit if order is fully matched
            if remaining_quantity == 0 {
                break;
            }
        }

        // Batch remove empty price levels
        for price in &empty_price_levels {
            match_side.remove(price);
        }

        // Batch remove filled orders from tracking
        for order_id in &filled_orders {
            self.order_locations.remove(order_id);
        }

        // Return vectors to pool for reuse
        MATCHING_POOL.with(|pool| {
            pool.return_filled_orders_vec(filled_orders);
            pool.return_price_vec(empty_price_levels);
        });

        // Check for insufficient liquidity in market orders
        if limit_price.is_none() && remaining_quantity == quantity {
            return Err(OrderBookError::InsufficientLiquidity {
                side,
                requested: quantity,
                available: 0,
            });
        }

        // Set final result properties
        match_result.remaining_quantity = remaining_quantity;
        match_result.is_complete = remaining_quantity == 0;

        Ok(match_result)
    }

    /// Optimized peek match without memory pooling or sorting
    ///
    /// # Performance Optimization
    /// Uses SkipMap's natural ordering to eliminate sorting overhead.
    /// Time complexity: O(M log N) where M = price levels inspected.
    pub fn peek_match(&self, side: Side, quantity: u64, price_limit: Option<u128>) -> u64 {
        let price_levels = match side {
            Side::Buy => &self.asks,
            Side::Sell => &self.bids,
        };

        if price_levels.is_empty() {
            return 0;
        }

        let mut matched_quantity = 0u64;

        // Iterate through prices in optimal order (already sorted by SkipMap)
        let price_iter: Box<dyn Iterator<Item = _>> = match side {
            Side::Buy => Box::new(price_levels.iter()),
            Side::Sell => Box::new(price_levels.iter().rev()),
        };

        // Process each price level
        for entry in price_iter {
            // Early termination when we have enough quantity
            if matched_quantity >= quantity {
                break;
            }

            let price = *entry.key();

            // Check price limit
            if let Some(limit) = price_limit {
                match side {
                    Side::Buy if price > limit => break,
                    Side::Sell if price < limit => break,
                    _ => {}
                }
            }

            // Get available quantity at this level
            let price_level = entry.value();
            let available_quantity = price_level.total_quantity();
            let needed_quantity = quantity.saturating_sub(matched_quantity);
            let quantity_to_match = needed_quantity.min(available_quantity);
            matched_quantity = matched_quantity.saturating_add(quantity_to_match);
        }

        matched_quantity
    }

    /// Batch operation for multiple order matches (additional optimization)
    pub fn match_orders_batch(
        &self,
        orders: &[(OrderId, Side, u64, Option<u128>)],
    ) -> Vec<Result<MatchResult, OrderBookError>> {
        let mut results = Vec::with_capacity(orders.len());

        for &(order_id, side, quantity, limit_price) in orders {
            let result = OrderBook::<T>::match_order(self, order_id, side, quantity, limit_price);
            results.push(result);
        }

        results
    }
}
