/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 15/7/25
******************************************************************************/

use crossbeam::atomic::AtomicCell;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct PriceLevelCache {
    best_bid_price: AtomicCell<u128>,
    best_ask_price: AtomicCell<u128>,
    cache_valid: AtomicBool,
}

impl Serialize for PriceLevelCache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PriceLevelCache", 3)?;
        state.serialize_field("best_bid_price", &self.best_bid_price.load())?;
        state.serialize_field("best_ask_price", &self.best_ask_price.load())?;
        state.serialize_field("cache_valid", &self.cache_valid.load(Ordering::Relaxed))?;
        state.end()
    }
}

impl PriceLevelCache {
    pub fn new() -> Self {
        Self {
            best_bid_price: AtomicCell::new(0),
            best_ask_price: AtomicCell::new(0),
            cache_valid: AtomicBool::new(false),
        }
    }

    pub fn invalidate(&self) {
        self.cache_valid.store(false, Ordering::Relaxed);
    }

    pub fn get_cached_best_bid(&self) -> Option<u128> {
        if self.cache_valid.load(Ordering::Relaxed) {
            let price = self.best_bid_price.load();
            if price > 0 { Some(price) } else { None }
        } else {
            None
        }
    }

    pub fn get_cached_best_ask(&self) -> Option<u128> {
        if self.cache_valid.load(Ordering::Relaxed) {
            let price = self.best_ask_price.load();
            if price > 0 { Some(price) } else { None }
        } else {
            None
        }
    }

    pub fn update_best_prices(&self, best_bid: Option<u128>, best_ask: Option<u128>) {
        if let Some(bid) = best_bid {
            self.best_bid_price.store(bid);
        } else {
            self.best_bid_price.store(0);
        }

        if let Some(ask) = best_ask {
            self.best_ask_price.store(ask);
        } else {
            self.best_ask_price.store(0);
        }

        self.cache_valid.store(true, Ordering::Relaxed);
    }
}
