use reqwest::Client;
use dotenv::dotenv;
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use bucket_storage::{app, AppState, Environment};


async fn spawn_app(pool: PgPool) -> SocketAddr {
    dotenv().ok();

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();
    let app_state = AppState::custom(pool).await;

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(app_state).into_make_service())
            .await
            .unwrap()
    });

    addr
}

pub struct AppData {
    pub addr: SocketAddr,
}

impl AppData {
    pub async fn new(pool: PgPool) -> Self {
        Self {
            addr: spawn_app(pool).await,
        }
    }

    pub fn client(&self) -> Client {
        Client::builder()
            .build()
            .expect("Failed to build reqwest client")
    }

    pub fn api(&self, uri: &str) -> String {
        let mut url = format!("http://{}", self.addr);
        if let Some(char) = uri.trim().chars().nth(0) {
            if char != '/' {
               url.push('/');
            }
            url.push_str(uri);
        }
        url
    }
}
