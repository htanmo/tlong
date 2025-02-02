use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ShortenRequest {
    pub long_url: String,
}

#[derive(Debug, Serialize)]
pub struct ShortenResponse {
    pub short_url: String,
    pub long_url: String,
}

#[derive(Serialize)]
pub struct UrlDetailResponse {
    pub short_url: String,
    pub long_url: String,
    pub created_at: String,
}
