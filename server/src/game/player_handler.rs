use crate::{InboundEvent, OutboundEvent};
use algo_core::{event::GameEvent, player::PlayerId};
use protocol::{EventKind, NextEventId, WithMetadata};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use super::ServerInternalEvent;

#[derive(Debug)]
pub struct PlayerHandler {
    // sender
    tx: UnboundedSender<ServerInternalEvent>,
    next_id: NextEventId,

    // receiver
    expected_response_id: Option<protocol::EventId>,
}

impl PlayerHandler {
    pub fn new(tx: UnboundedSender<ServerInternalEvent>) -> Self {
        Self {
            tx,
            next_id: NextEventId::default(),
            expected_response_id: None,
        }
    }

    pub fn send_message(&mut self, message: OutboundEvent) -> anyhow::Result<()> {
        let id = self.next_id.produce();

        self.tx.send(ServerInternalEvent::Out(WithMetadata {
            kind: protocol::EventKind::Request,
            id,
            event: message,
        }))?;

        Ok(())
    }

    pub fn send_game_event(&mut self, event: GameEvent) -> anyhow::Result<()> {
        let id = self.next_id.produce();

        let event = WithMetadata {
            kind: protocol::EventKind::Request,
            id,
            event: OutboundEvent::GameEvent(event),
        };

        debug!("{:?}", event);

        self.tx.send(ServerInternalEvent::Out(event))?;

        self.expected_response_id = Some(id);

        Ok(())
    }

    pub fn check_for_game_event_response(
        &mut self,
        received: WithMetadata<InboundEvent>,
    ) -> Option<GameEvent> {
        let WithMetadata { kind, id, event } = received;

        if kind == EventKind::Request {
            warn!("ignoring request: id={:?}, event={:?}", id, event);
            return None;
        }

        let Some(expected_id) = self.expected_response_id else {
            warn!("invalid player handler state");
            return None;
        };

        if id != expected_id {
            warn!("unexpected response: id={:?}, event={:?}", id, event);
            return None;
        }

        match event {
            InboundEvent::GameEventResponse(game_event) => Some(game_event),
            unexpected => {
                warn!(
                    "ignoring unexpected InboundEvent: id={:?}, event={:?}",
                    id, unexpected
                );
                None
            }
        }
    }

    pub fn notify_player_disconnected(&mut self, player_id: PlayerId) -> anyhow::Result<()> {
        let id = self.next_id.produce();

        self.tx.send(ServerInternalEvent::Out(WithMetadata {
            kind: EventKind::Request,
            id,
            event: OutboundEvent::PlayerDisconnected(player_id),
        }))?;

        Ok(())
    }
}
