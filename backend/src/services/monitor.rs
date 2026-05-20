//! System monitoring service.
//!
//! Collects real-time metrics about CPU, memory, disk, and network usage.

use std::sync::Mutex;

use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System,
};

use crate::models::system::{
    DiskInfo, LoadAverage, NetworkInterface, ProcessInfo, SystemMetrics,
};

/// System monitor that maintains a refreshable view of system state.
pub struct SystemMonitor {
    system: Mutex<System>,
}

impl SystemMonitor {
    /// Create a new system monitor instance.
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        SystemMonitor {
            system: Mutex::new(system),
        }
    }

    /// Collect a full snapshot of current system metrics.
    pub fn get_metrics(&self) -> SystemMetrics {
        let mut sys = self.system.lock().unwrap();
        sys.refresh_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
            / sys.cpus().len().max(1) as f32;

        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let memory_total_mb = sys.total_memory() / 1024 / 1024;
        let memory_used_mb = sys.used_memory() / 1024 / 1024;
        let memory_percent = if memory_total_mb > 0 {
            (memory_used_mb as f32 / memory_total_mb as f32) * 100.0
        } else {
            0.0
        };

        let swap_total_mb = sys.total_swap() / 1024 / 1024;
        let swap_used_mb = sys.used_swap() / 1024 / 1024;

        // Disk information
        let disks = Disks::new_with_refreshed_list();
        let disk_usage: Vec<DiskInfo> = disks
            .iter()
            .map(|d| {
                let total = d.total_space();
                let available = d.available_space();
                let used = total.saturating_sub(available);
                let usage_percent = if total > 0 {
                    (used as f32 / total as f32) * 100.0
                } else {
                    0.0
                };
                DiskInfo {
                    mount_point: d.mount_point().to_string_lossy().to_string(),
                    filesystem: d.file_system().to_string_lossy().to_string(),
                    total_gb: total as f64 / 1_073_741_824.0,
                    used_gb: used as f64 / 1_073_741_824.0,
                    available_gb: available as f64 / 1_073_741_824.0,
                    usage_percent,
                }
            })
            .collect();

        // Network interfaces
        let networks = Networks::new_with_refreshed_list();
        let network: Vec<NetworkInterface> = networks
            .iter()
            .map(|(name, data)| NetworkInterface {
                name: name.clone(),
                received_bytes: data.total_received(),
                transmitted_bytes: data.total_transmitted(),
                received_packets: data.total_packets_received(),
                transmitted_packets: data.total_packets_transmitted(),
            })
            .collect();

        // Load average (Linux-specific, read from /proc/loadavg)
        let load_average = read_load_average();

        SystemMetrics {
            cpu_usage_percent: cpu_usage,
            cpu_count: sys.cpus().len(),
            cpu_model,
            memory_total_mb,
            memory_used_mb,
            memory_percent,
            swap_total_mb,
            swap_used_mb,
            disk_usage,
            network,
            uptime_seconds: System::uptime(),
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "Linux".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
            load_average,
        }
    }

    /// Get a list of running processes sorted by CPU usage.
    pub fn get_processes(&self) -> Vec<ProcessInfo> {
        let mut sys = self.system.lock().unwrap();
        sys.refresh_all();

        let mut processes: Vec<ProcessInfo> = sys
            .processes()
            .iter()
            .map(|(pid, proc_info)| ProcessInfo {
                pid: pid.as_u32(),
                name: proc_info.name().to_string(),
                cpu_usage: proc_info.cpu_usage(),
                memory_mb: proc_info.memory() / 1024 / 1024,
                status: format!("{:?}", proc_info.status()),
                user: proc_info
                    .user_id()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                start_time: proc_info.start_time(),
            })
            .collect();

        // Sort by CPU usage descending
        processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        processes.truncate(100); // Return top 100 processes
        processes
    }
}

/// Read load average from /proc/loadavg (Linux only).
fn read_load_average() -> LoadAverage {
    match std::fs::read_to_string("/proc/loadavg") {
        Ok(content) => {
            let parts: Vec<&str> = content.split_whitespace().collect();
            LoadAverage {
                one_min: parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                five_min: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                fifteen_min: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            }
        }
        Err(_) => LoadAverage {
            one_min: 0.0,
            five_min: 0.0,
            fifteen_min: 0.0,
        },
    }
}
