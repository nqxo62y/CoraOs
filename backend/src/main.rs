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

pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub monitor: SystemMonitor,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    config::logging::init_logging();

    info!("CoraOS Server Management Platform starting...");

    let config = AppConfig::load()?;
    let bind_addr = config.bind_address();

    let db = config::database::init_database(&config.database_url).await?;
    info!("Database initialized successfully");

    services::auth::seed_default_admin(&db).await?;

    let monitor = SystemMonitor::new();

    let state = Arc::new(AppState {
        db,
        config,
        monitor,
    });

    let app = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("CoraOS listening on http://{}", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = routes::api_router(state.clone());

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new(&state.config.frontend_path))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
}
