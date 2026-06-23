use crate::protocol::{CatalogMetric, MetricDataType, MetricNumber};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, Disks, Networks, ProcessRefreshKind, RefreshKind, System};

pub const POLL_QUANTUM_MS: u64 = 50;

const MEMORY_UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "KiB", "MiB", "GiB", "TiB"];
const MEMORY_RATE_UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "KiB/s", "MiB/s"];

fn catalog_metric(
    id: &'static str,
    name: &'static str,
    default_unit: &'static str,
    available_units: &[&'static str],
    static_metric: bool,
    data_type: MetricDataType,
) -> CatalogMetric {
    CatalogMetric {
        id: id.to_string(),
        name: name.to_string(),
        default_unit: default_unit.to_string(),
        available_units: available_units.iter().map(|s| s.to_string()).collect(),
        r#static: static_metric,
        data_type,
    }
}

pub fn build_catalog() -> Vec<CatalogMetric> {
    vec![
        catalog_metric("cpu.usage", "CPU Usage", "%", &["%"], false, MetricDataType::Float),
        catalog_metric("cpu.cores", "CPU Core Count", "cores", &["cores"], true, MetricDataType::Integer),
        catalog_metric("mem.used", "Memory Used", "GB", MEMORY_UNITS, false, MetricDataType::Float),
        catalog_metric("mem.total", "Memory Total", "GB", MEMORY_UNITS, true, MetricDataType::Integer),
        catalog_metric("mem.usage", "Memory Usage", "%", &["%"], false, MetricDataType::Float),
        catalog_metric("swap.used", "Swap Used", "GB", MEMORY_UNITS, false, MetricDataType::Float),
        catalog_metric("swap.total", "Swap Total", "GB", MEMORY_UNITS, true, MetricDataType::Integer),
        catalog_metric("sys.uptime", "System Uptime", "s", &["s", "ms", "m", "h"], false, MetricDataType::Float),
        catalog_metric("sys.load1", "Load Average (1m)", "", &[""], false, MetricDataType::Float),
        catalog_metric("sys.load5", "Load Average (5m)", "", &[""], false, MetricDataType::Float),
        catalog_metric("sys.load15", "Load Average (15m)", "", &[""], false, MetricDataType::Float),
        catalog_metric("proc.count", "Process Count", "procs", &["procs"], false, MetricDataType::Integer),
        catalog_metric("net.interfaces", "Network Interface Names", "", &[""], false, MetricDataType::StringList),
        catalog_metric("net.ip_addresses", "Network IP Addresses", "", &[""], false, MetricDataType::StringList),
        catalog_metric("net.wifi_ssids", "Visible Wi-Fi Network Names (SSIDs)", "", &[""], false, MetricDataType::StringList),
        catalog_metric("net.routes", "Network Routes", "", &[""], false, MetricDataType::StringList),
        catalog_metric("net.default_gateway", "Default Gateway", "", &[""], false, MetricDataType::String),
        catalog_metric("net.iface_count", "Network Interface Count", "ifaces", &["ifaces"], false, MetricDataType::Integer),
        catalog_metric("net.rx_bytes", "Network Received (count, per-interface)", "B", MEMORY_UNITS, false, MetricDataType::StringList),
        catalog_metric("net.tx_bytes", "Network Transmitted (count, per-interface)", "B", MEMORY_UNITS, false, MetricDataType::StringList),
        catalog_metric("net.rx_packets", "Network Packets Received (count, per-interface)", "pkts", &["pkts"], false, MetricDataType::StringList),
        catalog_metric("net.tx_packets", "Network Packets Transmitted (count, per-interface)", "pkts", &["pkts"], false, MetricDataType::StringList),
        catalog_metric("net.rx_bytes_rate", "Network Received (rate, per-interface)", "B/s", MEMORY_RATE_UNITS, false, MetricDataType::StringList),
        catalog_metric("net.tx_bytes_rate", "Network Transmitted (rate, per-interface)", "B/s", MEMORY_RATE_UNITS, false, MetricDataType::StringList),
        catalog_metric("net.rx_packets_rate", "Network Packets Received (rate, per-interface)", "pps", &["pps"], false, MetricDataType::StringList),
        catalog_metric("net.tx_packets_rate", "Network Packets Transmitted (rate, per-interface)", "pps", &["pps"], false, MetricDataType::StringList),
        catalog_metric("disk.count", "Disk Count", "disks", &["disks"], false, MetricDataType::Integer),
        catalog_metric("disk.total", "Disk Total Space", "GB", MEMORY_UNITS, false, MetricDataType::Float),
        catalog_metric("disk.used", "Disk Used Space", "GB", MEMORY_UNITS, false, MetricDataType::Float),
        catalog_metric("disk.available", "Disk Available Space", "GB", MEMORY_UNITS, false, MetricDataType::Float),
        catalog_metric("disk.usage", "Disk Usage", "%", &["%"], false, MetricDataType::Float),
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
    let base = byte_multiplier(unit)?;
    Some(bytes / base)
}

fn byte_multiplier(unit: &str) -> Option<f64> {
    match unit {
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
    }
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

struct IfaceCounters {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_packets: u64,
    tx_packets: u64,
}

struct NetState {
    last_at: Option<Instant>,
    last: HashMap<String, IfaceCounters>,
    rx_bytes_rate: HashMap<String, f64>,
    tx_bytes_rate: HashMap<String, f64>,
    rx_packets_rate: HashMap<String, f64>,
    tx_packets_rate: HashMap<String, f64>,
}

impl NetState {
    fn new() -> Self {
        Self {
            last_at: None,
            last: HashMap::new(),
            rx_bytes_rate: HashMap::new(),
            tx_bytes_rate: HashMap::new(),
            rx_packets_rate: HashMap::new(),
            tx_packets_rate: HashMap::new(),
        }
    }
}

pub struct Collector {
    sys: Arc<Mutex<System>>,
    disks: Arc<Mutex<Disks>>,
    networks: Arc<Mutex<Networks>>,
    net_state: Arc<Mutex<NetState>>,
    gateway_cache: Arc<Mutex<Option<(String, Instant)>>>,
    wifi_cache: Arc<Mutex<Option<(Vec<String>, Instant)>>>,
    route_cache: Arc<Mutex<Option<(Vec<String>, Instant)>>>,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram().with_swap())
                .with_processes(ProcessRefreshKind::nothing()),
        );
        Self {
            sys: Arc::new(Mutex::new(sys)),
            disks: Arc::new(Mutex::new(Disks::new_with_refreshed_list())),
            networks: Arc::new(Mutex::new(Networks::new_with_refreshed_list())),
            net_state: Arc::new(Mutex::new(NetState::new())),
            gateway_cache: Arc::new(Mutex::new(None)),
            wifi_cache: Arc::new(Mutex::new(None)),
            route_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn needs_cpu(id: &str) -> bool {
        id.starts_with("cpu.")
    }

    fn needs_mem(id: &str) -> bool {
        id.starts_with("mem.") || id.starts_with("swap.")
    }

    fn needs_procs(id: &str) -> bool {
        id.starts_with("proc.")
    }

    fn needs_sys_load(id: &str) -> bool {
        id == "sys.uptime"
            || id == "sys.load1"
            || id == "sys.load5"
            || id == "sys.load15"
    }

    fn needs_disks(id: &str) -> bool {
        id.starts_with("disk.")
    }

    fn needs_networks(id: &str) -> bool {
        id.starts_with("net.")
    }

    fn refresh_networks(&self) {
        let mut networks = self.networks.lock().expect("Collector mutex poisoned");
        networks.refresh(true);
        let mut net_state = self.net_state.lock().expect("Collector mutex poisoned");
        let now = Instant::now();

        let mut current: HashMap<String, IfaceCounters> = HashMap::new();
        for (name, data) in networks.list().iter() {
            current.insert(
                name.to_string(),
                IfaceCounters {
                    rx_bytes: data.received(),
                    tx_bytes: data.transmitted(),
                    rx_packets: data.packets_received(),
                    tx_packets: data.packets_transmitted(),
                },
            );
        }

        net_state.rx_bytes_rate.clear();
        net_state.tx_bytes_rate.clear();
        net_state.rx_packets_rate.clear();
        net_state.tx_packets_rate.clear();

        if let Some(prev) = net_state.last_at {
            let elapsed = now.duration_since(prev).as_secs_f64();
            if elapsed > 0.0 {
                let mut rx_b: HashMap<String, f64> = HashMap::new();
                let mut tx_b: HashMap<String, f64> = HashMap::new();
                let mut rx_p: HashMap<String, f64> = HashMap::new();
                let mut tx_p: HashMap<String, f64> = HashMap::new();
                for (name, cur) in current.iter() {
                    if let Some(prior) = net_state.last.get(name) {
                        rx_b.insert(
                            name.clone(),
                            (cur.rx_bytes.saturating_sub(prior.rx_bytes)) as f64 / elapsed,
                        );
                        tx_b.insert(
                            name.clone(),
                            (cur.tx_bytes.saturating_sub(prior.tx_bytes)) as f64 / elapsed,
                        );
                        rx_p.insert(
                            name.clone(),
                            (cur.rx_packets.saturating_sub(prior.rx_packets)) as f64 / elapsed,
                        );
                        tx_p.insert(
                            name.clone(),
                            (cur.tx_packets.saturating_sub(prior.tx_packets)) as f64 / elapsed,
                        );
                    }
                }
                net_state.rx_bytes_rate = rx_b;
                net_state.tx_bytes_rate = tx_b;
                net_state.rx_packets_rate = rx_p;
                net_state.tx_packets_rate = tx_p;
            }
        }

        net_state.last_at = Some(now);
        net_state.last = current;
    }

    fn refresh_for(&self, ids: &[String]) {
        let want_cpu = ids.iter().any(|i| Self::needs_cpu(i));
        let want_mem = ids.iter().any(|i| Self::needs_mem(i));
        let want_procs = ids.iter().any(|i| Self::needs_procs(i));
        let want_sys_load = ids.iter().any(|i| Self::needs_sys_load(i));
        let want_disks = ids.iter().any(|i| Self::needs_disks(i));
        let want_net = ids.iter().any(|i| Self::needs_networks(i));

        if want_cpu || want_mem || want_procs || want_sys_load {
            let mut sys = self.sys.lock().expect("Collector mutex poisoned");
            if want_cpu {
                sys.refresh_cpu_usage();
            }
            if want_mem {
                sys.refresh_memory();
            }
            if want_procs {
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            }
        }
        if want_disks {
            let mut disks = self.disks.lock().expect("Collector mutex poisoned");
            disks.refresh(true);
        }
        if want_net {
            self.refresh_networks();
        }
    }

    fn read_raw(&self, metric_id: &str) -> Option<f64> {
        match metric_id {
            "sys.load5" => return Some(System::load_average().five),
            "sys.load15" => return Some(System::load_average().fifteen),
            _ => {}
        }

        let sys = self.sys.lock().expect("Collector mutex poisoned");
        match metric_id {
            "cpu.usage" => {
                let cpus = sys.cpus();
                if cpus.is_empty() {
                    None
                } else {
                    let sum: f64 = cpus.iter().map(|c| c.cpu_usage() as f64).sum();
                    Some(sum / cpus.len() as f64)
                }
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
        self.refresh_for(&[metric_id.to_string()]);
        let raw = self.read_raw(metric_id)?;
        self::convert_value(raw, metric_id, unit)
    }

    fn read_string(&self, metric_id: &str) -> Option<String> {
        match metric_id {
            "net.default_gateway" => {
                let mut cache = self.gateway_cache.lock().expect("Collector mutex poisoned");
                let ttl = Duration::from_secs(30);
                if let Some((ref val, ref at)) = *cache {
                    if at.elapsed() < ttl {
                        return Some(val.clone());
                    }
                }
                let val = match default_net::get_default_gateway() {
                    Ok(g) => g.ip_addr.to_string(),
                    Err(_) => return None,
                };
                *cache = Some((val.clone(), Instant::now()));
                Some(val)
            }
            _ => None,
        }
    }

    fn read_string_list(&self, metric_id: &str, unit: &str) -> Option<Vec<String>> {
        match metric_id {
            "net.interfaces" => {
                let networks = self.networks.lock().expect("Collector mutex poisoned");
                let mut names: Vec<String> = networks.list().keys().map(|s| s.to_string()).collect();
                names.sort();
                Some(names)
            }
            "net.ip_addresses" => {
                let networks = self.networks.lock().expect("Collector mutex poisoned");
                let mut addrs: Vec<String> = Vec::new();
                for (name, data) in networks.list().iter() {
                    for ipn in data.ip_networks() {
                        addrs.push(format!("{}:{}/{}", name, ipn.addr, ipn.prefix));
                    }
                }
                addrs.sort();
                Some(addrs)
            }
            "net.wifi_ssids" => {
                let mut cache = self.wifi_cache.lock().expect("Collector mutex poisoned");
                let ttl = Duration::from_secs(30);
                if let Some((ref list, ref at)) = *cache {
                    if at.elapsed() < ttl {
                        return Some(list.clone());
                    }
                }
                let list = read_wifi_ssids();
                *cache = Some((list.clone(), Instant::now()));
                Some(list)
            }
            "net.routes" => {
                let mut cache = self.route_cache.lock().expect("Collector mutex poisoned");
                let ttl = Duration::from_secs(30);
                if let Some((ref list, ref at)) = *cache {
                    if at.elapsed() < ttl {
                        return Some(list.clone());
                    }
                }
                let list = read_routes()?;
                *cache = Some((list.clone(), Instant::now()));
                Some(list)
            }
            "net.rx_bytes" | "net.tx_bytes" | "net.rx_packets" | "net.tx_packets" => {
                let networks = self.networks.lock().expect("Collector mutex poisoned");
                let is_bytes = metric_id == "net.rx_bytes" || metric_id == "net.tx_bytes";
                let mut out: Vec<String> = Vec::new();
                for (name, data) in networks.list().iter() {
                    let raw = match metric_id {
                        "net.rx_bytes" => data.total_received() as f64,
                        "net.tx_bytes" => data.total_transmitted() as f64,
                        "net.rx_packets" => data.total_packets_received() as f64,
                        "net.tx_packets" => data.total_packets_transmitted() as f64,
                        _ => return None,
                    };
                    let scaled = if is_bytes {
                        convert_bytes(raw, unit).unwrap_or(raw)
                    } else {
                        raw
                    };
                    out.push(format!("{}:{}", name, scaled));
                }
                Some(out)
            }
            "net.rx_bytes_rate" | "net.tx_bytes_rate" | "net.rx_packets_rate"
            | "net.tx_packets_rate" => {
                let st = self.net_state.lock().expect("Collector mutex poisoned");
                let src: &HashMap<String, f64> = match metric_id {
                    "net.rx_bytes_rate" => &st.rx_bytes_rate,
                    "net.tx_bytes_rate" => &st.tx_bytes_rate,
                    "net.rx_packets_rate" => &st.rx_packets_rate,
                    "net.tx_packets_rate" => &st.tx_packets_rate,
                    _ => return None,
                };
                let is_bytes = metric_id == "net.rx_bytes_rate" || metric_id == "net.tx_bytes_rate";
                let mut out: Vec<String> = Vec::new();
                for (k, v) in src.iter() {
                    let scaled = if is_bytes {
                        convert_bytes_per_sec(*v, unit).unwrap_or(*v)
                    } else {
                        *v
                    };
                    out.push(format!("{}:{}", k, scaled));
                }
                Some(out)
            }
            _ => None,
        }
    }

    pub fn sample_many(
        &self,
        requests: &[(String, String, MetricDataType)],
    ) -> Vec<crate::protocol::MetricValue> {
        if requests.is_empty() {
            return Vec::new();
        }
        let ids: Vec<String> = requests.iter().map(|(id, _, _)| id.clone()).collect();
        self.refresh_for(&ids);
        let mut out = Vec::with_capacity(requests.len());
        for (id, unit, dtype) in requests {
            match dtype {
                MetricDataType::String => {
                    let s = match self.read_string(id) {
                        Some(s) => s,
                        None => continue,
                    };
                    out.push(crate::protocol::MetricValue {
                        id: id.clone(),
                        value: MetricNumber::String(s),
                        unit: unit.clone(),
                    });
                }
                MetricDataType::StringList => {
                    let list = match self.read_string_list(id, unit) {
                        Some(l) => l,
                        None => continue,
                    };
                    out.push(crate::protocol::MetricValue {
                        id: id.clone(),
                        value: MetricNumber::StringList(list),
                        unit: unit.clone(),
                    });
                }
                _ => {
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
                        MetricDataType::String | MetricDataType::StringList => unreachable!(),
                    };
                    out.push(crate::protocol::MetricValue {
                        id: id.clone(),
                        value,
                        unit: unit.clone(),
                    });
                }
            }
        }
        out
    }
}

fn convert_bytes_per_sec(bytes_per_sec: f64, unit: &str) -> Option<f64> {
    let stripped = unit.strip_suffix("/s")?;
    let base = byte_multiplier(stripped)?;
    Some(bytes_per_sec / base)
}

fn convert_value(raw: f64, metric_id: &str, unit: &str) -> Option<f64> {
    if metric_id.starts_with("mem.")
        || metric_id.starts_with("swap.")
        || metric_id == "disk.total"
        || metric_id == "disk.used"
        || metric_id == "disk.available"
    {
        convert_bytes(raw, unit)
    } else if metric_id == "sys.uptime" {
        convert_seconds(raw, unit)
    } else {
        Some(raw)
    }
}

fn run_capture(cmd: &mut std::process::Command) -> Option<String> {
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn read_wifi_ssids() -> Vec<String> {
    let raw = run_capture(std::process::Command::new("nmcli").args([
        "-t", "-f", "SSID", "device", "wifi", "list",
    ]));
    match raw {
        Some(s) => s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>(),
        None => Vec::new(),
    }
}

fn read_routes() -> Option<Vec<String>> {
    let mut mgr = route_manager::RouteManager::new().ok()?;
    let routes = mgr.list().ok()?;
    let mut out: Vec<String> = Vec::new();
    for r in routes {
        let dest = format!("{}/{}", r.destination(), r.prefix());
        let if_name = r.if_name().map(|s| s.as_str()).unwrap_or("");
        let hop = match r.gateway() {
            Some(g) => g.to_string(),
            None => String::new(),
        };
        out.push(format!("{}:{}:{}", if_name, dest, hop));
    }
    Some(out)
}
