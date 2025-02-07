use super::{
    card::{
        guessing::NumSelected,
        instance::{self as card_instance, CardInstance},
        picking::PickableCard,
    },
    card_field::{CardField, CardFieldOwnedBy, MyCardField},
    dialog::{Dialog, DialogButton, PopupMessageExt as _},
    GameMode, CARD_DEPTH, CARD_HEIGHT, CARD_Z_GAP_RATIO, TALON_TRANSLATION,
};
use crate::{game::card::guessing::SpawnNumSelector, AppState};
use algo_core::{
    card::{CardColor, CardNumber, CardPrivInfo, CardPubInfo},
    player::PlayerId,
};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use client::utils::{
    observer_controller::{
        self, ObserveOnce, ObserverControllerPlugin, ObserverControllerSettings,
    },
    set_timeout::SetTimeout,
    AddObserverExt as _,
};
use itertools::Itertools as _;
use rand::{rngs::ThreadRng, seq::IndexedRandom, Rng as _};
use std::collections::{BTreeMap, BTreeSet};

mod talon;
use talon::{SandboxTalon, SpawnCards as _};

mod attacker;
use attacker::{AddAttacker, AttackTo, Attacker, AttackerSettings, SandboxAttackerPlugin};

mod camera_control;
use camera_control::SandboxCameraControlPlugin;

const SANDBOX_CTX_STATE: GameMode = GameMode::Sandbox;

const HALF_CARD_DEPTH: f32 = CARD_DEPTH / 2.0;

const INITIAL_DRAW_NUM_PER_PLAYER: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(GameMode = GameMode::Sandbox)]
enum SandboxState {
    #[default]
    DistributeCards,
    MyTurn,
    OpponentTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(SandboxState = SandboxState::MyTurn)]
enum MyTurnState {
    #[default]
    Draw,
    Attack,
    AttackSucceeded,
    AttackFailed,
    CheckWinCondition,
    ChooseAttackOrStay,
    Stay,
    Win,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(SandboxState = SandboxState::OpponentTurn)]
enum OpponentTurnState {
    #[default]
    Draw,
    Attack,
    AttackSucceeded,
    AttackFailed,
    CheckWinCondition,
    ChooseAttackOrStay,
    Stay,
    Win,
}

pub fn game_sandbox_plugin(app: &mut App) {
    app.add_plugins((
        ObserverControllerPlugin::<NumSelected>::new(ObserverControllerSettings::once())
            .state_scoped(GameMode::Sandbox),
        SandboxCameraControlPlugin,
        SandboxAttackerPlugin {
            settings: AttackerSettings {
                my_attacker_xf: Transform::from_xyz(0.0, HALF_CARD_DEPTH, 0.0),
                opponent_attacker_xf: Transform::from_xyz(0.0, HALF_CARD_DEPTH, 0.0),
            },
        },
    ))
    // .add_systems(
    //     Update,
    //     start_sandbox.run_if(in_state(AppState::Home).and(input_just_pressed(KeyCode::Enter))),
    // )
    .add_sub_state::<SandboxState>()
    .enable_state_scoped_entities::<SandboxState>()
    .add_sub_state::<MyTurnState>()
    .add_sub_state::<OpponentTurnState>()
    .init_resource::<SandboxPlayers>()
    .insert_non_send_resource(Option::<SandboxTalon>::None)
    .insert_non_send_resource(OpponentSimulator::new())
    .add_systems(
        OnEnter(GameMode::Sandbox),
        (init_sandbox_resources, setup_sandbox).chain(),
    )
    .add_state_scoped_observer(GameMode::Sandbox, InsertCardToField::handle_trigger)
    .add_state_scoped_observer(
        SandboxState::DistributeCards,
        DistributeCard::handle_trigger,
    )
    .add_systems(
        OnEnter(SandboxState::DistributeCards),
        setup_distribute_cards,
    )
    .add_systems(OnEnter(MyTurnState::Draw), setup_draw)
    .add_systems(OnEnter(MyTurnState::Attack), on_enter_my_attack)
    .add_systems(OnEnter(MyTurnState::AttackSucceeded), attack_succeeded)
    .add_systems(OnEnter(MyTurnState::AttackFailed), attack_failed)
    .add_systems(OnEnter(MyTurnState::CheckWinCondition), check_win_condition)
    .add_systems(
        OnEnter(MyTurnState::ChooseAttackOrStay),
        choose_attack_or_stay,
    )
    .add_systems(OnEnter(MyTurnState::Stay), on_enter_stay)
    .add_systems(OnEnter(MyTurnState::Win), on_enter_win)
    .add_systems(OnEnter(OpponentTurnState::Draw), opponent_draw)
    .add_systems(
        OnEnter(OpponentTurnState::Attack),
        OpponentSimulator::attack,
    )
    .add_systems(
        OnEnter(OpponentTurnState::AttackSucceeded),
        opponent_attack_succeeded,
    )
    .add_systems(
        OnEnter(OpponentTurnState::AttackFailed),
        opponent_attack_failed,
    )
    .add_systems(
        OnEnter(OpponentTurnState::CheckWinCondition),
        opponent_check_win_condition,
    )
    .add_systems(
        OnEnter(OpponentTurnState::ChooseAttackOrStay),
        OpponentSimulator::choose_attack_or_stay,
    )
    .add_systems(OnEnter(OpponentTurnState::Stay), opponent_stay)
    .add_systems(OnEnter(OpponentTurnState::Win), opponent_win)
    // DEBUG
    .add_systems(
        Update,
        (|query: Query<(&CardField, &SortFieldCards, Has<MyCardField>)>| {
            for (field, sorter, my_card_field) in &query {
                info!("my_card_field={}", my_card_field);
                info!("E: {:?}", field);
                info!("C: {:?}", sorter.cards);
            }
        })
        .run_if(in_state(GameMode::Sandbox).and(input_just_pressed(KeyCode::KeyG))),
    );
}

