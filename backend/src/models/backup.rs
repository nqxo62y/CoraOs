use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub name: String,
    pub include_paths: Vec<String>,
}

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
