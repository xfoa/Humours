use crate::protocol::CatalogMetric;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{CpuRefreshKind, ProcessRefreshKind, RefreshKind, System};

pub const POLL_QUANTUM_MS: u64 = 50;

pub fn build_catalog() -> Vec<CatalogMetric> {
    vec![
        CatalogMetric { id: "cpu.usage".to_string(), name: "CPU Usage".to_string(), unit: "%".to_string(), r#static: false },
        CatalogMetric { id: "cpu.cores".to_string(), name: "CPU Core Count".to_string(), unit: "cores".to_string(), r#static: true },
        CatalogMetric { id: "mem.used".to_string(), name: "Memory Used".to_string(), unit: "GB".to_string(), r#static: false },
        CatalogMetric { id: "mem.total".to_string(), name: "Memory Total".to_string(), unit: "GB".to_string(), r#static: true },
        CatalogMetric { id: "mem.usage".to_string(), name: "Memory Usage".to_string(), unit: "%".to_string(), r#static: false },
        CatalogMetric { id: "swap.used".to_string(), name: "Swap Used".to_string(), unit: "GB".to_string(), r#static: false },
        CatalogMetric { id: "swap.total".to_string(), name: "Swap Total".to_string(), unit: "GB".to_string(), r#static: true },
        CatalogMetric { id: "sys.uptime".to_string(), name: "System Uptime".to_string(), unit: "s".to_string(), r#static: false },
        CatalogMetric { id: "sys.load1".to_string(), name: "Load Average (1m)".to_string(), unit: "".to_string(), r#static: false },
        CatalogMetric { id: "proc.count".to_string(), name: "Process Count".to_string(), unit: "procs".to_string(), r#static: false },
    ]
}

pub fn round_to_quantum(ms: u64) -> u64 {
    if ms == 0 {
        return POLL_QUANTUM_MS;
    }
    let q = POLL_QUANTUM_MS;
    let rounded = ((ms + q - 1) / q) * q;
    rounded.max(q)
}

pub struct Collector {
    sys: Arc<Mutex<System>>,
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

    pub fn sample(&self, metric_id: &str) -> Option<f64> {
        let mut sys = self.sys.lock().ok()?;
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let bytes_to_gb = |b: u64| (b as f64) / 1024.0 / 1024.0 / 1024.0;

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
            "mem.used" => Some(bytes_to_gb(sys.used_memory())),
            "mem.total" => Some(bytes_to_gb(sys.total_memory())),
            "mem.usage" => {
                let total = sys.total_memory() as f64;
                if total == 0.0 {
                    None
                } else {
                    Some((sys.used_memory() as f64 / total) * 100.0)
                }
            }
            "swap.used" => Some(bytes_to_gb(sys.used_swap())),
            "swap.total" => Some(bytes_to_gb(sys.total_swap())),
            "sys.uptime" => Some(sysinfo::System::uptime() as f64),
            "sys.load1" => {
                let load = System::load_average();
                Some(load.one)
            }
            "proc.count" => Some(sys.processes().len() as f64),
            _ => None,
        }
    }

    pub fn sample_many(&self, ids: &[String]) -> HashMap<String, f64> {
        let mut out = HashMap::with_capacity(ids.len());
        for id in ids {
            if let Some(v) = self.sample(id) {
                out.insert(id.clone(), v);
            }
        }
        out
    }
}

