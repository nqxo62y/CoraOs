//! Backup management route handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::models::auth::AuthenticatedUser;
use crate::models::backup::CreateBackupRequest;
use crate::services::{audit, backup as backup_service};
use crate::AppState;

/// GET /api/backups
///
/// List all backups.
pub async fn list_backups(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let backups = backup_service::list_backups(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to list backups: {}", e)})),
        )
    })?;

    Ok(Json(json!({"backups": backups})))
}

/// POST /api/backups
///
/// Create a new backup. Requires operator or admin role.
pub async fn create_backup(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<CreateBackupRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Only operators and admins can create backups
    if auth_user.role == "viewer" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Operator access required to create backups"})),
        ));
    }

    // Validate input
    if payload.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Backup name is required"})),
        ));
    }

    if payload.include_paths.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "At least one path to backup is required"})),
        ));
    }

    let backup = backup_service::create_backup(
        &state.db,
        &payload.name,
        &payload.include_paths,
        &state.config.backup_path,
        &auth_user.user_id,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Backup failed: {}", e)})),
        )
    })?;

    // Audit log
    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "backup_create",
        Some(&backup.name),
        Some(&format!("Size: {} bytes", backup.size_bytes)),
        None,
    )
    .await;

    Ok(Json(json!({"backup": backup})))
}

/// DELETE /api/backups/:id
///
/// Delete a backup. Requires admin role.
pub async fn delete_backup(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required to delete backups"})),
        ));
    }

    backup_service::delete_backup(&state.db, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to delete backup: {}", e)})),
            )
        })?;

    // Audit log
    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "backup_delete",
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(Json(json!({"message": "Backup deleted successfully"})))
}
