// NOTE: This is for testing only

use axum::{extract::State, response::Result, Json};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{prelude::FromRow, PgPool};
use tracing::{debug, info};

use crate::error::AppError;

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "status")]
#[sqlx(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Done,
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "arnis_verdict")]
#[sqlx(rename_all = "lowercase")]
pub enum Verdict {
    Win,
    Lose,
    Draw,
    Pending,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Match {
    id: uuid::Uuid,

    created_at: chrono::DateTime<chrono::Utc>,
    card_deadline: chrono::DateTime<chrono::Utc>,
    batch: i16,
    section: String,
    status: Status,
    arnis_skill: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct MatchPlayer {
    pub id: uuid::Uuid,

    pub user_id: uuid::Uuid,
    pub match_id: uuid::Uuid,
    pub score: i16,
    pub card_damage: i16,
    pub arnis_verdict: Verdict,
    pub des_count: i16,
    pub ap_count: i16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Matchmake {
    id: uuid::Uuid,

    created_at: chrono::DateTime<chrono::Utc>,
    card_deadline: chrono::DateTime<chrono::Utc>,
    batch: i16,
    section: String,
    status: Status,
    arnis_skill: String,
    users: Vec<MatchmakeUser>,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct MatchmakeUser {
    id: uuid::Uuid,
    first_name: String,
    last_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatchPayload {
    section: String,
    skill: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct MatchJson {
    match_details: Value,
}

pub async fn matchmake(
    State(pool): State<PgPool>,
    Json(payload): Json<MatchPayload>,
) -> Result<Json<Vec<Matchmake>>, AppError> {
    let mut txn = pool.begin().await?;

    let user_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE section = ($1) AND role = 'user'",
    )
    .bind(payload.section.as_str())
    .fetch_one(&mut *txn)
    .await?;

    let match_count = user_count as usize / 2;

    let mut matches: Vec<Matchmake> = Vec::with_capacity(match_count);

    for _ in 0..match_count {
        let match_json = sqlx::query_as::<_, MatchJson>(
            r#"
        WITH UserCount AS (
            SELECT COUNT(*) AS user_count
            FROM users
            WHERE section = ($1) AND role = 'user'
        ),
        LatestMatch AS (
            SELECT 
                MAX(DATE_TRUNC('minute', created_at)) AS created_at,
                COUNT(DISTINCT created_at) AS batch
            FROM matches
            WHERE section = ($1)
        ),
        PreviousMatch AS (
            SELECT m.id, mp.user_id
            FROM matches m
            JOIN match_players mp ON mp.match_id = m.id
            WHERE DATE_TRUNC('minute', m.created_at) = (SELECT created_at FROM LatestMatch)
        ),
        ViralXRival AS (
            SELECT user_id
            FROM power_cards
            WHERE
                name = 'Viral x Rival'
                AND is_active = TRUE 
                AND is_used = FALSE
        ),
        PersistedMatches AS (
            SELECT
                pm.id
            FROM
                PreviousMatch pm
            JOIN ViralXRival vxr ON pm.user_id = vxr.user_id
        ),
        PersistedPairs AS (
            SELECT * FROM PreviousMatch WHERE id IN (SELECT id FROM PersistedMatches) 
        ),
        Players AS (
            SELECT id, first_name, last_name
            FROM users
            WHERE section = ($1) 
                AND role = 'user' 
                AND id NOT IN (SELECT user_id FROM PersistedPairs)
            ORDER BY RANDOM()
            LIMIT (SELECT CASE WHEN user_count >= 2 THEN 2 ELSE 0 END FROM UserCount)
        ),
        Match AS (
            INSERT INTO matches (section, arnis_skill)
            VALUES (($1), ($2))
            RETURNING *
        ),
        InsertedMatches AS (
            INSERT INTO match_players (user_id, match_id)
            SELECT p.id, (SELECT id FROM Match)
            FROM Players p
            RETURNING user_id, match_id
        )
        SELECT
            jsonb_build_object(
                'id', m.id,
                'created_at', m.created_at,
                'section', m.section,
                'arnis_skill', m.arnis_skill,
                'card_deadline', m.card_deadline,
                'batch', m.batch,
                'status', m.status,
                'players', jsonb_agg(
                    jsonb_build_object(
                    'id', im.user_id, 
                    'first_name', (SELECT first_name FROM Players WHERE id = im.user_id), 
                    'last_name', (SELECT last_name FROM Players WHERE id = im.user_id)
                )
                )
            ) AS match_details
        FROM
            Match m
        JOIN
            InsertedMatches im ON m.id = im.match_id
        GROUP BY m.id, m.created_at, m.section, m.arnis_skill, m.card_deadline, m.batch, m.status;
        "#,
        )
        .bind(payload.section.as_str())
        .bind(payload.skill.as_str())
        .fetch_optional(&mut *txn)
        .await?;

        debug!("{:?}", match_json);

        if let Some(match_json) = match_json {
            let match_details = serde_json::from_value::<Matchmake>(match_json.match_details)?;
            matches.push(match_details);
        }
    }

    txn.commit().await?;

    info!("Matchmaking successful.");

    Ok(Json(matches))
}

pub async fn matchmake2(
    State(pool): State<PgPool>,
    Json(payload): Json<MatchPayload>,
) -> Result<Json<Vec<Matchmake>>, AppError> {
    let mut txn = pool.begin().await?;

    let user_ids = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        WITH LatestMatch AS (
            SELECT 
                MAX(DATE_TRUNC('minute', created_at)) AS created_at,
                COUNT(DISTINCT created_at) AS batch
            FROM matches
            WHERE section = ($1)
        ),
        PreviousMatch AS (
            SELECT m.id, mp.user_id
            FROM matches m
            JOIN match_players mp ON mp.match_id = m.id
            WHERE DATE_TRUNC('minute', m.created_at) = (SELECT created_at FROM LatestMatch)
        ),
        ViralXRival AS (
            SELECT user_id
            FROM power_cards
            WHERE
                name = 'Viral x Rival'
                AND is_active = TRUE 
                AND is_used = FALSE
        ),
        PersistedMatches AS (
            SELECT
                pm.id
            FROM
                PreviousMatch pm
            JOIN ViralXRival vxr ON pm.user_id = vxr.user_id
        ),
        PersistedPairs AS (
            SELECT * FROM PreviousMatch WHERE id IN (SELECT id FROM PersistedMatches) 
        )
        SELECT id FROM users 
        WHERE section = ($1) 
            AND role = 'user' 
            AND id NOT IN (SELECT user_id FROM PersistedPairs)
        "#,
    )
    .bind(payload.section.as_str())
    .fetch_all(&mut *txn)
    .await?;

    let persisted_pairs = sqlx::query_scalar::<_, Vec<uuid::Uuid>>(
        r#"
        WITH LatestMatch AS (
            SELECT 
                MAX(DATE_TRUNC('minute', created_at)) AS created_at,
                COUNT(DISTINCT created_at) AS batch
            FROM matches
            WHERE section = ($1)
        ),
        PreviousMatch AS (
            SELECT m.id, mp.user_id
            FROM matches m
            JOIN match_players mp ON mp.match_id = m.id
            WHERE DATE_TRUNC('minute', m.created_at) = (SELECT created_at FROM LatestMatch)
        ),
        ViralXRival AS (
            SELECT user_id
            FROM power_cards
            WHERE
                name = 'Viral x Rival'
                AND is_active = TRUE 
                AND is_used = FALSE
        ),
        PersistedMatches AS (
            SELECT
                pm.id
            FROM
                PreviousMatch pm
            JOIN ViralXRival vxr ON pm.user_id = vxr.user_id
        ),
        PersistedPairs AS (
            SELECT * FROM PreviousMatch WHERE id IN (SELECT id FROM PersistedMatches) 
        )
        SELECT array_agg(user_id)
        FROM PreviousMatch 
        WHERE user_id IN (SELECT user_id FROM PersistedPairs)
        GROUP BY id;
        "#,
    )
    .bind(payload.section.as_str())
    .fetch_all(&mut *txn)
    .await?;

    let mut pairs = generate_pairs(user_ids);

    pairs.extend(persisted_pairs);

    let mut matches: Vec<Matchmake> = Vec::with_capacity(pairs.len());

    for pair in pairs.iter() {
        let new_match = sqlx::query_as::<_, Match>(
            r#"
            INSERT INTO matches (section, arnis_skill)
            VALUES (($1), ($2))
            RETURNING *
            "#,
        )
        .bind(payload.section.as_str())
        .bind(payload.skill.as_str())
        .fetch_one(&mut *txn)
        .await?;

        let mut matchmake = Matchmake {
            id: new_match.id,
            arnis_skill: new_match.arnis_skill,
            section: new_match.section,
            batch: new_match.batch,
            status: new_match.status,
            created_at: new_match.created_at,
            card_deadline: new_match.card_deadline,
            users: Vec::with_capacity(2),
        };

        for user_id in pair.iter() {
            let user = sqlx::query_as::<_, MatchmakeUser>(
                r#"
                INSERT INTO match_players (user_id, match_id)
                VALUES (($1), ($2))
                RETURNING user_id AS id,
                    (SELECT first_name FROM users WHERE id = ($1)) AS first_name,
                    (SELECT last_name FROM users WHERE id = ($1)) AS last_name;
                "#,
            )
            .bind(user_id)
            .bind(new_match.id)
            .fetch_one(&mut *txn)
            .await?;

            matchmake.users.push(user);
        }

        matches.push(matchmake);
    }

    txn.commit().await?;

    Ok(Json(matches))
}

fn generate_pairs(users: Vec<uuid::Uuid>) -> Vec<Vec<uuid::Uuid>> {
    let mut matched_pairs = Vec::with_capacity(users.len() / 2);
    let mut remaining_users = users;

    // Shuffle the user IDs to randomize the matchmaking
    remaining_users.shuffle(&mut rand::thread_rng());

    // Pair up users until there are not enough users left
    while remaining_users.len() >= 2 {
        let mut pair = Vec::with_capacity(2);
        for _ in 0..2 {
            let user = remaining_users.pop().unwrap();
            pair.push(user);
        }
        matched_pairs.push(pair);
    }

    matched_pairs
}
