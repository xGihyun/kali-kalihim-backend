use axum::extract;
use axum::http;
use axum::response::Result;
use serde::Deserialize;
use sqlx::prelude::FromRow;
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Deserialize, FromRow)]
pub struct UpdateScore {
    user_id: uuid::Uuid,
    score: i32,
    difference: i32,
    is_winner: bool,
}

// Query to update rankings
// WITH OverallRank AS (
//     SELECT id, DENSE_RANK() OVER (ORDER BY score DESC) AS new_rank
//     FROM users
// ), SectionRank AS (
//     SELECT id, DENSE_RANK() OVER (PARTITION BY section ORDER BY score DESC) AS new_rank
//     FROM users
// )
// UPDATE users u
// SET rank_overall = ovr.new_rank, rank_section = sr.new_rank
// FROM OverallRank ovr, SectionRank sr
// WHERE u.id = ovr.id AND u.id = sr.id

// Query to update activated power cards
// UPDATE power_cards
// SET is_used = TRUE
// WHERE id = ($3) AND is_active = TRUE AND is_used = FALSE

pub async fn update_score(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<UpdateScore>,
) -> Result<http::StatusCode, AppError> {
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
                    WHEN ($4) THEN (score + ($1)) + (($2) * (2 * des.count))
                    WHEN NOT ($4) AND ap.count > 0 THEN score + ($1)
                    ELSE (score + ($1)) - (($2) * (2 * des.count))
                END
        FROM DoubleEdgedSword des, AncientsProtection ap
        WHERE id = ($3);
        "#,
    )
    .bind(payload.score)
    .bind(payload.difference)
    .bind(payload.user_id)
    .bind(payload.is_winner)
    .execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
}
