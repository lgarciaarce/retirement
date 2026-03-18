use async_trait::async_trait;
use tracing::{debug, info};

use crate::error::{AppError, Result};
use crate::sources::MarketClient;
use crate::types::Market;

use super::types::GammaMarketResponse;

const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";

pub struct PolymarketRestClient {
    client: reqwest::Client,
}

impl Default for PolymarketRestClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl PolymarketRestClient {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MarketClient for PolymarketRestClient {
    async fn get_market_by_slug(&self, slug: &str) -> Result<Market> {
        let url = format!("{}/markets/slug/{}", GAMMA_API_BASE, slug);
        debug!(url = %url, "Fetching market from Gamma API");

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| {
                if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    AppError::MarketNotFound(slug.to_string())
                } else {
                    AppError::Http(e)
                }
            })?;

        let gamma_market: GammaMarketResponse = response.json().await?;
        let market = gamma_market.into_market()?;

        info!(
            slug = %market.slug,
            question = %market.question,
            active = market.active,
            outcomes = ?market.outcomes,
            prices = ?market.outcome_prices,
            "Fetched market"
        );

        Ok(market)
    }
}
