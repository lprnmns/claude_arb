use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "market")]
    Market,
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    #[serde(rename = "Gtc")]
    GTC, // Good till cancel (ALO in Hyperliquid)
    #[serde(rename = "Ioc")]
    IOC, // Immediate or cancel
    #[serde(rename = "Alo")]
    ALO, // Add liquidity only (maker only)
}

/// Order request
#[derive(Debug, Clone, Serialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: Side,
    pub price: Decimal,
    pub size: Decimal,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
}

/// Position
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub leverage: u32,
}

/// Market type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketType {
    Perp,
    Spot,
}

/// Arbitrage state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrageState {
    Idle,
    WaitingEntry,
    PositionOpen,
    WaitingExit,
    Error,
}

/// Trade signal
#[derive(Debug, Clone)]
pub struct TradeSignal {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub bps: Decimal,
    pub perp_price: Decimal,
    pub spot_price: Decimal,
    pub action: SignalAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAction {
    Enter,
    Exit,
    Hold,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub perp_order_id: Option<String>,
    pub spot_order_id: Option<String>,
    pub error: Option<String>,
}
