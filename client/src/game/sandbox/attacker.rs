use super::{SandboxPlayers, SANDBOX_CTX_STATE};
use crate::game::{CARD_HEIGHT, CARD_Z_GAP_RATIO};
use algo_core::player::PlayerId;
use bevy::prelude::*;
use client::utils::{animate_once::AnimateTransform, AddObserverExt};

#[derive(Default)]
pub struct SandboxAttackerPlugin {
    pub settings: AttackerSettings,
}

#[derive(Default, Clone, Copy, Resource)]
pub struct AttackerSettings {
    pub my_attacker_xf: Transform,
    pub opponent_attacker_xf: Transform,
}

impl Plugin for SandboxAttackerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings)
            .add_systems(OnEnter(SANDBOX_CTX_STATE), setup_attacker_fields)
            .add_state_scoped_observer(SANDBOX_CTX_STATE, AddAttacker::handle_trigger)
            .add_state_scoped_observer(SANDBOX_CTX_STATE, AttackTo::handle_trigger);
    }
}

#[derive(Component)]
struct MyAttackerField;

#[derive(Component)]
struct OpponentAttackerField;

#[derive(Component)]
struct AttackerFieldOwnedBy(PlayerId);

#[derive(Component)]
pub struct Attacker;

fn setup_attacker_fields(
    mut commands: Commands,
    players: Res<SandboxPlayers>,
    settings: Res<AttackerSettings>,
) {
    commands.spawn((
        MyAttackerField,
        AttackerFieldOwnedBy(players.self_player),
        settings.my_attacker_xf,
        Name::new("MyAttackerField"),
    ));

    commands.spawn((
        OpponentAttackerField,
        AttackerFieldOwnedBy(players.opponent_player),
        settings.opponent_attacker_xf,
        Name::new("OpponentAttackerField"),
    ));
}

#[derive(Event)]
pub struct AddAttacker {
    pub owner: PlayerId,
}

impl AddAttacker {
    fn handle_trigger(
        trigger: Trigger<Self>,
        attacker_fields: Query<(&Transform, &AttackerFieldOwnedBy)>,
        mut commands: Commands,
    ) {
        let attacker_entity = trigger.entity();
        commands.entity(attacker_entity).insert(Attacker);

        // Animate transform
        let attacker_field_xf = {
            let owner = trigger.event().owner;
            *attacker_fields
                .iter()
                .find(|(_, owned_by)| owned_by.0 == owner)
                .unwrap()
                .0
        };
        commands.trigger_targets(
            AnimateTransform::new(attacker_field_xf, 0.5, EaseFunction::QuarticOut),
            attacker_entity,
        );
    }
}

#[derive(Event)]
pub struct AttackTo {
    pub target_card: Entity,
}

impl AttackTo {
    fn handle_trigger(
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
