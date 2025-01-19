use crate::{
    card::{Card, CardNumber, CardView, TalonView},
    player::PlayerId,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// All possible events that occur during the game.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum GameEvent {
    /// The board is changed.
    BoardChanged(BoardChange),
    /// The game is started.
    GameStarted(TalonView),
    /// The turn order is determined.
    TurnOrderDetermined(Box<[PlayerId]>),
    /// The card is given out to the player.
    DrawnCard(PlayerId),
    /// The turn is given to the player.
    TurnStarted(PlayerId),
    /// The turn player has drawn a card.
    TurnPlayerDrawnCard,
    /// No cards left to draw; the game ends in a draw.
    OutOfCards,
    /// The turn player must decide what to do.
    ///
    /// If `optional` is true, they can choose not to attack and stay.
    /// Otherwise, they must attack.
    RequireAttackDecision {
        target_player: PlayerId,
        optional: bool,
    },
    /// The turn player decided whether to attack or stay.
    DecidedAttackOrStay { attack: bool },
    /// The turn player chose which card to attack to.
    AttackTargetChosen { target_idx: u32 },
    /// The turn player guessed the number of the card they targeted.
    MadeNumberGuess(CardNumber),
    /// The turn player correctly guessed the number
    /// and the opponent flipped the targeted card.
    AttackSucceeded,
    /// The turn player guessed the number incorrectly and
    /// placed the drawn card face-up on their field.
    AttackFailed,
    /// The player's placement has been determined.
    PlayerPlaceDetermined {
        player: PlayerId,
        /// A 0-based value where smaller numbers indicate a better placement.
        place: u8,
    },
    /// The turn is passed to the opponent.
    TurnEnded,
}

impl GameEvent {
    /// Returns `true` if the event represents a player's decision.
    pub(crate) fn is_decision(&self) -> bool {
        matches!(
            self,
            Self::DecidedAttackOrStay { .. }
                | Self::AttackTargetChosen { .. }
                | Self::MadeNumberGuess(_)
        )
    }

    /// Returns `true` if a player's decision is required after this event.
    pub(crate) fn is_decision_required(&self) -> bool {
        matches!(
            self,
            Self::RequireAttackDecision { .. }
                | Self::DecidedAttackOrStay { .. }
                | Self::AttackTargetChosen { .. }
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
        /// `None` if there's no card left.
        new_talon_top: Option<CardView>,
    },
    TalonToAttacker {
        new_talon_top: Option<CardView>,
    },
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
}
