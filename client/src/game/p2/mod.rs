use super::GameMode;
use crate::JoinedPlayers;
use algo_core::{card::TalonView, event::GameEvent};
use bevy::prelude::*;
use client::{
    client::{InboundEvent, OutboundEvent},
    EventHandler,
};

const P2_CTX_STATE: GameMode = GameMode::TwoPlayers;

/// Checks if the client has received an game event.
macro_rules! take_game_event {
    ($ev_handler:expr $(,)?) => {{
        let Some((id, ev)) = $ev_handler
            .storage
            .take_request_if(InboundEvent::is_game_event)
        else {
            return;
        };
        (id, ev.into_game_event())
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(GameMode = GameMode::TwoPlayers)]
enum P2State {
    #[default]
    WaitingForGameToStart,
    _Todo, // TODO
}

pub fn p2_plugin(app: &mut App) {
    app.add_sub_state::<P2State>()
        .enable_state_scoped_entities::<P2State>()
        .add_systems(OnEnter(P2_CTX_STATE), setup)
        .add_systems(
            FixedUpdate,
            recv_game_started_req.run_if(in_state(P2State::WaitingForGameToStart)),
        );
}

fn setup(joined_players: ResMut<JoinedPlayers>) {
    debug!("{:?}", *joined_players);
}

fn recv_game_started_req(mut ev_handler: ResMut<EventHandler>, mut commands: Commands) {
    let (id, ev) = take_game_event!(ev_handler);
    match ev {
        GameEvent::GameStarted(talon_view) => {
            debug!("game started: {:?}", talon_view);
            commands.spawn((
                StateScoped(P2_CTX_STATE),
                Talon(talon_view),
                Name::new("Talon"),
            ));
        }
        unexpected => warn!("unexpected GameEvent: {:?}", unexpected),
    }

    ev_handler
        .send_response(id, OutboundEvent::GameEventResponse(GameEvent::RespOk))
        .unwrap();
}

#[derive(Component)]
struct Talon(TalonView);
