// NOTE: The code for this is VERY bad. I wrote this a few months ago and copy pasted it.

use axum::{extract, http, response::Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::{info, warn};

use crate::error::AppError;

use self::model::{
    Block, Card, CreateBattleCard, Effect, PlayerTurn, PlayerTurnResults, Strike, Target,
    UserStatus,
};

use super::matchmake::MatchQuery;

// pub mod card_battle;
pub mod model;

const NUMBER_OF_CARDS: usize = 6;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct CardBattle {
    id: uuid::Uuid,
    card_name: Option<String>,
    card_effect: Option<String>,
    damage: f32,
    is_cancelled: bool,
    turn_number: i32,
    match_set_id: uuid::Uuid,
    user_id: uuid::Uuid,
}

pub async fn get_match_results(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(match_set_id): extract::Path<uuid::Uuid>,
) -> Result<axum::Json<Vec<CardBattle>>, AppError> {
    let card_battle = sqlx::query_as::<_, CardBattle>(
        "SELECT * FROM card_battle_history WHERE match_set_id = ($1) ORDER BY user_id, turn_number",
    )
    .bind(match_set_id)
    .fetch_all(&pool)
    .await?;

    Ok(axum::Json(card_battle))
}

pub async fn insert_cards(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<Vec<CreateBattleCard>>,
) -> Result<http::StatusCode, AppError> {
    let mut txn = pool.begin().await?;

    for (i, card) in payload.into_iter().enumerate() {
        sqlx::query(
            r#"
            WITH LatestMatch AS (
                SELECT id
                FROM match_sets
                WHERE og_user1_id = ($3) OR og_user2_id = ($3)
                ORDER BY created_at DESC
                LIMIT 1 
            )
            INSERT INTO battle_cards (name, skill, user_id, turn_number, match_set_id) 
            SELECT 
                ($1) AS name, 
                ($2) AS skill, 
                ($3) AS user_id, 
                ($4) AS turn_number,
                LatestMatch.id AS match_set_id
            FROM LatestMatch
            WHERE NOT EXISTS (
                SELECT 1
                FROM battle_cards
                WHERE user_id = ($3)
                  AND turn_number = ($4)
                  AND match_set_id = LatestMatch.id
            )
            "#,
        )
        .bind(card.name)
        .bind(card.skill)
        .bind(card.user_id)
        // .bind(card.match_set_id)
        .bind(i as i16 + 1)
        .execute(&mut *txn)
        .await?;
    }

    txn.commit().await?;

    Ok(http::StatusCode::CREATED)
}

async fn get_cards(
    pool: &PgPool,
    user_id: &uuid::Uuid,
    match_set_id: &uuid::Uuid,
) -> Result<Vec<Option<Card>>, AppError> {
    let battle_cards_res = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT name, skill 
        FROM battle_cards 
        WHERE user_id = ($1) AND match_set_id = ($2) 
        ORDER BY turn_number 
        LIMIT 6
        "#,
    )
    .bind(user_id)
    .bind(match_set_id)
    .fetch_all(pool)
    .await?;

    // Default to None since some users may not have submitted their cards
    let mut battle_cards: Vec<Option<Card>> = vec![None; NUMBER_OF_CARDS];

    for (name, skill) in battle_cards_res.iter() {
        match skill.as_str() {
            "strike" => {
                let strike_card = Strike::new(name.as_str());
                // Find the first None and replace it with the strike card
                if let Some(index) = battle_cards.iter().position(|card| card.is_none()) {
                    battle_cards[index] = Some(Card::Strike(strike_card));
                }
            }
            "block" => {
                let block_card = Block::new(name.as_str());
                // Find the first None and replace it with the block card
                if let Some(index) = battle_cards.iter().position(|card| card.is_none()) {
                    battle_cards[index] = Some(Card::Block(block_card));
                }
            }
            _ => {
                warn!("Unknown skill.");
            }
        }
    }

    // println!(">> Battle Cards: {:?}\n", battle_cards);

    Ok(battle_cards)
}

