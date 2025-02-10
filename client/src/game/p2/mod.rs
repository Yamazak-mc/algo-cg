use super::{card::instance::CardInstance, GameMode, CARD_DEPTH, TALON_TRANSLATION};
use crate::{AppState, JoinedPlayers};
use algo_core::{
    card::{CardView, TalonView},
    event::GameEvent,
};
use bevy::prelude::*;
use client::{
    client::{InboundEvent, OutboundEvent, DISCONNECTED_EV_ID},
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

macro_rules! unexpected_game_ev {
    ($ev:expr $(,)?) => {{
        warn!("unexpected GameEvent: {:?}", $ev);
        return;
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(GameMode = GameMode::TwoPlayers)]
enum P2State {
    #[default]
    WaitingForGameToStart,
    SetupTalon,
    _Todo, // TODO
    Disconnected,
}

/// Temporary data for initializing talon cards.
#[derive(Deref, DerefMut, Component)]
struct Talon(TalonView);

#[derive(Component, Reflect)]
struct TalonCardIndex(u32);

/// Tracks a card to draw next.
///
/// CardInstance entity that has corresponding `TalonCardIndex` value
/// will be the next card to be drawn.
#[derive(Component, Reflect)]
struct TalonTopCardIndex(u32);

pub fn p2_plugin(app: &mut App) {
    app.add_sub_state::<P2State>()
        .enable_state_scoped_entities::<P2State>()
        .add_systems(OnEnter(P2_CTX_STATE), setup)
        .add_systems(
            FixedUpdate,
            check_if_disconnected.run_if(in_state(P2_CTX_STATE)),
        )
        .add_systems(OnEnter(P2State::Disconnected), disconnected)
        .add_systems(
            FixedUpdate,
            recv_game_started_req.run_if(in_state(P2State::WaitingForGameToStart)),
        )
        .add_systems(OnEnter(P2State::SetupTalon), on_enter_setup_talon);
}

fn setup(joined_players: ResMut<JoinedPlayers>) {
    debug!("{:?}", *joined_players);
}

fn check_if_disconnected(
    mut ev_handler: ResMut<EventHandler>,
    mut state: ResMut<NextState<P2State>>,
) {
    if let Some(ev) = ev_handler.storage.take_request(DISCONNECTED_EV_ID) {
        warn!("disconnected from the server: {:?}", ev);
        state.set(P2State::Disconnected);
    }
}

fn disconnected(mut commands: Commands) {
    // TODO: Scene transition
    commands.set_state(AppState::Home);
}

fn recv_game_started_req(
    mut ev_handler: ResMut<EventHandler>,
    mut commands: Commands,
    mut state: ResMut<NextState<P2State>>,
) {
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
        unexpected => unexpected_game_ev!(unexpected),
    }

    ev_handler
        .send_response(id, OutboundEvent::GameEventResponse(GameEvent::RespOk))
        .unwrap();

    state.set(P2State::SetupTalon);
}

fn on_enter_setup_talon(talon: Single<(Entity, &Talon)>, mut commands: Commands) {
    let (talon_entity, talon) = *talon;

    // Spawn cards
    for (idx, color) in talon.cards.iter().enumerate() {
        let mut transform = Transform::from_translation(TALON_TRANSLATION);
        transform.translation.y += idx as f32 * CARD_DEPTH;

        commands.spawn((
            StateScoped(P2_CTX_STATE),
            transform,
            CardInstance::new(CardView::from_props(*color, None, false)),
            TalonCardIndex(idx as u32),
        ));
    }

    let top_idx = talon.cards.len() as u32 - 1;
    commands.spawn((StateScoped(P2_CTX_STATE), TalonTopCardIndex(top_idx)));

    // Cleanup temporary talon info
    commands.entity(talon_entity).despawn();
}
