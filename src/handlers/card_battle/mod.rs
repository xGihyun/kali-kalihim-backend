// NOTE: The code for this is VERY bad. I wrote this a few months ago and copy pasted it.

use axum::{extract, http, response::Result};
use sqlx::PgPool;

use crate::error::AppError;

use self::model::{
    BattleCard, Block, Card, Change, CreateBattleCard, Effect, Multiplier, PlayerTurn,
    PlayerTurnResults, Stat, Strike, Target,
};

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

// Runs when admin simulates the card battle
pub async fn card_battle(extract::State(pool): extract::State<PgPool>) -> Result<(), AppError> {
    let latest_matches = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid)>(
        r#"
        WITH
        LatestDate AS (
            SELECT MAX(DATE_TRUNC('minute', created_at)) AS latest_date
            FROM match_sets
        ),
        LatestMatches AS (
            SELECT id, user1_id, user2_id
            FROM match_sets
            WHERE DATE_TRUNC('minute', created_at) = (SELECT latest_date FROM LatestDate)
        )
        SELECT * FROM LatestMatches
        "#,
    )
    .fetch_all(&pool)
    .await?;

    for (i, (match_set_id, user1_id, user2_id)) in latest_matches.iter().enumerate() {
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

        // For debugging purposes
        if i == 0 {
            println!(">> User1: {}\n", user1_id);
            println!("{:?}\n\n", battle_results.user1);
            println!(">> User2: {}\n", user2_id);
            println!("{:?}\n\n", battle_results.user2);
        }
    }

    // TODO: Store the results in the database
    // - Use a transaction
    // - card_battle_history table will contain an id, user_id, damage, is_cancelled, card_name, card_effect (if possible, perhaps just make it a summary like "stat + x% for user1")

    Ok(())
}

#[derive(Debug)]
struct UserStatus {
    damage: f32,
    multiplier: Multiplier,
    damage_reduction: f32,
    effect: Option<Effect>,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus {
            damage: 0.0,
            multiplier: Multiplier::default(),
            damage_reduction: 0.0,
            effect: None,
        }
    }
}

fn player_turn(
    (user1_cards, user2_cards): (&Vec<Option<Card>>, &Vec<Option<Card>>),
    (user1_turns, user2_turns): (&mut Vec<PlayerTurn>, &mut Vec<PlayerTurn>),
) {
    let (mut user1_status, mut user2_status) = (UserStatus::default(), UserStatus::default());

    for i in 0..NUMBER_OF_CARDS {
        let (user1_current_card, user2_current_card) = (&user1_cards[i], &user2_cards[i]);

        // println!(">> Index: {i}\n");
        // println!(">> User 1 Current Card: {:?}\n", user1_current_card);
        // println!(">> User 2 Current Card: {:?}\n", user2_current_card);
        // println!(">> User 1 Current Status (Before): {:?}\n", user1_status);
        // println!(">> User 2 Current Status (Before): {:?}\n", user2_status);

        apply_effects(
            (&mut user1_status, &mut user2_status),
            (user1_current_card.as_ref(), user2_current_card.as_ref()),
        );

        // NOTE: Is there a better way to do this?
        if let Some(card) = user1_current_card {
            match card {
                Card::Strike(strike) => match strike {
                    Strike::LegStrike(strike_stat)
                    | Strike::TempleStrike(strike_stat)
                    | Strike::ShoulderStrike(strike_stat)
                    | Strike::ShoulderThrust(strike_stat)
                    | Strike::EyePoke(strike_stat)
                    | Strike::StomachThrust(strike_stat)
                    | Strike::HeadStrike(strike_stat) => {
                        user1_turns[i].damage = Strike::simulate(strike_stat, &mut user1_status);
                    }
                    _ => eprintln!("UNKNOWN: Strike Card."),
                },
                Card::Block(block) => match block {
                    Block::LegStrike(block_stat)
                    | Block::TempleStrike(block_stat)
                    | Block::ShoulderStrike(block_stat)
                    | Block::ShoulderThrust(block_stat)
                    | Block::EyePoke(block_stat)
                    | Block::StomachThrust(block_stat)
                    | Block::HeadStrike(block_stat) => {
                        user1_status.damage_reduction = block_stat.damage_reduction;

                        if let Some(Card::Strike(strike)) = user2_current_card {
                            let is_cancelled = block_stat.strike_to_cancel == *strike;

                            if is_cancelled {
                                user2_turns[i].is_cancelled = true;
                                user1_status.effect = Some(block_stat.effect.clone());
                            }
                        }
                    }
                    _ => eprintln!("UNKNOWN: Block Card"),
                },
            }
        }

        if let Some(card) = user2_current_card {
            match card {
                Card::Strike(strike) => match strike {
                    Strike::LegStrike(strike_stat)
                    | Strike::TempleStrike(strike_stat)
                    | Strike::ShoulderStrike(strike_stat)
                    | Strike::ShoulderThrust(strike_stat)
                    | Strike::EyePoke(strike_stat)
                    | Strike::StomachThrust(strike_stat)
                    | Strike::HeadStrike(strike_stat) => {
                        user2_turns[i].damage = Strike::simulate(strike_stat, &mut user2_status);
                    }
                    _ => eprintln!("UNKNOWN: Strike Card."),
                },
                Card::Block(block) => match block {
                    Block::LegStrike(block_stat)
                    | Block::TempleStrike(block_stat)
                    | Block::ShoulderStrike(block_stat)
                    | Block::ShoulderThrust(block_stat)
                    | Block::EyePoke(block_stat)
                    | Block::StomachThrust(block_stat)
                    | Block::HeadStrike(block_stat) => {
                        user2_status.damage_reduction = block_stat.damage_reduction;

                        if let Some(Card::Strike(strike)) = user1_current_card {
                            let is_cancelled = block_stat.strike_to_cancel == *strike;

                            if is_cancelled {
                                user1_turns[i].is_cancelled = true;
                                user2_status.effect = Some(block_stat.effect.clone());
                            }
                        }
                    }
                    _ => eprintln!("UNKNOWN: Block Card"),
                },
            }
        }

        // println!(">> User 1 Current Status (After): {:?}\n", user1_status);
        // println!(">> User 2 Current Status (After): {:?}\n\n", user2_status);
    }
}

