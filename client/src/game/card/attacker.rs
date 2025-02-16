use crate::game::{CARD_HEIGHT, CARD_Z_GAP_RATIO, CTX_STATE};
use bevy::prelude::*;
use client::utils::{animate_once::AnimateTransform, AddObserverExt};

pub fn attacker_plugin(app: &mut App) {
    app.add_state_scoped_observer_named(CTX_STATE, AttackTo::move_card_to_attack_target);
}

#[derive(Event)]
pub struct AttackTo {
    pub target_card: Entity,
}

impl AttackTo {
    fn move_card_to_attack_target(
        trigger: Trigger<Self>,
        mut commands: Commands,
        transforms: Query<&Transform>,
    ) {
        let attacker_entity = trigger.entity();
        let target_entity = trigger.event().target_card;

        // Animate transform
        let target_xf = *transforms.get(target_entity).unwrap();
        let mut xf = *transforms.get(attacker_entity).unwrap();

        xf.translation =
            target_xf.translation + CARD_HEIGHT * (1.0 + CARD_Z_GAP_RATIO) * target_xf.forward();

        commands.trigger_targets(
            AnimateTransform::new(xf, 0.5, EaseFunction::QuarticOut),
            attacker_entity,
        );
    }
}
