//! Order book operations like adding, modifying and canceling orders

use super::book::OrderBook;
use super::error::OrderBookError;
use pricelevel::{Hash32, MatchResult, OrderId, OrderType, Side, TimeInForce};
use std::sync::Arc;
use tracing::trace;

impl<T> OrderBook<T>
where
    T: Clone + Send + Sync + Default + 'static,
{
    /// Add a limit order to the book
    pub fn add_limit_order(
        &self,
        id: OrderId,
        price: u128,
        quantity: u64,
        side: Side,
        time_in_force: TimeInForce,
        extra_fields: Option<T>,
    ) -> Result<Arc<OrderType<T>>, OrderBookError> {
        let extra_fields: T = extra_fields.unwrap_or_default();
        let order = OrderType::Standard {
            id,
            price,
            quantity,
            side,
            user_id: Hash32::zero(),
            timestamp: crate::utils::current_time_millis(),
            time_in_force,
            extra_fields,
        };
        trace!(
            "Adding limit order {} {} {} {} {}",
            id, price, quantity, side, time_in_force
        );
        self.add_order(order)
    }

    /// Add an iceberg order to the book
    #[allow(clippy::too_many_arguments)]
    pub fn add_iceberg_order(
        &self,
        id: OrderId,
        price: u128,
        visible_quantity: u64,
        hidden_quantity: u64,
        side: Side,
        time_in_force: TimeInForce,
        extra_fields: Option<T>,
    ) -> Result<Arc<OrderType<T>>, OrderBookError> {
        let extra_fields: T = extra_fields.unwrap_or_default();
        let order = OrderType::IcebergOrder {
            id,
            price,
            visible_quantity,
            hidden_quantity,
            side,
            user_id: Hash32::zero(),
            timestamp: crate::utils::current_time_millis(),
            time_in_force,
            extra_fields,
        };
        trace!(
            "Adding iceberg order {} {} {} {} {}",
            id, price, visible_quantity, hidden_quantity, side
        );
        self.add_order(order)
    }

    /// Add a post-only order to the book
    pub fn add_post_only_order(
        &self,
        id: OrderId,
        price: u128,
        quantity: u64,
        side: Side,
        time_in_force: TimeInForce,
        extra_fields: Option<T>,
    ) -> Result<Arc<OrderType<T>>, OrderBookError> {
        let extra_fields: T = extra_fields.unwrap_or_default();
        let order = OrderType::PostOnly {
            id,
            price,
            quantity,
            side,
            user_id: Hash32::zero(),
            timestamp: crate::utils::current_time_millis(),
            time_in_force,
            extra_fields,
        };
        trace!(
            "Adding post-only order {} {} {} {} {}",
            id, price, quantity, side, time_in_force
        );
        self.add_order(order)
    }

    /// Submit a simple market order
    pub fn submit_market_order(
        &self,
        id: OrderId,
        quantity: u64,
        side: Side,
    ) -> Result<MatchResult, OrderBookError> {
        trace!("Submitting market order {} {} {}", id, quantity, side);
        OrderBook::<T>::match_market_order(self, id, quantity, side)
    }
}
