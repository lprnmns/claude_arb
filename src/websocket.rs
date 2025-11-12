use crate::errors::{BotError, Result};
use crate::orderbook::Orderbook;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// WebSocket manager for Hyperliquid
pub struct WebSocketManager {
    ws_url: String,
}

impl WebSocketManager {
    pub fn new(ws_url: String) -> Self {
        Self { ws_url }
    }

    /// Subscribe to orderbook updates for a symbol
    pub async fn subscribe_orderbook(
        &self,
        symbol: String,
        orderbook: Arc<Orderbook>,
    ) -> Result<()> {
        let url = self.ws_url.clone();
        let coin = symbol.clone();

        tokio::spawn(async move {
            loop {
                match Self::run_orderbook_stream(&url, &coin, orderbook.clone()).await {
                    Ok(_) => {
                        warn!("WebSocket connection closed for {}, reconnecting...", coin);
                    }
                    Err(e) => {
                        error!("WebSocket error for {}: {}, reconnecting in 5s...", coin, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

    async fn run_orderbook_stream(
        url: &str,
        symbol: &str,
        orderbook: Arc<Orderbook>,
    ) -> Result<()> {
        info!("Connecting to WebSocket for orderbook: {}", orderbook.symbol);

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| BotError::WebSocket(format!("Connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to L2 book
        let subscribe_msg = serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "l2Book",
                "coin": symbol
            }
        });

        info!(
            "📡 Subscribing to Hyperliquid L2Book - Orderbook: {}, Coin Symbol: {}",
            orderbook.symbol, symbol
        );

        write
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await
            .map_err(|e| BotError::WebSocket(format!("Subscribe failed: {}", e)))?;

        info!(
            "✅ WebSocket connected - Orderbook: {}, Symbol: {}",
            orderbook.symbol, symbol
        );

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = Self::handle_message(&text, &orderbook).await {
                        warn!("Failed to handle message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    write
                        .send(Message::Pong(data))
                        .await
                        .map_err(|e| BotError::WebSocket(format!("Pong failed: {}", e)))?;
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed for {}", symbol);
                    break;
                }
                Err(e) => {
                    return Err(BotError::WebSocket(format!("Stream error: {}", e)));
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(text: &str, orderbook: &Arc<Orderbook>) -> Result<()> {
        // Log raw message for debugging (truncated to avoid spam)
        debug!(
            "📨 Raw WebSocket message for {} (first 300 chars): {}",
            orderbook.symbol,
            &text[..text.len().min(300)]
        );

        // Try to parse the message, but don't fail if format is unexpected
        let msg: WsMessage = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                // Log parsing errors but continue (Hyperliquid may send various message types)
                debug!(
                    "Failed to parse WebSocket message (non-critical): {} - Message: {}",
                    e,
                    &text[..text.len().min(200)]
                ); // First 200 chars
                return Ok(()); // Skip this message, don't fail
            }
        };

        debug!(
            "📬 Parsed message for {} - Channel: {}, has data: {}",
            orderbook.symbol,
            msg.channel,
            msg.data.is_some()
        );

        match msg.channel.as_str() {
            "l2Book" => {
                if let Some(data) = msg.data {
                    debug!(
                        "📊 L2Book data for {} - Coin field: {}, Levels count: {}",
                        orderbook.symbol,
                        data.coin,
                        data.levels.len()
                    );
                    Self::update_orderbook(orderbook, data).await?;
                } else {
                    debug!("⚠️  L2Book message has no data for {}", orderbook.symbol);
                }
            }
            "subscriptionResponse" => {
                debug!("✅ Subscription confirmed for {}: {}", orderbook.symbol, text);
            }
            _ => {
                debug!("❓ Unknown channel for {}: {}", orderbook.symbol, msg.channel);
            }
        }

        Ok(())
    }

    async fn update_orderbook(orderbook: &Arc<Orderbook>, data: L2BookData) -> Result<()> {
        use std::collections::BTreeMap;

        let bids_count = data.levels.get(0).map(|v| v.len()).unwrap_or(0);
        let asks_count = data.levels.get(1).map(|v| v.len()).unwrap_or(0);

        debug!(
            "🔄 Starting orderbook update - {} | Coin: {} | Bids: {}, Asks: {}",
            orderbook.symbol, data.coin, bids_count, asks_count
        );

        // Build new bids snapshot
        let mut new_bids = BTreeMap::new();
        let mut best_bid_price = None;
        let mut best_bid_size = None;

        for (i, level) in data.levels.get(0).unwrap_or(&vec![]).iter().enumerate() {
            let price = level
                .px
                .parse::<Decimal>()
                .map_err(|e| BotError::Parse(format!("Invalid price: {}", e)))?;
            let size = level
                .sz
                .parse::<Decimal>()
                .map_err(|e| BotError::Parse(format!("Invalid size: {}", e)))?;

            if i == 0 {
                best_bid_price = Some(price);
                best_bid_size = Some(size);
            }

            new_bids.insert(price, size);
        }

        // Build new asks snapshot
        let mut new_asks = BTreeMap::new();
        let mut best_ask_price = None;
        let mut best_ask_size = None;

        for (i, level) in data.levels.get(1).unwrap_or(&vec![]).iter().enumerate() {
            let price = level
                .px
                .parse::<Decimal>()
                .map_err(|e| BotError::Parse(format!("Invalid price: {}", e)))?;
            let size = level
                .sz
                .parse::<Decimal>()
                .map_err(|e| BotError::Parse(format!("Invalid size: {}", e)))?;

            if i == 0 {
                best_ask_price = Some(price);
                best_ask_size = Some(size);
            }

            new_asks.insert(price, size);
        }

        // Replace entire orderbook with snapshot (clears old levels)
        orderbook.update_snapshot(new_bids, new_asks);

        // Log best prices
        if let (Some(bid_px), Some(bid_sz)) = (best_bid_price, best_bid_size) {
            debug!("  📈 Best BID: ${} x {}", bid_px, bid_sz);
        }
        if let (Some(ask_px), Some(ask_sz)) = (best_ask_price, best_ask_size) {
            debug!("  📉 Best ASK: ${} x {}", ask_px, ask_sz);
        }

        debug!(
            "✅ Orderbook updated - {} | Coin: {} | Bids: {}, Asks: {}",
            orderbook.symbol, data.coin, bids_count, asks_count
        );

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct WsMessage {
    channel: String,
    data: Option<L2BookData>,
}

#[derive(Debug, Deserialize)]
struct L2BookData {
    coin: String,
    levels: Vec<Vec<L2LevelWs>>,
    time: u64,
}

#[derive(Debug, Deserialize)]
struct L2LevelWs {
    px: String,
    sz: String,
    n: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_message_parsing() {
        let json = r#"{"channel":"l2Book","data":{"coin":"BTC","levels":[[{"px":"50000.0","sz":"1.5","n":1}],[{"px":"50100.0","sz":"2.0","n":1}]],"time":1234567890}}"#;

        let msg: WsMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.channel, "l2Book");
        assert!(msg.data.is_some());
    }
}
