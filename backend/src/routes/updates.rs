//! System update route handlers.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::models::auth::AuthenticatedUser;
use crate::services::{audit, updates as updates_service};
use crate::AppState;

/// GET /api/updates/check
///
/// Check for available system package updates.
pub async fn check_updates(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let updates = updates_service::check_updates().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to check updates: {}", e)})),
        )
    })?;

    Ok(Json(json!({
        "updates": updates,
        "count": updates.len()
    })))
}

/// POST /api/updates/apply
///
/// Apply all available system updates. Requires admin role.
pub async fn apply_updates(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Only admins can apply updates
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required to apply updates"})),
        ));
    }

    let result = updates_service::apply_updates().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to apply updates: {}", e)})),
        )
    })?;

    // Audit log
    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "system_update",
        Some("packages"),
        Some("Applied system updates"),
        None,
    )
    .await;

    Ok(Json(json!({"message": "Updates applied successfully", "output": result})))
}
