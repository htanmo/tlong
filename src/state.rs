use r2d2::Pool;
use redis::Client;
use sqlx::PgPool;

pub type RedisPool = Pool<Client>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub pg_db: PgPool,
    pub redis_db: RedisPool,
}

impl AppState {
    pub fn new(pg_db: PgPool, redis_db: RedisPool) -> Self {
        Self { pg_db, redis_db }
    }
}
