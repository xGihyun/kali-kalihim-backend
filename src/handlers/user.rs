use axum::extract;
use axum::response::Result;
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
    // Will be used for fetching specific columns on specific use cases
    // let users = sqlx::query!("SELECT id, first_name FROM users")
    //     .fetch_all(&pool)
    //     .await?;

    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(users))
}
