use std::{env, process};

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;

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

    // logging
    tracing_subscriber::fmt().with_env_filter(filter).init();

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
        .route("/api/v1/shorten", post(handlers::create_short_url))
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
            info!("Shutting down gracefully...");
        }
    }

    info!("Server stopped.");
}
