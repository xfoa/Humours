use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub bind_address: String,
    pub port: u16,
    pub tls_cert: String,
    pub tls_key: String,
    pub auth_token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8443,
            tls_cert: "cert.pem".to_string(),
            tls_key: "key.pem".to_string(),
            auth_token: "dev-token".to_string(),
        }
    }
}

pub fn load() -> anyhow::Result<Config> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "config.yaml".to_string());
    if Path::new(&path).exists() {
        let content = std::fs::read_to_string(&path)?;
        let cfg: Config = serde_yaml::from_str(&content)?;
        Ok(cfg)
    } else {
        tracing::warn!("config file not found, using defaults");
        Ok(Config::default())
    }
}
