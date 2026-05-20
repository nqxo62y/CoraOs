//! Application configuration module.
//!
//! Handles loading configuration from environment variables and config files.

pub mod database;
pub mod logging;

use anyhow::Result;
use serde::Deserialize;

/// Main application configuration loaded from environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Server bind host (default: 0.0.0.0 for LAN access)
    pub host: String,
    /// Server bind port (default: 8080)
    pub port: u16,
    /// SQLite database URL
    pub database_url: String,
    /// JWT secret key for token signing
    pub jwt_secret: String,
    /// JWT token expiration in hours (default: 24)
    pub jwt_expiration_hours: u64,
    /// Path to store backup files
    pub backup_path: String,
    /// Path to store log files
    pub log_path: String,
    /// Path to the frontend static files directory
    pub frontend_path: String,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn load() -> Result<Self> {
        let config = AppConfig {
            host: std::env::var("CORAOS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("CORAOS_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:../data/coraos.db?mode=rwc".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| generate_default_secret()),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            backup_path: std::env::var("BACKUP_PATH")
                .unwrap_or_else(|_| "../backups".to_string()),
            log_path: std::env::var("LOG_PATH")
                .unwrap_or_else(|_| "../logs".to_string()),
            frontend_path: std::env::var("FRONTEND_PATH")
                .unwrap_or_else(|_| "../frontend/dist".to_string()),
        };

        Ok(config)
    }

    /// Returns the socket address to bind the server to.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Generates a random secret for JWT signing when none is configured.
/// In production, always set JWT_SECRET environment variable.
fn generate_default_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
    hex::encode(secret)
}

// Simple hex encoding without external dependency
mod hex {
    pub fn encode(bytes: Vec<u8>) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
