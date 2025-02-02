use super::{
    card::{
        guessing::NumSelected,
        instance::{self as card_instance},
        picking::PickableCard,
    },
    card_field::{CardField, CardFieldOwnedBy, MyCardField},
    GameMode, CARD_DEPTH, CARD_HEIGHT, CARD_Z_GAP_RATIO, TALON_TRANSLATION,
};
use crate::{game::card::guessing::SpawnNumSelector, AppState};
use algo_core::{card::CardPrivInfo, player::PlayerId};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use client::utils::observer_controller::{
    self, ObserveOnce, ObserverControllerPlugin, ObserverControllerSettings,
};

mod talon;
use talon::{SandboxTalon, SpawnCards as _};

mod camera_control;
use camera_control::SandboxCameraControlPlugin;

pub fn game_sandbox_plugin(app: &mut App) {
    app.add_plugins((
        SandboxCameraControlPlugin {
            ctx_state: GameMode::Sandbox,
        },
        ObserverControllerPlugin::<NumSelected>::new(ObserverControllerSettings::once())
            .state_scoped(GameMode::Sandbox),
    ))
    .insert_non_send_resource(Option::<SandboxTalon>::None)
    .add_systems(
        Update,
        start_sandbox.run_if(in_state(AppState::Home).and(input_just_pressed(KeyCode::Enter))),
    )
    .add_systems(
        OnEnter(GameMode::Sandbox),
        (init_sandbox_resources, setup_sandbox, setup_sandbox_2).chain(),
    );
}

fn start_sandbox(
    mut app_state: ResMut<NextState<AppState>>,
    mut game_mode: ResMut<NextState<GameMode>>,
) {
    app_state.set(AppState::Game);
    game_mode.set(GameMode::Sandbox);
}

#[derive(Deref, DerefMut, Resource)]
struct CardPrivInfos(Vec<CardPrivInfo>);

fn init_sandbox_resources(mut commands: Commands, mut talon: NonSendMut<Option<SandboxTalon>>) {
    let mut cards = talon::Real.produce_cards();

    // DEBUG
    // let mut cards = ["White-(100)", "White-(1)", "Black-(100)", "Black-(0)"]
    //     .into_iter()
    //     .map(|v| v.parse().unwrap())
    //     .collect::<Vec<_>>()
    //     .produce_cards();

    let priv_infos = cards
        .iter_mut()
        .map(|v| v.priv_info.take().unwrap())
        .rev()
        .collect();

    *talon = Some(SandboxTalon::new(cards));

    commands.insert_resource(CardPrivInfos(priv_infos));
}

fn setup_sandbox(mut commands: Commands, mut talon: NonSendMut<Option<SandboxTalon>>) {
    let (self_player, opponent_player) = PlayerId::dummy_pair();
    let card_field_z = CARD_HEIGHT * (1.0 + CARD_Z_GAP_RATIO) * 2.0;

    // My field
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
    (*talon).as_mut().unwrap().init(
        &mut commands,
        Transform::from_translation(TALON_TRANSLATION),
    );

    commands.spawn((
        StateScoped(GameMode::Sandbox),
        PlayerIdCycle(Box::new([self_player, opponent_player].into_iter().cycle())),
    ));
}

// This function is separated from `setup_game_sandbox`
// to ensure that updates to `Children` are properly applied
// before calling `setup_talon_top`.
fn setup_sandbox_2(
    mut commands: Commands,
    mut talon: NonSendMut<Option<SandboxTalon>>,
    children: Query<&Children>,
) {
    setup_talon_top((*talon).as_mut().unwrap(), &mut commands, &children);
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

#[allow(clippy::too_many_arguments)]
fn on_click_talon_top(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut target_player: Single<&mut PlayerIdCycle>,
    mut card_fields: Query<(
        Entity,
        &mut CardField,
        &CardFieldOwnedBy,
        Option<&MyCardField>,
    )>,
    mut talon: NonSendMut<Option<SandboxTalon>>,
    mut priv_infos: ResMut<CardPrivInfos>,
    children: Query<&Children>,
) {
    let card_entity = trigger.entity();

    // Insert card into the field
    let target_player = target_player.next().unwrap();
    let (field_entity, mut field, _, my_card_field) = card_fields
        .iter_mut()
        .find(|(_, _, owner, _)| owner.0 == target_player)
        .unwrap();
    field.insert_card(field_entity, 0, card_entity, &mut commands);

    match my_card_field {
        Some(_) => {
            commands.entity(card_entity).insert(MyCard);
            commands.trigger_targets(
                card_instance::AddPrivInfo(priv_infos.pop().unwrap()),
                card_entity,
            );
            // TODO: Block the user from clicking this card while animating.
            commands.trigger_targets(
                ObserveOnce::<Pointer<Click>>::new(Observer::new(reveal_card)),
                card_entity,
            );
        }
        None => {
            commands
                .entity(card_entity)
                .insert((OpponentCard, Selectable));
            commands.trigger_targets(
                observer_controller::Insert::<Pointer<Click>>::new_active(|| {
                    Observer::new(select_card_number)
                }),
                card_entity,
            );
            // Discard info
            priv_infos.pop();
        }
    }

    // Update talon state
    let talon = (*talon).as_mut().unwrap();
    talon.draw_card();
    setup_talon_top(talon, &mut commands, &children);
}

#[derive(Component)]
struct MyCard;

#[derive(Component)]
struct OpponentCard;

#[derive(Component)]
struct Selectable;

fn reveal_card(
    trigger: Trigger<Pointer<Click>>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    let entity = trigger.entity();

    // Remove picking interaction
    commands
        .entity(children.get(entity).unwrap()[0])
        .remove::<PickableCard>();

    commands.trigger_targets(card_instance::Reveal, entity);
}

fn select_card_number(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    selectable_cards: Query<(Entity, Option<&Name>), With<Selectable>>,
    children: Query<&Children>,
) {
    let selected = trigger.entity();

    // Block interaction with other cards
    for (entity, name) in selectable_cards.iter().filter(|e| e.0 != selected) {
        debug!("blocking interaction: ID={}, name={:?}", entity, name);
        commands.trigger_targets(observer_controller::Pause::<Pointer<Click>>::new(), entity);
    }

    // Remove interaction
    commands.trigger_targets(
        observer_controller::Remove::<Pointer<Click>>::new(),
        selected,
    );
    commands.entity(selected).remove::<Selectable>();
    commands
        .entity(children.get(selected).unwrap()[0])
        .remove::<PickableCard>();

    // Spawn NumSelector
    commands.trigger_targets(SpawnNumSelector, selected);
    commands.trigger_targets(
        ObserveOnce::<NumSelected>::new(Observer::new(num_selected)),
        selected,
    );
}

fn num_selected(
    trigger: Trigger<NumSelected>,
    mut commands: Commands,
    selectable_cards: Query<Entity, With<Selectable>>,
) {
    let entity = trigger.entity();
    let num = trigger.event().0;

    // Activate interaction with other cards
    for entity in &selectable_cards {
        commands.trigger_targets(
            observer_controller::Activate::<Pointer<Click>>::new(),
            entity,
        );
    }

    commands.trigger_targets(card_instance::RevealWith(CardPrivInfo::new(num)), entity);
}
