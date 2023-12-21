use axum::response::Result;
use axum::{extract, http};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};

use crate::error::AppError;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct User {
    id: uuid::Uuid,
    email: String,
    password: String,
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

pub async fn get_users(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<User>>, AppError> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(users))
}

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
    axum::Json(payload): axum::Json<UpdateUser>,
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
