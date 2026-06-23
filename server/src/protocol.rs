use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetric {
    pub id: String,
    pub name: String,
    pub default_unit: String,
    pub available_units: Vec<String>,
    pub r#static: bool,
    pub data_type: MetricDataType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MetricDataType {
    Float,
    Integer,
    Boolean,
    String,
    StringList,
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
#[serde(untagged)]
pub enum MetricNumber {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    StringList(Vec<String>),
}

impl From<f64> for MetricNumber {
    fn from(v: f64) -> Self {
        MetricNumber::Float(v)
    }
}

impl From<i64> for MetricNumber {
    fn from(v: i64) -> Self {
        MetricNumber::Integer(v)
    }
}

impl From<bool> for MetricNumber {
    fn from(v: bool) -> Self {
        MetricNumber::Boolean(v)
    }
}

impl From<String> for MetricNumber {
    fn from(v: String) -> Self {
        MetricNumber::String(v)
    }
}

impl From<Vec<String>> for MetricNumber {
    fn from(v: Vec<String>) -> Self {
        MetricNumber::StringList(v)
    }
}

impl MetricNumber {
    pub fn as_f64(&self) -> f64 {
        match self {
            MetricNumber::Float(v) => *v,
            MetricNumber::Integer(v) => *v as f64,
            MetricNumber::Boolean(v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            }
            MetricNumber::String(_) => 0.0,
            MetricNumber::StringList(_) => 0.0,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetricNumber::String(v) => Some(v.as_str()),
            _ => None,
        }
    }

    pub fn as_string_list(&self) -> Option<&[String]> {
        match self {
            MetricNumber::StringList(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub id: String,
    pub value: MetricNumber,
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
