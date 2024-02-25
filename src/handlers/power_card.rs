use crate::error::AppError;
use axum::response::Result;
use axum::Json;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};
use tracing::info;

use super::matches::MatchUser;
use super::user::UserId;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct PowerCard {
    id: uuid::Uuid,
    name: String,
    is_used: bool,
    is_active: bool,
    user_id: uuid::Uuid,
}

impl PowerCard {
    pub fn get() -> Vec<String> {
        let power_cards: Vec<String> = vec![
            "Ancient's Protection".to_string(),
            "Double-edged Sword".to_string(),
            "Extra Wind".to_string(),
            "Twist of Fate".to_string(),
            "Viral x Rival".to_string(),
            // "Warlord's Domain".to_string(),
        ];

        power_cards
    }

    // Three (3) random cards per user, duplicates are allowed
    fn get_random_cards(amount: usize) -> Vec<String> {
        let power_cards = Self::get();

        let mut rng = rand::thread_rng();
        power_cards
            .choose_multiple(&mut rng, amount)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct GetPowerCard {
    id: uuid::Uuid,
    name: String,
    is_used: bool,
    is_active: bool,
}

pub async fn get_cards(
    State(pool): State<PgPool>,
    Query(query): Query<UserId>,
) -> Result<axum::Json<Vec<GetPowerCard>>, AppError> {
    let power_cards = sqlx::query_as(
        "SELECT id, name, is_used, is_active FROM power_cards WHERE user_id = ($1) ORDER BY name",
    )
    .bind(query.user_id)
    .fetch_all(&pool)
    .await?;

    Ok(axum::Json(power_cards))
}

#[derive(Debug, Deserialize)]
pub struct InsertCard {
    name: Option<String>, // If the card is specified, only that card will be inserted
    user_id: uuid::Uuid,
    amount: Option<usize>,
}

// #[derive(Debug, Deserialize)]
// pub struct InsertCardQuery {
// }

// Extra Wind function already here
// Could be better
pub async fn insert_card(
    State(pool): State<PgPool>,
    Json(payload): Json<InsertCard>,
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
            // let amount = payload.amount.unwrap_or(3);
            let random_cards = PowerCard::get();
            let mut power_cards: Vec<PowerCard> = Vec::with_capacity(random_cards.len());

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

#[derive(Debug, Deserialize)]
pub struct UpdateCard {
    user_id: uuid::Uuid,
    is_activated: bool,
    is_used: bool,
}

pub async fn update_card(
    State(pool): State<PgPool>,
    Path(card_id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateCard>,
) -> Result<StatusCode, AppError> {
    sqlx::query("UPDATE power_cards SET is_active = ($1), is_used = ($2) WHERE id = ($3) AND user_id = ($4)")
        .bind(payload.is_activated)
        .bind(payload.is_used)
        .bind(card_id)
        .bind(payload.user_id)
        .execute(&pool)
        .await?;

    Ok(StatusCode::OK)
}

pub async fn update_cards(State(pool): State<PgPool>) -> Result<StatusCode, AppError> {
    sqlx::query(
        r#"
        UPDATE power_cards pc
        SET is_used = true
        FROM users u
        WHERE pc.user_id = u.id
        AND pc.is_active = true
        AND pc.is_used = false
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(StatusCode::OK)
}

// Implement the functions for every power card

#[derive(Debug, Deserialize)]
pub struct WarlordsDomainPayload {
    user_id: uuid::Uuid,
    match_set_id: uuid::Uuid,
    arnis_skill: String,
}

pub async fn warlords_domain(
    State(pool): State<PgPool>,
    Json(payload): Json<WarlordsDomainPayload>,
) -> Result<StatusCode, AppError> {
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

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct TwistOfFatePayload {
    user_id: uuid::Uuid,
    selected_user_id: uuid::Uuid,
}

// TODO: Negate the effect if either of the users have used Viral x Rival
// Make a CTE for Viral x Rival
// Before doing the swap, check if user1 or user2 have an activated VxR
// If they do, do not swap
pub async fn twist_of_fate_legacy(
    State(pool): State<PgPool>,
    Json(payload): Json<TwistOfFatePayload>,
) -> Result<StatusCode, AppError> {
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
    .bind(payload.selected_user_id)
    .execute(&pool)
    .await?;

    Ok(StatusCode::OK)
}

pub async fn twist_of_fate(
    State(pool): State<PgPool>,
    Json(payload): Json<TwistOfFatePayload>,
) -> Result<StatusCode, AppError> {
    info!("Running Twist of Fate...");

    let mut txn = pool.begin().await?;

    sqlx::query(
        r#"
        WITH CurrentMatch AS (
            SELECT m.id, mu.user_id
            FROM match_users mu
            JOIN matches m ON m.id = mu.match_id 
            WHERE mu.user_id = ($1)
            ORDER BY m.created_at DESC
            LIMIT 1
        ),
        CurrentOpponent AS (
            SELECT mu.*
            FROM match_users mu 
            JOIN CurrentMatch cm ON cm.id = mu.match_id
            WHERE match_id = cm.id 
            AND mu.user_id <> cm.user_id 
            LIMIT 1
        ),
        SelectedMatch AS (
            SELECT m.id, mu.user_id
            FROM match_users mu
            JOIN matches m ON m.id = mu.match_id 
            WHERE mu.user_id = ($2)
            ORDER BY m.created_at DESC
            LIMIT 1
        ),
        SelectedOpponent AS (
            SELECT mu.*
            FROM match_users mu 
            JOIN SelectedMatch sm ON sm.id = mu.match_id
            WHERE match_id = sm.id 
            AND mu.user_id = sm.user_id 
            LIMIT 1
        )

        INSERT INTO match_users_og (user_id, match_id)
        SELECT 
            co.user_id,
            cm.id AS match_id
        FROM CurrentMatch cm
        JOIN CurrentOpponent co ON cm.id = co.match_id

        UNION ALL

        SELECT 
            so.user_id,
            sm.id AS match_id
        FROM SelectedMatch sm
        JOIN SelectedOpponent so ON sm.id = so.match_id;
        "#,
    )
    .bind(payload.user_id)
    .bind(payload.selected_user_id)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        r#"
        WITH CurrentMatch AS (
            SELECT m.id, mu.user_id
            FROM match_users mu
            JOIN matches m ON m.id = mu.match_id 
            WHERE mu.user_id = ($1)
            ORDER BY m.created_at DESC
            LIMIT 1
        ),
        CurrentOpponent AS (
            SELECT mu.user_id AS id
            FROM match_users mu 
            JOIN CurrentMatch cm ON cm.id = mu.match_id
            WHERE match_id = cm.id 
            AND mu.user_id <> cm.user_id 
            LIMIT 1
        ),
        SelectedMatch AS (
            SELECT m.id, mu.user_id
            FROM match_users mu
            JOIN matches m ON m.id = mu.match_id 
            WHERE mu.user_id = ($2)
            ORDER BY m.created_at DESC
            LIMIT 1
        )

        UPDATE match_users mu
        SET user_id = 
        CASE
            WHEN (mu.match_id = cm.id AND mu.user_id = co.id) THEN sm.user_id
            WHEN (mu.match_id = sm.id AND mu.user_id = sm.user_id) THEN co.id
            ELSE mu.user_id
        END
        FROM SelectedMatch sm, CurrentMatch cm, CurrentOpponent co
        WHERE (mu.match_id = cm.id)
            OR (mu.match_id = sm.id)
        "#,
    )
    .bind(payload.user_id)
    .bind(payload.selected_user_id)
    .execute(&mut *txn)
    .await?;

    txn.commit().await?;

    info!("Twist of Fate successful.");

    Ok(StatusCode::OK)
}
