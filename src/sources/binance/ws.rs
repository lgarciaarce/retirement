use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info, trace, warn};

use crate::error::Result;
use crate::sources::PriceSource;
use crate::types::PriceTick;

use super::types::BinanceCombinedStream;

const BINANCE_WS_BASE: &str = "wss://stream.binance.com:9443/stream?streams=";
const RECONNECT_DELAY_MS: u64 = 3000;

pub struct BinanceWsClient {
    symbols: Vec<String>,
}

impl BinanceWsClient {
    pub fn new(symbols: Vec<String>) -> Self {
        Self { symbols }
    }

    fn build_url(&self) -> String {
        let streams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect();
        format!("{}{}", BINANCE_WS_BASE, streams.join("/"))
    }
}

#[async_trait]
impl PriceSource for BinanceWsClient {
    async fn subscribe(&self, sender: mpsc::Sender<PriceTick>) -> Result<()> {
        let url = self.build_url();

        loop {
            info!(url = %url, "Connecting to Binance WebSocket");

            match connect_async(&url).await {
                Ok((ws_stream, _response)) => {
                    info!("Binance WebSocket connected");
                    let (_write, mut read) = ws_stream.split();

                    loop {
                        match read.next().await {
                            Some(Ok(msg)) => {
                                if msg.is_text() {
                                    let text = msg.into_text().unwrap_or_default();
                                    match serde_json::from_str::<BinanceCombinedStream>(&text) {
                                        Ok(combined) => {
                                            if let Some(tick) = combined.data.into_tick() {
                                                trace!("{}", tick);
                                                if sender.send(tick).await.is_err() {
                                                    info!("Price channel closed, stopping Binance WS");
                                                    return Ok(());
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(error = %e, "Failed to deserialize Binance message");
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                error!(error = %e, "Binance WebSocket read error");
                                break;
                            }
                            None => {
                                warn!("Binance WebSocket stream ended");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to connect to Binance WebSocket");
                }
            }

            warn!(delay_ms = RECONNECT_DELAY_MS, "Reconnecting to Binance WebSocket");
            tokio::time::sleep(tokio::time::Duration::from_millis(RECONNECT_DELAY_MS)).await;
        }
    }
}
