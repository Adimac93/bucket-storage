use std::env;
use std::net::{SocketAddr};
use dotenv::dotenv;
use tokio::fs;
use tokio::fs::File;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use bucket_storage::{app, AppState, Environment};

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bucket_storage=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let env = env::var("APP_ENVIRONMENT").expect("APP_ENVIRONMENT var missing");
    let environment = Environment::try_from(env).unwrap();
    let addr = match environment {
        Environment::Development => SocketAddr::from(([127, 0, 0, 1], 3001)),
        Environment::Production => {
            let port = env::var("PORT").expect("PORT var missing").parse::<u16>().expect("Failed to parse PORT var");
            SocketAddr::from(([0, 0, 0, 0], port)) }
    };

    setup().await;
    let app_state = AppState::new(environment).await;


    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app(app_state).into_make_service()
        )
        .await
        .expect("Failed to run axum server");
}

async fn setup() {
    if let Ok(file) = File::open("./store").await {
        if let Ok(meta) = file.metadata().await {
            info!("Using existing bucket directory");
            if !meta.is_dir() {
                panic!("Occupied bucket directory");
            }
        }
    } else {
        fs::create_dir("./store").await.unwrap();
        info!("Created bucket directory");
        return;
    }

}


