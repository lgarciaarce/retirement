use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::config::settings::{AppConfig, OperationMode};
use crate::error::Result;
use crate::sources::binance::BinanceWsClient;
use crate::sources::polymarket::{PolymarketRestClient, PolymarketWsClient};
use crate::sources::{MarketClient, OrderbookSource, PriceSource};
use crate::types::{CryptoPair, OrderbookEvent, OrderbookManager, PriceTick};

pub struct Engine {
    config: AppConfig,
}

impl Engine {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub async fn run(self) -> Result<()> {
        info!(mode = %self.config.mode, "Starting engine");

        if let OperationMode::SimulatePersisted = self.config.mode {
            warn!("SimulatePersisted mode is not yet implemented");
            return Ok(());
        }

        // Discover markets for each pair
        let rest_client = PolymarketRestClient::new();
        let mut all_asset_ids: Vec<String> = Vec::new();
        let mut ob_manager = OrderbookManager::new();

        for &crypto in &self.config.pairs {
            let slug = build_epoch_slug(crypto, 300); // 5m markets
            info!(pair = %crypto, slug = %slug, "Looking up market");

            match rest_client.get_market_by_slug(&slug).await {
                Ok(market) => {
                    info!(
                        pair = %crypto,
                        question = %market.question,
                        active = market.active,
                        tokens = ?market.clob_token_ids,
                        "Discovered market"
                    );
                    for (asset_id, asset_info) in market.extract_assets(crypto) {
                        info!(asset_id = %asset_id, info = %asset_info, "Registered asset");
                        ob_manager.register_asset(&market.id, &asset_id, asset_info);
                        all_asset_ids.push(asset_id);
                    }
                }
                Err(e) => {
                    warn!(pair = %crypto, slug = %slug, error = %e, "Failed to fetch market, continuing without it");
                }
            }
        }

        // Create channels
        let (tick_tx, mut tick_rx) = mpsc::channel::<PriceTick>(1024);
        let (ob_tx, mut ob_rx) = mpsc::channel::<OrderbookEvent>(1024);

        // Spawn Binance WS
        let binance_symbols = self.config.binance_symbols();
        let binance_client = BinanceWsClient::new(binance_symbols);
        let tick_tx_clone = tick_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = binance_client.subscribe(tick_tx_clone).await {
                error!(error = %e, "Binance WS task failed");
            }
        });

        // Spawn Polymarket WS
        if !all_asset_ids.is_empty() {
            let poly_ws = PolymarketWsClient::new(all_asset_ids);
            let ob_tx_clone = ob_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = poly_ws.subscribe(ob_tx_clone).await {
                    error!(error = %e, "Polymarket WS task failed");
                }
            });
        } else {
            info!("No Polymarket asset IDs discovered, skipping WS subscription");
        }

        // Drop our copies so channels close when producers stop
        drop(tick_tx);
        drop(ob_tx);

        info!("Engine running. Press Ctrl+C to stop.");

        // Main event loop
        loop {
            tokio::select! {
                Some(tick) = tick_rx.recv() => {
                    debug!("Tick: {}", tick);
                }
                Some(ob) = ob_rx.recv() => {
                    trace!("OB event: {}", ob);
                    if let Some(snap) = ob_manager.apply(&ob) {
                        debug!("Orderbook updated:\n{}", snap);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl+C, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Build the epoch slug for a given pair and interval.
fn build_epoch_slug(crypto: CryptoPair, interval_secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let epoch = (now / interval_secs) * interval_secs;

    let interval_label = match interval_secs {
        300 => "5m",
        900 => "15m",
        _ => "5m",
    };

    format!("{}-{}-{}", crypto.slug_prefix(), interval_label, epoch)
}
