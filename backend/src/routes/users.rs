//! User management route handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::models::auth::AuthenticatedUser;
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User, UserResponse};
use crate::services::{audit, auth as auth_service};
use crate::AppState;

/// GET /api/users
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        ));
    }

    let rows = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, email, \
         is_active, created_at, updated_at, last_login FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
    })?;

    let users: Vec<UserResponse> = rows.iter().map(|row| {
        let user = User {
            id: row.get("id"),
            username: row.get("username"),
            password_hash: row.get("password_hash"),
            role: row.get("role"),
            display_name: row.get("display_name"),
            email: row.get("email"),
            is_active: row.get::<i32, _>("is_active") != 0,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_login: row.get("last_login"),
        };
        UserResponse::from(user)
    }).collect();

    Ok(Json(users))
}

/// GET /api/users/:id
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" && auth_user.user_id != id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        ));
    }

    let row = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, email, \
         is_active, created_at, updated_at, last_login FROM users WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)})),
        )
    })?;

    match row {
        Some(row) => {
            let user = User {
                id: row.get("id"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                role: row.get("role"),
                display_name: row.get("display_name"),
                email: row.get("email"),
                is_active: row.get::<i32, _>("is_active") != 0,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_login: row.get("last_login"),
            };
            Ok(Json(UserResponse::from(user)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        )),
    }
}

/// POST /api/users
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required to create users"})),
        ));
    }

    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Validation failed", "details": errors.to_string()})),
        ));
    }

    let existing = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": "Username already exists"})),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = auth_service::hash_password(&payload.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Password hashing failed: {}", e)})),
        )
    })?;
    let role = payload.role.as_deref().unwrap_or("viewer");

    if !["admin", "operator", "viewer"].contains(&role) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid role. Must be: admin, operator, or viewer"})),
        ));
    }

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, display_name, email) \
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(role)
    .bind(&payload.display_name)
    .bind(&payload.email)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create user: {}", e)})),
        )
    })?;

    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "user_create",
        Some(&payload.username),
        Some(&format!("Role: {}", role)),
        None,
    )
    .await;

    let response = UserResponse {
        id,
        username: payload.username,
        role: role.to_string(),
        display_name: payload.display_name,
        email: payload.email,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_login: None,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// PUT /api/users/:id
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" && auth_user.user_id != id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        ));
    }

    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Validation failed", "details": errors.to_string()})),
        ));
    }

    let existing = sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database error: {}", e)})),
            )
        })?;

    if existing.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ));
    }

    if let Some(ref role) = payload.role {
        if !["admin", "operator", "viewer"].contains(&role.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid role"})),
            ));
        }
        sqlx::query("UPDATE users SET role = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(role)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;
    }

    if let Some(ref display_name) = payload.display_name {
        sqlx::query("UPDATE users SET display_name = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(display_name)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;
    }

    if let Some(ref email) = payload.email {
        sqlx::query("UPDATE users SET email = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(email)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;
    }

    if let Some(is_active) = payload.is_active {
        sqlx::query("UPDATE users SET is_active = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(is_active)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;
    }

    if let Some(ref password) = payload.password {
        let hash = auth_service::hash_password(password).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)})))
        })?;
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&hash)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{}", e)}))))?;
    }

    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "user_update",
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(Json(json!({"message": "User updated successfully"})))
}

/// DELETE /api/users/:id
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if auth_user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        ));
    }

    if auth_user.user_id == id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Cannot delete your own account"})),
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to delete user: {}", e)})),
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ));
    }

    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "user_delete",
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(Json(json!({"message": "User deleted successfully"})))
}
