use super::{SandboxPlayers, SANDBOX_CTX_STATE};
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
            .add_state_scoped_observer_named(SANDBOX_CTX_STATE, AddAttacker::handle_trigger);
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
        StateScoped(SANDBOX_CTX_STATE),
        MyAttackerField,
        AttackerFieldOwnedBy(players.self_player),
        settings.my_attacker_xf,
        Name::new("MyAttackerField"),
    ));

    commands.spawn((
        StateScoped(SANDBOX_CTX_STATE),
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

        // Animate transform
        let attacker_field_xf = {
            let owner = trigger.event().owner;
            *attacker_fields
                .iter()
                .find(|(_, owned_by)| owned_by.0 == owner)
                .unwrap()
                .0
        };

        commands
            .entity(attacker_entity)
            .insert(Attacker)
            .trigger(AnimateTransform::new(
                attacker_field_xf,
                0.5,
                EaseFunction::QuarticOut,
            ));
    }
}
