use super::{
    card::{flip_animation::CardFlipAnimation, instance::CardInstance, picking::PickableCard},
    card_field::{CardField, CardFieldOwnedBy, MyCardField},
    GameMode, CARD_DEPTH, CARD_HEIGHT, CARD_Z_GAP_RATIO, TALON_TRANSLATION,
};
use crate::AppState;
use algo_core::player::PlayerId;
use bevy::{input::common_conditions::input_just_pressed, prelude::*};

mod talon;
use client::utils::observe_once::{ObserveOnce, ObserveOncePlugin};
use talon::SandboxTalon;

mod camera_control;
use camera_control::SandboxCameraControlPlugin;

pub fn game_sandbox_plugin(app: &mut App) {
    let card_spawner = talon::Map::new(talon::Real, |(i, mut card)| {
        if i % 2 == 1 {
            card.priv_info = None;
        }
        card
    });

    app.add_plugins((
        SandboxCameraControlPlugin {
            ctx_state: GameMode::Sandbox,
        },
        ObserveOncePlugin::<Pointer<Click>>::new(),
    ))
    .insert_non_send_resource(SandboxTalon::new(card_spawner))
    .add_systems(
        Update,
        start_sandbox.run_if(in_state(AppState::Home).and(input_just_pressed(KeyCode::Enter))),
    )
    .add_systems(
        OnEnter(GameMode::Sandbox),
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
        StateScoped(AppState::Game),
        MyCardField,
        CardFieldOwnedBy(self_player),
        Transform::from_xyz(0.0, CARD_DEPTH / 2.0, card_field_z),
    ));

    // Opponent's field
    commands.spawn((
        StateScoped(AppState::Game),
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

fn setup_talon_top(talon: &mut SandboxTalon, commands: &mut Commands, children: &Query<&Children>) {
    let Some(talon_top) = talon.peek_card() else {
        return;
    };

    commands.trigger_targets(
        ObserveOnce::<Pointer<Click>>::new(Observer::new(on_click_talon_top)),
        talon_top,
    );

    let children = children.get(talon_top).unwrap();
    commands.entity(children[0]).insert(PickableCard);
}

fn on_click_talon_top(
    trigger: Trigger<Pointer<Click>>,
    cards: Query<&CardInstance>,
    mut commands: Commands,
    mut target_player: Single<&mut PlayerIdCycle>,
    mut card_fields: Query<(Entity, &mut CardField, &CardFieldOwnedBy)>,
    mut talon: NonSendMut<SandboxTalon>,
    children: Query<&Children>,
) {
    let card_entity = trigger.entity();
    let card = cards.get(card_entity).unwrap();

    // Update talon state
    talon.draw_card();

    // If the card is facing down, add a new observer
    if !card.pub_info.revealed {
        // TODO: Block the user from clicking this card while animating.
        commands.trigger_targets(
            ObserveOnce::<Pointer<Click>>::new(Observer::new(reveal_card)),
            card_entity,
        );
    }

    // Insert card into the field
    let target_player = target_player.next().unwrap();
    let (field_entity, mut field, _) = card_fields
        .iter_mut()
        .find(|(_, _, owner)| owner.0 == target_player)
        .unwrap();
    field.insert_card(field_entity, 0, card_entity, &mut commands);

    // Prepare next talon top
    setup_talon_top(&mut talon, &mut commands, &children);
}

fn reveal_card(
    trigger: Trigger<Pointer<Click>>,
    mut cards: Query<(&mut CardInstance, &Children)>,
    mut commands: Commands,
    animation: Res<CardFlipAnimation>,
    mut animation_player: Query<&mut AnimationPlayer>,
) {
    let entity = trigger.entity();
    let (mut card, children) = cards.get_mut(entity).unwrap();

    // Update card state
    card.pub_info.revealed = true;
    commands.entity(children[0]).remove::<PickableCard>();

    // Animation
    commands
        .entity(children[0])
        .insert(animation.animation_target);
    animation_player
        .get_mut(animation.animation_target.player)
        .unwrap()
        .start(animation.node_idx);
}
