use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct BadgeDb {
    id: uuid::Uuid,
    name: String,
    description: String,
    user_id: uuid::Uuid,
}

pub enum Badge {
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

impl Badge {
    pub fn info(badge: Badge) -> Result<BadgeInfo, AppError> {
        match badge {
            Badge::TopPlayer => Ok(BadgeInfo { 
                name: String::from("Top 5"), 
                description: String::from("Five shining stars mark your brilliance. This badge celebrates your place among the top 5 performers.") 
            }),
            Badge::BestInSkill(skill_badge) => {
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
