use async_trait::async_trait;
use tracing::{debug, info};

use crate::error::{AppError, Result};
use crate::sources::MarketClient;
use crate::types::Market;

use super::types::{CryptoPriceResponse, GammaMarketResponse};

pub const GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com";
const CRYPTO_PRICE_BASE: &str = "https://polymarket.com/api/crypto/crypto-price";

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

    /// Fetch the opening (strike) price for a round from the crypto-price API.
    ///
    /// Returns `Ok(Some(price))` if the round has started and the open price is
    /// available, `Ok(None)` if the round hasn't started yet.
    pub async fn get_crypto_price(
        &self,
        symbol: &str,
        event_start_time: &str,
        end_date: &str,
        variant: &str,
    ) -> Result<Option<f64>> {
        let url = format!(
            "{}?symbol={}&eventStartTime={}&endDate={}&variant={}",
            CRYPTO_PRICE_BASE, symbol, event_start_time, end_date, variant,
        );
        debug!(url = %url, "Fetching crypto price");

        let resp: CryptoPriceResponse = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(AppError::Http)?
            .json()
            .await?;

        debug!(response = ?resp, "Crypto price response");
        Ok(resp.open_price)
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
            event_start_time = %market.event_start_time,
            end_date = %market.end_date,
            "Fetched market"
        );

        Ok(market)
    }
}
