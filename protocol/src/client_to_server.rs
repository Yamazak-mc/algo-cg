use algo_core::event::GameEvent;
use bevy_ecs::event::Event;
use serde::{Deserialize, Serialize};

/// An event that clients send to the server.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Event)]
pub enum ClientToServerEvent {
    RequestJoin,
    GameEventResponse(GameEvent),
}
