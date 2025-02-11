use crate::{card::Card, event::CardLocation};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[repr(transparent)]
pub struct PlayerId(u32);

impl From<u32> for PlayerId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl PlayerId {
    #[allow(unused)]
    pub fn dummy_pair() -> (Self, Self) {
        (Self(101), Self(102))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Player {
    pub field: Vec<Card>,
    pub attacker: Option<Card>,
}

impl Player {
    pub fn insert_card_to_field(&mut self, card: Card) -> u32 {
        let Err(idx) = self.field.binary_search(&card) else {
            panic!("duplicated card detected: {:?}", card);
        };

        self.field.insert(idx, card);

        idx as u32
    }

    pub fn insert_attacker(&mut self, card: Card) -> CardLocation {
        if let Some(attacker) = self.attacker.take() {
            panic!("attacker already exists: {:?}", attacker);
        }

        self.attacker = Some(card);

        CardLocation::Attacker
    }
}

/// Tracks the player whose turn is next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TurnPlayer {
    ids: VecDeque<PlayerId>,
}

impl TurnPlayer {
    pub fn new(ids: impl IntoIterator<Item = PlayerId>) -> Self {
        let ids = ids.into_iter().collect();
        Self { ids }
    }

    /// Returns the next turn player's ID.
    pub fn get(&self) -> Option<PlayerId> {
        self.ids.front().cloned()
    }

    /// Advances the turn to the next player.
    pub fn advance(&mut self) {
        let id = self.ids.pop_front().unwrap();
        self.ids.push_back(id);
    }

    pub fn turn_order(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.ids.iter().cloned()
    }
}

pub struct AssignPlayerId(PlayerId);

impl Default for AssignPlayerId {
    fn default() -> Self {
        Self(PlayerId(0))
    }
}

impl AssignPlayerId {
    pub fn assign(&mut self) -> PlayerId {
        self.0 .0 += 1;
        self.0
    }
}
