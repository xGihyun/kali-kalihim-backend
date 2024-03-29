use axum::response::Result;
use axum::{extract, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Execute;
use sqlx::{prelude::FromRow, PgPool, Row};
use tracing::debug;

use crate::error::AppError;

use super::power_card::PowerCard;

#[derive(Debug, Deserialize)]
pub struct UserId {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct User {
    id: uuid::Uuid,
    email: String,
    section: String,
    first_name: String,
    last_name: String,
    age: i32,
    contact_number: String,
    sex: i16,
    rank_overall: i32,
    rank_section: i32,
    rank_title: Option<String>,
    score: i32,
    role: String,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    is_private: bool,
}

// Fetch user without their sensitive info
// NOTE: Unfortunately, I'm not sure if there is a good way to dynamically fetch specific columns using
// SQLx
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct UserFetch {
    id: uuid::Uuid,
    section: String,
    first_name: String,
    last_name: String,
    sex: i16,
    rank_overall: i32,
    rank_section: i32,
    rank_title: Option<String>,
    score: i32,
    avatar_url: Option<String>,
    banner_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UsersQuery {
    section: Option<String>,
    order_by: Option<String>,
    order: Option<String>,
    limit: Option<u32>,
    skip: Option<u32>,
    fields: Option<String>,
}

pub async fn get_users(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<UsersQuery>,
) -> Result<axum::Json<Value>, AppError> {
    let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT ");

    if let Some(fields) = query.fields.as_ref() {
        query_builder.push(fields);
    } else {
        query_builder.push("*");
    }

    query_builder.push(" FROM users");

    if let Some(section) = query.section {
        let sections: Vec<&str> = section.split(',').collect();

        if sections.len() == 1 {
            query_builder.push(format_args!(" WHERE section = '{}'", section));
        } else {
            let mut comma_sep = query_builder.separated(", ");

            comma_sep.push_unseparated(" WHERE section IN (");

            for section in sections {
                comma_sep.push(section);
            }

            comma_sep.push_unseparated(")");
        }

        // query_builder.push(" AND role = 'user'");
    } else {
        // query_builder.push(" WHERE role = 'user'");
    }

    if let Some(order_by) = query.order_by {
        query_builder.push(format_args!(
            " ORDER BY {} {}",
            order_by,
            query.order.unwrap_or("asc".to_string())
        ));
    }

    if let Some(limit) = query.limit {
        query_builder.push(format_args!(" LIMIT {}", limit,));
    }

    if let Some(skip) = query.skip {
        query_builder.push(format_args!(" OFFSET {}", skip,));
    }

    let sql = query_builder.build().sql();

    if let Some(fields) = query.fields.as_ref() {
        let pg_rows = sqlx::query(sql).fetch_all(&pool).await?;
        let fields: Vec<&str> = fields.split(',').collect();

        let mut json_vec: Vec<Value> = Vec::new();

        for row in pg_rows.iter() {
            let mut json_row = serde_json::Map::new();

            for field in fields.iter() {
                let value: Value = if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(*field) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<Option<String>, _>(*field) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<Option<i16>, _>(*field) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<Option<i32>, _>(*field) {
                    json!(val)
                } else if let Ok(val) = row.try_get::<Option<f64>, _>(*field) {
                    json!(val)
                } else {
                    json!(null)
                };

                json_row.insert(field.to_string(), value);
            }

            let json = serde_json::to_value(json_row)?;

            json_vec.push(json);
        }

        let values = serde_json::to_value(json_vec)?;

        Ok(axum::Json(values))
    } else {
        let users = sqlx::query_as::<_, User>(sql).fetch_all(&pool).await?;

        let values = serde_json::to_value(users)?;

        Ok(axum::Json(values))
    }
}

// TODO: Improve filtering, use comma separated fields query to fetch specific columns only
// pub async fn get_users(
//     extract::State(pool): extract::State<PgPool>,
//     extract::Query(query): extract::Query<UsersQuery>,
// ) -> Result<axum::Json<Vec<UserFetch>>, AppError> {
//     let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
//         sqlx::QueryBuilder::new("SELECT id, section, first_name, last_name, sex, rank_overall, rank_section, rank_title, score, avatar_url, banner_url FROM users");
//
//     if let Some(section) = query.section {
//         let sections: Vec<&str> = section.split(',').collect();
//
//         if sections.len() == 1 {
//             query_builder.push(format!(" WHERE section = '{}'", section));
//         } else {
//             let sections: Vec<String> = section
//                 .split(',')
//                 .map(|s| format!("'{}'", s.trim()))
//                 .collect();
//             let section_list = sections.join(", ");
//             query_builder.push(format!(" WHERE section IN ({})", section_list));
//         }
//     }
//
//     if let Some(order_by) = query.order_by {
//         query_builder.push(format!(
//             " ORDER BY {} {}",
//             order_by,
//             query.order.unwrap_or("asc".to_string())
//         ));
//     }
//
//     if let Some(limit) = query.limit {
//         query_builder.push(format!(" LIMIT {} ", limit,));
//     }
//
//     if let Some(skip) = query.skip {
//         query_builder.push(format!(" OFFSET {} ", skip,));
//     }
//
//     let users = sqlx::query_as::<_, UserFetch>(query_builder.sql())
//         .fetch_all(&pool)
//         .await?;
//
//     Ok(axum::Json(users))
// }

#[derive(Debug, Serialize, FromRow)]
pub struct UserCount {
    total: i64,
}

pub async fn get_users_count(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<UserCount>, AppError> {
    let total = sqlx::query_as::<_, UserCount>("SELECT COUNT(*) AS total FROM users")
        .fetch_one(&pool)
        .await?;

    Ok(axum::Json(total))
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    filter: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct UserName {
    first_name: String,
    last_name: String,
}

pub async fn get_user(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(user_id): extract::Path<uuid::Uuid>,
    extract::Query(query): extract::Query<UserQuery>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    if query.filter.is_some() {
        let user = sqlx::query_as::<_, UserName>(
            "SELECT first_name, last_name FROM users WHERE id = ($1)",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        let user_value = serde_json::to_value(user)?;

        Ok(axum::Json(user_value))
    } else {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ($1)")
            .bind(user_id)
            .fetch_one(&pool)
            .await?;
        let user_value = serde_json::to_value(user)?;

        Ok(axum::Json(user_value))
    }
}

// For admin
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    email: Option<String>,
    section: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    age: Option<i32>,
    contact_number: Option<String>,
    sex: Option<i16>,
    score: Option<i32>,
    role: Option<String>,
}

pub async fn update_user(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(user_id): extract::Path<uuid::Uuid>,
    extract::Json(payload): extract::Json<UpdateUser>,
) -> Result<StatusCode, AppError> {
    let mut txn = pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE users 
        SET 
            email = COALESCE(NULLIF($1, ''), email), 
            section = COALESCE(NULLIF($2, ''), section), 
            first_name = COALESCE(NULLIF($3, ''), first_name), 
            last_name = COALESCE(NULLIF($4, ''), last_name),
            age = COALESCE($5, age),
            sex = COALESCE($6, sex),
            contact_number = COALESCE($7, contact_number),
            score = COALESCE($8, score),
            role = COALESCE(NULLIF($9, ''), role)
        WHERE id = ($10);
        "#,
    )
    .bind(payload.email)
    .bind(payload.section)
    .bind(payload.first_name)
    .bind(payload.last_name)
    .bind(payload.age)
    .bind(payload.sex)
    .bind(payload.contact_number)
    .bind(payload.score)
    .bind(payload.role)
    .bind(user_id)
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

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct UpdateAvatar {
    user_id: uuid::Uuid,
    url: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct UpdateQuery {
    column: String, // avatar_url || banner_url
}

// Use a comma separated query
pub async fn update_column(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<UpdateQuery>,
    extract::Json(payload): extract::Json<UpdateAvatar>,
) -> Result<StatusCode, AppError> {
    let sql = format!("UPDATE users SET {} = ($1) WHERE id = ($2)", query.column);

    sqlx::query(sql.as_str())
        .bind(payload.url)
        .bind(payload.user_id)
        .execute(&pool)
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct UpdatePrivateStatus {
    is_private: bool,
}

pub async fn update_private_status(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(user_id): extract::Path<uuid::Uuid>,
    extract::Json(payload): extract::Json<UpdatePrivateStatus>,
) -> Result<StatusCode, AppError> {
    sqlx::query("UPDATE users SET is_private = ($1) WHERE id = ($2)")
        .bind(payload.is_private)
        .bind(user_id)
        .execute(&pool)
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct Register {
    id: uuid::Uuid,
    email: String,
    section: String,
    first_name: String,
    last_name: String,
    age: i32,
    contact_number: String,
    sex: i16,
}

pub async fn register(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<Register>,
) -> Result<(StatusCode, axum::Json<User>), AppError> {
    let mut txn = pool.begin().await?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, section, first_name, last_name, age, contact_number, sex)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(payload.id)
    .bind(payload.email)
    .bind(payload.section)
    .bind(payload.first_name)
    .bind(payload.last_name)
    .bind(payload.age)
    .bind(payload.contact_number)
    .bind(payload.sex)
    .fetch_one(&mut *txn)
    .await?;

    let power_cards_str = PowerCard::get();
    let mut power_cards: Vec<PowerCard> = Vec::with_capacity(power_cards_str.len());

    for card_name in power_cards_str.iter() {
        let card = sqlx::query_as::<_, PowerCard>(
            r#"
                    INSERT INTO power_cards (name, user_id) 
                    VALUES ($1, $2)
                    RETURNING *
                    "#,
        )
        .bind(card_name)
        .bind(payload.id)
        .fetch_one(&mut *txn)
        .await?;

        power_cards.push(card);
    }

    txn.commit().await?;

    Ok((StatusCode::CREATED, axum::Json(user)))
}

#[derive(Debug, Deserialize)]
pub struct DeleteUserQuery {
    force: bool,
}

pub async fn delete_users(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<DeleteUserQuery>,
    extract::Json(users): extract::Json<Vec<String>>,
) -> Result<StatusCode, AppError> {
    let mut txn = pool.begin().await?;
    let users: Vec<String> = users
        .into_iter()
        .map(|user_id| format!("'{}'", user_id.trim()))
        .collect();

    // NOTE: Should've used SQLx's QueryBuilder
    let comma_sep_users = users.join(", ");

    debug!("{:?}", query);

    if query.force {
        // Delete battle cards
        sqlx::query(
            format!(
                "DELETE FROM battle_cards WHERE user_id IN ({})",
                comma_sep_users
            )
            .as_str(),
        )
        .execute(&mut *txn)
        .await?;

        // Delete card battle history
        sqlx::query(
            format!(
                "DELETE FROM card_battle_history WHERE user_id IN ({})",
                comma_sep_users
            )
            .as_str(),
        )
        .execute(&mut *txn)
        .await?;

        // Delete matches
        // NOTE: DANGEREOUS: If users have been swapped via twist of fate, it will delete the copy of
        // the orginal match
        sqlx::query(
            format!(
                "DELETE FROM match_sets WHERE user1_id IN ({}) OR user2_id IN ({}) OR og_user1_id IN ({}) OR og_user2_id IN ({})",
                comma_sep_users, comma_sep_users, comma_sep_users, comma_sep_users
            )
            .as_str(),
        )
        .execute(&mut *txn)
        .await?;
    }

    debug!("{}", comma_sep_users);

    // Delete power card
    sqlx::query(
        format!(
            "DELETE FROM power_cards WHERE user_id IN ({})",
            comma_sep_users
        )
        .as_str(),
    )
    .execute(&mut *txn)
    .await?;

    // Delete user
    sqlx::query(format!("DELETE FROM users WHERE id IN ({})", comma_sep_users).as_str())
        .execute(&mut *txn)
        .await?;

    // Update rankings
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

    Ok(StatusCode::NO_CONTENT)
}
