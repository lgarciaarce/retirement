pub mod binance;
pub mod polymarket;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::Result;
use crate::types::{Market, OrderbookEvent, PriceTick};

#[async_trait]
pub trait PriceSource: Send + Sync {
    async fn subscribe(&self, sender: mpsc::Sender<PriceTick>) -> Result<()>;
}

#[async_trait]
pub trait OrderbookSource: Send + Sync {
    async fn subscribe(&self, sender: mpsc::Sender<OrderbookEvent>) -> Result<()>;
}

#[async_trait]
pub trait MarketClient: Send + Sync {
    async fn get_market_by_slug(&self, slug: &str) -> Result<Market>;
}
