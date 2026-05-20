use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::models::auth::AuthenticatedUser;
use crate::services::{audit, updates as updates_service};
use crate::AppState;

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

pub async fn apply_updates(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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
