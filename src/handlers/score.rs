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

pub async fn update_score(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<UpdateScore>,
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
