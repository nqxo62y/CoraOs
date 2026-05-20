//! System monitoring route handlers.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::models::system::{ProcessInfo, SystemMetrics};
use crate::AppState;

/// GET /api/system/metrics
///
/// Returns current system metrics including CPU, memory, disk, and network usage.
pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<SystemMetrics> {
    let metrics = state.monitor.get_metrics();
    Json(metrics)
}

/// GET /api/system/processes
///
/// Returns a list of running processes sorted by CPU usage.
pub async fn get_processes(State(state): State<Arc<AppState>>) -> Json<Vec<ProcessInfo>> {
    let processes = state.monitor.get_processes();
    Json(processes)
}
