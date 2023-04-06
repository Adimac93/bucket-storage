use std::env;
use axum::Router;
use sqlx::{migrate, PgPool};
use axum::extract::{DefaultBodyLimit, FromRef};
pub mod auth;
pub mod errors;
pub mod files;

pub fn app(app_state: AppState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(files::router())
        .layer(DefaultBodyLimit::disable())
        .with_state(app_state)
}

#[derive(FromRef, Clone)]
pub struct AppState {
    pub pool: PgPool
}

impl AppState {
    pub async fn new(environment: Environment) -> Self {
        let pool = PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL var missing")).await.unwrap();
        if environment == Environment::Production {
            migrate!("./migrations").run(&pool).await.expect("Failed to migrate");
        }
        Self { pool }
    }

    pub async fn custom(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(PartialEq)]
pub enum Environment {
    Development,
    Production
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "production" | "prod" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not supported environment. Use either `local` or `production`"
            )),
        }
    }
}