fn change_stat(effect: &Effect, multiplier: &mut Multiplier) {
    match effect.stat {
        Stat::Accuracy => match effect.action {
            Change::Decrease => {
                multiplier.accuracy -= effect.amount;
            }
            Change::Increase => {
                multiplier.accuracy += effect.amount;
            }
        },
        Stat::Damage => match effect.action {
            Change::Decrease => {
                multiplier.damage -= effect.amount;
            }
            Change::Increase => {
                multiplier.damage += effect.amount;
            }
        },
    }
}

fn apply_effects(
    (user1_status, user2_status): (&mut UserStatus, &mut UserStatus),
    (user1_card, user2_card): (Option<&Card>, Option<&Card>),
) {
    if let Some(card) = user1_card {
        match card {
            Card::Strike(_) => {
                if let Some(effect) = &user1_status.effect {
                    match effect.target {
                        Target::Opponent => {
                            change_stat(&effect, &mut user2_status.multiplier);
                        }
                        Target::Owner => {
                            change_stat(&effect, &mut user1_status.multiplier);
                        }
                    }
                }
            }
            Card::Block(_) => {}
        }
    }

    if let Some(card) = user2_card {
        match card {
            Card::Strike(_) => {
                if let Some(effect) = &user2_status.effect {
                    match effect.target {
                        Target::Opponent => {
                            change_stat(&effect, &mut user1_status.multiplier);
                        }
                        Target::Owner => {
                            change_stat(&effect, &mut user2_status.multiplier);
                        }
                    }
                }
            }
            Card::Block(_) => {}
        }
    }
}

// pub fn card_battle(player_1_cards_str: String, player_2_cards_str: String) -> Result<(), AppError> {
//     let player_1_cards_json = format!(r#"{player_1_cards_str}"#);
//     let player_2_cards_json = format!(r#"{player_2_cards_str}"#);
//
//     let player_1_cards: Vec<BattleCard> =
//         serde_json::from_str(&player_1_cards_json).expect("Failed to parse Player 1 cards");
//     let player_2_cards: Vec<BattleCard> =
//         serde_json::from_str(&player_2_cards_json).expect("Failed to parse Player 2 cards");
//
//     let mut player_1_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); 6];
//     let mut player_2_turns: Vec<PlayerTurn> = vec![PlayerTurn::default(); 6];
//
//     player_turn(
//         &player_1_cards,
//         &player_2_cards,
//         &mut player_1_turns,
//         &mut player_2_turns,
//     );
//
//     let battle_results = PlayerDamages {
//         player_1: player_1_turns,
//         player_2: player_2_turns,
//     };
//
//     println!("{:?}", battle_results);
//
//     // let battle_results_js = JsValue::from_serde(&battle_results).expect("Failed to serialize");
//
//     Ok(())
// }
