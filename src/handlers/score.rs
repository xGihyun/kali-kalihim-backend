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
}

// Add a query whether to increment or just change the score literally
pub async fn update_score(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<UpdateScore>,
) -> Result<http::StatusCode, AppError> {
    // current score + payload score
    sqlx::query!(
        "UPDATE users SET score = score + ($1) WHERE id = ($2)",
        payload.score,
        payload.user_id
    )
    .execute(&pool)
    .await?;

    Ok(http::StatusCode::OK)
}
