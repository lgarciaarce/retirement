use std::collections::HashMap;
use std::time::Instant;

use tracing::debug;

use crate::types::order::{OrderRequest, OrderSide, OrderType};

use super::{Strategy, StrategyContext};

const MIN_SPREAD: f64 = 0.05;
const MAX_ASK: f64 = 0.45;
const COOLDOWN_SECS: u64 = 5;
const ORDER_SIZE: f64 = 10.0;

pub struct SpreadLoggerStrategy {
    last_order_time: HashMap<String, Instant>,
}

impl SpreadLoggerStrategy {
    pub fn new() -> Self {
        Self {
            last_order_time: HashMap::new(),
        }
    }

    fn cooldown_elapsed(&self, asset_id: &str) -> bool {
        match self.last_order_time.get(asset_id) {
            Some(t) => t.elapsed().as_secs() >= COOLDOWN_SECS,
            None => true,
        }
    }
}

impl Default for SpreadLoggerStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for SpreadLoggerStrategy {
    fn name(&self) -> &str {
        "SpreadLogger"
    }

    fn on_tick(&mut self, ctx: &StrategyContext) -> Vec<OrderRequest> {
        let mut orders = Vec::new();

        for (asset_id, snap) in ctx.orderbooks.all_snapshots() {
            let (best_bid, best_ask) = match (snap.best_bid(), snap.best_ask()) {
                (Some(b), Some(a)) => (b.price, a.price),
                _ => continue,
            };

            let spread = best_ask - best_bid;
            if spread < MIN_SPREAD || best_ask > MAX_ASK {
                continue;
            }

            debug!(
                strategy = "SpreadLogger",
                asset = %asset_id,
                spread = spread,
                best_bid = best_bid,
                best_ask = best_ask,
                "Trade signal: wide spread with low ask"
            );

            if self.cooldown_elapsed(asset_id)
                && ctx.portfolio.pending_order_count(asset_id) == 0
            {
                orders.push(OrderRequest {
                    asset_id: asset_id.clone(),
                    side: OrderSide::Buy,
                    order_type: OrderType::Limit,
                    price: Some(best_ask),
                    size: ORDER_SIZE,
                });
                self.last_order_time.insert(asset_id.clone(), Instant::now());
            }
        }

        orders
    }
}
