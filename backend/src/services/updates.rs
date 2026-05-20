use anyhow::{anyhow, Result};
use std::process::Command;
use tracing::info;

use crate::models::system::UpdateInfo;

pub fn check_updates() -> Result<Vec<UpdateInfo>> {
    let refresh = Command::new("sudo")
        .args(["-n", "apt-get", "update", "-qq"])
        .output()
        .map_err(|e| anyhow!("Failed to run apt-get update: {}", e))?;

    if !refresh.status.success() {
        let stderr = String::from_utf8_lossy(&refresh.stderr);
        return Err(anyhow!("apt-get update failed: {}", stderr.trim()));
    }

    let output = Command::new("apt")
        .args(["list", "--upgradable"])
        .output()
        .map_err(|e| anyhow!("Failed to list upgradable packages: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let updates: Vec<UpdateInfo> = stdout
        .lines()
        .skip(1)
        .filter_map(|line| parse_update_line(line))
        .collect();

    info!("Found {} available updates", updates.len());
    Ok(updates)
}

pub fn apply_updates() -> Result<String> {
    let output = Command::new("sudo")
        .args(["-n", "apt-get", "upgrade", "-y", "-qq"])
        .output()
        .map_err(|e| anyhow!("Failed to run apt-get upgrade: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("System updates applied successfully");
        Ok(stdout.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("apt-get upgrade failed: {}", stderr.trim()))
    }
}

fn parse_update_line(line: &str) -> Option<UpdateInfo> {
    let parts: Vec<&str> = line.splitn(2, '/').collect();
    if parts.len() < 2 {
        return None;
    }

    let package = parts[0].to_string();
    let rest = parts[1];

    let segments: Vec<&str> = rest.split_whitespace().collect();
    if segments.len() < 2 {
        return None;
    }

    let repository = segments[0].to_string();
    let new_version = segments[1].to_string();

    let current_version = if let Some(from_idx) = rest.find("from: ") {
        let start = from_idx + 6;
        let end = rest[start..].find(']').map(|i| start + i).unwrap_or(rest.len());
        rest[start..end].to_string()
    } else {
        "unknown".to_string()
    };

    Some(UpdateInfo {
        package,
        current_version,
        new_version,
        repository,
    })
}
