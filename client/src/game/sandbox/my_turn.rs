use crate::game::{card::{guessing::{NumSelected, SpawnNumSelector}, instance as card_instance, picking::PickableCard}, card_field::MyCardField, sandbox::{attacker::AddAttacker, MyCard}};

use super::{attacker::{AttackTo, Attacker}, talon::SandboxTalon, AttackTarget, CardPrivInfos, HiddenCardPrivInfo, InsertCardToField, SandboxPlayers, SandboxState, Selectable};
use algo_core::card::CardPrivInfo;
use bevy::prelude::*;
use client::utils::{observer_controller::{self, ObserveOnce}, set_timeout::SetTimeout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(SandboxState = SandboxState::MyTurn)]
enum MyTurnState {
    #[default]
    Draw,
    Attack,
    AttackSucceeded,
    AttackFailed,
}

pub fn my_turn_plugin(app: &mut App) {
    app.add_systems(OnEnter(MyTurnState::Draw), setup_draw)
        .add_systems(OnEnter(MyTurnState::Attack), on_enter_my_attack)
        .add_systems(OnEnter(MyTurnState::AttackSucceeded), attack_succeeded)
        .add_systems(OnEnter(MyTurnState::AttackFailed), attack_failed);
}

fn setup_draw(mut commands: Commands, mut talon: NonSendMut<Option<SandboxTalon>>) {
    setup_talon_top(
        (*talon).as_mut().unwrap(),
        &mut commands,
        Observer::new(on_click_talon_top),
    );
}

// `Pointer<Click>` is triggered on the child entity of `CardInstance` by `mesh_picking` backend.
// It then bubbles up to the parent entity, triggering this function.
//
// Reference: https://bevyengine.org/news/bevy-0-15/#bubbling-observers
fn on_click_talon_top(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    sandbox_players: Res<SandboxPlayers>,
    mut priv_infos: ResMut<CardPrivInfos>,
    mut talon: NonSendMut<Option<SandboxTalon>>,
    mut my_turn_state: ResMut<NextState<MyTurnState>>,
) {
    let card_entity = (*talon).as_mut().unwrap().draw_card().unwrap();
    debug_assert_eq!(card_entity, trigger.entity());

    // setup attacker
    commands.trigger_targets(
        AddAttacker {
            owner: sandbox_players.self_player,
        },
        card_entity,
    );

    // Add info
    commands.entity(card_entity).insert(MyCard);
    commands.trigger_targets(
        card_instance::AddPrivInfo(priv_infos.pop().unwrap()),
        card_entity,
    );

    my_turn_state.set(MyTurnState::Attack);

    // Cleanup
    commands.entity(card_entity).remove::<PickableCard>();
}

fn on_enter_my_attack(
    mut commands: Commands,
    prev_attack_target: Option<Single<Entity, With<AttackTarget>>>,
    selectable_cards: Query<Entity, With<Selectable>>,
) {
    let targets: Vec<_> = selectable_cards.iter().collect();

    for target in &targets {
        commands.entity(*target).insert(PickableCard);
    }

    if !targets.is_empty() {
        commands.trigger_targets(
            observer_controller::Activate::<Pointer<Click>>::new(),
            targets,
        );
    }

    // Cleanup previous attack target
    if let Some(entity) = prev_attack_target {
        commands.entity(*entity).remove::<AttackTarget>();
    }
}

fn attack_target_selected(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    selectable_cards: Query<(Entity, Option<&Name>), With<Selectable>>,
    attacker: Single<Entity, With<Attacker>>,
) {
    let selected = trigger.entity();

    // Block interaction
    for (entity, name) in &selectable_cards {
        debug!("blocking interaction: ID={}, name={:?}", entity, name);
        commands.trigger_targets(observer_controller::Pause::<Pointer<Click>>::new(), entity);
        commands.entity(entity).remove::<PickableCard>();
    }

    // Spawn NumSelector
    commands.trigger_targets(SpawnNumSelector, selected);
    commands.trigger_targets(
        ObserveOnce::<NumSelected>::new(Observer::new(num_selected)),
        selected,
    );

    // Mark attack target
    commands.entity(selected).insert(AttackTarget);

    // Move Attacker
    commands.trigger_targets(
        AttackTo {
            target_card: selected,
        },
        *attacker,
    );
}

fn num_selected(
    trigger: Trigger<NumSelected>,
    query: Query<&HiddenCardPrivInfo>,
    mut my_turn_state: ResMut<NextState<MyTurnState>>,
) {
    let attacked = trigger.entity();
    let guess = trigger.event().0;
    let hidden_num = query.get(attacked).unwrap().number;

    my_turn_state.set(if guess == hidden_num {
        MyTurnState::AttackSucceeded
    } else {
        MyTurnState::AttackFailed
    });
}

fn attack_succeeded(
    mut commands: Commands,
    attacked: Single<(Entity, &HiddenCardPrivInfo), With<AttackTarget>>,
) {
    let (attacked, hidden_info) = *attacked;

    // Update the attacked card
    commands.trigger_targets(
        card_instance::RevealWith(CardPrivInfo::new(hidden_info.0.number)),
        attacked,
    );
    commands
        .entity(attacked)
        .remove::<(HiddenCardPrivInfo, Selectable)>();
    commands.trigger_targets(
        observer_controller::Remove::<Pointer<Click>>::new(),
        attacked,
    );

    // TODO
    commands.trigger(SetTimeout::new(0.5).with_state(MyTurnState::Attack));
}

fn attack_failed(
    mut commands: Commands,
    attacker: Single<Entity, With<Attacker>>,
    field: Single<Entity, With<MyCardField>>,
) {
    commands.entity(*attacker).remove::<Attacker>();

    // Flip the card
    commands.trigger_targets(card_instance::Reveal, *attacker);

    // Then insert
    commands.trigger(SetTimeout::new(0.5).with_trigger_targets(
        InsertCardToField {
            card_entity: *attacker,
        },
        *field,
    ));

    // Pass a turn to the opponent after the animation
    commands.trigger(SetTimeout::new(1.0).with_state(SandboxState::OpponentTurn));
}
