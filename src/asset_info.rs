use crate::errors::{BotError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Asset information from Hyperliquid meta API
#[derive(Debug, Clone)]
pub struct AssetInfo {
    /// Perpetual asset index (for order API)
    pub perp_index: u32,
    /// Spot asset index (for order API, will be 10000 + spot_index)
    pub spot_index: u32,
    /// Symbol name (e.g., "HYPE")
    pub symbol: String,
}

impl AssetInfo {
    /// Get order API asset value for perpetual
    pub fn perp_asset(&self) -> u32 {
        self.perp_index
    }

    /// Get order API asset value for spot (10000 + index)
    pub fn spot_asset(&self) -> u32 {
        10000 + self.spot_index
    }
}

#[derive(Debug, Deserialize)]
struct PerpMeta {
    universe: Vec<PerpAsset>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpAsset {
    name: String,
    #[serde(default)]
    is_delisted: bool,
}

#[derive(Debug, Deserialize)]
struct SpotMeta {
    universe: Vec<SpotAsset>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotAsset {
    tokens: Vec<u32>,
    name: String,
    index: u32,
}

/// Fetch asset info from Hyperliquid meta API
pub async fn fetch_asset_info(
    api_url: &str,
    perp_symbol: &str,
    spot_symbol: &str,
) -> Result<AssetInfo> {
    let client = reqwest::Client::new();

    // Fetch perpetual meta
    let perp_meta: PerpMeta = client
        .post(api_url)
        .json(&serde_json::json!({"type": "meta"}))
        .send()
        .await
        .map_err(|e| BotError::Api(format!("Failed to fetch perp meta: {}", e)))?
        .json()
        .await
        .map_err(|e| BotError::Parse(format!("Failed to parse perp meta: {}", e)))?;

    // Find perp index
    let perp_index = perp_meta
        .universe
        .iter()
        .enumerate()
        .find(|(_, asset)| asset.name == perp_symbol && !asset.is_delisted)
        .map(|(idx, _)| idx as u32)
        .ok_or_else(|| {
            BotError::InvalidState(format!("Perp symbol {} not found in meta", perp_symbol))
        })?;

    // Fetch spot meta
    let spot_meta: SpotMeta = client
        .post(api_url)
        .json(&serde_json::json!({"type": "spotMeta"}))
        .send()
        .await
        .map_err(|e| BotError::Api(format!("Failed to fetch spot meta: {}", e)))?
        .json()
        .await
        .map_err(|e| BotError::Parse(format!("Failed to parse spot meta: {}", e)))?;

    // For spot, we need to find the pair with our symbol
    // For HYPE/USDC, we look for tokens containing HYPE token ID (150) and USDC (0)
    let spot_index = if spot_symbol == "HYPE/USDC" || spot_symbol == "HYPE" {
        // HYPE token ID is 150, USDC is 0
        spot_meta
            .universe
            .iter()
            .find(|asset| {
                asset.tokens.len() == 2
                    && asset.tokens.contains(&150) // HYPE token
                    && asset.tokens.contains(&0) // USDC token
            })
            .map(|asset| asset.index)
            .ok_or_else(|| {
                BotError::InvalidState(format!("Spot pair {} not found in spotMeta", spot_symbol))
            })?
    } else {
        // For other symbols, try to match by name
        spot_meta
            .universe
            .iter()
            .find(|asset| asset.name.contains(spot_symbol) || spot_symbol.contains(&asset.name))
            .map(|asset| asset.index)
            .ok_or_else(|| {
                BotError::InvalidState(format!("Spot pair {} not found in spotMeta", spot_symbol))
            })?
    };

    Ok(AssetInfo {
        perp_index,
        spot_index,
        symbol: perp_symbol.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_hype_asset_info() {
        let info = fetch_asset_info(
            "https://api.hyperliquid.xyz/info",
            "HYPE",
            "HYPE/USDC",
        )
        .await
        .unwrap();

        assert_eq!(info.perp_index, 159);
        assert_eq!(info.spot_index, 107);
        assert_eq!(info.perp_asset(), 159);
        assert_eq!(info.spot_asset(), 10107);
    }
}
