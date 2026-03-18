use super::arb_logger::ArbLoggerStrategy;
use super::spread_logger::SpreadLoggerStrategy;
use super::{Strategy, StrategyContext};
use crate::types::order::OrderRequest;

pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    pub fn on_tick(&mut self, ctx: &StrategyContext) -> Vec<OrderRequest> {
        let mut all_orders = Vec::new();
        for strategy in &mut self.strategies {
            all_orders.extend(strategy.on_tick(ctx));
        }
        all_orders
    }

    pub fn on_orderbook_update(&mut self, ctx: &StrategyContext) -> Vec<OrderRequest> {
        let mut all_orders = Vec::new();
        for strategy in &mut self.strategies {
            all_orders.extend(strategy.on_orderbook_update(ctx));
        }
        all_orders
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_default_strategies() -> StrategyRegistry {
    let mut r = StrategyRegistry::new();
    r.register(Box::new(SpreadLoggerStrategy::new()));
    r.register(Box::new(ArbLoggerStrategy::new()));
    // Add new strategies here
    r
}
