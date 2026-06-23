use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetric {
    pub id: String,
    pub name: String,
    pub default_unit: String,
    pub available_units: Vec<String>,
    pub r#static: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub metrics: Vec<CatalogMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeEntry {
    pub id: String,
    #[serde(default)]
    pub refresh_rate_ms: Option<u64>,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub metrics: Vec<SubscribeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub id: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub timestamp: u64,
    pub metrics: Vec<MetricValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

impl CatalogMessage {
    pub fn new(metrics: Vec<CatalogMetric>) -> Self {
        Self { msg_type: "catalog".to_string(), metrics }
    }
}

impl DataMessage {
    pub fn new(timestamp: u64, metrics: Vec<MetricValue>) -> Self {
        Self { msg_type: "data".to_string(), timestamp, metrics }
    }
}

impl ErrorMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self { msg_type: "error".to_string(), message: message.into() }
    }
}
