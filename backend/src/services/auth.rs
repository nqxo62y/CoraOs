//! Authentication service handling login, token generation, and password hashing.

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::{Row, SqlitePool};
use tracing::info;
use uuid::Uuid;

use crate::models::auth::Claims;
use crate::models::user::{User, UserRole};

/// Hash a plaintext password using Argon2id.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid hash format: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate a JWT token for an authenticated user.
pub fn generate_token(
    user_id: &str,
    username: &str,
    role: &str,
    secret: &str,
    expiration_hours: u64,
) -> Result<(String, String)> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(expiration_hours as i64);

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp: expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok((token, expires_at.to_rfc3339()))
}

/// Validate and decode a JWT token, returning the claims if valid.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Authenticate a user by username and password.
pub async fn authenticate_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<Option<User>> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, email, \
         is_active, created_at, updated_at, last_login \
         FROM users WHERE username = ? AND is_active = 1"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

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

            if verify_password(password, &user.password_hash)? {
                sqlx::query("UPDATE users SET last_login = datetime('now') WHERE id = ?")
                    .bind(&user.id)
                    .execute(pool)
                    .await?;
                Ok(Some(user))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Create the default admin user if no users exist in the database.
pub async fn seed_default_admin(pool: &SqlitePool) -> Result<()> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if row.0 == 0 {
        let id = Uuid::new_v4().to_string();
        let password_hash = hash_password("admin123")?;
        let role = UserRole::Admin.as_str();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, display_name) \
             VALUES (?, 'admin', ?, ?, 'Administrator')"
        )
        .bind(&id)
        .bind(&password_hash)
        .bind(role)
        .execute(pool)
        .await?;

        info!("Default admin user created (username: admin, password: admin123)");
        info!("IMPORTANT: Change the default password immediately after first login!");
    }

    Ok(())
}