// Runs when admin simulates the card battle
pub async fn card_battle(
    extract::State(pool): extract::State<PgPool>,
    extract::Query(query): extract::Query<MatchQuery>,
) -> Result<(), AppError> {
    let mut txn = pool.begin().await?;

    let matches = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid)>(
        r#"SELECT id, user1_id, user2_id FROM match_sets WHERE set = ($1) AND section = ($2)"#,
    )
    .bind(query.set)
    .bind(query.section)
    .fetch_all(&mut *txn)
    .await?;

    for (i, (match_set_id, user1_id, user2_id)) in matches.iter().enumerate() {
        info!("----- MATCH START -----");
        info!("{match_set_id}");

        // Each user can only have 6 cards
        let user1_cards = get_cards(&pool, user1_id, match_set_id).await?;
        let user2_cards = get_cards(&pool, user2_id, match_set_id).await?;

        let mut user1_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); NUMBER_OF_CARDS];
        let mut user2_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); NUMBER_OF_CARDS];

        player_turn(
            (&user1_cards, &user2_cards),
            (&mut user1_turns, &mut user2_turns),
        )?;

        let battle_results = PlayerTurnResults {
            user1: (*user1_id, user1_turns),
            user2: (*user2_id, user2_turns),
        };

        process_match_results(&pool, &battle_results, match_set_id).await?;
        update_total_damage(&pool, match_set_id).await?;

        // For debugging purposes
        if i == 0 {
            info!(">> User1: {}\n", user1_id);
            info!("{:?}\n\n", battle_results.user1);
            info!(">> User2: {}\n", user2_id);
            info!("{:?}\n\n", battle_results.user2);
        }
    }

    Ok(())
}

