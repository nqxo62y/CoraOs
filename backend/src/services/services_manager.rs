//! Systemd service management.
//!
//! Provides start, stop, restart, enable, disable operations and status queries
//! for systemd units on the host system.

use anyhow::{anyhow, Result};
use std::process::Command;

use crate::models::system::{ServiceAction, ServiceInfo};

/// List all systemd services with their current status.
pub fn list_services() -> Result<Vec<ServiceInfo>> {
    let output = Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
        ])
        .output()
        .map_err(|e| anyhow!("Failed to execute systemctl: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let services: Vec<ServiceInfo> = stdout
        .lines()
        .filter_map(|line| parse_service_line(line))
        .collect();

    Ok(services)
}

/// Get detailed status of a specific service.
pub fn get_service_status(service_name: &str) -> Result<ServiceInfo> {
    // Validate service name to prevent command injection
    validate_service_name(service_name)?;

    let output = Command::new("systemctl")
        .args(["show", service_name, "--no-pager"])
        .output()
        .map_err(|e| anyhow!("Failed to query service status: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut info = ServiceInfo {
        name: service_name.to_string(),
        description: String::new(),
        load_state: String::new(),
        active_state: String::new(),
        sub_state: String::new(),
        unit_file_state: String::new(),
    };

    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "Description" => info.description = value.to_string(),
                "LoadState" => info.load_state = value.to_string(),
                "ActiveState" => info.active_state = value.to_string(),
                "SubState" => info.sub_state = value.to_string(),
                "UnitFileState" => info.unit_file_state = value.to_string(),
                _ => {}
            }
        }
    }

    Ok(info)
}

/// Execute an action on a systemd service.
pub fn execute_service_action(service_name: &str, action: &ServiceAction) -> Result<String> {
    validate_service_name(service_name)?;

    let output = Command::new("systemctl")
        .args([action.as_str(), service_name])
        .output()
        .map_err(|e| anyhow!("Failed to execute service action: {}", e))?;

    if output.status.success() {
        Ok(format!(
            "Service '{}' {} successfully",
            service_name,
            past_tense(action)
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!(
            "Failed to {} service '{}': {}",
            action.as_str(),
            service_name,
            stderr.trim()
        ))
    }
}

/// Parse a single line from systemctl list-units output.
fn parse_service_line(line: &str) -> Option<ServiceInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }

    let name = parts[0].trim_end_matches(".service").to_string();
    let load_state = parts[1].to_string();
    let active_state = parts[2].to_string();
    let sub_state = parts[3].to_string();
    let description = if parts.len() > 4 {
        parts[4..].join(" ")
    } else {
        String::new()
    };

    Some(ServiceInfo {
        name,
        description,
        load_state,
        active_state,
        sub_state,
        unit_file_state: String::new(),
    })
}

/// Validate that a service name contains only safe characters.
fn validate_service_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 256 {
        return Err(anyhow!("Invalid service name length"));
    }

    // Allow alphanumeric, hyphens, underscores, dots, and @ (for template units)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@')
    {
        return Err(anyhow!(
            "Service name contains invalid characters. Only alphanumeric, -, _, ., @ allowed."
        ));
    }

    // Prevent path traversal
    if name.contains("..") {
        return Err(anyhow!("Service name must not contain path traversal"));
    }

    Ok(())
}

/// Convert a service action to past tense for status messages.
fn past_tense(action: &ServiceAction) -> &str {
    match action {
        ServiceAction::Start => "started",
        ServiceAction::Stop => "stopped",
        ServiceAction::Restart => "restarted",
        ServiceAction::Enable => "enabled",
        ServiceAction::Disable => "disabled",
    }
}
