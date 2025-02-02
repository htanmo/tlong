use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{debug, error, info};

use crate::{
    models::UrlDetail,
    types::{ShortenRequest, ShortenResponse, UrlDetailResponse},
    utils::{encode_long_url, valid_short_code, valid_url},
};

pub async fn health_check() -> (StatusCode, Json<Value>) {
    let response = json!({
        "status": "ok",
        "version": "1.0.0",
    });
    (StatusCode::OK, Json(response))
}

pub async fn create_short_url(
    State(pool): State<PgPool>,
    payload: Result<Json<ShortenRequest>, JsonRejection>,
) -> impl IntoResponse {
    let payload = match payload {
        Ok(payload) => payload.0,
        Err(rejection) => {
            let error_message = match rejection {
                JsonRejection::MissingJsonContentType(_) => {
                    json!({"error": "Expected 'Content-Type: application/json' header"})
                }
                JsonRejection::JsonSyntaxError(_) => json!({"error": "JSON syntax error"}),
                JsonRejection::JsonDataError(_) => json!({"error": "JSON data structure mismatch"}),
                _ => json!({"error": "Unknown JSON parsing error"}),
            };
            error!("JSON parsing error: {:?}", rejection);
            return (StatusCode::BAD_REQUEST, Json(error_message)).into_response();
        }
    };

    // Validate the long URL
    if !valid_url(&payload.long_url) {
        error!("Invalid URL format: {}", payload.long_url);
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid URL format"})),
        )
            .into_response();
    }

    // Generate short code
    let short_code = encode_long_url(&payload.long_url).await[0..8].to_string();
    debug!("Generated short code: {}", short_code);

    // Insert into database
    let query = sqlx::query(
        "INSERT INTO urls (long_url, short_code) VALUES ($1, $2) ON CONFLICT (short_code) DO NOTHING",
    )
    .bind(&payload.long_url)
    .bind(&short_code);

    match query.execute(&pool).await {
        Ok(_) => {
            let short_url = format!("http://0.0.0.0:3000/{}", short_code);
            info!("Created short URL: {}", short_url);
            let response = ShortenResponse {
                long_url: payload.long_url,
                short_url,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            error!("Database error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create short URL"})),
            )
                .into_response()
        }
    }
}

pub async fn handle_short_url(
    State(pool): State<PgPool>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    // Short code validation
    if !valid_short_code(&short_code) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Fetch long url from database
    let query = r#"
        SELECT long_url
        FROM urls
        WHERE short_code = $1
    "#;
    let result: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(query)
        .bind(&short_code)
        .fetch_optional(&pool)
        .await;

    match result {
        Ok(data) => match data {
            Some(long_url) => {
                // Redirect
                info!("Redirecting to long URL: {}", long_url);
                Redirect::permanent(&long_url).into_response()
            }
            None => {
                error!("Short code not found: {}", short_code);
                StatusCode::NOT_FOUND.into_response()
            }
        },
        Err(e) => {
            error!("Database error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_short_url(
    State(pool): State<PgPool>,
    Path(short_code): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    if !valid_short_code(&short_code) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Checking if the short code exists
    let result: Option<String> = sqlx::query_scalar(
        "
        DELETE FROM urls
        WHERE short_code = $1
        RETURNING short_code
        ",
    )
    .bind(&short_code)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match result {
        Some(_) => Ok(Json(json!({"message": "short url deleted successfully"}))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn get_all_short_url(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<UrlDetailResponse>>, StatusCode> {
    // Fetching all the urls from the database
    let results = sqlx::query_as::<_, UrlDetail>(
        "
        SELECT short_code, long_url, created_at
        FROM urls
        ORDER BY created_at DESC
        ",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<UrlDetailResponse> = results
        .into_iter()
        .map(|row| UrlDetailResponse {
            short_url: format!("http:0.0.0.0:8080/{}", row.short_code),
            long_url: row.long_url,
            created_at: row.created_at.to_string(),
        })
        .collect();

    Ok(Json(response))
}
