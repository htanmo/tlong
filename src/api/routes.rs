use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

use crate::state::AppState;

use super::handlers;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/{short_code}", get(handlers::handle_short_url))
        .route("/api/v1/health", get(handlers::health_check))
        .route("/api/v1/shorten", post(handlers::create_short_url))
        .route("/api/v1/shorten", get(handlers::get_all_short_url))
        .route("/api/v1/{short_code}", delete(handlers::delete_short_url))
        .route("/api/v1/{short_code}", get(handlers::get_short_url_details))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err| async move {
                    tracing::error!("Internal error: {}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "An unexpected error occurred. Please try again later.".to_string(),
                    )
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(200, Duration::from_secs(1))),
        )
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
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .with_state(state)
}
