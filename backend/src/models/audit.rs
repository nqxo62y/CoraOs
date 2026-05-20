//! Audit logging models.

use serde::{Deserialize, Serialize};

/// Audit log entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

/// Query parameters for filtering audit logs.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
}
