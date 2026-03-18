use clap::Parser;
use tracing::error;
use tracing_subscriber::EnvFilter;

use retirement::config::cli::Cli;
use retirement::config::settings::AppConfig;
use retirement::engine::Engine;

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cli = Cli::parse();

    // Init tracing with env-filter; RUST_LOG overrides --log-level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cli.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let config = match AppConfig::from_cli(&cli) {
        Ok(c) => c,
        Err(e) => {
            error!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let engine = Engine::new(config);
    if let Err(e) = engine.run().await {
        error!("Engine error: {}", e);
        std::process::exit(1);
    }
}
