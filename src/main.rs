use std::{env, process};

use dotenvy::dotenv;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use tokio::signal;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod api;
mod config;
mod db;
mod state;
mod types;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Configure logging
    let filter = EnvFilter::builder()
        .with_env_var("APP_LOG")
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "log/".to_string());

    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "tlong.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(fmt::layer().json().with_writer(non_blocking_writer))
        .with(filter)
        .init();

    // App configuration
    let config = config::Config::load();

    // Postgres
    let pg_db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to connect to database: {e}");
            process::exit(1);
        });

    // Run database migrations
    if let Err(e) = sqlx::migrate!().run(&pg_db).await {
        error!("Migration failed: {e}");
        process::exit(1);
    }
    info!("Database migrations applied successfully.");

    // Redis
    let client = Client::open(config.redis_url).unwrap_or_else(|e| {
        error!("Failed to create redis database connection: {e}");
        process::exit(1);
    });
    let redis_db = r2d2::Pool::builder()
        .max_size(25)
        .build(client)
        .unwrap_or_else(|e| {
            error!("Failed to connect to redis database: {e}");
            process::exit(1);
        });

    // Application state
    let state = AppState::new(pg_db, redis_db, config.base_url);

    // Build the application router
    let app = api::routes::router(state);

    info!("Starting server on {}", &config.server_addr);

    // Server configuration
    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind to address: {e}");
            process::exit(1);
        });

    // Start the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            error!("Server error: {e}");
            process::exit(1);
        });

    info!("Server stopped.");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
