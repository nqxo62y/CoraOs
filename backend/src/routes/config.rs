//! Server configuration route handlers.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use sqlx::Row;

use crate::models::auth::AuthenticatedUser;
use crate::models::system::{ConfigEntry, UpdateConfigRequest};
use crate::services::audit;
use crate::AppState;

/// GET /api/config
pub async fn list_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ConfigEntry>>, (StatusCode, Json<Value>)> {
    let rows = sqlx::query("SELECT key, value, description, updated_at FROM server_config ORDER BY key")
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
        })?;

    let configs: Vec<ConfigEntry> = rows.iter().map(|row| ConfigEntry {
        key: row.get("key"),
        value: row.get("value"),
        description: row.get("description"),
        updated_at: row.get("updated_at"),
    }).collect();

    Ok(Json(configs))
}

/// POST /api/config
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required to modify configuration"})),
        ));
    }

    if payload.key.is_empty() || payload.key.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Configuration key must be 1-128 characters"})),
        ));
    }

    if payload.value.len() > 4096 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Configuration value must not exceed 4096 characters"})),
        ));
    }

    sqlx::query(
        "INSERT INTO server_config (key, value, description, updated_at, updated_by) \
         VALUES (?, ?, ?, datetime('now'), ?) \
         ON CONFLICT(key) DO UPDATE SET \
             value = excluded.value, \
             description = excluded.description, \
             updated_at = datetime('now'), \
             updated_by = excluded.updated_by"
    )
    .bind(&payload.key)
    .bind(&payload.value)
    .bind(&payload.description)
    .bind(&auth_user.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to update config: {}", e)})),
        )
    })?;

    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "config_update",
        Some(&payload.key),
        Some(&format!("Set to: {}", payload.value)),
        None,
    )
    .await;

    Ok(Json(json!({"message": format!("Configuration '{}' updated", payload.key)})))
}
