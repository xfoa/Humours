# Hardware Query

A cross-platform Rust library for querying detailed system hardware information with advanced monitoring and power management capabilities.

## 🚀 Features

### Core Hardware Detection
- ✅ Cross-platform hardware detection (Windows, Linux, macOS)
- ✅ Detailed CPU information (cores, threads, cache, features)
- ✅ GPU detection and capabilities (CUDA, ROCm, DirectML support)
- ✅ Memory configuration and status
- ✅ Storage device enumeration and properties
- ✅ Network interface detection and capabilities
- ✅ Hardware acceleration support detection (NPU, TPU, FPGA)
- ✅ PCI/USB device enumeration
- ✅ ARM-specific hardware detection (Raspberry Pi, Jetson, etc.)

### 🔄 Real-time Monitoring (NEW!)
- ✅ Continuous hardware metrics monitoring
- ✅ Configurable update intervals and thresholds
- ✅ Event-driven notifications for thermal/power alerts
- ✅ Background monitoring with async support
- ✅ Comprehensive monitoring statistics

### ⚡ Power Management & Efficiency
- ✅ Real-time power consumption tracking
- ✅ Battery life estimation and health monitoring
- ✅ Power efficiency scoring and optimization
- ✅ Thermal throttling risk assessment
- ✅ Power optimization recommendations

### 🌡️ Enhanced Thermal Management
- ✅ Advanced temperature monitoring with history
- ✅ Thermal throttling prediction algorithms
- ✅ Cooling optimization recommendations
- ✅ Sustained performance capability analysis
- ✅ Fan curve analysis and optimization

### 🐳 Virtualization & Container Detection
- ✅ Comprehensive virtualization environment detection
- ✅ Container runtime identification (Docker, Kubernetes, etc.)
- ✅ Resource limits and restrictions analysis
- ✅ GPU passthrough capability detection
- ✅ Performance impact assessment
- ✅ Security feature analysis

### 🎯 AI/ML Optimization
- ✅ Hardware suitability assessment for AI workloads
- ✅ Performance scoring for different AI tasks
- ✅ Memory and compute requirement analysis
- ✅ Acceleration framework compatibility
- ✅ Optimization recommendations

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
hardware-query = "0.2.0"

# For real-time monitoring features
hardware-query = { version = "0.2.0", features = ["monitoring"] }
```

## 🔧 Feature Flags

### When to Use the `monitoring` Feature

**Enable `monitoring` if you need:**
- ✅ **Continuous hardware monitoring** - Track temperatures, power, and performance over time
- ✅ **Real-time alerts** - Get notified when hardware exceeds thresholds
- ✅ **Background monitoring** - Async monitoring that doesn't block your application
- ✅ **Event-driven architecture** - React to hardware changes as they happen
- ✅ **Server/system monitoring** - Long-running applications that need to track hardware health

**Skip `monitoring` if you only need:**
- ❌ One-time hardware queries (use `SystemOverview::quick()`)
- ❌ Static hardware information for configuration
- ❌ Simple compatibility checks
- ❌ Lightweight applications where async dependencies are unwanted

```toml
# Lightweight - no async dependencies
hardware-query = "0.2.0"

# Full featured - includes real-time monitoring
hardware-query = { version = "0.2.0", features = ["monitoring"] }
```

## 🚀 Simplified API (Recommended for New Users)

For most use cases, start with the simplified API:

```rust
use hardware_query::SystemOverview;

// Get essential system info in one line - no hardware expertise needed!
let overview = SystemOverview::quick()?;
println!("CPU: {} ({} cores)", overview.cpu.name, overview.cpu.cores);
println!("Memory: {:.1} GB", overview.memory_gb);
println!("Performance Score: {}/100", overview.performance_score);
println!("AI Ready: {}", overview.is_ai_ready());
```

For domain-specific needs:

```rust
use hardware_query::HardwarePresets;

// AI/ML assessment with built-in expertise
let ai_assessment = HardwarePresets::ai_assessment()?;
println!("AI Score: {}/100", ai_assessment.ai_score);

// Gaming performance recommendations
let gaming = HardwarePresets::gaming_assessment()?;
println!("Recommended: {} at {:?}", 
    gaming.recommended_settings.resolution,
    gaming.recommended_settings.quality_preset);
