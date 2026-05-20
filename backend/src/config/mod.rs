pub mod database;
pub mod logging;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub backup_path: String,
    pub log_path: String,
    pub frontend_path: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let port: u16 = std::env::var("CORAOS_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);

        let base = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let data_dir = base.join("data");
        let backup_dir = base.join("backups");
        let log_dir = base.join("logs");
        let frontend_dir = base.join("frontend").join("dist");

        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&backup_dir);
        let _ = std::fs::create_dir_all(&log_dir);

        let db_path = data_dir.join("coraos.db");
        let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let jwt_secret = load_or_generate_secret(&data_dir);

        let config = AppConfig {
            host: "0.0.0.0".to_string(),
            port,
            database_url,
            jwt_secret,
            jwt_expiration_hours: 24,
            backup_path: backup_dir.to_string_lossy().to_string(),
            log_path: log_dir.to_string_lossy().to_string(),
            frontend_path: frontend_dir.to_string_lossy().to_string(),
        };

        Ok(config)
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn load_or_generate_secret(data_dir: &std::path::Path) -> String {
    let secret_file = data_dir.join(".jwt_secret");

    if let Ok(secret) = std::fs::read_to_string(&secret_file) {
        let trimmed = secret.trim().to_string();
        if trimmed.len() >= 32 {
            return trimmed;
        }
    }

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret: String = (0..64).map(|_| format!("{:02x}", rng.gen::<u8>())).collect();

    let _ = std::fs::write(&secret_file, &secret);

    secret
}
