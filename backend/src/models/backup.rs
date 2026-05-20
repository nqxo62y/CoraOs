//! Backup management models.

use serde::{Deserialize, Serialize};

/// Backup record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub backup_type: String,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

/// Request to create a new backup.
#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub name: String,
    /// Directories to include in the backup.
    pub include_paths: Vec<String>,
}

/// Backup status values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl BackupStatus {
    pub fn as_str(&self) -> &str {
        match self {
            BackupStatus::Pending => "pending",
            BackupStatus::InProgress => "in_progress",
            BackupStatus::Completed => "completed",
            BackupStatus::Failed => "failed",
        }
    }
}
