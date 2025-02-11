use crate::{
    card::{Card, CardNumber, CardView, TalonView},
    player::PlayerId,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// All possible events that occur during the game.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum GameEvent {
    /// The board has been changed.
    BoardChanged(BoardChange),
    /// The game has started.
    GameStarted(TalonView),
    /// The turn order has been determined.
    TurnOrderDetermined(Vec<PlayerId>),
    /// The card is distributed to the player.
    CardDistributed(PlayerId),
    /// The turn is given to the player.
    TurnStarted(PlayerId),
    /// The turn player has drawn a card.
    TurnPlayerDrewCard,
    /// No cards left to draw; the game ends in a draw.
    NoCardsLeft,
    /// The turn player must select a card to attack.
    AttackTargetSelectionRequired { target_player: PlayerId },
    /// The turn player chose which card to attack to.
    AttackTargetSelected { target_idx: u32 },
    /// The turn player must guess the number.
    NumberGuessRequired,
    /// The turn player guessed the number of the card they targeted.
    NumberGuessed(CardNumber),
    /// The turn player correctly guessed the number
    /// and the opponent flipped the targeted card.
    AttackSucceeded,
    /// The turn player guessed the number incorrectly and
    /// placed the drawn card face-up on their field.
    AttackFailed,
    /// The player's field has no more face down cards.
    AttackedPlayerLost,
    /// The game is ended.
    GameEnded,
    /// The turn player must select where to attack or stay.
    AttackOrStayDecisionRequired,
    /// The turn player has decided whether to attack or stay.
    AttackOrStayDecided { attack: bool },
    /// The turn is passed to the opponent.
    TurnEnded,
    /// OK response
    RespOk,
}

impl GameEvent {
    /// Returns `true` if the event represents a player's decision.
    pub fn is_decision(&self) -> bool {
        matches!(
            self,
            Self::AttackTargetSelected { .. }
                | Self::NumberGuessed(_)
                | Self::AttackOrStayDecided { .. }
        )
    }

    /// Returns `true` if a turn player is required to respond with their decision.
    pub fn is_decision_required(&self) -> bool {
        matches!(
            self,
            Self::AttackTargetSelectionRequired { .. }
                | Self::NumberGuessRequired { .. }
                | Self::AttackOrStayDecisionRequired { .. }
        )
    }

    /// Returns a new instance of `Self`
    /// with information hidden from the specified viewer removed.
    pub(crate) fn view(&self, viewer: PlayerId) -> Self {
        match self {
            Self::BoardChanged(change) => Self::BoardChanged(change.view(viewer)),
            other => other.clone(),
        }
    }

    pub fn kind(&self) -> GameEventKind {
        match self {
            Self::BoardChanged(_) => GameEventKind::BoardChanged,
            Self::GameStarted(_) => GameEventKind::GameStarted,
            Self::TurnOrderDetermined(_) => GameEventKind::TurnOrderDetermined,
            Self::CardDistributed(_) => GameEventKind::CardDistributed,
            Self::TurnStarted(_) => GameEventKind::TurnStarted,
            Self::TurnPlayerDrewCard => GameEventKind::TurnPlayerDrewCard,
            Self::NoCardsLeft => GameEventKind::NoCardsLeft,
            Self::AttackTargetSelectionRequired { .. } => {
                GameEventKind::AttackTargetSelectionRequired
            }
            Self::AttackTargetSelected { .. } => GameEventKind::AttackTargetSelected,
            Self::NumberGuessRequired => GameEventKind::NumberGuessRequired,
            Self::NumberGuessed(_) => GameEventKind::NumberGuessed,
            Self::AttackSucceeded => GameEventKind::AttackSucceeded,
            Self::AttackFailed => GameEventKind::AttackFailed,
            Self::AttackedPlayerLost => GameEventKind::AttackedPlayerLost,
            Self::GameEnded => GameEventKind::GameEnded,
            Self::AttackOrStayDecisionRequired => GameEventKind::AttackOrStayDecisionRequired,
            Self::AttackOrStayDecided { .. } => GameEventKind::AttackOrStayDecided,
            Self::TurnEnded => GameEventKind::TurnEnded,
            Self::RespOk => GameEventKind::RespOk,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventKind {
    BoardChanged,
    GameStarted,
    TurnOrderDetermined,
    CardDistributed,
    TurnStarted,
    TurnPlayerDrewCard,
    NoCardsLeft,
    AttackTargetSelectionRequired,
    AttackTargetSelected,
    NumberGuessRequired,
    NumberGuessed,
    AttackSucceeded,
    AttackFailed,
    AttackedPlayerLost,
    GameEnded,
    AttackOrStayDecisionRequired,
    AttackOrStayDecided,
    TurnEnded,
    RespOk,
}

/// All possible board changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BoardChange {
    /// A card is moved.
    CardMoved {
        player: PlayerId,
        movement: CardMovement,
        card: CardView,
    },
    /// A card is revealed.
    CardRevealed {
        player: PlayerId,
        location: CardLocation,
        card: Card,
    },
}

impl BoardChange {
    /// Returns a new instance of `Self`
    /// with information hidden from the specified viewer removed.
    #[allow(clippy::clone_on_copy)]
    fn view(&self, viewer: PlayerId) -> Self {
        let mut ret = self.clone();

        match ret {
            Self::CardMoved {
                player,
                movement: _,
                ref mut card,
            } if player != viewer => {
                *card = card.public_view();
            }
            _ => (),
        }

        ret
    }
}

/// Represents locations of cards.
///
/// NOTE: This enum does not include a `Talon` variant,
///       as it is not needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum CardLocation {
    /// A card placed on the field.
    Field {
        /// The card's index from the left, viewed from the owner.
        idx: u32,
    },
    /// The card that the turn player has drawn and is currently used to attack.
    Attacker,
}

/// Represents a movement of cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum CardMovement {
    TalonToField {
        /// The card's index from the left, viewed from the owner.
        insert_at: u32,
    },
    TalonToAttacker,
    AttackerToField {
        insert_at: u32,
    },
}

/// A wrapper type for a [`GameEvent`].
pub struct GameEventRequest {
    pub event: GameEvent,
    /// what kind of response the game is expecting.
    pub expecting: GameEvResponseKind,
}

/// All possible kinds of responses to a [`GameEvent`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameEvResponseKind {
    /// Inform the game that the client has read the event.
    Acknowledgement,
    /// Respond to the event by making a decision.
    Decision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventQueue<T> {
    pub main_queue: VecDeque<T>,
    pub sub_queue: VecDeque<T>,
}

impl<T> Default for EventQueue<T> {
    fn default() -> Self {
        Self {
            main_queue: VecDeque::default(),
            sub_queue: VecDeque::default(),
        }
    }
}

impl<T> EventQueue<T> {
    /// Pops a next event to stage.
    ///
    /// If there is no event scheduled, returns `None`.
    pub fn pop_next(&mut self) -> Option<T> {
        if let Some(ev) = self.sub_queue.pop_front() {
            return Some(ev);
        }

        self.main_queue.pop_front()
    }

    pub fn push_main(&mut self, event: T) {
        self.main_queue.push_back(event);
    }

    pub fn push_sub(&mut self, event: T) {
        self.sub_queue.push_back(event);
    }
}
