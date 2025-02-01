use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use tracing::{debug, error};

use crate::{
    types::{ShortenRequest, ShortenResponse},
    utils::{encode_long_url, valid_url},
};

pub async fn create_short_url(
    State(pool): State<PgPool>,
    payload: Result<Json<ShortenRequest>, JsonRejection>,
) -> impl IntoResponse {
    match payload {
        Ok(Json(data)) => {
            // validate the long URL
            let long_url = data.long_url;
            if !valid_url(&long_url) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invalid URL format"})),
                )
                    .into_response();
            }

            // encoding the long url to generate short code
            let encoded_url = encode_long_url(&long_url).await;
            let short_code = encoded_url[0..8].to_string();
            debug!("Long url: {}, Short code: {}", long_url, short_code);

            // constructing the short url
            let base_url = "http://0.0.0.0:3000/";
            let short_url = format!("{}/{}", base_url, short_code);

            match sqlx::query(
                "
                INSERT INTO urls (long_url, short_code)
                VALUES ($1, $2)
                ON CONFLICT (short_code) DO NOTHING
                ",
            )
            .bind(&long_url)
            .bind(&short_code)
            .execute(&pool)
            .await
            {
                Ok(_) => {
                    debug!("Successfully inserted short URL: {}", short_url);
                    let response = ShortenResponse {
                        long_url,
                        short_url,
                    };
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Error inserting into database: {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to insert into the database"})),
                    )
                        .into_response()
                }
            }
        }
        Err(rejection) => {
            let error_message = match rejection {
                JsonRejection::MissingJsonContentType(_) => {
                    json!({"error": "Expected 'Content-Type: application/json' header"})
                }
                JsonRejection::JsonSyntaxError(_) => json!({"error": "JSON syntax error"}),
                JsonRejection::JsonDataError(_) => json!({"error": "JSON data structure mismatch"}),
                _ => json!({"error": "Unknown JSON parsing error"}),
            };
            (StatusCode::BAD_REQUEST, Json(error_message)).into_response()
        }
    }
}
