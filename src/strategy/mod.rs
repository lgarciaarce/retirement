pub mod arb_logger;
pub mod registry;

use crate::portfolio::PortfolioReader;
use crate::types::order::OrderRequest;
use crate::types::orderbook::OrderbookManager;
use crate::types::PriceTick;

pub struct StrategyContext<'a> {
    pub tick: Option<&'a PriceTick>,
    pub orderbooks: &'a OrderbookManager,
    pub portfolio: &'a dyn PortfolioReader,
}

pub trait Strategy: Send {
    fn name(&self) -> &str;
    fn on_tick(&mut self, _ctx: &StrategyContext) -> Vec<OrderRequest> { Vec::new() }
    fn on_orderbook_update(&mut self, _ctx: &StrategyContext) -> Vec<OrderRequest> { Vec::new() }
}
