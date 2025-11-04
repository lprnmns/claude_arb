use crate::config::Config;
use crate::errors::{BotError, Result};
use crate::orderbook::Orderbook;
use crate::types::*;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{debug, info};

/// Arbitrage strategy for perp-spot spread trading
pub struct ArbitrageStrategy {
    pub config: Config,
    perp_orderbook: Arc<Orderbook>,
    spot_orderbook: Arc<Orderbook>,
    state: ArbitrageState,
    entry_bps: Option<Decimal>,
}

impl ArbitrageStrategy {
    pub fn new(
        config: Config,
        perp_orderbook: Arc<Orderbook>,
        spot_orderbook: Arc<Orderbook>,
    ) -> Self {
        Self {
            config,
            perp_orderbook,
            spot_orderbook,
            state: ArbitrageState::Idle,
            entry_bps: None,
        }
    }

    /// Calculate BPS between perp and spot
    /// BPS = ((perp_price - spot_price) / spot_price) * 10000
    /// Positive BPS means perp is trading at premium
    pub fn calculate_bps(&self) -> Option<Decimal> {
        let perp_ask = self.perp_orderbook.best_ask()?;
        let spot_bid = self.spot_orderbook.best_bid()?;

        let bps = ((perp_ask.price - spot_bid.price) / spot_bid.price)
            * Decimal::from(10000);

        Some(bps)
    }

    /// Get current arbitrage state
    pub fn state(&self) -> ArbitrageState {
        self.state
    }

    /// Check if we should enter a position
    pub fn should_enter(&self) -> Option<TradeSignal> {
        if self.state != ArbitrageState::Idle {
            return None;
        }

        let bps = self.calculate_bps()?;
        let threshold = self.config.strategy.bps_threshold;

        if bps >= threshold {
            let perp_ask = self.perp_orderbook.best_ask()?;
            let spot_bid = self.spot_orderbook.best_bid()?;

            // Check liquidity
            let position_size_coins = self.calculate_position_size(spot_bid.price);

            if !self.check_liquidity(position_size_coins) {
                debug!("Insufficient liquidity for entry");
                return None;
            }

            info!("🎯 Entry signal! BPS: {} (threshold: {})", bps, threshold);

            Some(TradeSignal {
                timestamp: chrono::Utc::now(),
                bps,
                perp_price: perp_ask.price,
                spot_price: spot_bid.price,
                action: SignalAction::Enter,
            })
        } else {
            None
        }
    }

    /// Check if we should exit a position
    pub fn should_exit(&self) -> Option<TradeSignal> {
        if self.state != ArbitrageState::PositionOpen {
            return None;
        }

        let bps = self.calculate_bps()?;

        // Exit when spread narrows to 0 or goes negative
        if bps <= Decimal::ZERO {
            let perp_bid = self.perp_orderbook.best_bid()?;
            let spot_ask = self.spot_orderbook.best_ask()?;

            info!("🎯 Exit signal! BPS: {} (target: 0)", bps);

            Some(TradeSignal {
                timestamp: chrono::Utc::now(),
                bps,
                perp_price: perp_bid.price,
                spot_price: spot_ask.price,
                action: SignalAction::Exit,
            })
        } else {
            None
        }
    }

    /// Check if emergency exit is needed (loss exceeds threshold)
    pub fn should_emergency_exit(&self) -> bool {
        if self.state != ArbitrageState::PositionOpen {
            return false;
        }

        if let (Some(entry_bps), Some(current_bps)) = (self.entry_bps, self.calculate_bps()) {
            let bps_change = current_bps - entry_bps;

            // If spread widened too much, we're losing money
            if bps_change > self.config.risk.max_loss_bps {
                info!("⚠️ Emergency exit triggered! BPS change: {}", bps_change);
                return true;
            }
        }

        false
    }

    /// Calculate position size in coins based on USD amount
    pub fn calculate_position_size(&self, price: Decimal) -> Decimal {
        let leverage = Decimal::from(self.config.strategy.leverage);
        let notional = self.config.strategy.position_size_usd * leverage;
        notional / price
    }

    /// Check if there's sufficient liquidity for the trade
    fn check_liquidity(&self, required_size: Decimal) -> bool {
        let perp_ask = match self.perp_orderbook.best_ask() {
            Some(level) => level,
            None => return false,
        };

        let spot_bid = match self.spot_orderbook.best_bid() {
            Some(level) => level,
            None => return false,
        };

        // Check if top level has enough liquidity (conservative)
        perp_ask.size >= required_size && spot_bid.size >= required_size
    }

