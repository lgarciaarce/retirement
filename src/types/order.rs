use std::fmt;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OID-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Market => write!(f, "MARKET"),
            OrderType::Limit => write!(f, "LIMIT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub asset_id: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub size: f64,
}

impl fmt::Display for OrderRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let price_str = self
            .price
            .map(|p| format!("{:.4}", p))
            .unwrap_or_else(|| "MKT".to_string());
        write!(
            f,
            "{} {} {} @ {} sz={:.4}",
            self.side, self.order_type, self.asset_id, price_str, self.size
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Filled,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub request: OrderRequest,
    pub status: OrderStatus,
    pub created_at: Instant,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.id, self.request)
    }
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub order_id: OrderId,
    pub asset_id: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub filled_at: Instant,
}

impl fmt::Display for Fill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Fill {} {} {} @ {:.4} sz={:.4}",
            self.order_id, self.side, self.asset_id, self.price, self.size
        )
    }
}
