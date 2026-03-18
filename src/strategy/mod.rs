pub mod registry;
pub mod spread_logger;

use crate::portfolio::PortfolioReader;
use crate::types::order::OrderRequest;
use crate::types::orderbook::OrderbookManager;
use crate::types::PriceTick;

pub struct StrategyContext<'a> {
    pub tick: &'a PriceTick,
    pub orderbooks: &'a OrderbookManager,
    pub portfolio: &'a dyn PortfolioReader,
}

pub trait Strategy: Send {
    fn name(&self) -> &str;
    fn on_tick(&mut self, ctx: &StrategyContext) -> Vec<OrderRequest>;
}
