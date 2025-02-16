use algo_core::event::GameEvent;
use bevy::{ecs::system::SystemParam, prelude::*};
use client::{
    client::{InboundEvent, OutboundEvent},
    utils::AddObserverExt,
    EventHandler,
};

use super::P2_CTX_STATE;

pub fn response_plugin(app: &mut App) {
    app.add_systems(OnEnter(P2_CTX_STATE), setup)
        .add_state_scoped_observer_named(P2_CTX_STATE, Resp::send_resp);
}

fn setup(mut commands: Commands) {
    commands.spawn((StateScoped(P2_CTX_STATE), RespId(None)));
}

#[derive(Event)]
pub struct Resp(Option<GameEvent>);

impl Resp {
    pub const OK: Self = Self(Some(GameEvent::RespOk));

    #[allow(unused)]
    pub fn new(ev: GameEvent) -> Self {
        Self(Some(ev))
    }

    fn send_resp(mut trigger: Trigger<Self>, mut ev_handler: GameEvHandler) {
        if let Some(ev) = trigger.0.take() {
            ev_handler.send_game_ev(ev);
        }
    }
}

#[derive(Component)]
struct RespId(Option<protocol::EventId>);

#[derive(SystemParam)]
pub struct GameEvHandler<'w> {
    ev_handler: ResMut<'w, EventHandler>,
    resp_id: Single<'w, &'static mut RespId>,
}

impl GameEvHandler<'_> {
    pub fn recv_game_ev(&mut self) -> Option<GameEvent> {
        let (id, ev) = self
            .ev_handler
            .storage
            .take_request_if(InboundEvent::is_game_event)?;

        self.resp_id.0 = Some(id);

        Some(ev.into_game_event())
    }

    pub fn send_game_ev(&mut self, event: GameEvent) {
        let Some(id) = self.resp_id.0.take() else {
            warn!(
                "GameEvent response has already been sent. ignoring: {:?}",
                event
            );
            return;
        };
        self.ev_handler
            .send_response(id, OutboundEvent::GameEventResponse(event))
            .unwrap();
    }
}
