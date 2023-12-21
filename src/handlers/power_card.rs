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
        "Viral x Rival".to_string(),
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
pub struct WarlordsDomainPayload {
    user_id: uuid::Uuid,
    match_set_id: uuid::Uuid,
    arnis_skill: String,
}

pub async fn warlords_domain(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<WarlordsDomainPayload>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query(
        r#"
        WITH CurrentMatch AS (
            SELECT 
                id,
                CASE 
                    WHEN user1_id = ($1) THEN user2_id
                    ELSE user1_id
                END AS current_opponent_id
            FROM match_sets
            WHERE id = ($2)
            ORDER BY created_at DESC
            LIMIT 1
        )
        UPDATE match_sets AS ms
        SET 
            arnis_skill = 
                CASE
                    WHEN pc.is_active = TRUE and pc.is_used = FALSE THEN ms.og_arnis_skill
                    ELSE ($3)
                END
        FROM CurrentMatch cm
        JOIN power_cards pc ON user_id = cm.current_opponent_id AND name = 'Warlord''s Domain'
        WHERE ms.id = ($2)
        "#,
    )
    .bind(payload.user_id)
    .bind(payload.match_set_id)
    .bind(payload.arnis_skill)
    .execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct TwistOfFatePayload {
    user_id: uuid::Uuid,
    selected_opponent_id: uuid::Uuid,
}

// TODO: Negate the effect if either of the users have used Viral x Rival
// Make a CTE for Viral x Rival
// Before doing the swap, check if user1 or user2 have an activated VxR
// If they do, do not swap
pub async fn twist_of_fate(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<TwistOfFatePayload>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query(
        r#"
        WITH CurrentMatch AS (
            SELECT 
                id,
                CASE 
                    WHEN user1_id = ($1) THEN user2_id
                    ELSE user1_id
                END AS current_opponent_id
            FROM match_sets
            WHERE user1_id = ($1) OR user2_id = ($1)
            ORDER BY created_at DESC
            LIMIT 1
        ), SelectedMatch AS (
            SELECT 
                id,
                CASE 
                    WHEN user1_id = ($2) THEN user1_id
                    ELSE user2_id
                END AS selected_opponent_id
            FROM match_sets
            WHERE user1_id = ($2) OR user2_id = ($2)
            ORDER BY created_at DESC
            LIMIT 1
        )
        UPDATE match_sets AS ms
        SET 
            user1_id = 
                CASE 
                    WHEN ms.id = cm.id AND ms.user1_id <> ($1) THEN sm.selected_opponent_id
                    WHEN ms.id = sm.id AND ms.user1_id = sm.selected_opponent_id THEN cm.current_opponent_id
                    ELSE ms.user1_id
                END,
            user2_id = 
                CASE 
                    WHEN ms.id = cm.id AND ms.user2_id <> ($1) THEN sm.selected_opponent_id
                    WHEN ms.id = sm.id AND ms.user2_id = sm.selected_opponent_id THEN cm.current_opponent_id
                    ELSE ms.user2_id
                END
        FROM CurrentMatch cm, SelectedMatch sm
        "#,
    )
    .bind(payload.user_id)
    // .bind(payload.current_match_id)
    .bind(payload.selected_opponent_id).execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
}
