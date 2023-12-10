// Ignore unused imports for now to remove some noise
#![allow(unused_imports)]
#![allow(warnings)]

use anyhow::Context;
use axum::{
    http,
    response::Response,
    routing::{get, post},
    Router,
};
use std::env;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let db_url = env::var("DATABASE_URL").context("DATABASE_URL env not found.")?;
    let ip_addr = env::var("IP_ADDRESS").unwrap_or("127.0.0.1".to_string());

    let app = Router::new()
        .route("/", get(health))
        .layer(CorsLayer::permissive());

    let listener = TcpListener::bind(format!("{}:8000", ip_addr)).await?;

    println!(
        "Server has started, listening on: {:?}\n",
        listener.local_addr()?
    );

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn health() -> (http::StatusCode, String) {
    (http::StatusCode::OK, "Hello, World!".to_string())
}
