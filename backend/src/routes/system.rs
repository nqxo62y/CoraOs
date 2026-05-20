use std::sync::Arc;

use axum::{extract::State, Json};

use crate::models::system::{ProcessInfo, SystemMetrics};
use crate::AppState;

pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<SystemMetrics> {
    let metrics = state.monitor.get_metrics();
    Json(metrics)
}

pub async fn get_processes(State(state): State<Arc<AppState>>) -> Json<Vec<ProcessInfo>> {
    let processes = state.monitor.get_processes();
    Json(processes)
}
