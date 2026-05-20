//! CoraOS Backend - Production-grade Debian Linux server management platform
//!
//! Entry point that initializes the application, database, and HTTP server.

mod config;
mod middleware;
mod models;
mod routes;
mod services;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::AppConfig;
use crate::services::monitor::SystemMonitor;

/// Shared application state accessible from all route handlers.
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub monitor: SystemMonitor,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env if present
    dotenvy::dotenv().ok();

    // Initialize logging
    config::logging::init_logging();

    info!("CoraOS Server Management Platform starting...");

    // Load configuration
    let config = AppConfig::load()?;
    let bind_addr = config.bind_address();

    // Initialize database
    let db = config::database::init_database(&config.database_url).await?;
    info!("Database initialized successfully");

    // Seed default admin user if no users exist
    services::auth::seed_default_admin(&db).await?;

    // Initialize system monitor
    let monitor = SystemMonitor::new();

    // Build shared state
    let state = Arc::new(AppState {
        db,
        config,
        monitor,
    });

    // Build the application router
    let app = build_router(state.clone());

    // Start the server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("CoraOS listening on http://{}", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Constructs the full application router with all routes and middleware.
fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = routes::api_router(state.clone());

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
}
