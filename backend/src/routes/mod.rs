pub mod auth;
pub mod backup;
pub mod config;
pub mod logs;
pub mod services;
pub mod storage;
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

pub fn api_router(state: Arc<AppState>) -> Router {
    let public_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/health", get(health_check));

    let authenticated_routes = Router::new()
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/system/metrics", get(system::get_metrics))
        .route("/system/processes", get(system::get_processes))
        .route("/services", get(services::list_services))
        .route("/services/:name", get(services::get_service))
        .route("/services/:name/action", post(services::service_action))
        .route("/logs", get(logs::get_logs))
        .route("/logs/units", get(logs::get_log_units))
        .route("/config", get(config::list_config))
        .route("/config", post(config::update_config))
        .route("/updates/check", get(updates::check_updates))
        .route("/updates/apply", post(updates::apply_updates))
        .route("/backups", get(backup::list_backups))
        .route("/backups", post(backup::create_backup))
        .route("/backups/:id", axum::routing::delete(backup::delete_backup))
        .route("/users", get(users::list_users))
        .route("/users", post(users::create_user))
        .route("/users/:id", get(users::get_user))
        .route("/users/:id", axum::routing::put(users::update_user))
        .route("/users/:id", axum::routing::delete(users::delete_user))
        .route("/audit", get(logs::get_audit_logs))
        .route("/storage/ftp", get(storage::ftp_status))
        .route("/storage/ftp/install", post(storage::ftp_install))
        .route("/storage/ftp/config", post(storage::ftp_update_config))
        .route("/storage/ftp/users", post(storage::ftp_add_user))
        .route("/storage/ftp/users/:name", axum::routing::delete(storage::ftp_delete_user))
        .route("/storage/raid", get(storage::raid_list))
        .route("/storage/raid", post(storage::raid_create))
        .route("/storage/raid/:name", axum::routing::delete(storage::raid_delete))
        .route("/storage/disks", get(storage::list_block_devices))
        .route("/storage/mounts", get(storage::list_mounts))
        .route("/storage/mount", post(storage::mount_device))
        .route("/storage/unmount", post(storage::unmount_device))
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

async fn health_check() -> &'static str {
    "OK"
}
