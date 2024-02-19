use axum::{extract::{State, Path}, http::StatusCode, response::Result, Json};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};

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

impl BadgeType {
    pub fn info(badge: BadgeType) -> Result<BadgeInfo, AppError> {
        match badge {
            BadgeType::TopPlayer => Ok(BadgeInfo { 
                name: String::from("Top 5"), 
                description: String::from("Five shining stars mark your brilliance. This badge celebrates your place among the top 5 performers.") 
            }),
            BadgeType::BestInSkill(skill_badge) => {
                match skill_badge {
                    SkillBadge::Blocks(info) | SkillBadge::Strikes(info) | SkillBadge::ForwardSinawali(info) | SkillBadge::SinawaliVariation(info) => {
                        Ok(info)
                    }
                    SkillBadge::Unknown => Err(AppError::new(StatusCode::NOT_FOUND, "Invalid badge."))
                }
            }
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

pub async fn create_badge(
    State(pool): State<PgPool>,
) -> Result<StatusCode, AppError> {

    todo!();

    Ok(StatusCode::CREATED)
}
