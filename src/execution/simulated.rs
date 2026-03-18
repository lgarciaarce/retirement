use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::error::Result;
use crate::types::order::{Fill, Order};

use super::OrderExecutor;

pub struct SimulatedExecutor {
    latency: Duration,
}

impl SimulatedExecutor {
    pub fn new() -> Self {
        Self {
            latency: Duration::from_millis(40),
        }
    }
}

impl Default for SimulatedExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrderExecutor for SimulatedExecutor {
    async fn submit(&self, order: Order, fill_tx: mpsc::Sender<Fill>) -> Result<()> {
        let latency = self.latency;
        let fill_price = order
            .request
            .price
            .unwrap_or(0.0);

        debug!(
            order_id = %order.id,
            latency_ms = latency.as_millis(),
            "Simulated executor: scheduling fill"
        );

        tokio::spawn(async move {
            tokio::time::sleep(latency).await;

            let fill = Fill {
                order_id: order.id,
                asset_id: order.request.asset_id.clone(),
                side: order.request.side,
                price: fill_price,
                size: order.request.size,
                filled_at: Instant::now(),
            };

            if let Err(e) = fill_tx.send(fill).await {
                warn!(error = %e, "Failed to send fill — channel closed");
            }
        });

        Ok(())
    }
}
