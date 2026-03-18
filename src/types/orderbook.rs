use std::fmt;

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub enum OrderbookEvent {
    Snapshot {
        asset_id: String,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
        timestamp: Option<String>,
    },
    PriceChange {
        asset_id: String,
        price: f64,
    },
    LastTrade {
        asset_id: String,
        price: f64,
    },
    BestBidAsk {
        asset_id: String,
        best_bid: f64,
        best_ask: f64,
    },
}

impl fmt::Display for OrderbookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderbookEvent::Snapshot { asset_id, bids, asks, .. } => {
                write!(f, "[Snapshot] {} bids={} asks={}", asset_id, bids.len(), asks.len())
            }
            OrderbookEvent::PriceChange { asset_id, price } => {
                write!(f, "[PriceChange] {} price={:.4}", asset_id, price)
            }
            OrderbookEvent::LastTrade { asset_id, price } => {
                write!(f, "[LastTrade] {} price={:.4}", asset_id, price)
            }
            OrderbookEvent::BestBidAsk { asset_id, best_bid, best_ask } => {
                write!(f, "[BBA] {} bid={:.4} ask={:.4}", asset_id, best_bid, best_ask)
            }
        }
    }
}
