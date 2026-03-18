use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Channel send error: {0}")]
    Channel(String),

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
