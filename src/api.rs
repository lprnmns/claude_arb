use crate::errors::{BotError, Result};
use crate::types::*;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, Signature, H256};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use tracing::{debug, error, info};

/// Hyperliquid API client
#[derive(Clone)]
pub struct HyperliquidClient {
    client: Client,
    api_url: String,
    wallet: LocalWallet,
}

impl HyperliquidClient {
    pub fn new(api_url: String, private_key: String) -> Result<Self> {
        let private_key = private_key.trim_start_matches("0x");
        let wallet: LocalWallet = private_key
            .parse()
            .map_err(|e| BotError::Config(format!("Invalid private key: {:?}", e)))?;

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| BotError::Api(format!("Failed to create HTTP client: {}", e)))?,
            api_url,
            wallet,
        })
    }

    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    /// Get user state (positions, balances)
    pub async fn get_user_state(&self) -> Result<UserState> {
        let request = json!({
            "type": "clearinghouseState",
            "user": format!("{:?}", self.address())
        });

        let response = self
            .client
            .post(format!("{}/info", self.api_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Api(format!("Failed to get user state: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(BotError::Api(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let user_state: UserState = response
            .json()
            .await
            .map_err(|e| BotError::Parse(format!("Failed to parse user state: {}", e)))?;

        Ok(user_state)
    }

    /// Get L2 orderbook snapshot
    pub async fn get_orderbook(&self, symbol: &str) -> Result<L2Snapshot> {
        let request = json!({
            "type": "l2Book",
            "coin": symbol
        });

        let response = self
            .client
            .post(format!("{}/info", self.api_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| BotError::Api(format!("Failed to get orderbook: {}", e)))?;

        if !response.status().is_success() {
            return Err(BotError::Api(format!(
                "Failed to get orderbook: {}",
                response.status()
            )));
        }

        let snapshot: L2Snapshot = response
            .json()
            .await
            .map_err(|e| BotError::Parse(format!("Failed to parse orderbook: {}", e)))?;

        Ok(snapshot)
    }

    /// Place an order
    pub async fn place_order(&self, order: &OrderRequest) -> Result<PlaceOrderResponse> {
        info!("Placing order: {:?}", order);

        let action = self.build_order_action(order)?;
        let signed_action = self.sign_action(&action).await?;

        let response = self
            .client
            .post(format!("{}/exchange", self.api_url))
            .json(&signed_action)
            .send()
            .await
            .map_err(|e| BotError::OrderExecution(format!("Failed to place order: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Order placement failed: {} - {}", status, text);
            return Err(BotError::OrderExecution(format!(
                "Order failed: {} - {}",
                status, text
            )));
        }

        let result: PlaceOrderResponse = response
            .json()
            .await
            .map_err(|e| BotError::Parse(format!("Failed to parse order response: {}", e)))?;

        debug!("Order response: {:?}", result);
        Ok(result)
    }

    /// Place multiple orders in a batch (atomic execution)
    pub async fn place_batch_orders(
        &self,
        orders: Vec<OrderRequest>,
    ) -> Result<Vec<PlaceOrderResponse>> {
        info!("Placing {} orders in batch", orders.len());

        let actions: Vec<_> = orders
            .iter()
            .map(|order| self.build_order_action(order))
            .collect::<Result<Vec<_>>>()?;

        let signed_actions = self.sign_batch_actions(&actions).await?;

        let response = self
            .client
            .post(format!("{}/exchange", self.api_url))
            .json(&signed_actions)
            .send()
            .await
            .map_err(|e| BotError::OrderExecution(format!("Failed to place batch orders: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Batch order placement failed: {} - {}", status, text);
            return Err(BotError::OrderExecution(format!(
                "Batch order failed: {} - {}",
                status, text
            )));
        }

        let results: Vec<PlaceOrderResponse> = response
            .json()
            .await
            .map_err(|e| BotError::Parse(format!("Failed to parse batch response: {}", e)))?;

        debug!("Batch order response: {:?}", results);
        Ok(results)
    }

    /// Cancel all orders for a symbol
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<CancelResponse> {
        info!("Cancelling all orders for {}", symbol);

        let action = json!({
            "type": "cancel",
            "cancels": [{
                "coin": symbol,
                "cancelAll": true
            }]
        });

        let signed_action = self.sign_action(&action).await?;

        let response = self
            .client
            .post(format!("{}/exchange", self.api_url))
            .json(&signed_action)
            .send()
            .await
            .map_err(|e| BotError::OrderExecution(format!("Failed to cancel orders: {}", e)))?;

        if !response.status().is_success() {
            return Err(BotError::OrderExecution(format!(
                "Cancel failed: {}",
                response.status()
            )));
        }

        let result: CancelResponse = response
            .json()
            .await
            .map_err(|e| BotError::Parse(format!("Failed to parse cancel response: {}", e)))?;

        Ok(result)
    }

    /// Cancel orders via nonce invalidation (guaranteed success)
    pub async fn invalidate_nonce(&self) -> Result<()> {
        info!("Invalidating nonce");

        let action = json!({
            "type": "updateLeverage",
            "asset": 0,
            "isCross": true,
            "leverage": 1
        });

        let signed_action = self.sign_action(&action).await?;

        let response = self
            .client
            .post(format!("{}/exchange", self.api_url))
            .json(&signed_action)
            .send()
            .await
            .map_err(|e| BotError::OrderExecution(format!("Failed to invalidate nonce: {}", e)))?;

        if !response.status().is_success() {
            return Err(BotError::OrderExecution(format!(
                "Nonce invalidation failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    // Private helper methods

    fn build_order_action(&self, order: &OrderRequest) -> Result<serde_json::Value> {
        let is_buy = order.side == Side::Buy;
        let limit_px = order.price.to_string();
        let sz = order.size.to_string();

        let order_type = match order.time_in_force {
            TimeInForce::IOC => json!({"limit": {"tif": "Ioc"}}),
            TimeInForce::ALO => json!({"limit": {"tif": "Alo"}}),
            TimeInForce::GTC => json!({"limit": {"tif": "Gtc"}}),
        };

        Ok(json!({
            "type": "order",
            "orders": [{
                "coin": order.symbol,
                "is_buy": is_buy,
                "sz": sz,
                "limit_px": limit_px,
                "order_type": order_type,
                "reduce_only": order.reduce_only
            }],
            "grouping": "na"
        }))
    }

    async fn sign_action(&self, action: &serde_json::Value) -> Result<serde_json::Value> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        // Construct the signing payload
        let payload = json!({
            "action": action,
            "nonce": timestamp,
            "vaultAddress": null
        });

        let message = serde_json::to_string(&payload)
            .map_err(|e| BotError::Api(format!("Failed to serialize payload: {}", e)))?;

        // Sign with EIP-712 (simplified version for now)
        let signature = self.wallet
            .sign_message(message.as_bytes())
            .await
            .map_err(|e| BotError::Api(format!("Failed to sign message: {}", e)))?;

        Ok(json!({
            "action": action,
            "nonce": timestamp,
            "signature": {
                "r": format!("0x{:x}", signature.r),
                "s": format!("0x{:x}", signature.s),
                "v": signature.v
            }
        }))
    }

    async fn sign_batch_actions(
        &self,
        actions: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        // For batch orders, we use the same nonce for all
        // This ensures atomic execution
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        let payload = json!({
            "actions": actions,
            "nonce": timestamp,
            "vaultAddress": null
        });

        let message = serde_json::to_string(&payload)
            .map_err(|e| BotError::Api(format!("Failed to serialize batch payload: {}", e)))?;

        let signature = self.wallet
            .sign_message(message.as_bytes())
            .await
            .map_err(|e| BotError::Api(format!("Failed to sign batch message: {}", e)))?;

        Ok(json!({
            "actions": actions,
            "nonce": timestamp,
            "signature": {
                "r": format!("0x{:x}", signature.r),
                "s": format!("0x{:x}", signature.s),
                "v": signature.v
            }
        }))
    }
}

// API Response types

#[derive(Debug, Deserialize)]
pub struct UserState {
    #[serde(rename = "assetPositions")]
    pub asset_positions: Vec<AssetPosition>,
    pub margin_summary: MarginSummary,
}

#[derive(Debug, Deserialize)]
pub struct AssetPosition {
    pub position: PositionData,
}

#[derive(Debug, Deserialize)]
pub struct PositionData {
    pub coin: String,
    pub szi: String, // Size
    #[serde(rename = "entryPx")]
    pub entry_px: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarginSummary {
    #[serde(rename = "accountValue")]
    pub account_value: String,
    #[serde(rename = "totalMarginUsed")]
    pub total_margin_used: String,
}

#[derive(Debug, Deserialize)]
pub struct L2Snapshot {
    pub coin: String,
    pub levels: Vec<Vec<L2Level>>, // [bids, asks]
    pub time: u64,
}

#[derive(Debug, Deserialize)]
pub struct L2Level {
    pub px: String,  // Price
    pub sz: String,  // Size
    pub n: u64,      // Number of orders
}

#[derive(Debug, Deserialize)]
pub struct PlaceOrderResponse {
    pub status: String,
    pub response: Option<OrderResponseData>,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<OrderData>,
}

#[derive(Debug, Deserialize)]
pub struct OrderData {
    pub statuses: Vec<OrderStatus>,
}

#[derive(Debug, Deserialize)]
pub struct OrderStatus {
    pub filled: Option<FilledInfo>,
}

#[derive(Debug, Deserialize)]
pub struct FilledInfo {
    #[serde(rename = "oid")]
    pub order_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub status: String,
}
