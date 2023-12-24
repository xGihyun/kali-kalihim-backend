// NOTE: The code for this is VERY bad. I wrote this a few months ago and copy pasted it.

use axum::{extract, http, response::Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::error::AppError;

use self::model::{
    BattleCard, Block, Card, Change, CreateBattleCard, Effect, Multiplier, PlayerTurn,
    PlayerTurnResults, Stat, Strike, Target, UserStatus,
};

use super::matchmake::MatchQuery;

// pub mod card_battle;
pub mod model;

const NUMBER_OF_CARDS: usize = 6;

pub async fn insert_cards(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<Vec<CreateBattleCard>>,
) -> Result<http::StatusCode, AppError> {
    let mut txn = pool.begin().await?;

    for card in payload {
        sqlx::query(
            "INSERT INTO battle_cards (name, skill, user_id, match_set_id, turn_number) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(card.name)
        .bind(card.skill)
        .bind(card.user_id)
        .bind(card.match_set_id)
        .bind(card.turn_number)
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
                println!("Unknown skill.");
            }
        }
    }

    // println!(">> Battle Cards: {:?}\n", battle_cards);

    Ok(battle_cards)
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct CardBattle {
    id: uuid::Uuid,
    card_name: Option<String>,
    card_effect: Option<String>,
    damage: f32,
    is_cancelled: bool,
    turn_number: i32,
    match_set_id: uuid::Uuid,
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
        println!("\n----- MATCH START -----\n");

        // Each user can only have 6 cards
        let user1_cards = get_cards(&pool, &user1_id, &match_set_id).await?;
        let user2_cards = get_cards(&pool, &user2_id, &match_set_id).await?;

        let mut user1_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); NUMBER_OF_CARDS];
        let mut user2_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); NUMBER_OF_CARDS];

        player_turn(
            (&user1_cards, &user2_cards),
            (&mut user1_turns, &mut user2_turns),
        );

        let battle_results = PlayerTurnResults {
            user1: user1_turns,
            user2: user2_turns,
        };

        insert_turns(&pool, &battle_results, match_set_id).await?;

        // For debugging purposes
        if i == 0 {
            println!(">> User1: {}\n", user1_id);
            println!("{:?}\n\n", battle_results.user1);
            println!(">> User2: {}\n", user2_id);
            println!("{:?}\n\n", battle_results.user2);
        }
    }

    Ok(())
}

async fn insert_turns(
    pool: &PgPool,
    results: &PlayerTurnResults,
    match_set_id: &uuid::Uuid,
) -> Result<(), AppError> {
    let mut txn = pool.begin().await?;

    for (i, turn) in results.user1.clone().into_iter().enumerate() {
        let card_battle_results = sqlx::query(
            r#"
            INSERT INTO card_battle_history (card_name, card_effect, damage, is_cancelled, turn_number, match_set_id)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        )
        .bind(turn.card_name)
        .bind(turn.card_effect)
        .bind(turn.damage)
        .bind(turn.is_cancelled)
        .bind(i as i32 + 1)
        .bind(match_set_id)
        .fetch_all(&mut *txn)
        .await?;
    }

    for (i, turn) in results.user2.clone().into_iter().enumerate() {
        let card_battle_results = sqlx::query(
            r#"
            INSERT INTO card_battle_history (card_name, card_effect, damage, is_cancelled, turn_number, match_set_id)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        )
        .bind(turn.card_name)
        .bind(turn.card_effect)
        .bind(turn.damage)
        .bind(turn.is_cancelled)
        .bind(i as i32 + 1)
        .bind(match_set_id)
        .fetch_all(&mut *txn)
        .await?;
    }

    txn.commit().await?;

    Ok(())
}

fn player_turn(
    (user1_cards, user2_cards): (&Vec<Option<Card>>, &Vec<Option<Card>>),
    (user1_turns, user2_turns): (&mut Vec<PlayerTurn>, &mut Vec<PlayerTurn>),
) -> anyhow::Result<(), AppError> {
    let (mut user1_status, mut user2_status) = (UserStatus::default(), UserStatus::default());
    let (mut user1_status_temp, mut user2_status_temp) =
        (UserStatus::default(), UserStatus::default());

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

        user1_status_temp = user1_status.clone();
        user2_status_temp = user2_status.clone();

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
            Some(effect) => {
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
            Some(effect) => {
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
