use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetric {
    pub id: String,
    pub name: String,
    pub unit: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub metrics: Vec<SubscribeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub timestamp: u64,
    pub values: HashMap<String, f64>,
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
    pub fn new(timestamp: u64, values: HashMap<String, f64>) -> Self {
        Self { msg_type: "data".to_string(), timestamp, values }
    }
}

impl ErrorMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self { msg_type: "error".to_string(), message: message.into() }
    }
}
