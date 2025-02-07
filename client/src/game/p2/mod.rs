use crate::{AppState, JoinedPlayers};

use super::GameMode;
use bevy::prelude::*;
use client::{client::ReceivedRequest, utils::AddObserverExt};

const P2_CTX_STATE: GameMode = GameMode::TwoPlayers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(GameMode = GameMode::TwoPlayers)]
enum P2State {
    #[default]
    WaitingForGameToStart,
    _Todo, // TODO
}

pub fn p2_plugin(app: &mut App) {
    app.add_systems(OnEnter(P2_CTX_STATE), setup)
        .add_state_scoped_observer(P2State::WaitingForGameToStart, recv_game_started_req);
}

fn setup(mut commands: Commands, joined_players: ResMut<JoinedPlayers>) {
    info!("{:?}", *joined_players);
}

fn recv_game_started_req(trigger: Trigger<ReceivedRequest>) {
    info!("received: {:?}", trigger.event());
}
