use axum::response::Result;
use axum::{extract, http};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{prelude::FromRow, PgPool};

use crate::error::AppError;

use super::user::UserId;

// This is horrible
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
    set: i32,
    user1_total_damage: Option<f32>,
    user2_total_damage: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct UserMatchQuery {
    fields: Option<String>,
    limit: Option<i32>,
}

// This can be merged with get_matches()
pub async fn get_latest_matches(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<UserMatchQuery>,
    extract::Json(payload): extract::Json<UserId>,
) -> Result<axum::Json<Vec<Matchmake>>, AppError> {
    let mut sql = r#"
        SELECT ms.*, u1.first_name AS user1_first_name, u1.last_name AS user1_last_name, u2.first_name AS user2_first_name, u2.last_name AS user2_last_name
        FROM match_sets ms
        JOIN users u1 ON user1_id = u1.id
        JOIN users u2 ON user2_id = u2.id 
        WHERE user1_id = ($1) OR user2_id = ($1)
        ORDER BY created_at DESC
        "#.to_string();

    if let Some(limit) = query.limit {
        sql.push_str(format!(" LIMIT {} ", limit).as_str());
    }

    let latest_match = sqlx::query_as::<_, Matchmake>(sql.as_str())
        .bind(payload.user_id)
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(latest_match))
}

#[derive(Debug, Serialize, FromRow)]
pub struct MatchDate {
    created_at: chrono::DateTime<chrono::Utc>,
    card_deadline: chrono::DateTime<chrono::Utc>,
}

