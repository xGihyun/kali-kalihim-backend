use axum::extract;
use axum::http;
use axum::response::Result;
use serde::Deserialize;
use sqlx::prelude::FromRow;
use sqlx::PgPool;
use tracing::debug;

use crate::error::AppError;

#[derive(Debug, Deserialize, FromRow)]
pub struct UpdateScore {
    user_id: uuid::Uuid,
    score: i32,
    difference: i32,
    is_winner: String, // "win", "lose", or "draw"
    match_set_id: uuid::Uuid,
}

pub async fn update_ranks(
    extract::State(pool): extract::State<PgPool>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query(
        r#"
        WITH OverallRank AS (
            SELECT id, DENSE_RANK() OVER (ORDER BY score DESC) AS new_rank
            FROM users
        ), SectionRank AS (
            SELECT id, DENSE_RANK() OVER (PARTITION BY section ORDER BY score DESC) AS new_rank
            FROM users
        )
        UPDATE users u
        SET rank_overall = ovr.new_rank, rank_section = sr.new_rank
        FROM OverallRank ovr, SectionRank sr
        WHERE u.id = ovr.id AND u.id = sr.id
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
}

// NOTE: This is horrible
pub async fn update_score(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<UpdateScore>,
) -> Result<http::StatusCode, AppError> {
    let mut txn = pool.begin().await?;

    debug!("{:?}", payload);

    // current score + payload score
    sqlx::query(
        r#"
        WITH DoubleEdgedSword AS (
            SELECT COUNT(*)
            FROM power_cards
            WHERE 
                user_id = ($3) 
                AND name = 'Double-edged Sword' 
                AND is_active = TRUE 
                AND is_used = FALSE
        ),
            AncientsProtection AS (
            SELECT COUNT(*)
            FROM power_cards
            WHERE 
                user_id = ($3) 
                AND name = 'Ancient''s Protection' 
                AND is_active = TRUE 
                AND is_used = FALSE
        )
        UPDATE users 
        SET 
            score = 
                CASE 
                    WHEN ($4) = 'win' THEN 
                        (score + ($1)) + (CASE WHEN des.count = 0 THEN ($2) ELSE (($2) * (2 * des.count)) END)
                    WHEN (($4) = 'lose' AND ap.count > 0) OR ($4) = 'draw' THEN 
                        score + ($1)
                    ELSE 
                        (score + ($1)) - (CASE WHEN des.count = 0 THEN ($2) ELSE (($2) * (2 * des.count)) END)
                END
        FROM DoubleEdgedSword des, AncientsProtection ap
        WHERE id = ($3);
        "#,
    )
    .bind(payload.score)
    .bind(payload.difference)
    .bind(payload.user_id)
    .bind(payload.is_winner.as_str())
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        r#"
        WITH DoubleEdgedSword AS (
            SELECT COUNT(*)
            FROM power_cards
            WHERE 
                user_id = ($1) 
                AND name = 'Double-edged Sword' 
                AND is_active = TRUE 
                AND is_used = FALSE
        ),
            AncientsProtection AS (
            SELECT COUNT(*)
            FROM power_cards
            WHERE 
                user_id = ($1) 
                AND name = 'Ancient''s Protection' 
                AND is_active = TRUE 
                AND is_used = FALSE
        )
        UPDATE match_sets
        SET 
            user1_score = CASE WHEN user1_id = ($1) THEN ($4) ELSE user1_score END,
            user2_score = CASE WHEN user2_id = ($1) THEN ($4) ELSE user2_score END,
            user1_arnis_verdict = CASE WHEN user1_id = ($1) THEN ($2) ELSE user1_arnis_verdict END,
            user2_arnis_verdict = CASE WHEN user2_id = ($1) THEN ($2) ELSE user2_arnis_verdict END,
            user1_des_count = CASE WHEN user1_id = ($1) THEN des.count ELSE user1_des_count END,
            user1_ap_count = CASE WHEN user1_id = ($1) THEN ap.count ELSE user1_ap_count END,
            user2_des_count = CASE WHEN user2_id = ($1) THEN des.count ELSE user2_des_count END,
            user2_ap_count = CASE WHEN user2_id = ($1) THEN ap.count ELSE user2_ap_count END
        FROM DoubleEdgedSword des, AncientsProtection ap
        WHERE id = ($3);
        "#,
    )
    .bind(payload.user_id)
    .bind(payload.is_winner.as_str())
    .bind(payload.match_set_id)
    .bind(payload.score)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        r#"
        WITH OverallRank AS (
            SELECT id, DENSE_RANK() OVER (ORDER BY score DESC) AS new_rank
            FROM users
        ), SectionRank AS (
            SELECT id, DENSE_RANK() OVER (PARTITION BY section ORDER BY score DESC) AS new_rank
            FROM users
        )
        UPDATE users u
        SET rank_overall = ovr.new_rank, rank_section = sr.new_rank
        FROM OverallRank ovr, SectionRank sr
        WHERE u.id = ovr.id AND u.id = sr.id
        "#,
    )
    .execute(&mut *txn)
    .await?;

    txn.commit().await?;

    Ok(http::StatusCode::OK)
}
