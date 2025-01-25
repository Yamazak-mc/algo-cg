use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::card::{Card, CardView};

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
    pub fn dummy() -> Self {
        Self(101)
    }

    #[allow(unused)]
    pub fn dummy_2() -> Self {
        Self(102)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Player {
    pub field: Vec<Card>,
    pub attacker: Option<Card>,
}

impl Player {
    pub fn public_view(&self) -> PlayerView {
        let cards = self.field.iter().map(Card::public_view).collect();
        PlayerView::new(cards)
    }
}

pub struct PlayerView {
    pub cards: Vec<CardView>,
}

impl PlayerView {
    pub fn new(cards: Vec<CardView>) -> Self {
        Self { cards }
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
