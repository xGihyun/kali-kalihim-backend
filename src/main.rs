// Ignore unused imports for now to remove some noise
// #![allow(unused_imports)]
// #![allow(warnings)]

use anyhow::Context;
use axum::{
    http,
    routing::{get, patch, post},
    Router,
};
use dotenv::dotenv;
use std::env;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod error;
mod handlers;

use handlers::{card_battle, matchmake, power_card, score, section, user};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = env::var("DATABASE_URL").context("DATABASE_URL env not found.")?;
    let ip_addr = env::var("IP_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let max_connections = env::var("MAX_CONNECTIONS")
        .unwrap_or("10".to_string())
        .parse::<u32>()?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&db_url)
        .await?;

    let app = Router::new()
        .route("/", get(health))
        // Auth
        // .route("/login", post(user::login))
        .route("/register", post(user::register))
        // User
        .route("/users", get(user::get_users).delete(user::delete_users))
        .route(
            "/users/:user_id",
            get(user::get_user).patch(user::update_user),
        )
        .route(
            "/users/:user_id/private",
            patch(user::update_private_status),
        )
        .route("/users/count", get(user::get_users_count))
        // Could be way better
        .route("/users/update/column", post(user::update_column))
        .route("/scores", patch(score::update_score))
        // .route("/ranks", patch(score::update_ranks))
        // Matches
        .route("/matches", get(matchmake::get_matches))
        .route(
            "/matches/:match_set_id",
            get(matchmake::get_match).patch(matchmake::update_match_status),
        )
        // .route("/matches/update", post(matchmake::update_match_status))
        .route("/matches/latest", post(matchmake::get_latest_matches))
        .route("/matches/original", post(matchmake::get_original_matches))
        .route(
            "/matches/latest_date",
            post(matchmake::get_latest_match_date),
        )
        .route(
            "/matches/latest/:user_id",
            get(matchmake::get_latest_opponent),
        )
        .route("/max_sets", get(matchmake::get_max_sets))
        .route("/matchmake", post(matchmake::matchmake))
        // Section
        .route(
            "/sections",
            get(section::get_sections)
                .post(section::insert_section)
                .delete(section::delete_section),
        )
        .route("/sections/count", get(section::get_sections_with_count))
        // Power Card
        .route(
            "/power_cards",
            get(power_card::get_cards)
                .post(power_card::insert_card)
                .patch(power_card::update_cards),
        )
        .route("/power_cards/:card_id", patch(power_card::update_card))
        // .route("/power_cards/update", post(power_card::update_card))
        // .route("/power_cards/insert", post(power_card::insert_card))
        // NOTE: These should be query params instead
        .route(
            "/power_cards/warlords_domain",
            patch(power_card::warlords_domain),
        )
        .route(
            "/power_cards/twist_of_fate",
            patch(power_card::twist_of_fate),
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

    info!("{:<12} - {}", "LISTENING", listener.local_addr()?);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn health() -> (http::StatusCode, String) {
    (http::StatusCode::OK, "Hello, World!".to_string())
}
