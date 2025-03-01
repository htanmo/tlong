use std::{env, process};

pub struct Config {
    pub base_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub server_addr: String,
}

impl Config {
    pub fn load() -> Self {
        let database_url = get_env("DATABASE_URL");
        let redis_url = get_env("REDIS_URL");
        let server_addr = get_env_or("SERVER_ADDRESS", "0.0.0.0:8080");
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| {
            tracing::warn!(
                "BASE_URL environment variable not set, using default: {}",
                &server_addr
            );
            format!("http://{}", server_addr)
        });
        Self {
            base_url,
            database_url,
            redis_url,
            server_addr,
        }
    }
}

fn get_env(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| {
        tracing::error!("{} environment variable is required but not set.", var);
        process::exit(1);
    })
}

fn get_env_or(var: &str, default: &str) -> String {
    env::var(var).unwrap_or_else(|_| {
        tracing::warn!(
            "{} environment variable not set, using default: {}",
            var,
            default
        );
        default.to_string()
    })
}
