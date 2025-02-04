use super::instance::CardInstance;
use crate::game::{CARD_HEIGHT, CTX_STATE};
use bevy::prelude::*;
use client::utils::{
    world_to_2d::{AddFollower, DespawnFollower},
    AddObserverExt as _,
};

const FONT_SIZE: f32 = 32.0;

const TAG_3D_OFFSET: Vec3 = Vec3::new(0.0, 0.0, CARD_HEIGHT / 2.0);
const TAG_2D_OFFSET: Vec3 = Vec3::new(0.0, -FONT_SIZE * 0.75, 0.0);

pub fn card_tag_plugin(app: &mut App) {
    app.add_state_scoped_observer(CTX_STATE, SpawnCardTag::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, DespawnCardTag::handle_trigger);
}

#[derive(Event)]
pub struct SpawnCardTag;

impl SpawnCardTag {
    fn handle_trigger(trigger: Trigger<Self>, mut commands: Commands, query: Query<&CardInstance>) {
        let card_entity = trigger.entity();
        let card = query.get(card_entity).unwrap().get();
        if card.pub_info.revealed || card.priv_info.is_none() {
            return;
        }

        let tag_entity = commands
            .spawn((
                StateScoped(CTX_STATE),
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
        commands.entity(card_entity).insert(HasCardTag);
    }
}

#[derive(Event)]
pub struct DespawnCardTag;

impl DespawnCardTag {
    fn handle_trigger(trigger: Trigger<Self>, query: Query<&HasCardTag>, mut commands: Commands) {
        let card_entity = trigger.entity();
        if query.get(card_entity).is_ok() {
            commands.trigger_targets(DespawnFollower, card_entity);
            commands.entity(card_entity).remove::<HasCardTag>();
        }
    }
}

#[derive(Component)]
pub struct HasCardTag;
