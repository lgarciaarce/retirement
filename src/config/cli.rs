use clap::Parser;

use super::settings::OperationMode;

/// Polymarket crypto updown trading bot
#[derive(Parser, Debug)]
#[command(name = "retirement", version, about)]
pub struct Cli {
    /// Operation mode
    #[arg(long, default_value = "simulate-live")]
    pub mode: OperationMode,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "debug")]
    pub log_level: String,

    /// Comma-separated token pairs (btc, eth, sol, doge)
    #[arg(long, default_value = "btc,eth,sol,doge", value_delimiter = ',')]
    pub pairs: Vec<String>,
}
