// NOTE: This is for testing only

use axum::{
    extract::{Path, State},
    response::Result,
    Json,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{prelude::FromRow, PgPool};
use tracing::info;

use crate::error::AppError;

#[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "status")]
#[sqlx(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Done,
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, FromRow, Clone)]
pub struct MatchUser {
    pub id: uuid::Uuid,

    pub user_id: uuid::Uuid,
    pub match_id: uuid::Uuid,
    pub score: i16,
    pub card_damage: i16,
    pub arnis_verdict: Verdict,
    pub des_count: i16,
    pub ap_count: i16,
}

#[derive(Debug, Deserialize, Serialize, FromRow, Clone)]
pub struct MatchUserClient {
    id: uuid::Uuid,

    user_id: uuid::Uuid,
    match_id: uuid::Uuid,
    score: i16,
    card_damage: i16,
    arnis_verdict: Verdict,
    des_count: i16,
    ap_count: i16,
    is_swapped: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatchClient {
    id: uuid::Uuid,

    created_at: chrono::DateTime<chrono::Utc>,
    card_deadline: chrono::DateTime<chrono::Utc>,
    batch: i16,
    section: String,
    status: Status,
    arnis_skill: String,
    users: Vec<MatchUserClient>,
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

pub async fn get_user_matches(
    State(pool): State<PgPool>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<MatchClient>>, AppError> {
    info!("Getting matches...");

    let mut txn = pool.begin().await?;

    let matches = sqlx::query_as::<_, Match>(
        r#"
        SELECT m.* FROM matches m 
        JOIN match_users mu ON mu.match_id = m.id
        WHERE mu.user_id = ($1)
        ORDER BY m.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *txn)
    .await?;

    let match_user_og = sqlx::query_as::<_, MatchUser>(
        r#"
        WITH UserMatches AS (
            SELECT match_id, created_at FROM matches m 
            JOIN match_users mu ON mu.match_id = m.id
            WHERE mu.user_id = ($1)
        )
        SELECT * FROM match_users_og muo
        JOIN UserMatches um ON muo.match_id = um.match_id
        ORDER BY um.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *txn)
    .await?;

    let match_users = sqlx::query_as::<_, MatchUser>(
        r#"
        WITH UserMatches AS (
            SELECT match_id, created_at FROM matches m 
            JOIN match_users mu ON mu.match_id = m.id
            WHERE mu.user_id = ($1)
        )
        SELECT * FROM match_users mu
        JOIN UserMatches um ON mu.match_id = um.match_id
        ORDER BY um.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *txn)
    .await?;

    let mut matches_client: Vec<MatchClient> = Vec::with_capacity(matches.len());

    matches_client.extend(matches.iter().map(|m| {
        let mut user_clients = match_users
            .iter()
            .map(|mu| MatchUserClient {
                user_id: mu.user_id,
                des_count: mu.des_count,
                id: mu.id,
                match_id: mu.match_id,
                score: mu.score,
                ap_count: mu.ap_count,
                card_damage: mu.card_damage,
                arnis_verdict: mu.arnis_verdict.clone(),
                is_swapped: false,
            })
            .filter(|mu| mu.match_id == m.id)
            .collect::<Vec<MatchUserClient>>();

        if let Some(muo) = match_user_og.clone() {
            if muo.match_id == m.id {
                let og_user = MatchUserClient {
                    user_id: muo.user_id,
                    des_count: muo.des_count,
                    id: muo.id,
                    match_id: muo.match_id,
                    score: muo.score,
                    ap_count: muo.ap_count,
                    card_damage: muo.card_damage,
                    arnis_verdict: muo.arnis_verdict,
                    is_swapped: true,
                };

                user_clients.push(og_user);
            }
        }

        MatchClient {
            id: m.id,
            batch: m.batch,
            status: m.status.clone(),
            section: m.section.clone(),
            created_at: m.created_at,
            arnis_skill: m.arnis_skill.clone(),
            card_deadline: m.card_deadline,
            users: user_clients,
        }
    }));

    Ok(Json(matches_client))
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Opponent {
    id: uuid::Uuid,
    first_name: String,
    last_name: String,
    score: i32,
    avatar_url: Option<String>,
    banner_url: Option<String>,
}

pub async fn get_opponent_data(
    State(pool): State<PgPool>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<Opponent>, AppError> {
    let opponent = sqlx::query_as::<_, Opponent>(
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
        OpponentDetails AS (
            SELECT u.id, u.first_name, u.last_name, u.score, u.avatar_url, u.banner_url
            FROM users u 
            JOIN CurrentOpponent co ON co.user_id = u.id
        )
        SELECT * FROM OpponentDetails
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    Ok(Json(opponent))
}

pub async fn matchmake(
    State(pool): State<PgPool>,
    Json(payload): Json<MatchPayload>,
) -> Result<Json<Vec<Matchmake>>, AppError> {
    info!("Matchmaking...");

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
            JOIN match_users mp ON mp.match_id = m.id
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
            JOIN match_users mp ON mp.match_id = m.id
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
                INSERT INTO match_users (user_id, match_id)
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

    info!("Matchmaking successful.");

    Ok(Json(matches))
}

fn generate_pairs(users: Vec<uuid::Uuid>) -> Vec<Vec<uuid::Uuid>> {
    info!("Generating pairs...");

    let mut matched_pairs = Vec::with_capacity(users.len() / 2);
    let mut remaining_users = users;

    remaining_users.shuffle(&mut rand::thread_rng());

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
