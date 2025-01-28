use super::{
    card::{
        flip_animation::CardFlipAnimation,
        picking::PickableCard,
        tag::{AddCardTag, RemoveCardTag},
    },
    CardField, CardFieldOwnedBy, CardInstance, GameMode, MyCardField, CARD_DEPTH, CARD_HEIGHT,
    CARD_Z_GAP_RATIO, GAME_SCOPE, TALON_TRANSLATION,
};
use crate::AppState;
use algo_core::player::PlayerId;
use bevy::{input::common_conditions::input_just_pressed, prelude::*};

mod talon;
use talon::SandboxTalon;

mod camera_control;
use camera_control::camera_control_plugin;

const CTX_STATE: GameMode = GameMode::Sandbox;

pub fn game_sandbox_plugin(app: &mut App) {
    app.add_plugins(camera_control_plugin)
        .insert_non_send_resource(SandboxTalon::new(talon::Real))
        .add_systems(
            Update,
            start_sandbox.run_if(in_state(AppState::Home).and(input_just_pressed(KeyCode::Enter))),
        )
        .add_systems(
            OnEnter(CTX_STATE),
            (setup_game_sandbox, setup_game_sandbox_2).chain(),
        );
}

fn start_sandbox(
    mut app_state: ResMut<NextState<AppState>>,
    mut game_mode: ResMut<NextState<GameMode>>,
) {
    app_state.set(AppState::Game);
    game_mode.set(GameMode::Sandbox);
}

fn setup_game_sandbox(mut commands: Commands, mut talon: NonSendMut<SandboxTalon>) {
    // My field
    let self_player = PlayerId::dummy();
    let opponent_player = PlayerId::dummy_2();

    let card_field_z = CARD_HEIGHT * (1.0 + CARD_Z_GAP_RATIO) * 2.0;

    commands.spawn((
        GAME_SCOPE,
        MyCardField,
        CardFieldOwnedBy(self_player),
        Transform::from_xyz(0.0, CARD_DEPTH / 2.0, card_field_z),
    ));

    // Opponent's field
    commands.spawn((
        GAME_SCOPE,
        CardFieldOwnedBy(opponent_player),
        Transform::from_xyz(0.0, CARD_DEPTH / 2.0, -card_field_z).looking_to(Vec3::Z, Vec3::Y),
    ));

    // Initialize the talon
    talon.init(
        &mut commands,
        Transform::from_translation(TALON_TRANSLATION),
    );

    commands.spawn(PlayerIdCycle(Box::new(
        [self_player, opponent_player].into_iter().cycle(),
    )));
}

// This function is separated from `setup_game_sandbox`
// to ensure that updates to `Children` are properly applied
// before calling `setup_talon_top`.
fn setup_game_sandbox_2(
    mut commands: Commands,
    mut talon: NonSendMut<SandboxTalon>,
    children: Query<&Children>,
) {
    setup_talon_top(&mut talon, &mut commands, &children);
}

#[derive(Component, Deref, DerefMut)]
struct PlayerIdCycle(Box<dyn Iterator<Item = PlayerId> + Send + Sync + 'static>);

#[derive(Debug, Clone, Copy, Component)]
struct TalonTop {
    observer: Entity,
}

fn setup_talon_top(talon: &mut SandboxTalon, commands: &mut Commands, children: &Query<&Children>) {
    let Some(talon_top) = talon.peek_card() else {
        return;
    };

    let observer = commands
        .spawn(Observer::new(on_click_talon_top).with_entity(talon_top))
        .id();
    commands.entity(talon_top).insert(TalonTop { observer });

    let children = children.get(talon_top).unwrap();
    commands.entity(children[0]).insert(PickableCard);
}

fn on_click_talon_top(
    trigger: Trigger<Pointer<Click>>,
    talon_top: Single<(&TalonTop, &CardInstance)>,
    mut commands: Commands,
    mut target_player: Single<&mut PlayerIdCycle>,
    mut card_fields: Query<(Entity, &mut CardField, &CardFieldOwnedBy)>,
    mut talon: NonSendMut<SandboxTalon>,
    children: Query<&Children>,
) {
    let card_entity = trigger.entity();

    // DEBUG
    commands.trigger_targets(AddCardTag, card_entity);

    // Update talon state
    talon.draw_card();
    commands.entity(talon_top.0.observer).despawn_recursive();
    commands.entity(card_entity).remove::<TalonTop>();

    // If the card is facing down, add a new observer
    if !talon_top.1.pub_info.revealed {
        // TODO: Block the user from clicking this card while animating.
        let observer = commands
            .spawn(Observer::new(reveal_card).with_entity(card_entity))
            .id();
        commands.entity(card_entity).insert(Revealable { observer });
    }

    // Get target card field
    let target_player = target_player.next().unwrap();
    let Some((field_entity, mut field, _)) = card_fields
        .iter_mut()
        .find(|(_, _, owner)| owner.0 == target_player)
    else {
        warn!("failed to find card field target");
        return;
    };

    // Insert card into the field
    field.insert_card(field_entity, 0, card_entity, &mut commands);

    // Prepare next talon top
    setup_talon_top(&mut talon, &mut commands, &children);
}

#[derive(Debug, Clone, Copy, Component)]
struct Revealable {
    observer: Entity,
}

fn reveal_card(
    trigger: Trigger<Pointer<Click>>,
    mut card: Query<(&mut CardInstance, &Revealable, &Children)>,
    mut commands: Commands,
    animation: Res<CardFlipAnimation>,
    mut animation_player: Query<&mut AnimationPlayer>,
) {
    let entity = trigger.entity();
    let (mut card, revealable, children) = card.get_mut(entity).unwrap();

    // Cleanup observer
    commands.entity(revealable.observer).despawn();
    // The card is no longer Pickable.
    commands.entity(entity).remove::<Revealable>();
    commands.entity(children[0]).remove::<PickableCard>();

    // Update state
    card.pub_info.revealed = true;
    commands.trigger_targets(RemoveCardTag, entity);

    // Animation
    commands
        .entity(children[0])
        .insert(animation.animation_target);
    animation_player
        .get_mut(animation.animation_target.player)
        .unwrap()
        .start(animation.node_idx);
}
