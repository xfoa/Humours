use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind_address: String,
    pub port: u16,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub auth_token: String,
    #[serde(default = "default_refresh")]
    pub default_refresh_rate_ms: u64,
    #[serde(default = "default_poll")]
    pub poll_interval_ms: u64,
}

fn default_refresh() -> u64 { 500 }
fn default_poll() -> u64 { 50 }

#[derive(Debug, Parser)]
#[command(name = "humours-server", about = "Real-time hardware metrics streaming server")]
pub struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }
}
