use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Order execution error: {0}")]
    OrderExecution(String),

    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),

    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BotError>;
