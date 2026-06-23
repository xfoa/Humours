use crate::protocol::{CatalogMetric, MetricDataType, MetricNumber};
use std::sync::{Arc, Mutex};
use sysinfo::{CpuRefreshKind, ProcessRefreshKind, RefreshKind, System};

pub const POLL_QUANTUM_MS: u64 = 50;

const MEMORY_UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "KiB", "MiB", "GiB", "TiB"];

pub fn build_catalog() -> Vec<CatalogMetric> {
    let mem_units: Vec<String> = MEMORY_UNITS.iter().map(|s| s.to_string()).collect();
    vec![
        CatalogMetric { id: "cpu.usage".to_string(), name: "CPU Usage".to_string(), default_unit: "%".to_string(), available_units: vec!["%".to_string()], r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "cpu.cores".to_string(), name: "CPU Core Count".to_string(), default_unit: "cores".to_string(), available_units: vec!["cores".to_string()], r#static: true, data_type: MetricDataType::Integer },
        CatalogMetric { id: "mem.used".to_string(), name: "Memory Used".to_string(), default_unit: "GB".to_string(), available_units: mem_units.clone(), r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "mem.total".to_string(), name: "Memory Total".to_string(), default_unit: "GB".to_string(), available_units: mem_units.clone(), r#static: true, data_type: MetricDataType::Integer },
        CatalogMetric { id: "mem.usage".to_string(), name: "Memory Usage".to_string(), default_unit: "%".to_string(), available_units: vec!["%".to_string()], r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "swap.used".to_string(), name: "Swap Used".to_string(), default_unit: "GB".to_string(), available_units: mem_units.clone(), r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "swap.total".to_string(), name: "Swap Total".to_string(), default_unit: "GB".to_string(), available_units: mem_units, r#static: true, data_type: MetricDataType::Integer },
        CatalogMetric { id: "sys.uptime".to_string(), name: "System Uptime".to_string(), default_unit: "s".to_string(), available_units: vec!["s".to_string(), "ms".to_string(), "m".to_string(), "h".to_string()], r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "sys.load1".to_string(), name: "Load Average (1m)".to_string(), default_unit: "".to_string(), available_units: vec!["".to_string()], r#static: false, data_type: MetricDataType::Float },
        CatalogMetric { id: "proc.count".to_string(), name: "Process Count".to_string(), default_unit: "procs".to_string(), available_units: vec!["procs".to_string()], r#static: false, data_type: MetricDataType::Integer },
    ]
}

pub fn round_to_quantum(ms: u64) -> u64 {
    if ms == 0 {
        return POLL_QUANTUM_MS;
    }
    let q = POLL_QUANTUM_MS;
    let rounded = ms.div_ceil(q) * q;
    rounded.max(q)
}

pub fn convert_bytes(bytes: f64, unit: &str) -> Option<f64> {
    let base = match unit {
        "B" => Some(1.0_f64),
        "KB" => Some(1000.0),
        "MB" => Some(1_000_000.0),
        "GB" => Some(1_000_000_000.0),
        "TB" => Some(1_000_000_000_000.0),
        "KiB" => Some(1024.0),
        "MiB" => Some(1024.0 * 1024.0),
        "GiB" => Some(1024.0 * 1024.0 * 1024.0),
        "TiB" => Some(1024.0 * 1024.0 * 1024.0 * 1024.0),
        _ => None,
    }?;
    Some(bytes / base)
}

pub fn convert_seconds(seconds: f64, unit: &str) -> Option<f64> {
    match unit {
        "s" => Some(seconds),
        "ms" => Some(seconds * 1000.0),
        "m" => Some(seconds / 60.0),
        "h" => Some(seconds / 3600.0),
        _ => None,
    }
}

pub struct Collector {
    sys: Arc<Mutex<System>>,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(sysinfo::MemoryRefreshKind::new().with_ram().with_swap())
                .with_processes(ProcessRefreshKind::new()),
        );
        Self { sys: Arc::new(Mutex::new(sys)) }
    }

    fn refresh(&self) {
        if let Ok(mut sys) = self.sys.lock() {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
        }
    }

    fn read_raw(&self, metric_id: &str) -> Option<f64> {
        let sys = self.sys.lock().ok()?;
        match metric_id {
            "cpu.usage" => {
                let cpus = sys.cpus();
                if cpus.is_empty() {
                    return None;
                }
                let sum: f64 = cpus.iter().map(|c| c.cpu_usage() as f64).sum();
                Some(sum / cpus.len() as f64)
            }
            "cpu.cores" => Some(sys.cpus().len() as f64),
            "mem.used" => Some(sys.used_memory() as f64),
            "mem.total" => Some(sys.total_memory() as f64),
            "mem.usage" => {
                let total = sys.total_memory() as f64;
                if total == 0.0 {
                    None
                } else {
                    Some((sys.used_memory() as f64 / total) * 100.0)
                }
            }
            "swap.used" => Some(sys.used_swap() as f64),
            "swap.total" => Some(sys.total_swap() as f64),
            "sys.uptime" => Some(sysinfo::System::uptime() as f64),
            "sys.load1" => Some(System::load_average().one),
            "proc.count" => Some(sys.processes().len() as f64),
            _ => None,
        }
    }

    pub fn sample(&self, metric_id: &str, unit: &str) -> Option<f64> {
        self.refresh();
        let raw = self.read_raw(metric_id)?;
        self::convert_value(raw, metric_id, unit)
    }

    pub fn sample_many(
        &self,
        requests: &[(String, String, MetricDataType)],
    ) -> Vec<crate::protocol::MetricValue> {
        if requests.is_empty() {
            return Vec::new();
        }
        self.refresh();
        let mut out = Vec::with_capacity(requests.len());
        for (id, unit, dtype) in requests {
            let raw = match self.read_raw(id) {
                Some(r) => r,
                None => continue,
            };
            let converted = match self::convert_value(raw, id, unit) {
                Some(v) => v,
                None => continue,
            };
            let value = match dtype {
                MetricDataType::Integer => MetricNumber::Integer(converted as i64),
                MetricDataType::Boolean => MetricNumber::Boolean(converted != 0.0),
                MetricDataType::Float => MetricNumber::Float(converted),
            };
            out.push(crate::protocol::MetricValue {
                id: id.clone(),
                value,
                unit: unit.clone(),
            });
        }
        out
    }
}

fn convert_value(raw: f64, metric_id: &str, unit: &str) -> Option<f64> {
    if metric_id.starts_with("mem.") || metric_id.starts_with("swap.") {
        convert_bytes(raw, unit)
    } else if metric_id == "sys.uptime" {
        convert_seconds(raw, unit)
    } else {
        Some(raw)
    }
}
