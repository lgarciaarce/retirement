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
        market: String,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
        timestamp: Option<String>,
    },
    PriceChange {
        asset_id: String,
        market: String,
        price: f64,
        size: f64,
        side: String,
        best_bid: f64,
        best_ask: f64,
        timestamp: Option<String>,
    },
    LastTrade {
        asset_id: String,
        market: String,
        price: f64,
        size: f64,
        side: String,
        timestamp: Option<String>,
    },
}

impl fmt::Display for OrderbookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderbookEvent::Snapshot { asset_id, bids, asks, .. } => {
                write!(f, "[Snapshot] {} bids={} asks={}", asset_id, bids.len(), asks.len())
            }
            OrderbookEvent::PriceChange { asset_id, price, best_bid, best_ask, side, .. } => {
                write!(f, "[PriceChange] {} price={:.4} bid={:.4} ask={:.4} side={}", asset_id, price, best_bid, best_ask, side)
            }
            OrderbookEvent::LastTrade { asset_id, price, size, side, .. } => {
                write!(f, "[LastTrade] {} price={:.4} size={:.4} side={}", asset_id, price, size, side)
            }
        }
    }
}
