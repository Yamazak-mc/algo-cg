use std::fmt;

use algo_core::{event::GameEvent, player::PlayerId};
use bevy_ecs::event::Event;
use serde::{Deserialize, Serialize};

/// An event that a server sends to clients.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Event)]
pub enum ServerToClientEvent {
    RequestJoinAccepted(JoinInfo),
    PlayerJoined(JoinInfo),
    PlayerDisconnected(PlayerId),
    GameEvent(GameEvent),
    ServerShutdown,
    Error(Box<str>),
}

impl ServerToClientEvent {
    pub fn is_game_event(&self) -> bool {
        matches!(self, Self::GameEvent(_))
    }

    pub fn into_game_event(self) -> GameEvent {
        match self {
            Self::GameEvent(game_event) => game_event,
            v => panic!("not a game event: {:?}", v),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize, Event)]
pub struct JoinInfo {
    pub joined_player: JoinedPlayerInfo,
    pub room_size: u8,
}

impl fmt::Debug for JoinInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.joined_player)
        // Ignoring `room_size`, as it is always 2 currently.
    }
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize, Event)]
pub enum JoinedPlayerInfo {
    First(PlayerId),
    Second {
        just_joined: PlayerId,
        waiting_player: PlayerId,
    },
}

impl fmt::Debug for JoinedPlayerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        map.entry(&"joined", &self.assigned_player_id());
        if let Some(id) = self.waiting_player_id() {
            map.entry(&"waiting", &id);
        }
        map.finish()
    }
}

impl JoinedPlayerInfo {
    pub fn assigned_player_id(&self) -> PlayerId {
        match self {
            Self::First(id) => *id,
            Self::Second { just_joined, .. } => *just_joined,
        }
    }

    pub fn waiting_player_id(&self) -> Option<PlayerId> {
        match self {
            Self::First(_) => None,
            Self::Second {
                just_joined: _,
                waiting_player,
            } => Some(*waiting_player),
        }
    }

    pub fn join_position(&self) -> u8 {
        match self {
            Self::First(_) => 1,
            Self::Second { .. } => 2,
        }
    }
}
