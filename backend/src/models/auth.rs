//! Authentication-related models and types.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Login request payload.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Successful login response.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: super::user::UserResponse,
    pub expires_at: String,
}

/// JWT claims embedded in the token.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Expiration timestamp (Unix epoch seconds)
    pub exp: usize,
    /// Issued at timestamp
    pub iat: usize,
}

/// Authenticated user context extracted from JWT middleware.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}
