use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { metrics: Vec<MetricSubscription> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricSubscription {
    pub id: String,
    pub refresh_rate_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "catalog")]
    Catalog {
        metrics: Vec<super::hardware::MetricCatalog>,
    },
    #[serde(rename = "data")]
    Data {
        timestamp: i64,
        values: HashMap<String, f64>,
    },
}
