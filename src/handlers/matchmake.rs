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
    user1_first_name: String,
    user2_first_name: String,
    user1_last_name: String,
    user2_last_name: String,
    section: String,
    arnis_skill: String,
    arnis_footwork: String,
    card_deadline: chrono::DateTime<chrono::Utc>,
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct Arnis {
    section: String,
    skill: String,
    footwork: String,
}

// TODO: Admin will choose skill, footwork is randomized (or vice versa?)
pub async fn matchmake(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<Arnis>,
) -> Result<axum::Json<Vec<Matchmake>>, AppError> {
    let match_pairs = sqlx::query_as::<_, Matchmake>(
        r#"
        WITH
          LatestDate AS (
            SELECT MAX(DATE_TRUNC('minute', created_at)) AS latest_date
            FROM match_sets
          ),
          PreviousMatches AS (
            SELECT id, user1_id, user2_id, user1_first_name, user2_first_name, user1_last_name, user2_last_name
            FROM match_sets
            WHERE DATE_TRUNC('minute', created_at) = (SELECT latest_date FROM LatestDate)
          ),
          ViralXRival AS (
            SELECT user_id
            FROM power_cards
            WHERE
                name = 'Viral x Rival'
                AND is_active = TRUE 
                AND is_used = FALSE
          ),
          PersistedPairs AS (
            SELECT
              m.id AS match_id,
              m.user1_id,
              m.user2_id,
              m.user1_first_name,
              m.user2_first_name,
              m.user1_last_name,
              m.user2_last_name
            FROM
              PreviousMatches m
            JOIN ViralXRival vxr ON m.user1_id = vxr.user_id OR m.user2_id = vxr.user_id
          ),
          RankedUsers AS (
            SELECT
              id,
              first_name,
              last_name,
              row_number() OVER (ORDER BY random()) AS user_rank
            FROM users u
            LEFT JOIN PersistedPairs pp ON u.id = pp.user1_id OR u.id = pp.user2_id
            WHERE section = ($1) AND pp.user1_id IS NULL
          )

        INSERT INTO match_sets (
          user1_id, user2_id, og_user1_id, og_user2_id, user1_first_name, user1_last_name, user2_first_name, user2_last_name,
          section, arnis_skill, arnis_footwork, og_arnis_skill
        )
        SELECT
          u1.id AS user1_id,
          u2.id AS user2_id,
          u1.id AS og_user1_id,
          u2.id AS og_user2_id,
          u1.first_name AS user1_first_name,
          u1.last_name AS user1_last_name,
          u2.first_name AS user2_first_name,
          u2.last_name AS user2_last_name,
          ($1) AS section,
          ($2) AS arnis_skill,
          ($3) AS arnis_footwork,
          ($2) AS og_arnis_skill
        FROM
          RankedUsers u1
          JOIN RankedUsers u2 ON u1.user_rank = (u2.user_rank - 1) % u2.user_rank
        WHERE
          u2.user_rank % 2 = 0

        UNION

        SELECT
          user1_id,
          user2_id,
          user1_id AS og_user1_id,
          user2_id AS og_user2_id,
          user1_first_name,
          user1_last_name,
          user2_first_name,
          user2_last_name,
          ($1) AS section,
          ($2) AS arnis_skill,
          ($3) AS arnis_footwork,
          ($2) AS og_arnis_skill
        FROM
          PersistedPairs
        RETURNING *;
        "#
    )
    .bind(payload.section)
    .bind(payload.skill)
    .bind(payload.footwork)
    .fetch_all(&pool).await?;

    Ok(axum::Json(match_pairs))
}
