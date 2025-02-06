use crate::JoinedPlayers;

use super::GameMode;
use bevy::prelude::*;

const P2_CTX_STATE: GameMode = GameMode::TwoPlayers;

pub fn p2_plugin(app: &mut App) {
    app.add_systems(OnEnter(P2_CTX_STATE), setup);
}

fn setup(mut commands: Commands, joined_players: ResMut<JoinedPlayers>) {
    info!("{:?}", *joined_players);
}
