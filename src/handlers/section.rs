use axum::{extract, http, response::Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::error::AppError;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Section {
    id: String,
    name: String,
    user_limit: i32,
}

pub async fn get_sections(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<Section>>, AppError> {
    let sections = sqlx::query_as::<_, Section>("SELECT * FROM sections")
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(sections))
}

#[derive(Debug, Deserialize)]
pub struct CreateSection {
    name: String,
    user_limit: i32,
}

pub async fn insert_section(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<CreateSection>,
) -> Result<axum::Json<Section>, AppError> {
    let section = sqlx::query_as::<_, Section>(
        r#"
        INSERT INTO sections (id, name, user_limit) 
        VALUES ((LOWER(REPLACE(TRIM(BOTH ' ' FROM $1), ' ', '_'))), $1, $2) 
        RETURNING *
        "#,
    )
    .bind(payload.name)
    .bind(payload.user_limit)
    .fetch_one(&pool)
    .await?;

    Ok(axum::Json(section))
}

// TODO: Add update and delete functions
