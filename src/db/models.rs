use chrono::{DateTime, Utc};

#[derive(Debug, sqlx::FromRow)]
pub struct UrlDetail {
    pub long_url: String,
    pub short_code: String,
    pub created_at: DateTime<Utc>,
}
