use std::env;

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;

mod handlers;
mod types;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();

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
        .route("/api/v1/shorten", post(handlers::create_short_url))
        .with_state(db);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
