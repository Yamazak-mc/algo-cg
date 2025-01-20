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
    pub player_id: PlayerId,
    pub join_position: u8,
    pub room_size: u8,
}
