use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

/// Price level in the orderbook
pub type Price = Decimal;
pub type Size = Decimal;

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: Price,
    pub size: Size,
}

/// L2 Orderbook with lock-free reads using RwLock
#[derive(Debug)]
pub struct Orderbook {
    pub symbol: String,
    pub bids: Arc<RwLock<BTreeMap<Price, Size>>>, // Highest to lowest
    pub asks: Arc<RwLock<BTreeMap<Price, Size>>>, // Lowest to highest
    pub last_update: Arc<RwLock<DateTime<Utc>>>,
}

impl Orderbook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: Arc::new(RwLock::new(BTreeMap::new())),
            asks: Arc::new(RwLock::new(BTreeMap::new())),
            last_update: Arc::new(RwLock::new(Utc::now())),
        }
    }

    /// Update a single price level
    pub fn update_level(&self, side: OrderSide, price: Price, size: Size) {
        match side {
            OrderSide::Bid => {
                let mut bids = self.bids.write();
                if size == Decimal::ZERO {
                    bids.remove(&price);
                } else {
                    bids.insert(price, size);
                }
            }
            OrderSide::Ask => {
                let mut asks = self.asks.write();
                if size == Decimal::ZERO {
                    asks.remove(&price);
                } else {
                    asks.insert(price, size);
                }
            }
        }
        *self.last_update.write() = Utc::now();
    }

    /// Get best bid (highest buy price)
    pub fn best_bid(&self) -> Option<PriceLevel> {
        let bids = self.bids.read();
        bids.iter()
            .next_back() // BTreeMap is sorted, so last is highest
            .map(|(price, size)| PriceLevel {
                price: *price,
                size: *size,
            })
    }

    /// Get best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<PriceLevel> {
        let asks = self.asks.read();
        asks.iter()
            .next() // First is lowest
            .map(|(price, size)| PriceLevel {
                price: *price,
                size: *size,
            })
    }

    /// Get mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;
        Some((bid.price + ask.price) / Decimal::from(2))
    }

    /// Get spread in bps
    pub fn spread_bps(&self) -> Option<Decimal> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;
        Some(((ask.price - bid.price) / bid.price) * Decimal::from(10000))
    }

    /// Check available liquidity at a specific price level
    pub fn get_liquidity(&self, side: OrderSide, target_price: Price) -> Decimal {
        match side {
            OrderSide::Bid => {
                let bids = self.bids.read();
                bids.range(..=target_price)
                    .map(|(_, size)| size)
                    .sum()
            }
            OrderSide::Ask => {
                let asks = self.asks.read();
                asks.range(target_price..)
                    .map(|(_, size)| size)
                    .sum()
            }
        }
    }

    /// Get top N levels
    pub fn get_depth(&self, side: OrderSide, levels: usize) -> Vec<PriceLevel> {
        match side {
            OrderSide::Bid => {
                let bids = self.bids.read();
                bids.iter()
                    .rev()
                    .take(levels)
                    .map(|(price, size)| PriceLevel {
                        price: *price,
                        size: *size,
                    })
                    .collect()
            }
            OrderSide::Ask => {
                let asks = self.asks.read();
                asks.iter()
                    .take(levels)
                    .map(|(price, size)| PriceLevel {
                        price: *price,
                        size: *size,
                    })
                    .collect()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_orderbook_operations() {
        let ob = Orderbook::new("TEST".to_string());

        // Add some levels
        ob.update_level(OrderSide::Bid, dec!(100.0), dec!(10.0));
        ob.update_level(OrderSide::Bid, dec!(99.0), dec!(15.0));
        ob.update_level(OrderSide::Ask, dec!(101.0), dec!(8.0));
        ob.update_level(OrderSide::Ask, dec!(102.0), dec!(12.0));

        // Test best bid/ask
        assert_eq!(ob.best_bid().unwrap().price, dec!(100.0));
        assert_eq!(ob.best_ask().unwrap().price, dec!(101.0));

        // Test mid price
        assert_eq!(ob.mid_price().unwrap(), dec!(100.5));

        // Test spread
        let spread = ob.spread_bps().unwrap();
        assert!(spread > dec!(99.0) && spread < dec!(101.0)); // ~100 bps

        // Test liquidity
        let bid_liquidity = ob.get_liquidity(OrderSide::Bid, dec!(99.0));
        assert_eq!(bid_liquidity, dec!(25.0)); // 10 + 15
    }
}
