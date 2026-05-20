//! System monitoring and service management models.

use serde::{Deserialize, Serialize};

/// Real-time system metrics snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub cpu_count: usize,
    pub cpu_model: String,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_percent: f32,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub disk_usage: Vec<DiskInfo>,
    pub network: Vec<NetworkInterface>,
    pub uptime_seconds: u64,
    pub hostname: String,
    pub os_name: String,
    pub kernel_version: String,
    pub load_average: LoadAverage,
}

/// Disk partition information.
#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub filesystem: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
}

/// Network interface statistics.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
    pub received_packets: u64,
    pub transmitted_packets: u64,
}

/// System load averages.
#[derive(Debug, Clone, Serialize)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

/// Process information for the process monitor.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: u64,
    pub status: String,
    pub user: String,
    pub start_time: u64,
}

/// Systemd service status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
}

/// Request to perform a service action.
#[derive(Debug, Deserialize)]
pub struct ServiceActionRequest {
    pub service: String,
    pub action: ServiceAction,
}

/// Available service actions.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}

impl ServiceAction {
    pub fn as_str(&self) -> &str {
        match self {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
            ServiceAction::Enable => "enable",
            ServiceAction::Disable => "disable",
        }
    }
}

/// Log entry from journalctl.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub unit: String,
    pub priority: String,
    pub message: String,
}

/// Log query parameters.
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub unit: Option<String>,
    pub priority: Option<String>,
    pub lines: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub search: Option<String>,
}

/// System update information.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub package: String,
    pub current_version: String,
    pub new_version: String,
    pub repository: String,
}

/// Server configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub updated_at: String,
}

/// Request to update a configuration value.
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}
