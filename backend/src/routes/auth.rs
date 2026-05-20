//! Authentication route handlers.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use sqlx::Row;
use validator::Validate;

use crate::models::auth::{AuthenticatedUser, LoginRequest, LoginResponse};
use crate::models::user::{User, UserResponse};
use crate::services::{audit, auth as auth_service};
use crate::AppState;

/// POST /api/auth/login
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<Value>)> {
    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Validation failed", "details": errors.to_string()})),
        ));
    }

    let user = auth_service::authenticate_user(&state.db, &payload.username, &payload.password)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Authentication error: {}", e)})),
            )
        })?;

    match user {
        Some(user) => {
            let (token, expires_at) = auth_service::generate_token(
                &user.id,
                &user.username,
                &user.role,
                &state.config.jwt_secret,
                state.config.jwt_expiration_hours,
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Token generation failed: {}", e)})),
                )
            })?;

            let _ = audit::log_action(
                &state.db,
                Some(&user.id),
                "login",
                Some("auth"),
                Some("Successful login"),
                None,
            )
            .await;

            let response = LoginResponse {
                token,
                user: UserResponse::from(user),
                expires_at,
            };

            Ok(Json(response))
        }
        None => {
            let _ = audit::log_action(
                &state.db,
                None,
                "login_failed",
                Some("auth"),
                Some(&format!("Failed login for username: {}", payload.username)),
                None,
            )
            .await;

            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid username or password"})),
            ))
        }
    }
}

/// GET /api/auth/me
pub async fn me(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<UserResponse>, (StatusCode, Json<Value>)> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, email, \
         is_active, created_at, updated_at, last_login FROM users WHERE id = ?"
    )
    .bind(&auth_user.user_id)
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

/// POST /api/auth/logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> Json<Value> {
    let _ = audit::log_action(
        &state.db,
        Some(&auth_user.user_id),
        "logout",
        Some("auth"),
        None,
        None,
    )
    .await;

    Json(json!({"message": "Logged out successfully"}))
}
