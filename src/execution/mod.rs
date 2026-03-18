pub mod simulated;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::Result;
use crate::types::order::{Fill, Order};

#[async_trait]
pub trait OrderExecutor: Send + Sync {
    async fn submit(&self, order: Order, fill_tx: mpsc::Sender<Fill>) -> Result<()>;
}
