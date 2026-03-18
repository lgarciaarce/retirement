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

/// Polymarket WS events — tagged by event type.
/// Unknown fields are silently ignored (deny_unknown_fields is NOT set)
/// since the API sends extra fields like `hash`, `tick_size`, `last_trade_price` on book events.
#[derive(Debug, Deserialize)]
#[serde(tag = "event_type")]
pub enum PolymarketWsEvent {
    #[serde(rename = "book")]
    Book {
        asset_id: String,
        market: String,
        bids: Vec<PolymarketLevel>,
        asks: Vec<PolymarketLevel>,
        timestamp: Option<String>,
    },
    #[serde(rename = "price_change")]
    PriceChange {
        market: String,
        price_changes: Vec<PolymarketPriceChange>,
        timestamp: Option<String>,
    },
    #[serde(rename = "last_trade_price")]
    LastTradePrice {
        asset_id: String,
        market: String,
        price: String,
        size: String,
        side: String,
        timestamp: Option<String>,
    },
    #[serde(rename = "tick_size_change")]
    TickSizeChange {
        asset_id: String,
        market: String,
        old_tick_size: String,
        new_tick_size: String,
        timestamp: Option<String>,
    },
}

/// Parse a WS text message, which may be a single event object or an array of events.
pub fn parse_ws_message(text: &str) -> Vec<PolymarketWsEvent> {
    // Try as array first (most common from the WS)
    if let Ok(events) = serde_json::from_str::<Vec<PolymarketWsEvent>>(text) {
        return events;
    }
    // Fall back to single object
    if let Ok(event) = serde_json::from_str::<PolymarketWsEvent>(text) {
        return vec![event];
    }
    vec![]
}

#[derive(Debug, Deserialize)]
pub struct PolymarketLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Deserialize)]
pub struct PolymarketPriceChange {
    pub asset_id: String,
    pub price: String,
    pub size: String,
    pub side: String,
    #[serde(default)]
    pub best_bid: Option<String>,
    #[serde(default)]
    pub best_ask: Option<String>,
}

impl PolymarketWsEvent {
    pub fn into_orderbook_events(self) -> Vec<OrderbookEvent> {
        match self {
            PolymarketWsEvent::Book { asset_id, market, bids, asks, timestamp } => {
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
                vec![OrderbookEvent::Snapshot { asset_id, market, bids, asks, timestamp }]
            }
            PolymarketWsEvent::PriceChange { market, price_changes, timestamp } => {
                price_changes
                    .into_iter()
                    .filter_map(|pc| {
                        Some(OrderbookEvent::PriceChange {
                            asset_id: pc.asset_id,
                            market: market.clone(),
                            price: pc.price.parse().ok()?,
                            size: pc.size.parse().ok()?,
                            side: pc.side,
                            best_bid: pc.best_bid.and_then(|s| s.parse().ok()),
                            best_ask: pc.best_ask.and_then(|s| s.parse().ok()),
                            timestamp: timestamp.clone(),
                        })
                    })
                    .collect()
            }
            PolymarketWsEvent::LastTradePrice { asset_id, market, price, size, side, timestamp } => {
                let Some(price) = price.parse().ok() else { return vec![] };
                let Some(size) = size.parse().ok() else { return vec![] };
                vec![OrderbookEvent::LastTrade { asset_id, market, price, size, side, timestamp }]
            }
            PolymarketWsEvent::TickSizeChange { .. } => {
                vec![]
            }
        }
    }
}
