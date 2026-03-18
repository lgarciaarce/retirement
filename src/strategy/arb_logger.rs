use tracing::debug;

use crate::types::order::OrderRequest;
use crate::types::Outcome;

use super::{Strategy, StrategyContext};

pub struct ArbLoggerStrategy;

impl ArbLoggerStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArbLoggerStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for ArbLoggerStrategy {
    fn name(&self) -> &str {
        "ArbLogger"
    }

    fn on_orderbook_update(&mut self, ctx: &StrategyContext) -> Vec<OrderRequest> {
        for market_id in ctx.orderbooks.market_ids() {
            let asset_ids = ctx.orderbooks.assets_for_market(market_id);

            let mut up_snap = None;
            let mut down_snap = None;

            for asset_id in asset_ids {
                if let Some(snap) = ctx.orderbooks.snapshot(asset_id) {
                    match snap.info.outcome {
                        Outcome::Up => up_snap = Some(snap),
                        Outcome::Down => down_snap = Some(snap),
                    }
                }
            }

            let (up, down) = match (up_snap, down_snap) {
                (Some(u), Some(d)) => (u, d),
                _ => continue,
            };
            debug!("HERE");
            // Buy arb: cost to buy both sides < 1.0
            if let (Some(up_ask), Some(down_ask)) = (up.best_ask(), down.best_ask()) {
                let buy_combo = up_ask.price + down_ask.price;
                if buy_combo < 1.0 {
                    debug!(
                        strategy = "ArbLogger",
                        market = %market_id,
                        up_ask = format!("{:.4}", up_ask.price),
                        down_ask = format!("{:.4}", down_ask.price),
                        combo = format!("{:.4}", buy_combo),
                        "Buy arb signal: up_ask + down_ask < 1.0"
                    );
                }
            }

            // Sell arb: proceeds from selling both sides > 1.0
            if let (Some(up_bid), Some(down_bid)) = (up.best_bid(), down.best_bid()) {
                let sell_combo = up_bid.price + down_bid.price;
                if sell_combo > 1.0 {
                    debug!(
                        strategy = "ArbLogger",
                        market = %market_id,
                        up_bid = format!("{:.4}", up_bid.price),
                        down_bid = format!("{:.4}", down_bid.price),
                        combo = format!("{:.4}", sell_combo),
                        "Sell arb signal: up_bid + down_bid > 1.0"
                    );
                }
            }
        }

        Vec::new()
    }
}
