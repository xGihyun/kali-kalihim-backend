use sqlx::PgExecutor;

use crate::error::AppError;

pub mod badge;
pub mod card_battle;
pub mod matches;
pub mod matchmake;
pub mod power_card;
pub mod rubric;
pub mod score;
pub mod section;
pub mod user;

pub async fn update_ranks<'c>(executor: impl PgExecutor<'c>) -> Result<(), AppError> {
    let sql = r#"
        WITH OverallRank AS (
            SELECT id, DENSE_RANK() OVER (ORDER BY score DESC) AS new_rank
            FROM users
            WHERE role = 'user'
        ), SectionRank AS (
            SELECT id, DENSE_RANK() OVER (PARTITION BY section ORDER BY score DESC) AS new_rank
            FROM users
            WHERE role = 'user'
        )
        UPDATE users u
        SET rank_overall = ovr.new_rank, rank_section = sr.new_rank
        FROM OverallRank ovr, SectionRank sr
        WHERE u.id = ovr.id AND u.id = sr.id
        "#;

    executor.execute(sql).await?;

    Ok(())
}
