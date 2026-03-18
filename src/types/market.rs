use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CryptoPair {
    Btc,
    Eth,
    Sol,
    Xrp,
    Doge,
}

impl CryptoPair {
    pub fn binance_symbol(self) -> &'static str {
        match self {
            CryptoPair::Btc => "btcusdt",
            CryptoPair::Eth => "ethusdt",
            CryptoPair::Sol => "solusdt",
            CryptoPair::Xrp => "xrpusdt",
            CryptoPair::Doge => "dogeusdt",
        }
    }

    pub fn slug_prefix(self) -> &'static str {
        match self {
            CryptoPair::Btc => "btc-updown",
            CryptoPair::Eth => "eth-updown",
            CryptoPair::Sol => "sol-updown",
            CryptoPair::Xrp => "xrp-updown",
            CryptoPair::Doge => "doge-updown",
        }
    }
}

impl fmt::Display for CryptoPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoPair::Btc => write!(f, "BTC"),
            CryptoPair::Eth => write!(f, "ETH"),
            CryptoPair::Sol => write!(f, "SOL"),
            CryptoPair::Xrp => write!(f, "XRP"),
            CryptoPair::Doge => write!(f, "DOGE"),
        }
    }
}

impl FromStr for CryptoPair {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "btc" => Ok(CryptoPair::Btc),
            "eth" => Ok(CryptoPair::Eth),
            "sol" => Ok(CryptoPair::Sol),
            "xrp" => Ok(CryptoPair::Xrp),
            "doge" => Ok(CryptoPair::Doge),
            _ => Err(format!("Unknown pair '{}'. Supported: btc, eth, sol, xrp, doge", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Up,
    Down,
}

impl Outcome {
    pub fn from_outcome_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "up" => Some(Outcome::Up),
            "down" => Some(Outcome::Down),
            _ => None,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Up => write!(f, "Up"),
            Outcome::Down => write!(f, "Down"),
        }
    }
}

/// Identifies a specific Polymarket asset (one side of a market).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetInfo {
    pub crypto: CryptoPair,
    pub outcome: Outcome,
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.crypto, self.outcome)
    }
}

#[derive(Debug, Clone)]
pub struct Market {
    pub id: String,
    pub slug: String,
    pub question: String,
    pub condition_id: String,
    pub outcomes: Vec<String>,
    pub outcome_prices: Vec<f64>,
    pub clob_token_ids: Vec<String>,
    pub active: bool,
    pub closed: bool,
}

impl Market {
    /// Extract (asset_id, AssetInfo) pairs from this market.
    /// Determines Up/Down from the outcomes array.
    pub fn extract_assets(&self, crypto: CryptoPair) -> Vec<(String, AssetInfo)> {
        self.clob_token_ids
            .iter()
            .zip(self.outcomes.iter())
            .filter_map(|(token_id, outcome_str)| {
                let outcome = Outcome::from_outcome_str(outcome_str)?;
                Some((token_id.clone(), AssetInfo { crypto, outcome }))
            })
            .collect()
    }
}
