//! User model and related types.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// User roles for access control.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Operator,
    Viewer,
}

impl UserRole {
    pub fn as_str(&self) -> &str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Operator => "operator",
            UserRole::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => UserRole::Admin,
            "operator" => UserRole::Operator,
            _ => UserRole::Viewer,
        }
    }

    /// Check if this role has at least the given permission level.
    pub fn has_permission(&self, required: &UserRole) -> bool {
        let self_level = self.level();
        let required_level = required.level();
        self_level >= required_level
    }

    fn level(&self) -> u8 {
        match self {
            UserRole::Viewer => 1,
            UserRole::Operator => 2,
            UserRole::Admin => 3,
        }
    }
}

/// Database representation of a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login: Option<String>,
}

/// Public user data returned in API responses (no password hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub last_login: Option<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            role: user.role,
            display_name: user.display_name,
            email: user.email,
            is_active: user.is_active,
            created_at: user.created_at,
            last_login: user.last_login,
        }
    }
}

/// Request body for creating a new user.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 32, message = "Username must be 3-32 characters"))]
    pub username: String,
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
    pub role: Option<String>,
    #[validate(length(max = 64))]
    pub display_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
}

/// Request body for updating a user.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    #[validate(length(max = 64))]
    pub display_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    pub is_active: Option<bool>,
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: Option<String>,
}
