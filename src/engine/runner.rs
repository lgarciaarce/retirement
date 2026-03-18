use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
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

        let rest_client = PolymarketRestClient::new();
        let mut ob_manager = OrderbookManager::new();

        // Initial market discovery
        let asset_ids = discover_markets(
            &rest_client,
            &self.config.pairs,
            &mut ob_manager,
            MARKET_INTERVAL_SECS,
        )
        .await;

        // Create channels
        let (tick_tx, mut tick_rx) = mpsc::channel::<PriceTick>(1024);
        let (ob_tx, mut ob_rx) = mpsc::channel::<OrderbookEvent>(1024);
        let (fill_tx, mut fill_rx) = mpsc::channel::<Fill>(256);

        // Spawn Binance WS (lives across all epochs)
        let binance_symbols = self.config.binance_symbols();
        let binance_client = BinanceWsClient::new(binance_symbols);
        let tick_tx_clone = tick_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = binance_client.subscribe(tick_tx_clone).await {
                error!(error = %e, "Binance WS task failed");
            }
        });

        // Spawn initial Polymarket WS
        let mut poly_ws_handle = spawn_poly_ws(asset_ids, &ob_tx);

        // Drop tick_tx — Binance WS holds its clone.
        // ob_tx is kept alive — cloned for each new Polymarket WS task on epoch rolls.
        drop(tick_tx);

        // Initialize strategy, execution, and portfolio modules
        let mut current_epoch = current_epoch_secs(MARKET_INTERVAL_SECS);
        let mut strategies = build_default_strategies();
        let executor = SimulatedExecutor::new();
        let mut portfolio = InMemoryPortfolio::new(INITIAL_BALANCE);
        let mut next_order_id: u64 = 1;
        let mut was_active = false;

        info!(
            balance = INITIAL_BALANCE,
            buffer_secs = STRATEGY_BUFFER_SECS,
            epoch = current_epoch,
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

            let epoch_deadline = next_epoch_deadline(MARKET_INTERVAL_SECS);

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
                _ = tokio::time::sleep_until(epoch_deadline) => {
                    let new_epoch = current_epoch_secs(MARKET_INTERVAL_SECS);
                    if new_epoch != current_epoch {
                        current_epoch = new_epoch;
                        info!(epoch = current_epoch, "Market epoch rolled — re-discovering markets");

                        // Abort old Polymarket WS task
                        if let Some(handle) = poly_ws_handle.take() {
                            handle.abort();
                        }

                        // Clear stale orderbook state and drain buffered events
                        ob_manager.clear();
                        while ob_rx.try_recv().is_ok() {}

                        // Discover new markets
                        let asset_ids = discover_markets(
                            &rest_client,
                            &self.config.pairs,
                            &mut ob_manager,
                            MARKET_INTERVAL_SECS,
                        )
                        .await;

                        // Spawn new Polymarket WS
                        poly_ws_handle = spawn_poly_ws(asset_ids, &ob_tx);
                    }
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

/// Discover markets for each crypto pair and register assets in the orderbook manager.
async fn discover_markets(
    rest_client: &PolymarketRestClient,
    pairs: &[CryptoPair],
    ob_manager: &mut OrderbookManager,
    interval_secs: u64,
) -> Vec<String> {
    let mut asset_ids = Vec::new();

    for &crypto in pairs {
        let slug = build_epoch_slug(crypto, interval_secs);
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
                    asset_ids.push(asset_id);
                }
            }
            Err(e) => {
                warn!(pair = %crypto, slug = %slug, error = %e, "Failed to fetch market, continuing without it");
            }
        }
    }

    asset_ids
}

/// Spawn a Polymarket WS task for the given asset IDs.
fn spawn_poly_ws(
    asset_ids: Vec<String>,
    ob_tx: &mpsc::Sender<OrderbookEvent>,
) -> Option<JoinHandle<()>> {
    if asset_ids.is_empty() {
        info!("No Polymarket asset IDs discovered, skipping WS subscription");
        return None;
    }

    let poly_ws = PolymarketWsClient::new(asset_ids);
    let ob_tx_clone = ob_tx.clone();
    Some(tokio::spawn(async move {
        if let Err(e) = poly_ws.subscribe(ob_tx_clone).await {
            error!(error = %e, "Polymarket WS task failed");
        }
    }))
}

/// Get the current epoch start timestamp.
fn current_epoch_secs(interval_secs: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (now / interval_secs) * interval_secs
}

/// Compute a tokio deadline for the next epoch boundary.
fn next_epoch_deadline(interval_secs: u64) -> tokio::time::Instant {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let epoch_start = (now / interval_secs) * interval_secs;
    let next_epoch = epoch_start + interval_secs;
    let secs_until = next_epoch - now;
    tokio::time::Instant::now() + Duration::from_secs(secs_until)
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
