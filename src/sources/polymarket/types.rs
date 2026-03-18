use serde::Deserialize;

use crate::error::Result;
use crate::types::market::Market;
use crate::types::orderbook::{OrderbookEvent, PriceLevel};

/// Response from Gamma API GET /markets/slug/{slug}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaMarketResponse {
    pub id: Option<String>,
    pub slug: Option<String>,
    pub question: Option<String>,
    pub condition_id: Option<String>,
    /// JSON-encoded string: "[\"Up\",\"Down\"]"
    pub outcomes: Option<String>,
    /// JSON-encoded string: "[\"0.55\",\"0.45\"]"
    pub outcome_prices: Option<String>,
    /// JSON-encoded string: "[\"token_id_1\",\"token_id_2\"]"
    pub clob_token_ids: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
}

impl GammaMarketResponse {
    pub fn into_market(self) -> Result<Market> {
        let outcomes: Vec<String> = match &self.outcomes {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => vec![],
        };

        let outcome_prices: Vec<f64> = match &self.outcome_prices {
            Some(s) => {
                let strings: Vec<String> = serde_json::from_str(s).unwrap_or_default();
                strings
                    .iter()
                    .filter_map(|p| p.parse::<f64>().ok())
                    .collect()
            }
            None => vec![],
        };

        let clob_token_ids: Vec<String> = match &self.clob_token_ids {
            Some(s) => serde_json::from_str(s).unwrap_or_default(),
            None => vec![],
        };

        Ok(Market {
            id: self.id.unwrap_or_default(),
            slug: self.slug.unwrap_or_default(),
            question: self.question.unwrap_or_default(),
            condition_id: self.condition_id.unwrap_or_default(),
            outcomes,
            outcome_prices,
            clob_token_ids,
            active: self.active.unwrap_or(false),
            closed: self.closed.unwrap_or(false),
        })
    }
}

/// Polymarket WS events — tagged by event type
#[derive(Debug, Deserialize)]
#[serde(tag = "event_type")]
pub enum PolymarketWsEvent {
    #[serde(rename = "book")]
    Book {
        asset_id: String,
        bids: Vec<PolymarketLevel>,
        asks: Vec<PolymarketLevel>,
        timestamp: Option<String>,
    },
    #[serde(rename = "price_change")]
    PriceChange {
        asset_id: String,
        price: String,
    },
    #[serde(rename = "last_trade_price")]
    LastTradePrice {
        asset_id: String,
        price: String,
    },
    #[serde(rename = "best_bid_ask")]
    BestBidAsk {
        asset_id: String,
        best_bid: String,
        best_ask: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct PolymarketLevel {
    pub price: String,
    pub size: String,
}

impl PolymarketWsEvent {
    pub fn into_orderbook_event(self) -> Option<OrderbookEvent> {
        match self {
            PolymarketWsEvent::Book { asset_id, bids, asks, timestamp } => {
                let bids = bids
                    .into_iter()
                    .filter_map(|l| {
                        Some(PriceLevel {
                            price: l.price.parse().ok()?,
                            size: l.size.parse().ok()?,
                        })
                    })
                    .collect();
                let asks = asks
                    .into_iter()
                    .filter_map(|l| {
                        Some(PriceLevel {
                            price: l.price.parse().ok()?,
                            size: l.size.parse().ok()?,
                        })
                    })
                    .collect();
                Some(OrderbookEvent::Snapshot { asset_id, bids, asks, timestamp })
            }
            PolymarketWsEvent::PriceChange { asset_id, price } => {
                Some(OrderbookEvent::PriceChange {
                    asset_id,
                    price: price.parse().ok()?,
                })
            }
            PolymarketWsEvent::LastTradePrice { asset_id, price } => {
                Some(OrderbookEvent::LastTrade {
                    asset_id,
                    price: price.parse().ok()?,
                })
            }
            PolymarketWsEvent::BestBidAsk { asset_id, best_bid, best_ask } => {
                Some(OrderbookEvent::BestBidAsk {
                    asset_id,
                    best_bid: best_bid.parse().ok()?,
                    best_ask: best_ask.parse().ok()?,
                })
            }
        }
    }
}
