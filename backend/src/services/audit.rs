use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::models::audit::{AuditLog, AuditLogQuery};

pub async fn log_action(
    pool: &SqlitePool,
    user_id: Option<&str>,
    action: &str,
    resource: Option<&str>,
    details: Option<&str>,
    ip_address: Option<&str>,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, resource, details, ip_address) \
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(user_id)
    .bind(action)
    .bind(resource)
    .bind(details)
    .bind(ip_address)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn query_logs(pool: &SqlitePool, query: &AuditLogQuery) -> Result<Vec<AuditLog>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let offset = ((page - 1) * per_page) as i64;
    let limit = per_page as i64;

    let rows = match (&query.user_id, &query.action) {
        (Some(user_id), Some(action)) => {
            sqlx::query(
                "SELECT id, user_id, action, resource, details, ip_address, created_at \
                 FROM audit_logs WHERE user_id = ? AND action = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(user_id)
            .bind(action)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        (Some(user_id), None) => {
            sqlx::query(
                "SELECT id, user_id, action, resource, details, ip_address, created_at \
                 FROM audit_logs WHERE user_id = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        (None, Some(action)) => {
            sqlx::query(
                "SELECT id, user_id, action, resource, details, ip_address, created_at \
                 FROM audit_logs WHERE action = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(action)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query(
                "SELECT id, user_id, action, resource, details, ip_address, created_at \
                 FROM audit_logs ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };

    let logs = rows.iter().map(|row| AuditLog {
        id: row.get("id"),
        user_id: row.get("user_id"),
        action: row.get("action"),
        resource: row.get("resource"),
        details: row.get("details"),
        ip_address: row.get("ip_address"),
        created_at: row.get("created_at"),
    }).collect();

    Ok(logs)
}

pub async fn count_logs(pool: &SqlitePool, query: &AuditLogQuery) -> Result<i64> {
    let count: (i64,) = match (&query.user_id, &query.action) {
        (Some(user_id), Some(action)) => {
            sqlx::query_as(
                "SELECT COUNT(*) FROM audit_logs WHERE user_id = ? AND action = ?",
            )
            .bind(user_id)
            .bind(action)
            .fetch_one(pool)
            .await?
        }
        (Some(user_id), None) => {
            sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await?
        }
        (None, Some(action)) => {
            sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE action = ?")
                .bind(action)
                .fetch_one(pool)
                .await?
        }
        (None, None) => {
            sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
                .fetch_one(pool)
                .await?
        }
    };

    Ok(count.0)
}
