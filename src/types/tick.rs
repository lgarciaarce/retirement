use std::fmt;

#[derive(Debug, Clone)]
pub enum TickSource {
    Binance,
    Polymarket,
}

impl fmt::Display for TickSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TickSource::Binance => write!(f, "Binance"),
            TickSource::Polymarket => write!(f, "Polymarket"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PriceTick {
    pub source: TickSource,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp_ms: u64,
    pub is_buyer_maker: bool,
}

impl fmt::Display for PriceTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} price={:.6} qty={:.6} ts={} maker={}",
            self.source, self.symbol, self.price, self.quantity, self.timestamp_ms, self.is_buyer_maker
        )
    }
}
