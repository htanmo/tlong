use std::{env, process, time::Duration};

use axum::{
    routing::{delete, get, post},
    Router,
};
use dotenvy::dotenv;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use tokio::signal;
use tower_http::{
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{error, info, level_filters::LevelFilter, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod handlers;
mod models;
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

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/var/log/tlong", "tlong.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(fmt::layer().json().with_writer(non_blocking_writer))
        .with(filter)
        .init();

    // Database configuration
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("DATABASE_URL environment variable is required but not set.");
        process::exit(1);
    });

    // Postgres
    let pg_db = PgPoolOptions::new()
        .max_connections(50)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url)
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
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| {
        error!("REDIS_URL environment variable is required but not set.");
        process::exit(1);
    });
    let client = Client::open(redis_url).unwrap_or_else(|e| {
        error!("Failed to create redis database connection: {e}");
        process::exit(1);
    });
    let redis_db = r2d2::Pool::builder()
        .max_size(15)
        .build(client)
        .unwrap_or_else(|e| {
            error!("Failed to connect to redis database: {e}");
            process::exit(1);
        });

    // Server address
    let address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| {
        warn!("SERVER_ADDRESS environment variable not set, using default address.");
        "0.0.0.0:8080".to_string()
    });

    // Base url
    let base_url = env::var("BASE_URL").unwrap_or_else(|_| {
        warn!("BASE_URL environment variable not set, using SERVER_ADDRESS.");
        format!("http://{}", &address)
    });

    // Application state
    let state = AppState::new(pg_db, redis_db, base_url);

    // Build the application router
    let app = Router::new()
        .route("/{short_code}", get(handlers::handle_short_url))
        .route("/api/v1/health", get(handlers::health_check))
        .route("/api/v1/shorten", post(handlers::create_short_url))
        .route("/api/v1/shorten", get(handlers::get_all_short_url))
        .route("/api/v1/{short_code}", delete(handlers::delete_short_url))
        .route("/api/v1/{short_code}", get(handlers::get_short_url_details))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .latency_unit(LatencyUnit::Millis)
                        .level(Level::DEBUG),
                )
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(15)))
        .with_state(state);

    info!("Starting server on {}", &address);

    // Server configuration
    let listener = tokio::net::TcpListener::bind(address)
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
