use std::collections::HashMap;

use tracing::debug;

use crate::types::order::{Fill, Order, OrderId, OrderSide, OrderStatus};

use super::fees::polymarket_fee_pct;
use super::{PortfolioManager, PortfolioReader};

pub struct InMemoryPortfolio {
    positions: HashMap<String, f64>,
    pending_orders: HashMap<OrderId, Order>,
    balance: f64,
}

impl InMemoryPortfolio {
    pub fn new(initial_balance: f64) -> Self {
        Self {
            positions: HashMap::new(),
            pending_orders: HashMap::new(),
            balance: initial_balance,
        }
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

    fn balance(&self) -> f64 {
        self.balance
    }
}

impl PortfolioManager for InMemoryPortfolio {
    fn record_pending_order(&mut self, order: &Order) {
        debug!(order = %order, "Recording pending order");
        self.pending_orders.insert(order.id, order.clone());
    }

    fn apply_fill(&mut self, fill: &Fill) {
        let fee_pct = polymarket_fee_pct(fill.price);
        let notional = fill.price * fill.size;
        let fee = notional * fee_pct;

        let (delta, balance_change) = match fill.side {
            OrderSide::Buy => (fill.size, -(notional + fee)),
            OrderSide::Sell => (-fill.size, notional - fee),
        };

        self.balance += balance_change;
        let pos = self.positions.entry(fill.asset_id.clone()).or_insert(0.0);
        *pos += delta;

        debug!(
            asset = %fill.asset_id,
            delta = delta,
            fee_pct = format!("{:.4}%", fee_pct * 100.0),
            fee = format!("{:.4}", fee),
            balance_change = format!("{:.4}", balance_change),
            new_position = *pos,
            balance = format!("{:.2}", self.balance),
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
