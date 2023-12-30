use axum::response::Result;
use axum::{extract, http};
use chrono::format;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{prelude::FromRow, PgPool};

use crate::error::AppError;

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
    contact_number: i32,
    sex: i16,
    rank_overall: i32,
    rank_section: i32,
    rank_title: Option<String>,
    score: i32,
    role: String,
    avatar_url: Option<String>,
    banner_url: Option<String>,
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
}

// TODO: Improve filtering, use comma separated fields query to fetch specific columns only
pub async fn get_users(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<UsersQuery>,
) -> Result<axum::Json<Vec<UserFetch>>, AppError> {
    let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT id, section, first_name, last_name, sex, rank_overall, rank_section, rank_title, score, avatar_url, banner_url FROM users");

    if let Some(section) = query.section {
        query_builder.push(format!(" WHERE section = '{}'", section));
    }

    if let Some(order_by) = query.order_by {
        query_builder.push(format!(
            " ORDER BY {} {}",
            order_by,
            query.order.unwrap_or("asc".to_string())
        ));
    }

    let users = sqlx::query_as::<_, UserFetch>(query_builder.sql())
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(users))
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
    if let Some(filter) = query.filter {
        let user = sqlx::query_as::<_, UserName>(
            "SELECT first_name, last_name FROM users WHERE id = ($1)",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        let user_value = serde_json::to_value(user).unwrap();

        Ok(axum::Json(user_value))
    } else {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ($1)")
            .bind(user_id)
            .fetch_one(&pool)
            .await?;
        let user_value = serde_json::to_value(user).unwrap();

        Ok(axum::Json(user_value))
    }
}

// For admin
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    id: uuid::Uuid,
    email: String,
    section: String,
    first_name: String,
    last_name: String,
    age: i32,
    contact_number: i32,
    sex: i16,
    score: i32,
    role: String,
}

pub async fn update_user(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<UpdateUser>,
) -> Result<http::StatusCode, AppError> {
    sqlx::query(
        r#"
        UPDATE users 
        SET 
            email = ($1), 
            section = ($2), 
            first_name = ($3), 
            last_name = ($4),
            age = ($5),
            sex = ($6),
            contact_number = ($7),
            score = ($8),
            role = ($9)
        WHERE id = ($10)
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
    .bind(payload.id)
    .execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
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
) -> Result<http::StatusCode, AppError> {
    let sql = format!("UPDATE users SET {} = ($1) WHERE id = ($2)", query.column);

    sqlx::query(sql.as_str())
        .bind(payload.url)
        .bind(payload.user_id)
        .execute(&pool)
        .await?;

    Ok(http::StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct Register {
    id: uuid::Uuid,
    email: String,
    section: String,
    first_name: String,
    last_name: String,
    age: i32,
    contact_number: i32,
    sex: i16,
}

pub async fn register(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(payload): extract::Json<Register>,
) -> Result<axum::Json<User>, AppError> {
    let res = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, section, first_name, last_name, age, contact_number, sex)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(&payload.id)
    .bind(&payload.email)
    .bind(&payload.section)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.age)
    .bind(&payload.contact_number)
    .bind(&payload.sex)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(user) => Ok(axum::Json(user)),
        Err(err) => match err {
            sqlx::Error::Database(db_err) => Err(AppError::new(
                http::StatusCode::CONFLICT,
                format!(
                    "Failed to register. Check if user already exists: {}",
                    db_err.to_string()
                ),
            )),
            _ => Err(AppError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to register: {}", err.to_string()),
            )),
        },
    }
}
