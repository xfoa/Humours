use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Serialize, Clone)]
pub struct MetricCatalog {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub refresh_rate_ms: u64,
}

pub fn discover_metrics() -> Vec<MetricCatalog> {
    vec![
        MetricCatalog {
            id: "cpu.usage".to_string(),
            name: "CPU Usage".to_string(),
            unit: "%".to_string(),
            refresh_rate_ms: 1000,
        },
        MetricCatalog {
            id: "mem.used".to_string(),
            name: "Memory Used".to_string(),
            unit: "GB".to_string(),
            refresh_rate_ms: 1000,
        },
        MetricCatalog {
            id: "mem.total".to_string(),
            name: "Memory Total".to_string(),
            unit: "GB".to_string(),
            refresh_rate_ms: 5000,
        },
    ]
}

pub struct MetricCollector {
    system: System,
}

impl MetricCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    pub fn get_value(&self, metric_id: &str) -> Option<f64> {
        match metric_id {
            "cpu.usage" => {
                self.system.cpus().first().map(|cpu| cpu.cpu_usage() as f64)
            }
            "mem.used" => {
                Some(self.system.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0)) // GB
            }
            "mem.total" => {
                Some(self.system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)) // GB
            }
            _ => None,
        }
    }
}