```

## Advanced Usage

```rust
use hardware_query::HardwareInfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get complete system information
    let hw_info = HardwareInfo::query()?;
    
    // Access CPU information
    let cpu = hw_info.cpu();
    println!("CPU: {} {} with {} cores, {} threads",
        cpu.vendor(),
        cpu.model_name(),
        cpu.physical_cores(),
        cpu.logical_cores()
    );
    
    // Check virtualization environment
    let virt = hw_info.virtualization();
    if virt.is_virtualized() {
        println!("Running in: {} (Performance impact: {:.1}%)", 
            virt.environment_type,
            (1.0 - virt.get_performance_factor()) * 100.0
        );
    }
    
    // Power management
    if let Some(power) = hw_info.power_profile() {
        println!("Power State: {}", power.power_state);
        if let Some(power_draw) = power.total_power_draw {
            println!("Current Power Draw: {:.1}W", power_draw);
        }
        
        // Get optimization recommendations
        let optimizations = power.suggest_power_optimizations();
        for opt in optimizations {
            println!("💡 {}", opt.recommendation);
        }
    }
    
    // Thermal analysis
    let thermal = hw_info.thermal();
    if let Some(max_temp) = thermal.max_temperature() {
        println!("Max Temperature: {:.1}°C (Status: {})", 
            max_temp, thermal.thermal_status());
        
        // Predict thermal throttling
        let prediction = thermal.predict_thermal_throttling(1.0);
        if prediction.will_throttle {
            println!("⚠️  Thermal throttling predicted: {}", prediction.severity);
        }
        
        // Get cooling recommendations
        let cooling_recs = thermal.suggest_cooling_optimizations();
        for rec in cooling_recs.iter().take(3) {
            println!("🌡️ {}", rec.description);
        }
    }
    
    Ok(())
}
```

## Advanced Real-time Monitoring

```rust
use hardware_query::{HardwareMonitor, MonitoringConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure monitoring
    let mut config = MonitoringConfig::default();
    config.update_interval = Duration::from_secs(2);
    config.thermal_threshold = 75.0;
    
    let monitor = HardwareMonitor::with_config(config);
    
    // Add event callbacks
    monitor.on_event(|event| {
        match event {
            MonitoringEvent::ThermalAlert { sensor_name, temperature, .. } => {
                println!("🚨 Thermal Alert: {} at {:.1}°C", sensor_name, temperature);
            }
            MonitoringEvent::PowerAlert { current_power, .. } => {
                println!("⚡ Power Alert: {:.1}W", current_power);
            }
            _ => {}
        }
    }).await;
    
    // Start monitoring
    monitor.start_monitoring().await?;
    
    // Monitor for 60 seconds
    tokio::time::sleep(Duration::from_secs(60)).await;
    
    // Stop and get statistics
    monitor.stop_monitoring().await;
    let stats = monitor.get_stats().await;
    println!("Generated {} events ({} thermal alerts)", 
        stats.total_events, stats.thermal_alerts);
    
    Ok(())
}
```
    
    // Get GPU information
    for (i, gpu) in hw_info.gpus().iter().enumerate() {
        println!("GPU {}: {} {} with {} GB VRAM",
            i + 1,
            gpu.vendor(),
            gpu.model_name(),
            gpu.memory_gb()
        );
        
        // Check GPU capabilities
        println!("  CUDA support: {}", gpu.supports_cuda());
        println!("  ROCm support: {}", gpu.supports_rocm());
        println!("  DirectML support: {}", gpu.supports_directml());
    }
    
    // Memory information
    let mem = hw_info.memory();
    println!("System memory: {:.1} GB total, {:.1} GB available",
        mem.total_gb(),
        mem.available_gb()
    );
    
    // Storage information
    for (i, disk) in hw_info.storage_devices().iter().enumerate() {
        println!("Disk {}: {} {:.1} GB ({:?} type)",
            i + 1,
            disk.model,
            disk.capacity_gb,
            disk.storage_type
        );
    }
    
    // Network interfaces
    for interface in hw_info.network_interfaces() {
        println!("Network: {} - {}", 
            interface.name,
            interface.mac_address
        );
    }
    
    // Check specialized hardware
    for npu in hw_info.npus() {
        println!("NPU detected: {} {}", npu.vendor(), npu.model_name());
    }
    
    for tpu in hw_info.tpus() {
        println!("TPU detected: {} {}", tpu.vendor(), tpu.model_name());
    }
    
    // Check ARM-specific hardware (Raspberry Pi, Jetson, etc.)
    if let Some(arm) = hw_info.arm_hardware() {
        println!("ARM System: {}", arm.system_type);
    }
    
    // Check FPGA hardware
    for fpga in hw_info.fpgas() {
        println!("FPGA: {} {} with {} logic elements",
            fpga.vendor,
            fpga.family,
            fpga.logic_elements.unwrap_or(0)
        );
    }
    
    // Serialize hardware information to JSON
    let hw_json = hw_info.to_json()?;
    println!("Hardware JSON: {}", hw_json);
    
    Ok(())
}
```