    /// Build entry orders (short perp, long spot)
    pub fn build_entry_orders(&self) -> Result<Vec<OrderRequest>> {
        let perp_ask = self
            .perp_orderbook
            .best_ask()
            .ok_or_else(|| BotError::InvalidState("No perp ask available".into()))?;

        let spot_bid = self
            .spot_orderbook
            .best_bid()
            .ok_or_else(|| BotError::InvalidState("No spot bid available".into()))?;

        let position_size_usd = self.config.strategy.position_size_usd;
        let leverage = Decimal::from(self.config.strategy.leverage);

        // Calculate position sizes
        let perp_notional = position_size_usd * leverage;
        let perp_size = perp_notional / perp_ask.price;

        let spot_notional = position_size_usd; // Spot is 1x
        let spot_size = spot_notional / spot_bid.price;

        // Use IOC for immediate execution
        let orders = vec![
            // Short perp
            OrderRequest {
                symbol: self.config.symbols.perp_symbol.clone(),
                side: Side::Sell,
                price: perp_ask.price,
                size: perp_size,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::IOC,
                reduce_only: false,
            },
            // Long spot
            OrderRequest {
                symbol: self.config.symbols.spot_symbol.clone(),
                side: Side::Buy,
                price: spot_bid.price,
                size: spot_size,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::IOC,
                reduce_only: false,
            },
        ];

        Ok(orders)
    }

    /// Build exit orders (close short perp, close long spot)
    pub fn build_exit_orders(&self, position_size: Decimal) -> Result<Vec<OrderRequest>> {
        let perp_bid = self
            .perp_orderbook
            .best_bid()
            .ok_or_else(|| BotError::InvalidState("No perp bid available".into()))?;

        let spot_ask = self
            .spot_orderbook
            .best_ask()
            .ok_or_else(|| BotError::InvalidState("No spot ask available".into()))?;

        // Start with ALO (maker only) for fee rebate
        let orders = vec![
            // Close short perp (buy back)
            OrderRequest {
                symbol: self.config.symbols.perp_symbol.clone(),
                side: Side::Buy,
                price: perp_bid.price,
                size: position_size,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::ALO,
                reduce_only: true,
            },
            // Close long spot (sell)
            OrderRequest {
                symbol: self.config.symbols.spot_symbol.clone(),
                side: Side::Sell,
                price: spot_ask.price,
                size: position_size,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::ALO,
                reduce_only: true,
            },
        ];

        Ok(orders)
    }

    /// Update strategy state
    pub fn set_state(&mut self, state: ArbitrageState) {
        info!("Strategy state: {:?} -> {:?}", self.state, state);
        self.state = state;
    }

    /// Record entry BPS for PnL tracking
    pub fn record_entry(&mut self, bps: Decimal) {
        self.entry_bps = Some(bps);
        info!("Entry BPS recorded: {}", bps);
    }

    /// Clear entry data
    pub fn clear_entry(&mut self) {
        self.entry_bps = None;
    }

    /// Calculate estimated profit in BPS
    pub fn estimated_profit_bps(&self) -> Option<Decimal> {
        let entry = self.entry_bps?;
        let current = self.calculate_bps()?;
        Some(entry - current) // Profit = entry BPS - current BPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::orderbook::OrderSide;

    #[test]
    fn test_bps_calculation() {
        // Create mock config
        let config = Config {
            api_url: "".into(),
            ws_url: "".into(),
            private_key: "0000000000000000000000000000000000000000000000000000000000000001".into(),
            wallet_address: "0x0000000000000000000000000000000000000000".into(),
            strategy: crate::config::StrategyConfig {
                bps_threshold: dec!(20),
                position_size_usd: dec!(150),
                leverage: 2,
                timeout_seconds: 30,
            },
            risk: crate::config::RiskConfig {
                max_loss_bps: dec!(50),
                daily_loss_limit_usd: dec!(20),
                emergency_stop: false,
            },
            symbols: crate::config::SymbolConfig {
                perp_symbol: "HYPE".into(),
                spot_symbol: "HYPE".into(),
            },
        };

        let perp_ob = Arc::new(Orderbook::new("HYPE-PERP".into()));
        let spot_ob = Arc::new(Orderbook::new("HYPE-SPOT".into()));

        // Set up orderbooks
        // Perp ask at 101, Spot bid at 100
        perp_ob.update_level(OrderSide::Ask, dec!(101.0), dec!(100.0));
        spot_ob.update_level(OrderSide::Bid, dec!(100.0), dec!(100.0));

        let strategy = ArbitrageStrategy::new(config, perp_ob, spot_ob);

        // BPS = ((101 - 100) / 100) * 10000 = 100 bps
        let bps = strategy.calculate_bps().unwrap();
        assert!(bps > dec!(99) && bps < dec!(101));
    }
}
