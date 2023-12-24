use axum::http;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::error::AppError;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum Change {
    Increase,
    Decrease,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Stat {
    Accuracy,
    Damage,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Card {
    Strike(Strike),
    Block(Block),
}

impl Card {
    // pub fn apply_effects() {

    // }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum Target {
    Owner,
    Opponent,
}

#[derive(Debug, Deserialize, Clone, Serialize, PartialEq)]
pub struct Effect {
    pub action: Change,
    pub amount: f32,
    pub stat: Stat,
    pub target: Target,
}

impl Effect {
    pub fn change_stat(&self, multiplier: &mut Multiplier) {
        match self.stat {
            Stat::Accuracy => match self.action {
                Change::Decrease => {
                    multiplier.accuracy -= self.amount;
                }
                Change::Increase => {
                    multiplier.accuracy += self.amount;
                }
            },
            Stat::Damage => match self.action {
                Change::Decrease => {
                    multiplier.damage -= self.amount;
                }
                Change::Increase => {
                    multiplier.damage += self.amount;
                }
            },
        }
    }

    pub fn summarize(&self) -> String {
        let mut target: String;

        match self.target {
            Target::Owner => {
                target = "user".into();
            }
            Target::Opponent => {
                target = "opponent".into();
            }
        }

        match self.stat {
            Stat::Accuracy => match self.action {
                Change::Decrease => {
                    format!("Decrease {} accuracy by {:.2}%", target, self.amount)
                }
                Change::Increase => {
                    format!("Increase {} accuracy by {:.2}%", target, self.amount)
                }
            },
            Stat::Damage => match self.action {
                Change::Decrease => {
                    format!("Decrease {} damage by {:.2}%", target, self.amount)
                }
                Change::Increase => {
                    format!("Increase {} damage by {:.2}%", target, self.amount)
                }
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct StrikeStat {
    pub name: String,
    pub damage: f32,
    pub accuracy: f32,
    pub effect: Effect,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum Strike {
    LegStrike(StrikeStat),
    TempleStrike(StrikeStat),
    ShoulderStrike(StrikeStat),
    ShoulderThrust(StrikeStat),
    EyePoke(StrikeStat),
    StomachThrust(StrikeStat),
    HeadStrike(StrikeStat),
    Unknown,
}

impl Strike {
    pub fn new(name: &str) -> Self {
        match name {
            "leg_strike" => Strike::LegStrike(StrikeStat {
                name: name.into(),
                accuracy: 0.9,
                damage: 5.0,
                effect: Effect {
                    action: Change::Increase,
                    amount: 0.5,
                    stat: Stat::Accuracy,
                    target: Target::Owner,
                },
            }),
            "temple_strike" => Strike::TempleStrike(StrikeStat {
                name: name.into(),
                accuracy: 0.75,
                damage: 10.0,
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.5,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "shoulder_strike" => Strike::ShoulderStrike(StrikeStat {
                name: name.into(),
                accuracy: 0.8,
                damage: 10.0,
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.1,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "shoulder_thrust" => Strike::ShoulderThrust(StrikeStat {
                name: name.into(),
                damage: 8.0,
                accuracy: 0.85,
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.1,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "eye_poke" => Strike::EyePoke(StrikeStat {
                name: name.into(),
                damage: 12.0,
                accuracy: 0.6,
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.15,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "stomach_thrust" => Strike::StomachThrust(StrikeStat {
                name: name.into(),
                damage: 10.0,
                accuracy: 0.85,
                effect: Effect {
                    action: Change::Increase,
                    amount: 0.5,
                    stat: Stat::Damage,
                    target: Target::Owner,
                },
            }),
            "head_strike" => Strike::HeadStrike(StrikeStat {
                name: name.into(),
                damage: 18.0,
                accuracy: 0.5,
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.15,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            _ => Strike::Unknown,
        }
    }

    pub fn simulate(
        &self,
        user_status: &mut UserStatus,
        user_turn: &mut PlayerTurn,
        opponent_card: Option<&Card>,
        opponent_status: &mut UserStatus,
    ) -> anyhow::Result<(), AppError> {
        match self {
            Strike::LegStrike(strike_stat)
            | Strike::TempleStrike(strike_stat)
            | Strike::ShoulderStrike(strike_stat)
            | Strike::ShoulderThrust(strike_stat)
            | Strike::EyePoke(strike_stat)
            | Strike::StomachThrust(strike_stat)
            | Strike::HeadStrike(strike_stat) => {
                user_turn.card_name = Some(strike_stat.name.clone());
                user_turn.card_effect = Some(strike_stat.effect.summarize());
                opponent_status.damage_reduction = 0.0;
                user_status.damage_reduction = 0.0;

                let mut is_cancelled = false;

                if let Some(Card::Block(block)) = opponent_card {
                    if let Ok(block_stat) = block.get_stat() {
                        is_cancelled = *self == block_stat.strike_to_cancel;
                        opponent_status.damage_reduction = block_stat.damage_reduction;
                    }
                }

                let rng: f32 = rand::thread_rng().gen_range(0.0..1.0);
                let accuracy = strike_stat.accuracy * user_status.multiplier.accuracy;
                let damage = (strike_stat.damage
                    * (user_status.multiplier.damage - opponent_status.damage_reduction));

                if rng <= accuracy && is_cancelled == false {
                    user_status.damage += damage;
                    user_status.effect = Some(strike_stat.effect.clone());

                    // println!("\nCURRENT DAMAGE: {}\n", damage);

                    user_turn.damage = damage;
                } else {
                    // println!("\nMISSED!\n");
                    user_turn.damage = 0.0;
                }

                Ok(())
            }
            Strike::Unknown => Err(AppError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown battle card.",
            )),
        }
    }

    pub fn is_cancelled(&self, strike_to_cancel: Strike) -> bool {
        *self == strike_to_cancel
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStat {
    pub name: String,
    pub damage_reduction: f32,
    pub strike_to_cancel: Strike,
    pub effect: Effect,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Block {
    LegStrike(BlockStat),
    TempleStrike(BlockStat),
    ShoulderStrike(BlockStat),
    ShoulderThrust(BlockStat),
    EyePoke(BlockStat),
    StomachThrust(BlockStat),
    HeadStrike(BlockStat),
    Unknown,
}

impl Block {
    pub fn new(name: &str) -> Self {
        match name {
            "leg_strike" => Block::LegStrike(BlockStat {
                name: "leg_strike_block".into(),
                damage_reduction: 0.1,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Increase,
                    amount: 0.1,
                    stat: Stat::Accuracy,
                    target: Target::Owner,
                },
            }),
            "temple_strike" => Block::TempleStrike(BlockStat {
                name: "temple_strike_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.1,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "shoulder_strike" => Block::ShoulderStrike(BlockStat {
                name: "shoulder_strike_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Increase,
                    amount: 0.5,
                    stat: Stat::Damage,
                    target: Target::Owner,
                },
            }),
            "shoulder_thrust" => Block::ShoulderThrust(BlockStat {
                name: "shoulder_thrust_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.1,
                    stat: Stat::Accuracy,
                    target: Target::Opponent,
                },
            }),
            "eye_poke" => Block::EyePoke(BlockStat {
                name: "eye_poke_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.1,
                    stat: Stat::Damage,
                    target: Target::Opponent,
                },
            }),
            "stomach_thrust" => Block::StomachThrust(BlockStat {
                name: "stomach_thrust_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Increase,
                    amount: 0.5,
                    stat: Stat::Damage,
                    target: Target::Owner,
                },
            }),
            "head_strike" => Block::HeadStrike(BlockStat {
                name: "head_strike_block".into(),
                damage_reduction: 0.15,
                strike_to_cancel: Strike::new(name),
                effect: Effect {
                    action: Change::Decrease,
                    amount: 0.2,
                    stat: Stat::Damage,
                    target: Target::Opponent,
                },
            }),
            _ => Block::Unknown,
        }
    }

    pub fn simulate(
        &self,
        user_status: &mut UserStatus,
        user_turn: &mut PlayerTurn,
        opponent_card: Option<&Card>,
        opponent_turn: &mut PlayerTurn,
    ) -> anyhow::Result<(), AppError> {
        match self {
            Block::LegStrike(block_stat)
            | Block::TempleStrike(block_stat)
            | Block::ShoulderStrike(block_stat)
            | Block::ShoulderThrust(block_stat)
            | Block::EyePoke(block_stat)
            | Block::StomachThrust(block_stat)
            | Block::HeadStrike(block_stat) => {
                user_status.damage_reduction = block_stat.damage_reduction;
                user_turn.card_name = Some(block_stat.name.clone());

                if let Some(Card::Strike(strike)) = opponent_card {
                    opponent_turn.is_cancelled =
                        strike.is_cancelled(block_stat.strike_to_cancel.clone());
                }

                if opponent_turn.is_cancelled {
                    user_status.effect = Some(block_stat.effect.clone());
                    user_turn.card_effect = Some(block_stat.effect.summarize());
                }

                Ok(())
            }
            Block::Unknown => Err(AppError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown block battle card.",
            )),
        }
    }

    pub fn get_stat(&self) -> anyhow::Result<BlockStat, AppError> {
        match self {
            Block::LegStrike(block_stat)
            | Block::TempleStrike(block_stat)
            | Block::ShoulderStrike(block_stat)
            | Block::ShoulderThrust(block_stat)
            | Block::EyePoke(block_stat)
            | Block::StomachThrust(block_stat)
            | Block::HeadStrike(block_stat) => Ok(block_stat.clone()),
            Block::Unknown => Err(AppError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown block battle card.",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct BattleCard {
    pub id: uuid::Uuid,
    pub name: String,
    pub skill: String,
    pub user_id: uuid::Uuid,
    pub match_set_id: uuid::Uuid,
    pub turn_number: i16,
}

#[derive(Debug, Deserialize)]
pub struct CreateBattleCard {
    pub name: String,
    pub skill: String,
    pub user_id: uuid::Uuid,
    pub match_set_id: uuid::Uuid,
    pub turn_number: i16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerTurnResults {
    pub user1: Vec<PlayerTurn>,
    pub user2: Vec<PlayerTurn>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerTurn {
    pub card_name: Option<String>,
    pub card_effect: Option<String>,
    pub damage: f32,
    pub is_cancelled: bool,
}

impl Default for PlayerTurn {
    fn default() -> Self {
        PlayerTurn {
            card_name: None,
            card_effect: None,
            damage: 0.0,
            is_cancelled: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Multiplier {
    pub damage: f32,
    pub accuracy: f32,
}

impl Default for Multiplier {
    fn default() -> Self {
        Multiplier {
            damage: 1.0,
            accuracy: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserStatus {
    pub damage: f32,
    pub multiplier: Multiplier,
    pub damage_reduction: f32,
    pub effect: Option<Effect>,
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
