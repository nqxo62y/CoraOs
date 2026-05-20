//! Authentication middleware.
//!
//! Extracts and validates JWT tokens from request headers, providing
//! the authenticated user context to route handlers.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::models::auth::AuthenticatedUser;
use crate::services::auth::validate_token;
use crate::AppState;

/// Extract the authenticated user from the Authorization header.
/// Returns 401 if the token is missing or invalid.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header {
        Some(ref header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing or invalid authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            let user = AuthenticatedUser {
                user_id: claims.sub,
                username: claims.username,
                role: claims.role,
            };
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}

/// Middleware that requires the user to have admin role.
pub async fn require_admin(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let user = request.extensions().get::<AuthenticatedUser>().cloned();

    match user {
        Some(ref u) if u.role == "admin" => next.run(request).await,
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required"})),
        )
            .into_response(),
    }
}

/// Middleware that requires operator or admin role.
pub async fn require_operator(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let user = request.extensions().get::<AuthenticatedUser>().cloned();

    match user {
        Some(ref u) if u.role == "admin" || u.role == "operator" => next.run(request).await,
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Operator access required"})),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required"})),
        )
            .into_response(),
    }
}
