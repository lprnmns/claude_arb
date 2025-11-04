use crate::config::RiskConfig;
use crate::errors::{BotError, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc, Datelike};
use tracing::{info, warn};

/// Risk manager to enforce position limits and loss limits
#[derive(Clone)]
pub struct RiskManager {
    config: RiskConfig,
    state: Arc<RwLock<RiskState>>,
}

#[derive(Debug, Clone)]
struct RiskState {
    daily_pnl: Decimal,
    last_reset_day: u32,
    total_trades: u64,
    winning_trades: u64,
    losing_trades: u64,
    total_profit: Decimal,
    total_loss: Decimal,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(RiskState {
                daily_pnl: Decimal::ZERO,
                last_reset_day: Utc::now().day(),
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                total_profit: Decimal::ZERO,
                total_loss: Decimal::ZERO,
            })),
        }
    }

    /// Check if we can open a new position
    pub fn can_open_position(&self, _position_size_usd: Decimal) -> Result<()> {
        if self.config.emergency_stop {
            return Err(BotError::RiskLimitExceeded(
                "Emergency stop is active".into(),
            ));
        }

        let state = self.state.read();
        self.check_daily_reset(&mut state.clone());

        // Check daily loss limit
        if state.daily_pnl < -self.config.daily_loss_limit_usd {
            return Err(BotError::RiskLimitExceeded(format!(
                "Daily loss limit exceeded: ${:.2} (limit: ${:.2})",
                state.daily_pnl, self.config.daily_loss_limit_usd
            )));
        }

        Ok(())
    }

    /// Record trade PnL
    pub fn record_trade(&self, pnl: Decimal) {
        let mut state = self.state.write();
        self.check_daily_reset(&mut state);

        state.daily_pnl += pnl;
        state.total_trades += 1;

        if pnl > Decimal::ZERO {
            state.winning_trades += 1;
            state.total_profit += pnl;
            info!("✅ Winning trade: +${:.2} (Daily PnL: ${:.2})", pnl, state.daily_pnl);
        } else {
            state.losing_trades += 1;
            state.total_loss += pnl.abs();
            warn!("❌ Losing trade: -${:.2} (Daily PnL: ${:.2})", pnl.abs(), state.daily_pnl);
        }

        // Log statistics every 10 trades
        if state.total_trades % 10 == 0 {
            self.log_statistics();
        }
    }

    /// Get current statistics
    pub fn get_statistics(&self) -> RiskStatistics {
        let state = self.state.read();

        let win_rate = if state.total_trades > 0 {
            (state.winning_trades as f64 / state.total_trades as f64) * 100.0
        } else {
            0.0
        };

        let avg_win = if state.winning_trades > 0 {
            state.total_profit / Decimal::from(state.winning_trades)
        } else {
            Decimal::ZERO
        };

        let avg_loss = if state.losing_trades > 0 {
            state.total_loss / Decimal::from(state.losing_trades)
        } else {
            Decimal::ZERO
        };

        RiskStatistics {
            daily_pnl: state.daily_pnl,
            total_trades: state.total_trades,
            winning_trades: state.winning_trades,
            losing_trades: state.losing_trades,
            win_rate,
            avg_win,
            avg_loss,
            total_profit: state.total_profit,
            total_loss: state.total_loss,
        }
    }

    /// Log current statistics
    pub fn log_statistics(&self) {
        let stats = self.get_statistics();

        info!("📊 Risk Statistics:");
        info!("   Daily PnL: ${:.2}", stats.daily_pnl);
        info!("   Total Trades: {}", stats.total_trades);
        info!("   Win Rate: {:.1}%", stats.win_rate);
        info!("   Avg Win: ${:.2}", stats.avg_win);
        info!("   Avg Loss: ${:.2}", stats.avg_loss);
        info!("   Total Profit: ${:.2}", stats.total_profit);
        info!("   Total Loss: ${:.2}", stats.total_loss);
    }

    /// Reset daily statistics if day changed
    fn check_daily_reset(&self, state: &mut RiskState) {
        let current_day = Utc::now().day();
        if current_day != state.last_reset_day {
            info!("📅 Daily reset: Previous day PnL: ${:.2}", state.daily_pnl);
            state.daily_pnl = Decimal::ZERO;
            state.last_reset_day = current_day;
        }
    }

    /// Check if position loss exceeds maximum allowed
    pub fn check_position_loss(&self, entry_price: Decimal, current_price: Decimal, is_long: bool) -> bool {
        let loss_pct = if is_long {
            ((current_price - entry_price) / entry_price) * Decimal::from(100)
        } else {
            ((entry_price - current_price) / entry_price) * Decimal::from(100)
        };

        // Convert max_loss_bps to percentage
        let max_loss_pct = self.config.max_loss_bps / Decimal::from(100);

        if loss_pct < -max_loss_pct {
            warn!("⚠️ Position loss exceeds limit: {:.2}% (max: {:.2}%)",
                  loss_pct, max_loss_pct);
            return true;
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct RiskStatistics {
    pub daily_pnl: Decimal,
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub win_rate: f64,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub total_profit: Decimal,
    pub total_loss: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_risk_manager() {
        let config = RiskConfig {
            max_loss_bps: dec!(50),
            daily_loss_limit_usd: dec!(20),
            emergency_stop: false,
        };

        let manager = RiskManager::new(config);

        // Should allow opening position initially
        assert!(manager.can_open_position(dec!(150)).is_ok());

        // Record some trades
        manager.record_trade(dec!(5.0)); // Win
        manager.record_trade(dec!(-3.0)); // Loss

        let stats = manager.get_statistics();
        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.winning_trades, 1);
        assert_eq!(stats.losing_trades, 1);
        assert_eq!(stats.daily_pnl, dec!(2.0));
    }

    #[test]
    fn test_daily_loss_limit() {
        let config = RiskConfig {
            max_loss_bps: dec!(50),
            daily_loss_limit_usd: dec!(10),
            emergency_stop: false,
        };

        let manager = RiskManager::new(config);

        // Record large loss
        manager.record_trade(dec!(-15.0));

        // Should not allow new position
        assert!(manager.can_open_position(dec!(150)).is_err());
    }
}
