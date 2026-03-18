pub mod fees;
pub mod in_memory;

use crate::types::order::{Fill, Order, OrderId};

/// Read-only portfolio view — strategies use this to check positions and pending orders.
pub trait PortfolioReader: Send + Sync {
    fn position(&self, asset_id: &str) -> f64;
    fn pending_order_count(&self, asset_id: &str) -> usize;
    fn has_pending_orders(&self) -> bool;
    fn balance(&self) -> f64;
}

/// Mutable portfolio manager — engine uses this to record orders and apply fills.
pub trait PortfolioManager: PortfolioReader {
    fn record_pending_order(&mut self, order: &Order);
    fn apply_fill(&mut self, fill: &Fill);
    fn cancel_order(&mut self, order_id: OrderId);
}
