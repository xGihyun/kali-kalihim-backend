use axum::extract;
use axum::response::Result;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};

use crate::error::AppError;
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Matchmake {
    id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    user1_id: uuid::Uuid,
    user2_id: uuid::Uuid,
    og_user1_id: uuid::Uuid,
    og_user2_id: uuid::Uuid,
    arnis_skill: String,
    arnis_footwork: String,
    card_deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct Arnis {
    skill: String,
    footwork: String,
}

pub async fn matchmake(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<Arnis>,
) -> Result<axum::Json<Vec<Matchmake>>, AppError> {
    let match_pairs = sqlx::query_as!(Matchmake,
        r#"
        WITH
        numbered_users AS (
            SELECT
            id,
            row_number() OVER (
                ORDER BY
                random()
            ) AS user_rank
            FROM users
        )
        INSERT INTO match_sets (user1_id, user2_id, og_user1_id, og_user2_id, arnis_skill, arnis_footwork)
        SELECT
            u1.id AS user1_id,
            u2.id AS user2_id,
            u1.id AS og_user1_id,
            u2.id AS og_user2_id,
            ($1) AS arnis_skill,
            ($2) AS arnis_footwork
        FROM
            numbered_users u1
            JOIN numbered_users u2 ON u1.user_rank = (u2.user_rank - 1) % u2.user_rank
        WHERE
            u2.user_rank % 2 = 0
        RETURNING *
        "#,
        payload.skill,
        payload.footwork
    ).fetch_all(&pool).await?;

    Ok(axum::Json(match_pairs))
}
