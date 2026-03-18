use std::fmt;
use std::str::FromStr;

use crate::types::CryptoPair;

#[derive(Debug, Clone)]
pub enum OperationMode {
    Live,
    SimulateLive,
    SimulatePersisted,
}

impl fmt::Display for OperationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationMode::Live => write!(f, "live"),
            OperationMode::SimulateLive => write!(f, "simulate-live"),
            OperationMode::SimulatePersisted => write!(f, "simulate-persisted"),
        }
    }
}

impl FromStr for OperationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "live" => Ok(OperationMode::Live),
            "simulate-live" => Ok(OperationMode::SimulateLive),
            "simulate-persisted" => Ok(OperationMode::SimulatePersisted),
            _ => Err(format!(
                "Invalid mode '{}'. Expected: live, simulate-live, simulate-persisted",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: OperationMode,
    pub log_level: String,
    pub pairs: Vec<CryptoPair>,
}

impl AppConfig {
    pub fn from_cli(cli: &super::cli::Cli) -> Result<Self, String> {
        let pairs: Result<Vec<_>, _> = cli
            .pairs
            .iter()
            .map(|p| p.parse::<CryptoPair>())
            .collect();

        Ok(AppConfig {
            mode: cli.mode.clone(),
            log_level: cli.log_level.clone(),
            pairs: pairs?,
        })
    }

    pub fn binance_symbols(&self) -> Vec<String> {
        self.pairs
            .iter()
            .map(|p| p.binance_symbol().to_string())
            .collect()
    }
}
