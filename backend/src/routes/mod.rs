//! API route definitions.
//!
//! Organizes all HTTP endpoints into logical groups.

pub mod auth;
pub mod backup;
pub mod config;
pub mod logs;
pub mod services;
pub mod system;
pub mod updates;
pub mod users;
pub mod websocket;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::middleware::auth::require_auth;
use crate::AppState;

/// Build the complete API router with all route groups.
pub fn api_router(state: Arc<AppState>) -> Router {
    let public_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/health", get(health_check));

    let authenticated_routes = Router::new()
        // Auth
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        // System monitoring
        .route("/system/metrics", get(system::get_metrics))
        .route("/system/processes", get(system::get_processes))
        // Services (operator+)
        .route("/services", get(services::list_services))
        .route("/services/:name", get(services::get_service))
        .route("/services/:name/action", post(services::service_action))
        // Logs
        .route("/logs", get(logs::get_logs))
        .route("/logs/units", get(logs::get_log_units))
        // Configuration
        .route("/config", get(config::list_config))
        .route("/config", post(config::update_config))
        // Updates
        .route("/updates/check", get(updates::check_updates))
        .route("/updates/apply", post(updates::apply_updates))
        // Backups
        .route("/backups", get(backup::list_backups))
        .route("/backups", post(backup::create_backup))
        .route("/backups/:id", axum::routing::delete(backup::delete_backup))
        // Users (admin only)
        .route("/users", get(users::list_users))
        .route("/users", post(users::create_user))
        .route("/users/:id", get(users::get_user))
        .route("/users/:id", axum::routing::put(users::update_user))
        .route("/users/:id", axum::routing::delete(users::delete_user))
        // Audit logs
        .route("/audit", get(logs::get_audit_logs))
        // WebSocket for real-time updates
        .route("/ws", get(websocket::ws_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    Router::new()
        .merge(public_routes)
        .merge(authenticated_routes)
        .with_state(state)
}

/// Simple health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}
