use crate::error::AppError;
use axum::response::Result;
use axum::{extract, http};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct PowerCard {
    id: uuid::Uuid,
    name: String,
    is_used: bool,
    is_active: bool,
    user_id: uuid::Uuid,
}

// Three (3) random cards per user, duplicates are allowed
fn get_random_cards(amount: usize) -> Vec<String> {
    let power_cards: Vec<String> = vec![
        "Ancient's Protection".to_string(),
        "Double-edged Sword".to_string(),
        "Extra Wind".to_string(),
        "Twist of Fate".to_string(),
        "Viral-x-Rival".to_string(),
        "Warlord's Domain".to_string(),
    ];

    let mut rng = rand::thread_rng();
    power_cards
        .choose_multiple(&mut rng, amount)
        .cloned()
        .collect()
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct InsertCard {
    name: Option<String>,
    user_id: uuid::Uuid,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct CardAmount {
    amount: Option<usize>,
}

// Extra Wind function already here
// Could be better
pub async fn insert_card(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<CardAmount>,
    axum::Json(payload): axum::Json<InsertCard>,
) -> Result<axum::Json<Vec<PowerCard>>, AppError> {
    match payload.name {
        Some(name) => {
            let new_card = sqlx::query_as::<_, PowerCard>(
                r#"
            INSERT INTO power_cards (name, user_id) 
            VALUES ($1, $2)
            RETURNING *
            "#,
            )
            .bind(name)
            .bind(payload.user_id)
            .fetch_all(&pool)
            .await?;

            Ok(axum::Json(new_card))
        }
        None => {
            let mut txn = pool.begin().await?;
            let amount = query.amount.unwrap_or(3);
            let random_cards = get_random_cards(amount);
            let mut power_cards: Vec<PowerCard> = Vec::with_capacity(amount);

            for random_card in random_cards.iter() {
                let card = sqlx::query_as::<_, PowerCard>(
                    r#"
                    INSERT INTO power_cards (name, user_id) 
                    VALUES ($1, $2)
                    RETURNING *
                    "#,
                )
                .bind(random_card)
                .bind(payload.user_id)
                .fetch_one(&mut *txn)
                .await?;

                power_cards.push(card);
            }

            txn.commit().await?;

            Ok(axum::Json(power_cards))
        }
    }
}

#[derive(Debug, Deserialize, FromRow)]
pub struct UpdateCard {
    card_id: uuid::Uuid,
    user_id: uuid::Uuid,
    is_activated: bool,
    is_used: bool,
}

pub async fn update_card(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<UpdateCard>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query("UPDATE power_cards SET is_active = ($1), is_used = ($2) WHERE id = ($3) AND user_id = ($4)")
        .bind(payload.is_activated)
        .bind(payload.is_used)
        .bind(payload.card_id)
        .bind(payload.user_id)
        .execute(&pool)
        .await?;

    Ok(http::StatusCode::OK)
}

// Implement the functions for every power card

#[derive(Debug, Deserialize)]
struct MatchSetId {
    match_set_id: uuid::Uuid,
}

async fn warlords_domain(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<MatchSetId>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query("UPDATE match_sets SET arnis_skill = ($1) WHERE id = ($2)")
        .bind(payload.match_set_id)
        .execute(&pool)
        .await?;

    Ok(http::StatusCode::OK)
}

pub struct TwistOfFatePayload {
    match_id: uuid::Uuid,
    user_id: uuid::Uuid,
    current_opponent_id: uuid::Uuid,
    chosen_opponent_id: uuid::Uuid,
}

async fn twist_of_fate(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<TwistOfFatePayload>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"

            
        "#,
    );

    Ok(())
}