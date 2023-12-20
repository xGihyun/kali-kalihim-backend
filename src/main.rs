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
use dotenv::dotenv;
use std::env;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::cors::CorsLayer;

mod error;
mod handlers;

use handlers::{matchmake, power_card, score, user};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").context("DATABASE_URL env not found.")?;
    let ip_addr = env::var("IP_ADDRESS").unwrap_or("127.0.0.1".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await?;

    let app = Router::new()
        .route("/", get(health))
        .route("/users", get(user::get_users))
        .route("/scores/update", post(score::update_score))
        .route("/matchmake", post(matchmake::matchmake))
        .route("/power_card", post(power_card::update_card))
        .route("/power_card/insert", post(power_card::insert_card))
        .route(
            "/power_card/warlords_domain",
            post(power_card::warlords_domain),
        )
        .route("/power_card/twist_of_fate", post(power_card::twist_of_fate))
        .layer(CorsLayer::permissive())
        .with_state(pool);

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
