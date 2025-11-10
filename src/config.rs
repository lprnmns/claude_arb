use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub ws_url: String,
    pub private_key: String,
    pub wallet_address: String,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub symbols: SymbolConfig,
    pub dry_run: DryRunConfig,
    pub fees: FeeConfig,
}

#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub bps_threshold: Decimal,
    pub position_size_usd: Decimal,
    pub leverage: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_loss_bps: Decimal,
    pub daily_loss_limit_usd: Decimal,
    pub emergency_stop: bool,
}

#[derive(Debug, Clone)]
pub struct SymbolConfig {
    pub perp_symbol: String,
    pub spot_symbol: String,
}

#[derive(Debug, Clone)]
pub struct DryRunConfig {
    pub enabled: bool,
    pub fill_delay_ms: u64,
    pub fill_success_rate: u8,
}

#[derive(Debug, Clone)]
pub struct FeeConfig {
    // Perp fees in percentage (e.g., 0.045 for 0.045%)
    pub perp_taker_fee: Decimal,
    pub perp_maker_fee: Decimal,
    // Spot fees in percentage
    pub spot_taker_fee: Decimal,
    pub spot_maker_fee: Decimal,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let api_url = env::var("HYPERLIQUID_API_URL")
            .unwrap_or_else(|_| "https://api.hyperliquid.xyz".to_string());

        let ws_url = env::var("HYPERLIQUID_WS_URL")
            .unwrap_or_else(|_| "wss://api.hyperliquid.xyz/ws".to_string());

        let private_key = env::var("PRIVATE_KEY")
            .context("PRIVATE_KEY must be set in .env file")?;

        // Derive wallet address from private key (we'll implement this with ethers)
        let wallet_address = derive_address_from_private_key(&private_key)?;

        let strategy = StrategyConfig {
            bps_threshold: parse_decimal_env("BPS_THRESHOLD", "20.0")?,
            position_size_usd: parse_decimal_env("POSITION_SIZE_USD", "150.0")?,
            leverage: env::var("LEVERAGE")
                .unwrap_or_else(|_| "2".to_string())
                .parse()
                .context("Invalid LEVERAGE")?,
            timeout_seconds: env::var("TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid TIMEOUT_SECONDS")?,
        };

        let risk = RiskConfig {
            max_loss_bps: parse_decimal_env("MAX_LOSS_BPS", "50.0")?,
            daily_loss_limit_usd: parse_decimal_env("DAILY_LOSS_LIMIT_USD", "20.0")?,
            emergency_stop: env::var("EMERGENCY_STOP")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        };

        let symbols = SymbolConfig {
            perp_symbol: env::var("PERP_SYMBOL").unwrap_or_else(|_| "HYPE".to_string()),
            spot_symbol: env::var("SPOT_SYMBOL").unwrap_or_else(|_| "HYPE".to_string()),
        };

        let dry_run = DryRunConfig {
            enabled: env::var("DRY_RUN")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            fill_delay_ms: env::var("DRY_RUN_FILL_DELAY_MS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            fill_success_rate: env::var("DRY_RUN_FILL_SUCCESS_RATE")
                .unwrap_or_else(|_| "95".to_string())
                .parse()
                .unwrap_or(95)
                .min(100), // Cap at 100%
        };

        // Hyperliquid fee structure (as percentages)
        let fees = FeeConfig {
            perp_taker_fee: parse_decimal_env("PERP_TAKER_FEE", "0.045")?,  // 0.045%
            perp_maker_fee: parse_decimal_env("PERP_MAKER_FEE", "0.015")?,  // 0.015%
            spot_taker_fee: parse_decimal_env("SPOT_TAKER_FEE", "0.070")?,  // 0.070%
            spot_maker_fee: parse_decimal_env("SPOT_MAKER_FEE", "0.040")?,  // 0.040%
        };

        Ok(Config {
            api_url,
            ws_url,
            private_key,
            wallet_address,
            strategy,
            risk,
            symbols,
            dry_run,
            fees,
        })
    }
}

fn parse_decimal_env(key: &str, default: &str) -> Result<Decimal> {
    let value = env::var(key).unwrap_or_else(|_| default.to_string());
    value
        .parse::<Decimal>()
        .with_context(|| format!("Invalid {} value: {}", key, value))
}

fn derive_address_from_private_key(private_key: &str) -> Result<String> {
    use ethers::signers::{LocalWallet, Signer};

    let private_key = private_key.trim_start_matches("0x");
    let wallet: LocalWallet = private_key
        .parse()
        .context("Invalid private key format")?;

    Ok(format!("{:?}", wallet.address()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        // Test will be implemented when we have .env
    }
}
