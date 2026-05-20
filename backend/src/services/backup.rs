use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::process::Command;
use tracing::{error, info};
use uuid::Uuid;

use crate::models::backup::{Backup, BackupStatus};

pub async fn create_backup(
    pool: &SqlitePool,
    name: &str,
    include_paths: &[String],
    backup_dir: &str,
    user_id: &str,
) -> Result<Backup> {
    if name.is_empty() || name.len() > 128 {
        return Err(anyhow!("Backup name must be 1-128 characters"));
    }

    std::fs::create_dir_all(backup_dir)?;

    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let safe_name = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    let filename = format!("{}_{}.tar.gz", safe_name, timestamp);
    let file_path = format!("{}/{}", backup_dir, filename);

    sqlx::query(
        "INSERT INTO backups (id, name, file_path, size_bytes, backup_type, status, created_by) \
         VALUES (?, ?, ?, 0, 'manual', 'in_progress', ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(&file_path)
    .bind(user_id)
    .execute(pool)
    .await?;

    for path in include_paths {
        let p = Path::new(path);
        if !p.exists() {
            update_backup_status(pool, &id, BackupStatus::Failed).await?;
            return Err(anyhow!("Path does not exist: {}", path));
        }
        if path.contains("..") {
            update_backup_status(pool, &id, BackupStatus::Failed).await?;
            return Err(anyhow!("Path traversal not allowed: {}", path));
        }
    }

    let mut args = vec!["-czf".to_string(), file_path.clone()];
    for path in include_paths {
        args.push(path.clone());
    }

    let output = Command::new("tar").args(&args).output();

    match output {
        Ok(result) if result.status.success() => {
            let size = std::fs::metadata(&file_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            sqlx::query("UPDATE backups SET size_bytes = ?, status = 'completed' WHERE id = ?")
                .bind(size)
                .bind(&id)
                .execute(pool)
                .await?;

            info!("Backup '{}' created successfully: {} bytes", name, size);

            Ok(Backup {
                id,
                name: name.to_string(),
                file_path,
                size_bytes: size,
                backup_type: "manual".to_string(),
                status: "completed".to_string(),
                created_by: Some(user_id.to_string()),
                created_at: Utc::now().to_rfc3339(),
            })
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            error!("Backup failed: {}", stderr);
            update_backup_status(pool, &id, BackupStatus::Failed).await?;
            Err(anyhow!("Backup creation failed: {}", stderr))
        }
        Err(e) => {
            error!("Failed to execute tar: {}", e);
            update_backup_status(pool, &id, BackupStatus::Failed).await?;
            Err(anyhow!("Failed to execute tar command: {}", e))
        }
    }
}

pub async fn list_backups(pool: &SqlitePool) -> Result<Vec<Backup>> {
    let rows = sqlx::query(
        "SELECT id, name, file_path, size_bytes, backup_type, status, created_by, created_at \
         FROM backups ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    let backups = rows.iter().map(|row| Backup {
        id: row.get("id"),
        name: row.get("name"),
        file_path: row.get("file_path"),
        size_bytes: row.get("size_bytes"),
        backup_type: row.get("backup_type"),
        status: row.get("status"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
    }).collect();

    Ok(backups)
}

pub async fn delete_backup(pool: &SqlitePool, backup_id: &str) -> Result<()> {
    let row = sqlx::query(
        "SELECT file_path, name FROM backups WHERE id = ?"
    )
    .bind(backup_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("Backup not found"))?;

    let file_path: String = row.get("file_path");
    let name: String = row.get("name");

    let path = Path::new(&file_path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    sqlx::query("DELETE FROM backups WHERE id = ?")
        .bind(backup_id)
        .execute(pool)
        .await?;

    info!("Backup '{}' deleted", name);
    Ok(())
}

async fn update_backup_status(pool: &SqlitePool, id: &str, status: BackupStatus) -> Result<()> {
    sqlx::query("UPDATE backups SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
