use axum::{extract::{State, Path}, http::StatusCode, response::Result, Json};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};
use tracing::{debug, field::debug, info};

use crate::error::AppError;

pub struct BadgeInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CreateBadge {
    name: String,
    description: String,
    user_id: uuid::Uuid,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Badge {
    id: uuid::Uuid,
    name: String,
    description: String,
    user_id: uuid::Uuid,
}

pub enum BadgeType {
    TopPlayer,
    BestInSkill(SkillBadge),
    Unknown
    // BestInFootwork(FootworkBadge),
}

pub enum SkillBadge {
    Strikes(BadgeInfo),
    Blocks(BadgeInfo),
    ForwardSinawali(BadgeInfo),
    SinawaliVariation(BadgeInfo),
    Unknown,
}

pub enum FootworkBadge {
    Caballero,
    Triangle,
    ReverseTriangle,
    StarReach,
}

impl BadgeInfo {
    fn new(badge_type: BadgeType) -> Result<Self, AppError> {
        match badge_type {
            BadgeType::TopPlayer => Ok(BadgeInfo { 
                name: String::from("Top 5"), 
                description: String::from("Five shining stars mark your brilliance. This badge celebrates your place among the top 5 performers.") 
            }),
            BadgeType::BestInSkill(skill_badge) => {
                match skill_badge {
                    SkillBadge::Blocks(info) | SkillBadge::Strikes(info) | SkillBadge::ForwardSinawali(info) | SkillBadge::SinawaliVariation(info) => {
                        Ok(info)
                    }
                    SkillBadge::Unknown => Err(AppError::new(StatusCode::NOT_FOUND, "Invalid skill badge."))
                }
            },
            BadgeType::Unknown => Err(AppError::new(StatusCode::NOT_FOUND, "Invalid badge type."))
        }
    }
}

impl BadgeType {
    fn new(name: &str) -> Self {
        match name {
            "Top 5" => Self::TopPlayer,
            "Superior Striker" => Self::BestInSkill(SkillBadge::new("strikes")),
            "Defensive Blocker" => Self::BestInSkill(SkillBadge::new("blocks")),
            "Forward Sinawali Specialist" => Self::BestInSkill(SkillBadge::new("forward_sinawali")),
            "Sinawali Virtuoso" => Self::BestInSkill(SkillBadge::new("sinawali_variation")),
            _ => Self::Unknown
        }
    }

    pub fn info(badge: Self) -> Result<BadgeInfo, AppError> {
        match badge {
            Self::TopPlayer => Ok(BadgeInfo { 
                name: String::from("Top 5"), 
                description: String::from("Five shining stars mark your brilliance. This badge celebrates your place among the top 5 performers.") 
            }),
            Self::BestInSkill(skill_badge) => {
                match skill_badge {
                    SkillBadge::Blocks(info) | SkillBadge::Strikes(info) | SkillBadge::ForwardSinawali(info) | SkillBadge::SinawaliVariation(info) => {
                        Ok(info)
                    }
                    SkillBadge::Unknown => Err(AppError::new(StatusCode::NOT_FOUND, "Invalid skill badge."))
                }
            }
            Self::Unknown => Err(AppError::new(StatusCode::NOT_FOUND, "Invalid badge type."))
        }
    }
}

impl SkillBadge {
    pub fn new(skill: &str) -> Self {
        match skill {
            "strikes" => Self::Strikes(BadgeInfo {
                name: String::from("Superior Striker"),
                description: String::from("The best striker."),
            }),
            "blocks" => Self::Blocks(BadgeInfo {
                name: String::from("Defensive Blocker"),
                description: String::from("The best blocker."),
            }),
            "forward_sinawali" => Self::ForwardSinawali(BadgeInfo {
                name: String::from("Forward Sinawali Specialist"),
                description: String::from("Best in Forward Sinawali."),
            }),
            "sinawali_variation" => Self::SinawaliVariation(BadgeInfo {
                name: String::from("Sinawali Virtuoso"),
                description: String::from("Best in Sinawali Variation."),
            }),
            _ => Self::Unknown,
        }
    }
}

pub async fn get_badges(
    State(pool): State<PgPool>,
    Path(user_id): Path<uuid::Uuid>
) -> Result<Json<Vec<Badge>>, AppError> {
    let badges = sqlx::query_as::<_, Badge>(
        "SELECT * FROM badges WHERE user_id = ($1)"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(badges))
}

pub async fn toggle_badge(
    State(pool): State<PgPool>,
    Path(user_id): Path<uuid::Uuid>,
    Json(badges): Json<Vec<String>>
) -> Result<StatusCode, AppError> {
    let mut txn = pool.begin().await?;

    info!("Creating badges for user: {}", user_id);
    debug!("{:#?}", badges);

    let current_badges = sqlx::query_scalar::<_, String>("SELECT name FROM badges WHERE user_id = ($1)").bind(user_id).fetch_all(&mut *txn).await?;

    for badge in current_badges.iter() {
        if !badges.contains(badge) {
            sqlx::query(
                "DELETE FROM badges WHERE name = ($1) AND user_id = ($2)"
            )
            .bind(badge)
            .bind(user_id)
            .execute(&mut *txn)
            .await?;
        }
    }

    for badge in badges.iter() {
        let badge_type = BadgeType::new(badge.trim());
        let badge_info = BadgeInfo::new(badge_type)?;

        sqlx::query(
            r#"
            INSERT INTO badges (name, description, user_id)
            VALUES ($1, $2, $3)
            "#
        )
        .bind(badge_info.name)
        .bind(badge_info.description)
        .bind(user_id)
        .execute(&mut *txn)
        .await?;
    }

    txn.commit().await?;

    Ok(StatusCode::CREATED)
}