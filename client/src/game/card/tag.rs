use super::instance::CardInstance;
use crate::{game::CARD_HEIGHT, AppState};
use bevy::prelude::*;
use client::utils::{
    add_observer_ext::AddStateScopedObserver as _,
    world_to_2d::{AddFollower, DespawnFollower},
};

const FONT_SIZE: f32 = 32.0;

const TAG_3D_OFFSET: Vec3 = Vec3::new(0.0, 0.0, -CARD_HEIGHT / 2.0);
const TAG_2D_OFFSET: Vec3 = Vec3::new(0.0, FONT_SIZE * 0.75, 0.0);

pub fn card_tag_plugin(app: &mut App) {
    let ctx_state = AppState::Game;

    app.add_state_scoped_observer(ctx_state, AddCardTag::handle_trigger)
        .add_state_scoped_observer(ctx_state, RemoveCardTag::handle_trigger);
}

#[derive(Event)]
pub struct AddCardTag;

impl AddCardTag {
    fn handle_trigger(trigger: Trigger<Self>, mut commands: Commands, query: Query<&CardInstance>) {
        let card_entity = trigger.entity();
        let card = query.get(card_entity).unwrap();
        if card.pub_info.revealed || card.priv_info.is_none() {
            return;
        }

        let tag_entity = commands
            .spawn((
                Text2d(format!("{}", card.priv_info.unwrap().number.0)),
                TextFont::from_font_size(FONT_SIZE),
                // DEBUG
                Name::new("CardTag"),
            ))
            .id();

        commands.trigger_targets(
            AddFollower {
                follower: tag_entity,
                offset_3d: TAG_3D_OFFSET,
                offset_2d: TAG_2D_OFFSET,
            },
            card_entity,
        );
    }
}

#[derive(Event)]
pub struct RemoveCardTag;

impl RemoveCardTag {
    fn handle_trigger(trigger: Trigger<Self>, mut commands: Commands) {
        let card_entity = trigger.entity();

        commands.trigger_targets(DespawnFollower, card_entity);
    }
}
