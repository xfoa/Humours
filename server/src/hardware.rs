use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricCatalog {
    pub id: String,
    pub name: String,
    pub unit: String,
}

pub fn discover_metrics() -> Vec<MetricCatalog> {
    let metrics = vec![
        MetricCatalog {
            id: "cpu.usage".to_string(),
            name: "CPU Usage".to_string(),
            unit: "%".to_string(),
        },
        MetricCatalog {
            id: "mem.used".to_string(),
            name: "Memory Used".to_string(),
            unit: "GB".to_string(),
        },
        MetricCatalog {
            id: "mem.total".to_string(),
            name: "Memory Total".to_string(),
            unit: "GB".to_string(),
        },
    ];
    tracing::debug!("discovered {} metrics: {:?}", metrics.len(), metrics);
    metrics
}

pub struct MetricCollector {
    system: System,
    cpu_refreshed: bool,
    memory_refreshed: bool,
}

impl MetricCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        tracing::debug!("MetricCollector initialized with {} cpus", system.cpus().len());
        Self {
            system,
            cpu_refreshed: false,
            memory_refreshed: false,
        }
    }

    pub fn begin_batch(&mut self) {
        self.cpu_refreshed = false;
        self.memory_refreshed = false;
    }

    pub fn get_value(&mut self, metric_id: &str) -> Option<f64> {
        tracing::debug!("get_value called for metric_id: {}", metric_id);
        match metric_id {
            "cpu.usage" => {
                if !self.cpu_refreshed {
                    self.system.refresh_cpu_usage();
                    self.cpu_refreshed = true;
                }
                self.system.cpus().first().map(|cpu| cpu.cpu_usage() as f64)
            }
            "mem.used" => {
                if !self.memory_refreshed {
                    self.system.refresh_memory();
                    self.memory_refreshed = true;
                }
                Some(self.system.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0)) // GB
            }
            "mem.total" => {
                if !self.memory_refreshed {
                    self.system.refresh_memory();
                    self.memory_refreshed = true;
                }
                Some(self.system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)) // GB
            }
            _ => None,
        }
    }
}
