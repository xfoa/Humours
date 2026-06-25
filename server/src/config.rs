use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_DIR: &str = "humours";
const DEFAULT_CONFIG_FILE: &str = "config.toml";

const DEFAULT_CONFIG_CONTENT: &str = "\
bind_address = \"0.0.0.0\"
port = 8443
auth_token = \"changeme\"
default_refresh_rate_ms = 500
poll_interval_ms = 50
broadcast_buffer = 256
";

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    #[serde(default = "default_broadcast_buffer")]
    pub broadcast_buffer: usize,
}

fn default_refresh() -> u64 {
    500
}
fn default_poll() -> u64 {
    50
}
fn default_broadcast_buffer() -> usize {
    256
}

#[derive(Debug, Parser)]
#[command(name = "humours-server", about = "Real-time hardware metrics streaming server")]
pub struct Cli {
    /// Path to config file. If omitted, uses ~/.humours/config.toml
    /// (created with defaults if it doesn't exist).
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }

    pub fn default_config_path() -> Option<PathBuf> {
        let base = dirs::config_dir()?;
        Some(base.join(DEFAULT_CONFIG_DIR).join(DEFAULT_CONFIG_FILE))
    }

    pub fn resolve_or_create(cli_config: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
        if let Some(p) = cli_config {
            return Ok(p.clone());
        }

        let path = Self::default_config_path()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;

        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, DEFAULT_CONFIG_CONTENT)?;
            eprintln!("created default config at {}", path.display());
        }

        Ok(path)
    }
}