// Could be improved
// Could merge query with the query on process_match_results()
async fn update_total_damage(pool: &PgPool, match_set_id: &uuid::Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        WITH TotalDamage AS (
            SELECT
                user_id,
                match_set_id,
                SUM(damage) as total_damage 
            FROM
                card_battle_history
            WHERE
                match_set_id = ($1)
            GROUP BY
                user_id, match_set_id
        ), UpdateTotalDamage AS (
            UPDATE match_sets
            SET
                user1_total_damage = (SELECT total_damage FROM TotalDamage WHERE user_id = match_sets.user1_id),
                user2_total_damage = (SELECT total_damage FROM TotalDamage WHERE user_id = match_sets.user2_id)
            WHERE
                match_sets.id = ($1)
            RETURNING user1_total_damage, user2_total_damage
        )

        UPDATE users
        SET score = score + 10
        WHERE id = (
            SELECT
                CASE WHEN user1_total_damage > user2_total_damage THEN user1_id ELSE user2_id END
            FROM match_sets
            WHERE id = ($1)
        );
        "#,
    )
    .bind(match_set_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn process_match_results(
    pool: &PgPool,
    results: &PlayerTurnResults,
    match_set_id: &uuid::Uuid,
) -> Result<(), AppError> {
    let (user_id, turns) = results.user1.clone();
    insert_turns(&user_id, turns, match_set_id, pool).await?;

    let (user_id, turns) = results.user2.clone();
    insert_turns(&user_id, turns, match_set_id, pool).await?;

    Ok(())
}

async fn insert_turns(
    user_id: &uuid::Uuid,
    turns: Vec<PlayerTurn>,
    match_set_id: &uuid::Uuid,
    pool: &PgPool, // txn: &mut PgConnection,
) -> Result<(), AppError> {
    if turns[0].card_name.is_some() {
        let mut txn = pool.begin().await?;

        let sql = r#"
        INSERT INTO card_battle_history (
            user_id,
            card_name,
            card_effect,
            damage,
            is_cancelled,
            turn_number,
            match_set_id
        )
        SELECT
            ($1) AS user_id,
            ($2) AS card_name,
            ($3) AS card_effect,
            ($4) AS damage,
            ($5) AS is_cancelled,
            ($6) AS turn_number,
            ($7) AS match_set_id
        WHERE NOT EXISTS (
            SELECT 1
            FROM card_battle_history
            WHERE user_id = ($1)
              AND turn_number = ($6)
              AND match_set_id = ($7)
        )
    "#;

        for (i, turn) in turns.into_iter().enumerate() {
            sqlx::query(sql)
                .bind(user_id)
                .bind(turn.card_name)
                .bind(turn.card_effect)
                .bind(turn.damage)
                .bind(turn.is_cancelled)
                .bind(i as i32 + 1)
                .bind(match_set_id)
                .execute(&mut *txn)
                .await?;
        }

        txn.commit().await?;
    }

    Ok(())
}

fn player_turn(
    (user1_cards, user2_cards): (&Vec<Option<Card>>, &Vec<Option<Card>>),
    (user1_turns, user2_turns): (&mut Vec<PlayerTurn>, &mut Vec<PlayerTurn>),
) -> anyhow::Result<(), AppError> {
    let (mut user1_status, mut user2_status) = (UserStatus::default(), UserStatus::default());
    // let (mut user1_status_temp, mut user2_status_temp) =
    //     (UserStatus::default(), UserStatus::default());

    let (mut prev_effect1, mut prev_effect2): (Option<Effect>, Option<Effect>) = (None, None);

    for i in 0..NUMBER_OF_CARDS {
        let (user1_current_card, user2_current_card) =
            (user1_cards[i].as_ref(), user2_cards[i].as_ref());

        // println!("[!] TURN # {i}\n");
        // println!("[?] CARDS\n");
        // println!(">> User 1: {:?}\n", user1_current_card);
        // println!(">> User 2: {:?}\n", user2_current_card);
        // println!(">>> BEFORE\n");
        // println!(">> User 1 Current Status: {:?}\n", user1_status);
        // println!(">> User 2 Current Status: {:?}\n", user2_status);

        // user1_status_temp = user1_status.clone();
        // user2_status_temp = user2_status.clone();

        // Change multipliers
        apply_effects(
            (&mut user1_status, &mut user2_status),
            (user1_current_card, user2_current_card),
        );

        // NOTE: Is there a better way to do this?
        if let Some(card) = user1_current_card {
            match card {
                Card::Strike(strike) => {
                    strike.simulate(
                        &mut user1_status,
                        &mut user1_turns[i],
                        user2_current_card,
                        &mut user2_status,
                    )?;
                }
                Card::Block(block) => {
                    block.simulate(
                        &mut user1_status,
                        &mut user1_turns[i],
                        user2_current_card,
                        &mut user2_turns[i],
                    )?;
                }
            }
        }

        if let Some(card) = user2_current_card {
            match card {
                Card::Strike(strike) => {
                    strike.simulate(
                        &mut user2_status,
                        &mut user2_turns[i],
                        user1_current_card,
                        &mut user1_status,
                    )?;
                }
                Card::Block(block) => {
                    block.simulate(
                        &mut user2_status,
                        &mut user2_turns[i],
                        user1_current_card,
                        &mut user1_turns[i],
                    )?;
                }
            }
        }

        // println!(">>> AFTER\n");
        // println!(">> User 1 Current Status: {:?}\n", user1_status);
        // println!(">> User 2 Current Status: {:?}\n\n", user2_status);

        match prev_effect1 {
            Some(_) => {
                // println!("PREVIOUS EFFECT (1): {:?}", effect);
                user1_status = UserStatus::default();
                // user1_status.effect = None;
                // user1_status.multiplier = Multiplier::default();
                prev_effect1 = None;
            }
            None => {
                prev_effect1 = user1_status.effect.clone();
                // println!("NEW EFFECT (1)");
            }
        }

        match prev_effect2 {
            Some(_) => {
                // println!("PREVIOUS EFFECT (2): {:?}", effect);
                user2_status = UserStatus::default();
                // user1_status.effect = None;
                // user1_status.multiplier = Multiplier::default();
                prev_effect2 = None;
            }
            None => {
                prev_effect2 = user2_status.effect.clone();
                // println!("NEW EFFECT (2)");
            }
        }
    }

    Ok(())
}

fn apply_effect(
    user_status: &mut UserStatus,
    opponent_status: &mut UserStatus,
    user_card: Option<&Card>,
) {
    if let Some(card) = user_card {
        match card {
            Card::Strike(_) | Card::Block(_) => {
                if let Some(effect) = &user_status.effect {
                    match effect.target {
                        Target::Opponent => {
                            effect.change_stat(&mut opponent_status.multiplier);
                        }
                        Target::Owner => {
                            effect.change_stat(&mut user_status.multiplier);
                        }
                    }
                }
            }
        }
    }
}

fn apply_effects(
    (user1_status, user2_status): (&mut UserStatus, &mut UserStatus),
    (user1_card, user2_card): (Option<&Card>, Option<&Card>),
) {
    apply_effect(user1_status, user2_status, user1_card);
    apply_effect(user2_status, user1_status, user2_card);
}
