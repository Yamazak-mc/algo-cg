use super::instance::CardInstance;
use crate::game::CTX_STATE;
use bevy::prelude::*;

pub fn card_name_plugin(app: &mut App) {
    app.add_systems(Update, update_name.run_if(in_state(CTX_STATE)));
}

fn update_name(
    mut commands: Commands,
    query: Query<(Entity, &CardInstance), Changed<CardInstance>>,
) {
    for (entity, card) in &query {
        commands
            .entity(entity)
            .insert(Name::new(format!("Card-{}", card.get())));
    }
}
