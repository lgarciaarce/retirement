use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use crate::error::Result;
use crate::sources::OrderbookSource;
use crate::types::OrderbookEvent;

use super::types::PolymarketWsEvent;

const POLYMARKET_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
const RECONNECT_DELAY_MS: u64 = 3000;
const PING_INTERVAL_SECS: u64 = 10;

pub struct PolymarketWsClient {
    asset_ids: Vec<String>,
}

impl PolymarketWsClient {
    pub fn new(asset_ids: Vec<String>) -> Self {
        Self { asset_ids }
    }
}

#[async_trait]
impl OrderbookSource for PolymarketWsClient {
    async fn subscribe(&self, sender: mpsc::Sender<OrderbookEvent>) -> Result<()> {
        if self.asset_ids.is_empty() {
            info!("No asset IDs to subscribe to, skipping Polymarket WS");
            return Ok(());
        }

        loop {
            info!(url = POLYMARKET_WS_URL, "Connecting to Polymarket WebSocket");

            match connect_async(POLYMARKET_WS_URL).await {
                Ok((ws_stream, _)) => {
                    info!("Polymarket WebSocket connected");
                    let (mut write, mut read) = ws_stream.split();

                    // Subscribe to all asset IDs
                    for asset_id in &self.asset_ids {
                        let sub_msg = json!({
                            "type": "market",
                            "assets_ids": [asset_id],
                        });
                        if let Err(e) = write.send(Message::Text(sub_msg.to_string().into())).await {
                            warn!(error = %e, "Failed to send subscription message");
                            break;
                        }
                        debug!(asset_id = %asset_id, "Subscribed to Polymarket asset");
                    }

                    // Spawn ping task
                    let ping_handle = tokio::spawn(async move {
                        let mut interval =
                            tokio::time::interval(tokio::time::Duration::from_secs(PING_INTERVAL_SECS));
                        loop {
                            interval.tick().await;
                            if write.send(Message::Ping(vec![].into())).await.is_err() {
                                break;
                            }
                        }
                    });

                    // Read loop
                    loop {
                        match read.next().await {
                            Some(Ok(msg)) => {
                                if msg.is_text() {
                                    let text = msg.into_text().unwrap_or_default();
                                    match serde_json::from_str::<PolymarketWsEvent>(&text) {
                                        Ok(event) => {
                                            if let Some(ob_event) = event.into_orderbook_event() {
                                                debug!("{}", ob_event);
                                                if sender.send(ob_event).await.is_err() {
                                                    info!("Orderbook channel closed, stopping Polymarket WS");
                                                    ping_handle.abort();
                                                    return Ok(());
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            debug!(error = %e, raw = %text, "Ignoring non-event Polymarket message");
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                warn!(error = %e, "Polymarket WebSocket read error");
                                break;
                            }
                            None => {
                                warn!("Polymarket WebSocket stream ended");
                                break;
                            }
                        }
                    }

                    ping_handle.abort();
                }
                Err(e) => {
                    error!(error = %e, "Failed to connect to Polymarket WebSocket");
                }
            }

            warn!(delay_ms = RECONNECT_DELAY_MS, "Reconnecting to Polymarket WebSocket");
            tokio::time::sleep(tokio::time::Duration::from_millis(RECONNECT_DELAY_MS)).await;
        }
    }
}
