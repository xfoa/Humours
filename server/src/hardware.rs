use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricCatalog {
    pub id: String,
    pub name: String,
    pub unit: String,
}

pub fn discover_metrics() -> Vec<MetricCatalog> {
    let mut metrics = vec![
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

    if let Ok(hw) = hardware_query::HardwareInfo::query() {
        let cpu = hw.cpu();
        for metric in &mut metrics {
            if metric.id == "cpu.usage" {
                metric.name = format!("CPU Usage ({} {})", cpu.vendor(), cpu.model_name());
            }
        }
        let mem = hw.memory();
        let total_gb = mem.total_gb();
        for metric in &mut metrics {
            if metric.id == "mem.total" {
                metric.name = format!("Memory Total ({:.0} GB)", total_gb);
            } else if metric.id == "mem.used" {
                metric.name = format!("Memory Used / {:.0} GB", total_gb);
            }
        }
    }

    tracing::debug!("discovered {} metrics: {:?}", metrics.len(), metrics);
    metrics
}

pub struct MetricCollector {
    last_cpu: Option<hardware_query::CPUInfo>,
    last_mem: Option<hardware_query::MemoryInfo>,
    cpu_refreshed: bool,
    memory_refreshed: bool,
}

impl MetricCollector {
    pub fn new() -> Self {
        tracing::debug!("MetricCollector initialized");
        Self {
            last_cpu: None,
            last_mem: None,
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
                    self.last_cpu = hardware_query::CPUInfo::query().ok();
                    self.cpu_refreshed = true;
                }
                self.last_cpu.as_ref()?.core_usage().first().map(|&u| u as f64)
            }
            "mem.used" => {
                if !self.memory_refreshed {
                    self.last_mem = hardware_query::MemoryInfo::query().ok();
                    self.memory_refreshed = true;
                }
                Some(self.last_mem.as_ref()?.used_gb())
            }
            "mem.total" => {
                if !self.memory_refreshed {
                    self.last_mem = hardware_query::MemoryInfo::query().ok();
                    self.memory_refreshed = true;
                }
                Some(self.last_mem.as_ref()?.total_gb())
            }
            _ => None,
        }
    }
}
