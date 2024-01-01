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

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct SectionWithUserCount {
    id: String,
    name: String,
    user_limit: i32,
    user_count: i64,
}

pub async fn get_sections(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<Section>>, AppError> {
    let sections = sqlx::query_as::<_, Section>("SELECT * FROM sections ORDER BY name")
        .fetch_all(&pool)
        .await?;

    Ok(axum::Json(sections))
}

pub async fn get_sections_with_count(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<SectionWithUserCount>>, AppError> {
    let sections = sqlx::query_as::<_, SectionWithUserCount>(
        r#"
        WITH UsersInSection AS (
            SELECT section, COUNT(*) AS user_count 
            FROM users 
            GROUP BY section
        )
        SELECT 
        s.*, 
        COALESCE(uis.user_count, 0) AS user_count
        FROM sections s
        LEFT JOIN UsersInSection uis ON uis.section = s.id
        ORDER BY name
        "#,
    )
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
    extract::Json(payload): extract::Json<CreateSection>,
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

pub async fn delete_section(
    extract::State(pool): extract::State<PgPool>,
    extract::Json(sections): extract::Json<Vec<String>>,
) -> Result<http::StatusCode, AppError> {
    let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("DELETE FROM sections");

    if sections.len() == 1 {
        query_builder.push(format!(" WHERE id = '{}'", &sections[0]));
    } else {
        let sections: Vec<String> = sections.iter().map(|s| format!("'{}'", s.trim())).collect();

        let sections_comma_sep = sections.join(", ");
        query_builder.push(format!(" WHERE id IN ({})", sections_comma_sep));
    }

    sqlx::query(query_builder.sql()).execute(&pool).await?;

    Ok(http::StatusCode::NO_CONTENT)
}
