use std::env;

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, level_filters::LevelFilter};
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
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable is required but not set");

    // database config
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("failed to connect to database");

    // app routes
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/{short_code}", get(handlers::handle_short_url))
        .route("/api/v1/shorten", post(handlers::create_short_url))
        .with_state(db);

    // server address
    let address = "0.0.0.0:3000";
    info!("Starting server on {address}");

    // running the server on the above address
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
