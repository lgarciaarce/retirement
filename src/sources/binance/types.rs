use serde::Deserialize;

use crate::types::{PriceTick, TickSource};

/// Single trade event from Binance WS.
/// Field names match the Binance API single-char keys.
#[derive(Debug, Deserialize)]
pub struct BinanceTradeEvent {
    /// Event type (always "trade")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time
    #[serde(rename = "E")]
    pub event_time: u64,

    /// Symbol (e.g. "BTCUSDT")
    #[serde(rename = "s")]
    pub symbol: String,

    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: u64,

    /// Price
    #[serde(rename = "p")]
    pub price: String,

    /// Quantity
    #[serde(rename = "q")]
    pub quantity: String,

    /// Trade time
    #[serde(rename = "T")]
    pub trade_time: u64,

    /// Is the buyer the market maker?
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

impl BinanceTradeEvent {
    pub fn into_tick(self) -> Option<PriceTick> {
        let price = self.price.parse::<f64>().ok()?;
        let quantity = self.quantity.parse::<f64>().ok()?;
        Some(PriceTick {
            source: TickSource::Binance,
            symbol: self.symbol.to_lowercase(),
            price,
            quantity,
            timestamp_ms: self.trade_time,
            is_buyer_maker: self.is_buyer_maker,
        })
    }
}

/// Combined stream wrapper: Binance sends `{ "stream": "btcusdt@trade", "data": {...} }`
#[derive(Debug, Deserialize)]
pub struct BinanceCombinedStream {
    pub stream: String,
    pub data: BinanceTradeEvent,
}