fn start_sandbox(
    mut app_state: ResMut<NextState<AppState>>,
    mut game_mode: ResMut<NextState<GameMode>>,
) {
    app_state.set(AppState::Game);
    game_mode.set(GameMode::Sandbox);
}

#[derive(Resource)]
struct SandboxPlayers {
    self_player: PlayerId,
    opponent_player: PlayerId,
}

impl SandboxPlayers {
    fn iter(&self) -> impl Iterator<Item = PlayerId> + Clone {
        [self.self_player, self.opponent_player].into_iter()
    }
}

impl Default for SandboxPlayers {
    fn default() -> Self {
        let (self_player, opponent_player) = PlayerId::dummy_pair();
        Self {
            self_player,
            opponent_player,
        }
    }
}

#[derive(Deref, DerefMut, Resource)]
struct CardPrivInfos(Vec<CardPrivInfo>);

#[derive(Deref, DerefMut, Component)]
struct HiddenCardPrivInfo(CardPrivInfo);

#[derive(Default, Component)]
struct SortFieldCards {
    cards: Vec<(CardNumber, CardColor)>,
}

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

fn setup_sandbox(
    mut commands: Commands,
    players: Res<SandboxPlayers>,
    mut talon: NonSendMut<Option<SandboxTalon>>,
) {
    let self_player = players.self_player;
    let opponent_player = players.opponent_player;
    let card_field_z = CARD_HEIGHT * (1.0 + CARD_Z_GAP_RATIO) * 2.0;

    // My field
    commands.spawn((
        StateScoped(AppState::Game),
        MyCardField,
        CardFieldOwnedBy(self_player),
        Transform::from_xyz(0.0, HALF_CARD_DEPTH, card_field_z),
        SortFieldCards::default(),
        Name::new("MyCardField"),
    ));

    // Opponent's field
    commands.spawn((
        StateScoped(AppState::Game),
        CardFieldOwnedBy(opponent_player),
        Transform::from_xyz(0.0, HALF_CARD_DEPTH, -card_field_z).looking_to(Vec3::Z, Vec3::Y),
        SortFieldCards::default(),
        Name::new("OpponentCardField"),
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

#[derive(Component, Deref, DerefMut)]
struct PlayerIdCycle(Box<dyn Iterator<Item = PlayerId> + Send + Sync + 'static>);

#[derive(Component)]
struct MyCard;

#[derive(Component)]
struct OpponentCard;

#[derive(Component)]
struct Selectable;

#[derive(Component)]
struct AttackTarget;

fn setup_distribute_cards(mut commands: Commands, players: Res<SandboxPlayers>) {
    let mut t = 0.0;
    let interval = 0.5;
    for (i, (_, player)) in (0..INITIAL_DRAW_NUM_PER_PLAYER)
        .cartesian_product(players.iter())
        .enumerate()
    {
        t = (i + 1) as f32 * interval;
        commands.trigger(SetTimeout::new(t).with_trigger(DistributeCard { player }));
    }

    commands.trigger(SetTimeout::new(t + interval).with_state(SandboxState::MyTurn));
}

#[derive(Clone, Event)]
struct DistributeCard {
    player: PlayerId,
}

impl DistributeCard {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        mut talon: NonSendMut<Option<SandboxTalon>>,
        mut card_fields: Query<(Entity, &CardFieldOwnedBy, Has<MyCardField>)>,
        mut priv_infos: ResMut<CardPrivInfos>,
    ) {
        let card_entity = (*talon).as_mut().unwrap().draw_card().unwrap();
        let target_player = trigger.event().player;

        // Find target field
        let (field_entity, _, my_card_field) = card_fields
            .iter_mut()
            .find(|(_, owner, _)| owner.0 == target_player)
            .unwrap();

        // Update card entity
        let priv_info = priv_infos.pop().unwrap();
        match my_card_field {
            true => {
                commands
                    .entity(card_entity)
                    .insert(MyCard)
                    .trigger(card_instance::AddPrivInfo(priv_info));
            }
            false => {
                commands
                    .entity(card_entity)
                    .insert((OpponentCard, HiddenCardPrivInfo(priv_info), Selectable))
                    .trigger(observer_controller::Insert::<Pointer<Click>>::new_paused(
                        || Observer::new(attack_target_selected),
                    ));
            }
        }

        // Insert card
        commands.trigger_targets(InsertCardToField { card_entity }, field_entity);
    }
}

#[derive(Event)]
struct InsertCardToField {
    card_entity: Entity,
}

impl InsertCardToField {
    fn handle_trigger(
        trigger: Trigger<Self>,
        cards: Query<(&CardInstance, Option<&HiddenCardPrivInfo>)>,
        mut fields: Query<(&mut CardField, &mut SortFieldCards)>,
        mut commands: Commands,
    ) {
        let card_entity = trigger.event().card_entity;
        let field_entity = trigger.entity();

        // Get card info
        let card_info = {
            let (card, hidden) = cards.get(card_entity).unwrap();
            let card = card.get();
            (
                match (card.priv_info, hidden) {
                    (Some(v), _) => v.number,
                    (None, Some(v)) => v.0.number,
                    _ => {
                        warn!("could not get CardPrivInfo");
                        return;
                    }
                },
                card.pub_info.color,
            )
        };

        // Find a correct spot for the card
        let (mut field, mut sorter) = fields.get_mut(field_entity).unwrap();
        let Err(idx) = sorter.cards.binary_search(&card_info) else {
            warn!("Card duplicate found: {:?}", card_info);
            return;
        };

        // Insert the card
        sorter.cards.insert(idx, card_info);
        field.insert_card(field_entity, idx as u32, card_entity, &mut commands);
    }
}

fn setup_draw(mut commands: Commands, mut talon: NonSendMut<Option<SandboxTalon>>) {
    let Some(talon_top) = (*talon).as_mut().unwrap().peek_card() else {
        return;
    };

    commands
        .entity(talon_top)
        .insert(PickableCard)
        .trigger(ObserveOnce::<Pointer<Click>>::new(Observer::new(
            on_click_talon_top,
        )));
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

    // Update drawn card
    commands
        .entity(card_entity)
        .insert(MyCard)
        .trigger(card_instance::AddPrivInfo(priv_infos.pop().unwrap()))
        .trigger(AddAttacker {
            owner: sandbox_players.self_player,
        })
        .remove::<PickableCard>();

    my_turn_state.set(MyTurnState::Attack);
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
    selectable_cards: Query<Entity, With<Selectable>>,
    attacker: Single<Entity, With<Attacker>>,
) {
    let selected = trigger.entity();

    // Block interaction
    for entity in &selectable_cards {
        commands
            .entity(entity)
            .remove::<PickableCard>()
            .trigger(observer_controller::Pause::<Pointer<Click>>::new());
    }

    // Spawn NumSelector for the selected target
    commands
        .entity(selected)
        .insert(AttackTarget)
        .trigger(SpawnNumSelector)
        .trigger(ObserveOnce::<NumSelected>::new(Observer::new(num_selected)));

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
    commands
        .entity(attacked)
        .remove::<(HiddenCardPrivInfo, Selectable)>()
        .trigger(card_instance::RevealWith(CardPrivInfo::new(
            hidden_info.0.number,
        )))
        .trigger(observer_controller::Remove::<Pointer<Click>>::new());

    commands.trigger(SetTimeout::new(0.5).with_state(MyTurnState::CheckWinCondition));
}

fn attack_failed(
    mut commands: Commands,
    attacker: Single<Entity, With<Attacker>>,
    field: Single<Entity, With<MyCardField>>,
) {
    // Flip the card
    commands
        .entity(*attacker)
        .remove::<Attacker>()
        .trigger(card_instance::Reveal);

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

fn check_win_condition(
    field: Single<&CardField, Without<MyCardField>>,
    cards: Query<&CardInstance>,
    mut my_turn_state: ResMut<NextState<MyTurnState>>,
) {
    let all_revealed = field
        .cards()
        .iter()
        .all(|entity| cards.get(*entity).unwrap().get().pub_info.revealed);

    my_turn_state.set(if all_revealed {
        MyTurnState::Win
    } else {
        MyTurnState::ChooseAttackOrStay
    });
}

fn choose_attack_or_stay(mut commands: Commands) {
    commands.spawn((
        Dialog::new(
            None,
            [
                DialogButton::new(
                    "Attack",
                    |commands| commands.set_state(MyTurnState::Attack),
                    default(),
                ),
                DialogButton::new(
                    "Stay",
                    |commands| commands.set_state(MyTurnState::Stay),
                    default(),
                ),
            ],
        ),
        Transform::from_xyz(0.0, -80.0, 0.0),
    ));
}

fn on_enter_stay(
    mut commands: Commands,
    attacker: Single<Entity, With<Attacker>>,
    field: Single<Entity, With<MyCardField>>,
) {
    commands.entity(*attacker).remove::<Attacker>();

    // Insert the card into the field without flipping it
    commands.trigger(SetTimeout::new(0.5).with_trigger_targets(
        InsertCardToField {
            card_entity: *attacker,
        },
        *field,
    ));

    // Pass a turn to the opponent after the animation
    commands.trigger(SetTimeout::new(0.5).with_state(SandboxState::OpponentTurn));
}

fn on_enter_win(mut commands: Commands) {
    let duration_secs = 1.0;

    commands
        .spawn((
            StateScoped(SANDBOX_CTX_STATE),
            Transform::from_xyz(0.0, -80.0, 0.0),
        ))
        .insert_popup_message("You Win!", duration_secs);

    commands.trigger(SetTimeout::new(duration_secs).with_state(AppState::Home));
}

fn opponent_draw(
    mut commands: Commands,
    mut talon: NonSendMut<Option<SandboxTalon>>,
    sandbox_players: Res<SandboxPlayers>,
    mut priv_infos: ResMut<CardPrivInfos>,
    mut simulator: NonSendMut<OpponentSimulator>,
) {
    let card_entity = (*talon).as_mut().unwrap().draw_card().unwrap();

    commands
        .entity(card_entity)
        .insert((OpponentCard, HiddenCardPrivInfo(priv_infos.pop().unwrap())))
        .trigger(AddAttacker {
            owner: sandbox_players.opponent_player,
        });

    simulator.attacker = Some(card_entity);

    commands.trigger(SetTimeout::new(0.5).with_state(OpponentTurnState::Attack));
}

fn opponent_attack_succeeded(mut simulator: NonSendMut<OpponentSimulator>, mut commands: Commands) {
    commands.trigger_targets(
        card_instance::Reveal,
        simulator.attack_target.take().unwrap(),
    );

    commands.trigger(SetTimeout::new(0.5).with_state(OpponentTurnState::CheckWinCondition));
}

fn opponent_attack_failed(
    mut simulator: NonSendMut<OpponentSimulator>,
    hidden_infos: Query<(&HiddenCardPrivInfo,)>,
    mut commands: Commands,
    field: Single<Entity, (With<CardField>, Without<MyCardField>)>,
) {
    let attacker_entity = simulator.attacker.take().unwrap();
    let number = hidden_infos.get(attacker_entity).unwrap().0.number;

    commands
        .entity(attacker_entity)
        .trigger(card_instance::RevealWith(CardPrivInfo::new(number)))
        .remove::<(Attacker, HiddenCardPrivInfo, Selectable)>();

    commands.trigger_targets(
        InsertCardToField {
            card_entity: attacker_entity,
        },
        *field,
    );

    commands.trigger(SetTimeout::new(0.5).with_state(SandboxState::MyTurn));
}

// TODO: DRY
fn opponent_check_win_condition(
    field: Single<&CardField, With<MyCardField>>,
    cards: Query<&CardInstance>,
    mut opponent_turn_state: ResMut<NextState<OpponentTurnState>>,
) {
    let all_revealed = field
        .cards()
        .iter()
        .all(|entity| cards.get(*entity).unwrap().get().pub_info.revealed);

    opponent_turn_state.set(if all_revealed {
        OpponentTurnState::Win
    } else {
        OpponentTurnState::ChooseAttackOrStay
    });
}

fn opponent_stay(
    mut simulator: NonSendMut<OpponentSimulator>,
    mut commands: Commands,
    field: Single<Entity, (With<CardField>, Without<MyCardField>)>,
) {
    let attacker_entity = simulator.attacker.take().unwrap();

    commands
        .entity(attacker_entity)
        .insert(Selectable)
        .remove::<Attacker>()
        .trigger(observer_controller::Insert::<Pointer<Click>>::new_paused(
            || Observer::new(attack_target_selected),
        ));

    commands.trigger_targets(
        InsertCardToField {
            card_entity: attacker_entity,
        },
        *field,
    );

    commands.trigger(SetTimeout::new(0.5).with_state(SandboxState::MyTurn));
}

// TODO: DRY
fn opponent_win(mut commands: Commands) {
    let duration_secs = 1.0;

    commands
        .spawn((
            StateScoped(SANDBOX_CTX_STATE),
            Transform::from_xyz(0.0, -80.0, 0.0),
        ))
        .insert_popup_message("You Lose!", duration_secs);

    commands.trigger(SetTimeout::new(duration_secs).with_state(AppState::Home));
}

/// This simulator guesses numbers using only the information  
/// from the cards visible to the simulated player.  
///
/// If the guess is correct, there is a 50% chance that the simulator  
/// will attack again.  
///
/// More advanced simulators may also utilize the following information  
/// for number predictions:  
/// - The history of actions taken by both players (e.g., guessed numbers)
struct OpponentSimulator {
    rng: ThreadRng,
    attacker: Option<Entity>,
    attack_target: Option<Entity>,
}

impl OpponentSimulator {
    fn new() -> Self {
        Self {
            rng: rand::rng(),
            attacker: None,
            attack_target: None,
        }
    }

    fn attack(
        mut this: NonSendMut<OpponentSimulator>,
        mut commands: Commands,
        cards: Query<(Entity, &CardInstance, Option<&HiddenCardPrivInfo>)>,
    ) {
        let mut attack_targets = Vec::new();
        let mut numbers = BTreeMap::from([
            (CardColor::Black, BTreeSet::from_iter(0..12)),
            (CardColor::White, BTreeSet::from_iter(0..12)),
        ]);

        for (entity, card, hidden_info) in &cards {
            let card = card.get();
            let CardPubInfo { color, revealed } = card.pub_info;

            if let Some(hidden_info) = hidden_info {
                numbers
                    .get_mut(&color)
                    .unwrap()
                    .remove(&hidden_info.number.0);
            } else if revealed {
                numbers
                    .get_mut(&color)
                    .unwrap()
                    .remove(&card.priv_info.unwrap().number.0);
            } else if card.priv_info.is_some() {
                attack_targets.push((entity, *card));
            }
        }

        // Choose attack target;
        let (attack_target_entity, target_card) = *attack_targets.choose(&mut this.rng).unwrap();
        commands.trigger_targets(
            AttackTo {
                target_card: attack_target_entity,
            },
            this.attacker.unwrap(),
        );

        // Choose number
        let guess = **numbers
            .remove(&target_card.pub_info.color)
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .choose(&mut this.rng)
            .unwrap();

        // Announce opponent's guess
        let msg = format!("Guess: {}", guess);
        commands.trigger(SetTimeout::new(0.5).with_fn(|commands| {
            commands
                .spawn((
                    StateScoped(SANDBOX_CTX_STATE),
                    Transform::from_xyz(0.0, -80.0, 0.0),
                ))
                .insert_popup_message(msg, 1.0);
        }));

        // Compare attack result
        let next_state = if target_card.priv_info.unwrap().number == guess {
            this.attack_target = Some(attack_target_entity);
            OpponentTurnState::AttackSucceeded
        } else {
            OpponentTurnState::AttackFailed
        };
        commands.trigger(SetTimeout::new(1.0).with_state(next_state));
    }

    fn choose_attack_or_stay(mut this: NonSendMut<OpponentSimulator>, mut commands: Commands) {
        let (next_state, msg) = if this.rng.random() {
            (OpponentTurnState::Attack, "Attack Again")
        } else {
            (OpponentTurnState::Stay, "Stay")
        };

        commands
            .spawn((
                StateScoped(SANDBOX_CTX_STATE),
                Transform::from_xyz(0.0, -80.0, 0.0),
            ))
            .insert_popup_message(msg, 1.0);

        commands.trigger(SetTimeout::new(0.5).with_state(next_state));
    }
}
