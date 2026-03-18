use std::collections::HashMap;
use std::fmt;

use super::market::AssetInfo;

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

/// Maintained orderbook state for a single asset.
/// Bids sorted descending, asks sorted ascending by price.
#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub asset_id: String,
    pub info: AssetInfo,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

impl OrderbookSnapshot {
    pub fn new(asset_id: String, info: AssetInfo) -> Self {
        Self {
            asset_id,
            info,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    pub fn replace(&mut self, bids: Vec<PriceLevel>, asks: Vec<PriceLevel>) {
        self.bids = bids;
        self.asks = asks;
        self.sort();
    }

    /// Apply a single level update. size=0 removes the level.
    pub fn apply_level(&mut self, price: f64, size: f64, side: &str) {
        let levels = if side == "BUY" {
            &mut self.bids
        } else {
            &mut self.asks
        };

        if let Some(pos) = levels.iter().position(|l| (l.price - price).abs() < 1e-9) {
            if size == 0.0 {
                levels.remove(pos);
            } else {
                levels[pos].size = size;
            }
        } else if size > 0.0 {
            levels.push(PriceLevel { price, size });
            if side == "BUY" {
                self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            } else {
                self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            }
        }
    }

    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }

    fn sort(&mut self) {
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    }
}

impl fmt::Display for OrderbookSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bid_str = self.best_bid().map(|l| format!("{:.2}", l.price)).unwrap_or("-".into());
        let ask_str = self.best_ask().map(|l| format!("{:.2}", l.price)).unwrap_or("-".into());

        writeln!(f, "  Orderbook {} ({}) | best_bid={} best_ask={}", &self.asset_id[..8.min(self.asset_id.len())], self.info, bid_str, ask_str)?;

        let depth = 5.min(self.bids.len().max(self.asks.len()));
        writeln!(f, "  {:>12} {:>12} | {:>12} {:>12}", "BID_SZ", "BID_PX", "ASK_PX", "ASK_SZ")?;
        for i in 0..depth {
            let bid = self.bids.get(i);
            let ask = self.asks.get(i);
            let bid_sz = bid.map(|l| format!("{:.2}", l.size)).unwrap_or_default();
            let bid_px = bid.map(|l| format!("{:.2}", l.price)).unwrap_or_default();
            let ask_px = ask.map(|l| format!("{:.2}", l.price)).unwrap_or_default();
            let ask_sz = ask.map(|l| format!("{:.2}", l.size)).unwrap_or_default();
            writeln!(f, "  {:>12} {:>12} | {:>12} {:>12}", bid_sz, bid_px, ask_px, ask_sz)?;
        }
        if self.bids.len() > depth || self.asks.len() > depth {
            writeln!(f, "  ... +{} bids, +{} asks", self.bids.len().saturating_sub(depth), self.asks.len().saturating_sub(depth))?;
        }
        Ok(())
    }
}

/// Manages orderbook snapshots for all subscribed assets.
#[derive(Default)]
pub struct OrderbookManager {
    books: HashMap<String, OrderbookSnapshot>,
    /// asset_id → AssetInfo, registered during market discovery
    assets: HashMap<String, AssetInfo>,
    /// market_id → list of asset_ids belonging to that market
    market_assets: HashMap<String, Vec<String>>,
}

impl OrderbookManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an asset discovered from a market.
    pub fn register_asset(&mut self, market_id: &str, asset_id: &str, info: AssetInfo) {
        self.assets.insert(asset_id.to_string(), info);
        self.market_assets
            .entry(market_id.to_string())
            .or_default()
            .push(asset_id.to_string());
    }

    /// Get all asset_ids for a given market.
    pub fn assets_for_market(&self, market_id: &str) -> &[String] {
        self.market_assets.get(market_id).map_or(&[], |v| v.as_slice())
    }

    /// Get the AssetInfo for a given asset_id.
    pub fn asset_info(&self, asset_id: &str) -> Option<&AssetInfo> {
        self.assets.get(asset_id)
    }

    /// Apply an event and return the updated snapshot for debug logging.
    pub fn apply(&mut self, event: &OrderbookEvent) -> Option<&OrderbookSnapshot> {
        match event {
            OrderbookEvent::Snapshot { asset_id, bids, asks, .. } => {
                let info = self.resolve_info(asset_id);
                let snap = self.books.entry(asset_id.clone()).or_insert_with(|| OrderbookSnapshot::new(asset_id.clone(), info));
                snap.replace(bids.clone(), asks.clone());
                Some(snap)
            }
            OrderbookEvent::PriceChange { asset_id, price, size, side, .. } => {
                let info = self.resolve_info(asset_id);
                let snap = self.books.entry(asset_id.clone()).or_insert_with(|| OrderbookSnapshot::new(asset_id.clone(), info));
                snap.apply_level(*price, *size, side);
                Some(snap)
            }
            OrderbookEvent::LastTrade { .. } => None,
        }
    }

    pub fn snapshot(&self, asset_id: &str) -> Option<&OrderbookSnapshot> {
        self.books.get(asset_id)
    }

    pub fn all_snapshots(&self) -> impl Iterator<Item = (&String, &OrderbookSnapshot)> {
        self.books.iter()
    }

    pub fn market_ids(&self) -> impl Iterator<Item = &String> {
        self.market_assets.keys()
    }

    /// Clear all orderbook state — used when rolling to a new market epoch.
    pub fn clear(&mut self) {
        self.books.clear();
        self.assets.clear();
        self.market_assets.clear();
    }

    fn resolve_info(&self, asset_id: &str) -> AssetInfo {
        self.assets.get(asset_id).copied().unwrap_or(AssetInfo {
            crypto: crate::types::CryptoPair::Btc,
            outcome: crate::types::Outcome::Up,
        })
    }
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