## Specialized Hardware Support

The library provides comprehensive detection for AI/ML-oriented hardware:

- **NPUs**: Intel Movidius, GNA, XDNA; Apple Neural Engine; Qualcomm Hexagon
- **TPUs**: Google Cloud TPU and Edge TPU; Intel Habana
- **ARM Systems**: Raspberry Pi, NVIDIA Jetson, Apple Silicon with power management
- **FPGAs**: Intel/Altera and Xilinx families with AI optimization scoring

```rust
use hardware_query::HardwareInfo;

let hw_info = HardwareInfo::query()?;

// Check for specialized AI hardware
for npu in hw_info.npus() {
    println!("NPU: {} {}", 
        npu.vendor(), 
        npu.model_name()
    );
}

for tpu in hw_info.tpus() {
    println!("TPU: {} {}", 
        tpu.vendor(),
        tpu.model_name()
    );
}
```

## Platform Support

| Platform | CPU | GPU | Memory | Storage | Network | Battery | Thermal | PCI | USB |
|----------|-----|-----|--------|---------|---------|---------|---------|-----|-----|
| Windows  | ✅  | ✅  | ✅     | ✅      | ✅      | ✅      | ✅      | ✅  | ✅  |
| Linux    | ✅  | ✅  | ✅     | ✅      | ✅      | ✅      | ✅      | ✅  | ✅  |
| macOS    | ✅  | ✅  | ✅     | ✅      | ✅      | ✅      | ✅      | ✅  | ✅  |

## Optional Features

Enable additional GPU support with feature flags:

```toml
[dependencies]
hardware-query = { version = "0.2.0", features = ["nvidia", "amd", "intel"] }
```

- `nvidia`: NVIDIA GPU support via NVML
- `amd`: AMD GPU support via ROCm
- `intel`: Intel GPU support

## Examples

See the [examples](examples/) directory for complete usage examples:

- 🚀 [**Simplified API (Start Here!)**](examples/01_simplified_api.rs) - Easy-to-use API for common scenarios
- [Basic Hardware Detection](examples/02_basic_hardware.rs) - Fundamental hardware querying
- [Enhanced Hardware](examples/enhanced_hardware.rs) - Advanced hardware analysis
- [Comprehensive Hardware](examples/comprehensive_hardware.rs) - Complete system analysis
- [Advanced Usage](examples/advanced_usage.rs) - Expert-level features
- [Comprehensive AI Hardware](examples/comprehensive_ai_hardware.rs) - AI/ML-specific analysis
- [Enhanced Monitoring Demo](examples/enhanced_monitoring_demo.rs) - Real-time monitoring (requires `monitoring` feature)

## Building

```bash
# Build with default features
cargo build

# Build with GPU support
cargo build --features="nvidia,amd,intel"

# Run tests
cargo test

# Try the simplified API first (recommended for new users!)
cargo run --example 01_simplified_api

# Or run the basic hardware example
cargo run --example 02_basic_hardware
```

## Dependencies

- `sysinfo` - Cross-platform system information
- `serde` - Serialization framework
- `thiserror` - Error handling
- `num_cpus` - CPU core detection

### Optional Dependencies

- `nvml-wrapper` - NVIDIA GPU support (feature: nvidia)
- `wmi` - Windows Management Instrumentation (Windows only)
- `libc` - Linux system calls (Linux only)
- `core-foundation` - macOS system APIs (macOS only)

## Performance

The library is designed for performance with:

- Lazy evaluation of hardware information
- Minimal system calls
- Efficient data structures
- Optional caching for repeated queries

Typical query times:

- Complete hardware scan: 10-50ms
- CPU information: 1-5ms
- GPU information: 5-20ms
- Memory information: 1-3ms

## Error Handling

The library uses comprehensive error handling:

```rust
use hardware_query::{HardwareInfo, HardwareQueryError};

match HardwareInfo::query() {
    Ok(hw_info) => {
        // Use hardware information
    }
    Err(HardwareQueryError::SystemInfoUnavailable(msg)) => {
        eprintln!("System info unavailable: {}", msg);
    }
    Err(HardwareQueryError::PermissionDenied(msg)) => {
        eprintln!("Permission denied: {}", msg);
    }
    Err(e) => {
        eprintln!("Hardware query error: {}", e);
    }
}
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT OR Apache-2.0 License - see the [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) files for details.

## Acknowledgments

- Built on top of the excellent [`sysinfo`](https://github.com/GuillaumeGomez/sysinfo) crate
- Inspired by the need for better hardware detection in AI workload placement
- Thanks to all contributors who helped improve cross-platform compatibility
