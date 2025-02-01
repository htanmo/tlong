use std::{env, process};

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{error, info, level_filters::LevelFilter, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod handlers;
mod types;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_env_var("APP_LOG")
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    // log file config
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/var/log/tlong", "tlong.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // logger
    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(fmt::layer().json().with_writer(non_blocking_writer))
        .with(filter)
        .init();

    // database address
    let db_url = match env::var("DATABASE_URL") {
        Ok(var) => var,
        Err(_) => {
            error!("DATABASE_URL environment variable is required but not set.");
            process::exit(1);
        }
    };

    // database config
    let db = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            error!("Failed to connect to database: {e}");
            process::exit(1);
        }
    };

    // run pending migrations
    match sqlx::migrate!().run(&db).await {
        Ok(_) => {
            info!("Migration applied successfully...");
        }
        Err(e) => {
            error!("Migration failed: {e}");
            process::exit(1);
        }
    }

    // app routes
    let app = Router::new()
        .route("/{short_code}", get(handlers::handle_short_url))
        .route("/api/v1/health", get(handlers::health_check))
        .route("/api/v1/shorten", post(handlers::create_short_url))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .include_headers(false)
                        .level(Level::INFO),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .latency_unit(LatencyUnit::Millis)
                        .include_headers(false)
                        .level(Level::INFO),
                )
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .with_state(db);

    // server address
    let address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| {
        warn!("SERVER_ADDRESS environment variable not set, using default address.");
        "127.0.0.1:8080".to_string()
    });
    info!("Starting server on {address}");

    // running the server on the above address
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(_) => {
            error!("Failed to bind to address.");
            process::exit(1);
        }
    };

    // serve the application
    let server = axum::serve(listener, app);
    tokio::select! {
        _ = server => {}
        _ = signal::ctrl_c() => {
            info!("Shutting down server...");
        }
    }

    info!("Server stopped.");
}
