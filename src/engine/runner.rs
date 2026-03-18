use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::config::settings::{AppConfig, OperationMode};
use crate::error::Result;
use crate::execution::simulated::SimulatedExecutor;
use crate::execution::OrderExecutor;
use crate::portfolio::in_memory::InMemoryPortfolio;
use crate::portfolio::PortfolioManager;
use crate::sources::binance::BinanceWsClient;
use crate::sources::polymarket::{PolymarketRestClient, PolymarketWsClient};
use crate::sources::{MarketClient, OrderbookSource, PriceSource};
use crate::strategy::registry::build_default_strategies;
use crate::strategy::StrategyContext;
use crate::types::order::{Fill, Order, OrderId, OrderStatus};
use crate::types::{CryptoPair, OrderbookEvent, OrderbookManager, PriceTick};

const MARKET_INTERVAL_SECS: u64 = 300;
const STRATEGY_BUFFER_SECS: u64 = 15;
const INITIAL_BALANCE: f64 = 10_000.0;

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
            let slug = build_epoch_slug(crypto, MARKET_INTERVAL_SECS);
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
        let (fill_tx, mut fill_rx) = mpsc::channel::<Fill>(256);

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
        // NOTE: fill_tx must NOT be dropped — it's cloned into executor tasks.
        // Dropping it here would close fill_rx prematurely.

        // Initialize strategy, execution, and portfolio modules
        let mut strategies = build_default_strategies();
        let executor = SimulatedExecutor::new();
        let mut portfolio = InMemoryPortfolio::new(INITIAL_BALANCE);
        let mut next_order_id: u64 = 1;
        let mut was_active = false;

        info!(
            balance = INITIAL_BALANCE,
            buffer_secs = STRATEGY_BUFFER_SECS,
            "Engine running. Press Ctrl+C to stop."
        );

        // Main event loop
        loop {
            let mut orders = Vec::new();
            let active = is_strategy_window_active(MARKET_INTERVAL_SECS, STRATEGY_BUFFER_SECS);

            if active != was_active {
                if active {
                    info!("Strategy window opened");
                } else {
                    info!("Strategy window closed (market boundary buffer)");
                }
                was_active = active;
            }

            tokio::select! {
                Some(tick) = tick_rx.recv() => {
                    trace!("Tick: {}", tick);

                    if active {
                        let ctx = StrategyContext {
                            tick: Some(&tick),
                            orderbooks: &ob_manager,
                            portfolio: &portfolio,
                        };
                        orders = strategies.on_tick(&ctx);
                    }
                }
                Some(ob) = ob_rx.recv() => {
                    trace!("OB event: {}", ob);
                    if let Some(snap) = ob_manager.apply(&ob) {
                        trace!("Orderbook updated:\n{}", snap);
                    }

                    if active {
                        let ctx = StrategyContext {
                            tick: None,
                            orderbooks: &ob_manager,
                            portfolio: &portfolio,
                        };
                        orders = strategies.on_orderbook_update(&ctx);
                    }
                }
                Some(fill) = fill_rx.recv() => {
                    debug!(fill = %fill, "Fill received");
                    portfolio.apply_fill(&fill);
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl+C, shutting down");
                    break;
                }
            }

            // Process any orders emitted by strategies
            for req in orders {
                let order_id = OrderId(next_order_id);
                next_order_id += 1;

                let order = Order {
                    id: order_id,
                    request: req,
                    status: OrderStatus::Pending,
                    created_at: Instant::now(),
                };

                debug!(order = %order, "New order from strategy");
                portfolio.record_pending_order(&order);

                if let Err(e) = executor.submit(order, fill_tx.clone()).await {
                    warn!(error = %e, "Failed to submit order to executor");
                }
            }
        }

        Ok(())
    }
}

/// Check if strategies should be active based on the current position within
/// the market epoch. Strategies are paused during the first and last
/// `buffer_secs` seconds of each `interval_secs` epoch.
fn is_strategy_window_active(interval_secs: u64, buffer_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let epoch_start = (now / interval_secs) * interval_secs;
    let elapsed = now - epoch_start;
    elapsed >= buffer_secs && elapsed <= interval_secs - buffer_secs
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
