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

use handlers::{card_battle, matchmake, power_card, score, section, user};

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
        // Auth
        .route("/login", post(user::login))
        .route("/register", post(user::register))
        // User
        .route("/users", get(user::get_users))
        .route("/users/:user_id", get(user::get_user))
        .route("/users/update", post(user::update_user))
        .route("/scores/update", post(score::update_score))
        // Matches
        .route("/matches", get(matchmake::get_matches))
        .route("/matches/latest", post(matchmake::get_latest_match))
        .route("/max_sets", get(matchmake::get_max_sets))
        .route("/matchmake", post(matchmake::matchmake))
        // Section
        .route(
            "/sections",
            get(section::get_sections).post(section::insert_section),
        )
        // Power Card
        .route("/power_cards", post(power_card::get_cards))
        .route("/power_cards/update", post(power_card::update_card))
        .route("/power_cards/insert", post(power_card::insert_card))
        .route(
            "/power_cards/warlords_domain",
            post(power_card::warlords_domain),
        )
        .route(
            "/power_cards/twist_of_fate",
            post(power_card::twist_of_fate),
        )
        // Card Battle
        .route(
            "/card_battle",
            get(card_battle::card_battle).post(card_battle::insert_cards),
        )
        .route(
            "/card_battle/:match_set_id",
            get(card_battle::get_match_results),
        )
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
