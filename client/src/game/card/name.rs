use super::instance::CardInstance;
use crate::AppState;
use bevy::prelude::*;

pub fn card_name_plugin(app: &mut App) {
    let ctx_state = AppState::Game;

    app.add_systems(Update, update_name.run_if(in_state(ctx_state)));
}

fn update_name(
    mut commands: Commands,
    query: Query<(Entity, &CardInstance), Changed<CardInstance>>,
) {
    for (entity, card) in &query {
        commands
            .entity(entity)
            .insert(Name::new(format!("Card-{}", **card)));
    }
}
