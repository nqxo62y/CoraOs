//! Log viewer route handlers.
//!
//! Provides access to journalctl logs and audit logs.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::process::Command;

use crate::models::audit::AuditLogQuery;
use crate::models::system::{LogEntry, LogQuery};
use crate::services::audit;
use crate::AppState;

/// GET /api/logs
///
/// Query system logs from journalctl with optional filters.
pub async fn get_logs(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<LogEntry>>, (StatusCode, Json<Value>)> {
    let mut args = vec![
        "--no-pager".to_string(),
        "--output=json".to_string(),
    ];

    // Apply filters
    if let Some(ref unit) = query.unit {
        // Validate unit name
        if unit.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
            args.push(format!("--unit={}", unit));
        }
    }

    if let Some(ref priority) = query.priority {
        let valid_priorities = ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"];
        if valid_priorities.contains(&priority.as_str()) {
            args.push(format!("--priority={}", priority));
        }
    }

    let lines = query.lines.unwrap_or(100).min(1000);
    args.push(format!("--lines={}", lines));

    if let Some(ref since) = query.since {
        args.push(format!("--since={}", since));
    }

    if let Some(ref until) = query.until {
        args.push(format!("--until={}", until));
    }

    let output = Command::new("sudo")
        .args(["-n", "journalctl"])
        .args(&args)
        .output()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to read logs: {}", e)})),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries: Vec<LogEntry> = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(line) {
            let entry = LogEntry {
                timestamp: parsed
                    .get("__REALTIME_TIMESTAMP")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                unit: parsed
                    .get("_SYSTEMD_UNIT")
                    .or_else(|| parsed.get("SYSLOG_IDENTIFIER"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                priority: parsed
                    .get("PRIORITY")
                    .and_then(|v| v.as_str())
                    .unwrap_or("6")
                    .to_string(),
                message: parsed
                    .get("MESSAGE")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            };

            // Apply search filter if provided
            if let Some(ref search) = query.search {
                if !entry.message.to_lowercase().contains(&search.to_lowercase())
                    && !entry.unit.to_lowercase().contains(&search.to_lowercase())
                {
                    continue;
                }
            }

            entries.push(entry);
        }
    }

    Ok(Json(entries))
}

/// GET /api/logs/units
///
/// List available systemd units for log filtering.
pub async fn get_log_units(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<Value>)> {
    let output = Command::new("sudo")
        .args(["-n", "journalctl", "--field=_SYSTEMD_UNIT", "--no-pager"])
        .output()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to list units: {}", e)})),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let units: Vec<String> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect();

    Ok(Json(units))
}

/// GET /api/audit
///
/// Query audit logs with optional filters and pagination.
pub async fn get_audit_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let logs = audit::query_logs(&state.db, &query).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to query audit logs: {}", e)})),
        )
    })?;

    let total = audit::count_logs(&state.db, &query).await.unwrap_or(0);

    Ok(Json(json!({
        "logs": logs,
        "total": total,
        "page": query.page.unwrap_or(1),
        "per_page": query.per_page.unwrap_or(50)
    })))
}
