use axum::{extract::State, http::StatusCode, response::Result, Json};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Execute, PgPool};
use tracing::{debug, field::debug, info};

use crate::error::AppError;

#[derive(Debug, Serialize, FromRow)]
pub struct Rubric {
    id: i64,
    title: String,
    description: Option<String>,
    max_score: i16,
}

#[derive(Debug, Deserialize)]
pub struct CreateRubric {
    title: String,
    description: Option<String>,
    max_score: i16,
}

pub async fn create_rubric(
    State(pool): State<PgPool>,
    Json(rubric): Json<CreateRubric>,
) -> Result<StatusCode, AppError> {
    info!("Creating rubric...");
    debug!("{:?}", rubric);

    sqlx::query(
        r#"
        INSERT INTO rubrics (title, description, max_score)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(rubric.title)
    .bind(rubric.description)
    .bind(rubric.max_score)
    .execute(&pool)
    .await?;

    Ok(StatusCode::CREATED)
}

pub async fn get_rubrics(State(pool): State<PgPool>) -> Result<Json<Vec<Rubric>>, AppError> {
    let rubrics = sqlx::query_as::<_, Rubric>("SELECT * FROM rubrics")
        .fetch_all(&pool)
        .await?;

    Ok(Json(rubrics))
}

pub async fn delete_rubrics(
    State(pool): State<PgPool>,
    Json(rubrics): Json<Vec<i64>>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("DELETE FROM rubrics WHERE id IN (");

    let mut comma_sep = q_builder.separated(", ");

    rubrics.iter().for_each(|id| {
        comma_sep.push(id);
    });

    comma_sep.push_unseparated(")");

    let sql = q_builder.build().sql();

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::NO_CONTENT)
}
