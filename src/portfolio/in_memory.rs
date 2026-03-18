use std::collections::HashMap;

use tracing::debug;

use crate::types::order::{Fill, Order, OrderId, OrderSide, OrderStatus};

use super::{PortfolioManager, PortfolioReader};

pub struct InMemoryPortfolio {
    positions: HashMap<String, f64>,
    pending_orders: HashMap<OrderId, Order>,
}

impl InMemoryPortfolio {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            pending_orders: HashMap::new(),
        }
    }
}

impl Default for InMemoryPortfolio {
    fn default() -> Self {
        Self::new()
    }
}

impl PortfolioReader for InMemoryPortfolio {
    fn position(&self, asset_id: &str) -> f64 {
        self.positions.get(asset_id).copied().unwrap_or(0.0)
    }

    fn pending_order_count(&self, asset_id: &str) -> usize {
        self.pending_orders
            .values()
            .filter(|o| o.request.asset_id == asset_id)
            .count()
    }

    fn has_pending_orders(&self) -> bool {
        !self.pending_orders.is_empty()
    }
}

impl PortfolioManager for InMemoryPortfolio {
    fn record_pending_order(&mut self, order: &Order) {
        debug!(order = %order, "Recording pending order");
        self.pending_orders.insert(order.id, order.clone());
    }

    fn apply_fill(&mut self, fill: &Fill) {
        let delta = match fill.side {
            OrderSide::Buy => fill.size,
            OrderSide::Sell => -fill.size,
        };
        let pos = self.positions.entry(fill.asset_id.clone()).or_insert(0.0);
        *pos += delta;
        debug!(
            asset = %fill.asset_id,
            delta = delta,
            new_position = *pos,
            "Position updated"
        );

        if let Some(mut order) = self.pending_orders.remove(&fill.order_id) {
            order.status = OrderStatus::Filled;
        }
    }

    fn cancel_order(&mut self, order_id: OrderId) {
        if let Some(mut order) = self.pending_orders.remove(&order_id) {
            order.status = OrderStatus::Cancelled;
            debug!(order_id = %order_id, "Order cancelled");
        }
    }
}
