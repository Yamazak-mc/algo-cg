use bevy_ecs::event::Event;
use serde::{Deserialize, Serialize};

/// An event that a server sends to clients.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Event)]
pub enum ServerToClientEvent {
    RequestJoinAccepted,
    ServerShutdown,
    Error(Box<str>),
}