pub async fn get_latest_match_date(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<UserId>,
) -> Result<axum::Json<Option<MatchDate>>, AppError> {
    let latest_match = sqlx::query_as::<_, MatchDate>(
        r#"
        SELECT created_at, card_deadline
        FROM match_sets
        WHERE user1_id = ($1) OR user2_id = ($1)
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(payload.user_id)
    .fetch_optional(&pool)
    .await?;

    Ok(axum::Json(latest_match))
}

#[derive(Debug, Serialize, FromRow)]
pub struct LatestOpponentData {
    first_name: String,
    last_name: String,
    score: i32,
    avatar_url: Option<String>,
    banner_url: Option<String>,
}

pub async fn get_latest_opponent(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(user_id): extract::Path<uuid::Uuid>,
) -> Result<axum::Json<Option<LatestOpponentData>>, AppError> {
    let res = sqlx::query_as::<_, LatestOpponentData>(
        r#"
        WITH LatestMatch AS (
            SELECT
                CASE
                WHEN user1_id = ($1) THEN user2_id
                ELSE user1_id
                END AS opponent_id
            FROM match_sets
            WHERE user1_id = ($1) OR user2_id = ($1)
            ORDER BY created_at DESC
            LIMIT 1
        )
        SELECT first_name, last_name, score, avatar_url, banner_url FROM users WHERE id = (SELECT opponent_id FROM LatestMatch);
        "#
    ).bind(user_id).fetch_optional(&pool).await?;

    Ok(axum::Json(res))
}

#[derive(Debug, Deserialize)]
pub struct UpdateMatchStatus {
    match_set_id: uuid::Uuid,
    status: String,
}

pub async fn update_match_status(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<UpdateMatchStatus>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query("UPDATE match_sets SET status = ($1) WHERE id = ($2)")
        .bind(payload.status)
        .bind(payload.match_set_id)
        .execute(&pool)
        .await?;

    Ok(http::StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct MatchQuery {
    pub set: i32,
    pub section: String,
}

// This can be merged with get_latest_match()
pub async fn get_matches(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<MatchQuery>,
) -> Result<axum::Json<Vec<Matchmake>>, AppError> {
    let matches = sqlx::query_as::<_, Matchmake>(
        r#"
        SELECT *, u1.first_name AS user1_first_name, u1.last_name AS user1_last_name, u2.first_name AS user2_first_name, u2.last_name AS user2_last_name
        FROM match_sets ms
        JOIN users u1 ON user1_id = u1.id
        JOIN users u2 ON user2_id = u2.id 
        WHERE ms.set = ($1) AND ms.section = ($2)
        "#,
    )
    .bind(query.set)
    .bind(query.section)
    .fetch_all(&pool)
    .await?;

    Ok(axum::Json(matches))
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct MaxSet {
    section: String,
    max_set: i32,
}

pub async fn get_max_sets(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<MaxSet>>, AppError> {
    let max_sets = sqlx::query_as::<_, MaxSet>(
        "SELECT MAX(set) as max_set, ms.section FROM match_sets ms GROUP BY section",
    )
    .fetch_all(&pool)
    .await?;

    Ok(axum::Json(max_sets))
}

#[derive(Debug, Deserialize)]
pub struct Arnis {
    section: String,
    // skill: String,
    footwork: String,
}

// TODO: Admin will choose skill, footwork is randomized (or vice versa?)
pub async fn matchmake(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<Arnis>,
) -> Result<axum::Json<Vec<Matchmake>>, AppError> {
    let mut txn = pool.begin().await?;

    let match_pairs = sqlx::query_as::<_, Matchmake>(
        r#"
        WITH
        LatestMatch AS (
            SELECT 
                MAX(DATE_TRUNC('minute', created_at)) AS latest_date,
                COUNT(DISTINCT created_at) AS set
            FROM match_sets
            WHERE section = ($1)
        ),
        PreviousMatches AS (
            SELECT id, user1_id, user2_id
            FROM match_sets
            WHERE DATE_TRUNC('minute', created_at) = (SELECT latest_date FROM LatestMatch)
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
                m.user2_id
            FROM
                PreviousMatches m
            JOIN ViralXRival vxr ON m.user1_id = vxr.user_id OR m.user2_id = vxr.user_id
        ),
        RankedUsers AS (
            SELECT
                id,
                first_name,
                last_name,
                row_number() OVER (ORDER BY random()) AS user_rank,
                COUNT(*) OVER() AS total_users
            FROM users u
            LEFT JOIN PersistedPairs pp ON u.id = pp.user1_id OR u.id = pp.user2_id
            WHERE section = ($1) AND pp.user1_id IS NULL AND role = 'user'
        ),
        AdjustedRankedUsers AS (
            SELECT 
                id,
                first_name,
                last_name,
                user_rank,
                total_users,
                -- Exclude the highest-ranked user if there's an odd number of users
                CASE WHEN total_users % 2 <> 0 AND user_rank = total_users THEN TRUE ELSE FALSE END AS is_excluded
            FROM RankedUsers
        )
        INSERT INTO match_sets (
            user1_id, 
            user2_id, 
            og_user1_id, 
            og_user2_id,
            section, 
            arnis_skill, 
            arnis_footwork, 
            og_arnis_skill, 
            set
        )
        SELECT
            u1.id AS user1_id,
            u2.id AS user2_id,
            u1.id AS og_user1_id,
            u2.id AS og_user2_id,
            ($1) AS section,
            rs.random_skill AS arnis_skill,
            ($2) AS arnis_footwork,
            rs.random_skill AS og_arnis_skill,
            (SELECT set FROM LatestMatch) + 1 AS set
        FROM
            AdjustedRankedUsers u1
        JOIN AdjustedRankedUsers u2 ON u1.user_rank = (u2.user_rank - 1) % u2.user_rank
            CROSS JOIN LATERAL (
                SELECT
                    CASE 
                        WHEN random() < 0.2 THEN 'strikes'
                        WHEN random() < 0.4 THEN 'blocks'
                        WHEN random() < 0.6 THEN 'forward_sinawali'
                        WHEN random() < 0.8 THEN 'sideward_sinawali'
                        ELSE 'reversed_sinawali'
                    END AS random_skill
            ) AS rs
        -- Exclude rows where either user is marked for exclusion
        WHERE u1.is_excluded = FALSE AND u2.is_excluded = FALSE
        AND u2.user_rank % 2 = 0

        UNION

        SELECT
            user1_id,
            user2_id,
            user1_id AS og_user1_id,
            user2_id AS og_user2_id,
            ($1) AS section,
            rs.random_skill AS arnis_skill,
            ($2) AS arnis_footwork,
            rs.random_skill AS og_arnis_skill,
            (SELECT set FROM LatestMatch) + 1 AS set
        FROM
            PersistedPairs
        CROSS JOIN LATERAL (
            SELECT
                CASE 
                    WHEN random() < 0.2 THEN 'strikes'
                    WHEN random() < 0.4 THEN 'blocks'
                    WHEN random() < 0.6 THEN 'forward_sinawali'
                    WHEN random() < 0.8 THEN 'sideward_sinawali'
                    ELSE 'reversed_sinawali'
                END AS random_skill
        ) AS rs
        RETURNING *,
            (SELECT u1.first_name FROM users u1 WHERE u1.id = user1_id) AS user1_first_name,
            (SELECT u1.last_name FROM users u1 WHERE u1.id = user1_id) AS user1_last_name,
            (SELECT u2.first_name FROM users u2 WHERE u2.id = user2_id) AS user2_first_name,
            (SELECT u2.last_name FROM users u2 WHERE u2.id = user2_id) AS user2_last_name
        "#,
    )
    .bind(&payload.section)
    .bind(payload.footwork)
    .fetch_all(&mut *txn)
    .await?;

    sqlx::query(
        r#"
        UPDATE power_cards pc
        SET is_used = true
        FROM users u
        WHERE pc.user_id = u.id AND u.section = ($1)
        AND pc.is_active = true
        AND pc.is_used = false
        "#,
    )
    .bind(&payload.section)
    .execute(&mut *txn)
    .await?;

    txn.commit().await?;

    Ok(axum::Json(match_pairs))
}
