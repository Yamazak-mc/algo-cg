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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Event)]
pub struct JoinInfo {
    pub joined_player: JoinedPlayerInfo,
    pub room_size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Event)]
pub enum JoinedPlayerInfo {
    First(PlayerId),
    Second {
        just_joined: PlayerId,
        waiting_player: PlayerId,
    },
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
