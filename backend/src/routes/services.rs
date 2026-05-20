//! Service management route handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::models::auth::AuthenticatedUser;
use crate::models::system::{ServiceActionRequest, ServiceInfo};
use crate::services::{audit, services_manager};
use crate::AppState;

/// GET /api/services
///
/// List all systemd services.
pub async fn list_services(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<ServiceInfo>>, (StatusCode, Json<Value>)> {
    services_manager::list_services()
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to list services: {}", e)})),
            )
        })
}

/// GET /api/services/:name
///
/// Get detailed status of a specific service.
pub async fn get_service(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServiceInfo>, (StatusCode, Json<Value>)> {
    services_manager::get_service_status(&name)
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Service not found: {}", e)})),
            )
        })
}

/// POST /api/services/:name/action
///
/// Execute an action (start/stop/restart/enable/disable) on a service.
/// Requires operator or admin role.
pub async fn service_action(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(name): Path<String>,
    Json(payload): Json<ServiceActionRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check permission - only operators and admins can manage services
    if auth_user.role == "viewer" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Operator or admin access required to manage services"})),
        ));
    }

    let result = services_manager::execute_service_action(&name, &payload.action).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", e)})),
        )
    })?;

    // Audit log the action
    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        &format!("service_{}", payload.action.as_str()),
        Some(&name),
        Some(&result),
        None,
    )
    .await;

    Ok(Json(json!({"message": result})))
}
