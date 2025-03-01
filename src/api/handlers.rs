use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use redis::Commands;
use serde_json::{json, Value};
use tracing::{debug, error, info, instrument};

use crate::{
    db::models::UrlDetail, state::AppState, types::{ShortenRequest, ShortenResponse, UrlDetailResponse}, utils::{encode_long_url, valid_short_code, valid_url}
};

#[instrument]
pub async fn health_check() -> (StatusCode, Json<Value>) {
    let response = json!({
        "status": "ok",
        "version": "1.0.0",
    });
    (StatusCode::OK, Json(response))
}

#[instrument(skip(state, payload))]
pub async fn create_short_url(
    State(state): State<AppState>,
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
            error!(error = ?rejection, "JSON parsing error");
            return (StatusCode::BAD_REQUEST, Json(error_message)).into_response();
        }
    };

    if !valid_url(&payload.long_url) {
        error!(url = %payload.long_url, "Invalid URL format");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid URL format"})),
        )
            .into_response();
    }

    let short_code = encode_long_url(&payload.long_url).await[0..8].to_string();
    debug!(short_code = %short_code, "Generated short code");

    let query = sqlx::query(
        "INSERT INTO urls (long_url, short_code) VALUES ($1, $2) ON CONFLICT (short_code) DO NOTHING",
    )
    .bind(&payload.long_url)
    .bind(&short_code);

    match query.execute(&state.pg_db).await {
        Ok(_) => {
            let short_url = format!("{}/{}", state.base_url, short_code);
            info!(short_url = %short_url, "Created short URL");
            let response = ShortenResponse {
                short_code,
                short_url,
                long_url: payload.long_url,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Database error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create short URL"})),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
pub async fn handle_short_url(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    if !valid_short_code(&short_code) {
        error!(short_code = %short_code, "Invalid short code");
        return StatusCode::BAD_REQUEST.into_response();
    }

    let mut redis_conn = match state.redis_db.get() {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = %e, "Failed to get Redis connection");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match redis_conn.get::<_, Option<String>>(&short_code) {
        Ok(Some(long_url)) => {
            info!(short_code = %short_code, "Cache hit");
            return Redirect::permanent(&long_url).into_response();
        }
        Ok(None) => {
            info!(short_code = %short_code, "Cache miss");
        }
        Err(e) => {
            error!(error = %e, "Redis error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let query = r#"
        SELECT long_url
        FROM urls
        WHERE short_code = $1
    "#;
    let result: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(query)
        .bind(&short_code)
        .fetch_optional(&state.pg_db)
        .await;

    match result {
        Ok(Some(long_url)) => {
            info!(short_code = %short_code, "Redirecting to long URL");
            if let Err(e) = redis_conn.set_ex::<_, _, ()>(&short_code, &long_url, 3600) {
                error!(error = %e, "Failed to cache URL in Redis");
            }
            Redirect::permanent(&long_url).into_response()
        }
        Ok(None) => {
            error!(short_code = %short_code, "Short code not found");
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            error!(error = %e, "Database error");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[instrument(skip(state))]
pub async fn delete_short_url(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    if !valid_short_code(&short_code) {
        error!(short_code = %short_code, "Invalid short code");
        return Err(StatusCode::BAD_REQUEST);
    }

    let result: Option<String> = sqlx::query_scalar(
        "
        DELETE FROM urls
        WHERE short_code = $1
        RETURNING short_code
        ",
    )
    .bind(&short_code)
    .fetch_optional(&state.pg_db)
    .await
    .map_err(|e| {
        error!(error = %e, "Database error");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match result {
        Some(_) => {
            info!(short_code = %short_code, "Short URL deleted successfully");
            Ok(Json(json!({"message": "short url deleted successfully"})))
        }
        None => {
            error!(short_code = %short_code, "Short code not found");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[instrument(skip(state))]
pub async fn get_all_short_url(
    State(state): State<AppState>,
) -> Result<Json<Vec<UrlDetailResponse>>, StatusCode> {
    let results = sqlx::query_as::<_, UrlDetail>(
        "
        SELECT short_code, long_url, created_at
        FROM urls
        ORDER BY created_at DESC
        ",
    )
    .fetch_all(&state.pg_db)
    .await
    .map_err(|e| {
        error!(error = %e, "Database error");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response: Vec<UrlDetailResponse> = results
        .into_iter()
        .map(|row| UrlDetailResponse {
            short_url: format!("{}/{}", state.base_url, &row.short_code),
            short_code: row.short_code,
            long_url: row.long_url,
            created_at: row.created_at.to_string(),
        })
        .collect();

    Ok(Json(response))
}

#[instrument(skip(state))]
pub async fn get_short_url_details(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<UrlDetailResponse>, StatusCode> {
    if !valid_short_code(&short_code) {
        error!(short_code = %short_code, "Invalid short code");
        return Err(StatusCode::BAD_REQUEST);
    }

    match sqlx::query_as::<_, UrlDetail>(
        "SELECT long_url, short_code, created_at FROM urls WHERE short_code = $1",
    )
    .bind(&short_code)
    .fetch_optional(&state.pg_db)
    .await
    {
        Ok(Some(detail)) => {
            let response = UrlDetailResponse {
                short_url: format!("{}/{}", state.base_url, &detail.short_code),
                short_code: detail.short_code,
                long_url: detail.long_url,
                created_at: detail.created_at.to_string(),
            };
            Ok(Json(response))
        }
        Ok(None) => {
            error!(short_code = %short_code, "Short code not found");
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!(error = %e, "Database error");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
