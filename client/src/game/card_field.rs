use crate::AppState;
use algo_core::player::PlayerId;
use bevy::prelude::*;
use client::utils::{animate_once::AnimateOnce, AddObserverExt as _};

use super::CARD_WIDTH_PLUS_GAP;

const CARD_INSERTION_ANIMATION_SECS: f32 = 0.5;

pub fn card_field_plugin(app: &mut App) {
    app.add_state_scoped_observer(AppState::Game, CardPosition::init)
        .add_state_scoped_observer(AppState::Game, CardPosition::shift);
}

#[derive(Debug, Default, Component)]
#[require(Transform)]
pub struct CardField {
    cards: Vec<Entity>,
}

impl CardField {
    /// Inserts a pre-existing card into the field.
    pub fn insert_card(
        &mut self,
        self_entity: Entity,
        idx: u32,
        entity: Entity,
        commands: &mut Commands,
    ) {
        if !self.cards.is_empty() {
            commands.trigger_targets(OtherCardInserted { idx }, self.cards.clone());
        }

        commands.entity(entity).insert(CardPosition {
            origin: self_entity,
            idx,
            len: self.cards.len() as u32 + 1,
        });

        self.cards.insert(idx as usize, entity);
    }

    pub fn cards(&self) -> &[Entity] {
        &self.cards
    }
}

#[derive(Debug, Component)]
#[require(CardField)]
pub struct CardFieldOwnedBy(pub PlayerId);

/// A marker component used with `CardField`.
///
/// To get an opponent's `CardField`, use `Without<MyCardField>` filter.
#[derive(Debug, Component)]
#[require(CardField)]
pub struct MyCardField;

#[derive(Debug, Clone, Copy, Component)]
pub struct CardPosition {
    origin: Entity,
    idx: u32,
    len: u32,
}

impl CardPosition {
    fn init(
        trigger: Trigger<OnAdd, Self>,
        mut commands: Commands,
        mut query: Query<(&Self, &Transform)>,
        transform_query: Query<&Transform>,
    ) {
        let entity = trigger.entity();
        let (Self { origin, idx, len }, transform) = query.get_mut(entity).unwrap();
        let origin_xf = transform_query.get(*origin).unwrap();

        // Translation
        let animation = AnimateOnce::translation_and_rotation(
            *transform,
            Transform {
                translation: calculate_card_translation(*origin_xf, *idx, *len),
                rotation: transform.rotation * origin_xf.rotation,
                ..*transform
            },
            CARD_INSERTION_ANIMATION_SECS,
            EaseFunction::QuarticOut,
        );
        commands.trigger_targets(animation, entity);
    }

    fn shift(
        trigger: Trigger<OtherCardInserted>,
        mut commands: Commands,
        mut query: Query<(&Transform, &mut Self)>,
        origin_transform: Query<&Transform, With<CardField>>,
    ) {
        let entity = trigger.entity();
        let (xf, mut card_pos) = query.get_mut(entity).unwrap();
        let origin_xf = origin_transform.get(card_pos.origin).unwrap();

        card_pos.sync_idx_for_insertion(trigger.idx);

        // The card is already inserted to the field, no need to modify its rotation.
        let new_translation = calculate_card_translation(*origin_xf, card_pos.idx, card_pos.len);
        let animation = AnimateOnce::translation(
            xf.translation,
            new_translation,
            CARD_INSERTION_ANIMATION_SECS,
            EaseFunction::QuarticOut,
        );
        commands.trigger_targets(animation, entity);
    }

    fn sync_idx_for_insertion(&mut self, inserted_at: u32) {
        if self.idx >= inserted_at {
            self.idx += 1;
        }
        self.len += 1;
    }
}

#[derive(Debug, Event)]
struct OtherCardInserted {
    idx: u32,
}

fn calculate_card_translation(origin: Transform, idx: u32, len: u32) -> Vec3 {
    let j = idx as i32 - len as i32 / 2;
    let offset = if len % 2 == 0 { 0.5 } else { 0.0 };
    let distance = (j as f32 + offset) * CARD_WIDTH_PLUS_GAP;

    origin.translation + distance * origin.right()
}
