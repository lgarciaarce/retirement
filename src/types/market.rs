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
