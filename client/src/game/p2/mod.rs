use super::{
    card::{
        attacker::AttackTo,
        guessing::NumSelected,
        instance::{self as card_instance, CardInstance},
    },
    card_field::{CardField, CardPosition, MyCardField},
    GameMode, CARD_DEPTH, HALF_CARD_DEPTH, TALON_TRANSLATION,
};
use crate::{
    game::{
        card::{guessing::SpawnNumSelector, picking::PickableCard},
        card_field::CardFieldOwnedBy,
        CARD_HEIGHT, CARD_Z_GAP_RATIO,
    },
    AppState, JoinedPlayers,
};
use algo_core::{
    card::{CardView, TalonView},
    event::{BoardChange, CardLocation, CardMovement, GameEvent},
    player::PlayerId,
};
use bevy::prelude::*;
use client::{
    client::{InboundEvent, DISCONNECTED_EV_ID},
    log_macros::*,
    utils::{
        animate_once::AnimateTransform,
        log_display::{LogEvent, Message},
        observer_controller::ObserveOnce,
        set_timeout::SetTimeout,
        AddObserverExt,
    },
    EventHandler,
};

mod board_change;
use board_change::ApplyBoardChange;

mod response;
use response::{GameEvHandler, Resp};

mod ui;
use ui::popup::{QuestionAnswered, SpawnPopupMessage, SpawnQuestion};

const P2_CTX_STATE: GameMode = GameMode::TwoPlayers;

const ATTACKER_XF: Transform = Transform::from_xyz(0.0, HALF_CARD_DEPTH, 0.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(GameMode = GameMode::TwoPlayers)]
enum P2State {
    #[default]
    WaitingForGameToStart,
    SetupTalon,
    GameMain,
    _Todo, // TODO
    Disconnected,
}

#[derive(Component)]
struct DisconnectionInfo {
    reason: String,
}

/// Temporary data for initializing talon cards.
#[derive(Deref, DerefMut, Component)]
struct Talon(TalonView);

#[derive(Component, Reflect)]
struct TalonCardIndex(u32);

/// Tracks a card to draw next.
///
/// CardInstance entity that has corresponding `TalonCardIndex` value
/// will be the next card to be drawn.
#[derive(Component, Reflect)]
struct TalonTopCardIndex(u32);

#[derive(Clone, Component)]
struct TurnPlayer(Option<PlayerId>);

#[derive(Component)]
struct MyTurn;

#[derive(Component)]
struct Attacker;

#[derive(Component)]
struct AttackTargetPlayer(Option<PlayerId>);

#[derive(Component)]
struct AttackTargetCard(Option<Entity>);

pub fn p2_plugin(app: &mut App) {
    app.add_plugins((ui::ui_plugin, response::response_plugin))
        .add_sub_state::<P2State>()
        .enable_state_scoped_entities::<P2State>()
        .add_systems(OnEnter(P2_CTX_STATE), setup)
        .add_systems(
            FixedUpdate,
            check_if_disconnected.run_if(in_state(P2_CTX_STATE)),
        )
        .add_plugins(board_change::board_change_plugin)
        .add_systems(OnEnter(P2State::Disconnected), disconnected)
        .add_systems(FixedUpdate, recv_game_event.run_if(in_state(P2_CTX_STATE)))
        .add_systems(OnEnter(P2State::SetupTalon), setup_talon)
        .add_state_scoped_observer_named(P2_CTX_STATE, TurnStarted::turn_started)
        .add_state_scoped_observer_named(
            P2_CTX_STATE,
            AttackTargetSelectionRequired::attack_target_selection_required,
        )
        .add_state_scoped_observer_named(P2_CTX_STATE, AttackTargetSelected::attack_target_selected)
        .add_state_scoped_observer_named(P2_CTX_STATE, NumberGuessRequired::number_guess_required)
        .add_state_scoped_observer_named(P2_CTX_STATE, InformAttackResult::inform_attack_result)
        .add_state_scoped_observer_named(
            P2_CTX_STATE,
            AttackOrStayDecisionRequired::attack_or_stay_decision_required,
        )
        .add_state_scoped_observer_named(P2_CTX_STATE, chosen_attack_or_stay)
        .add_state_scoped_observer_named(P2_CTX_STATE, GameSet::game_set);
}

fn setup(mut commands: Commands, joined_players: ResMut<JoinedPlayers>) {
    debug!("{:?}", *joined_players);

    let card_field_z = CARD_HEIGHT * (1.0 + CARD_Z_GAP_RATIO) * 2.0;

    // My field
    commands.spawn((
        MyCardField,
        StateScoped(P2_CTX_STATE),
        CardFieldOwnedBy(joined_players.my_player.unwrap()),
        Transform::from_xyz(0.0, HALF_CARD_DEPTH, card_field_z),
        Name::new("MyCardField"),
    ));

    // Opponent's field
    commands.spawn((
        StateScoped(P2_CTX_STATE),
        CardFieldOwnedBy(joined_players.opponent_player.unwrap()),
        Transform::from_xyz(0.0, HALF_CARD_DEPTH, -card_field_z).looking_to(Vec3::Z, Vec3::Y),
        Name::new("OpponentCardField"),
    ));

    commands.spawn((
        StateScoped(P2_CTX_STATE),
        TurnPlayer(None),
        Name::new("Tracker.TurnPlayer"),
    ));
    commands.spawn((
        StateScoped(P2_CTX_STATE),
        AttackTargetPlayer(None),
        Name::new("Tracker.AttackTargetPlayer"),
    ));
    commands.spawn((
        StateScoped(P2_CTX_STATE),
        AttackTargetCard(None),
        Name::new("Tracker.AttackTargetCard"),
    ));
}

fn check_if_disconnected(
    mut ev_handler: ResMut<EventHandler>,
    mut state: ResMut<NextState<P2State>>,
    mut commands: Commands,
) {
    let mut reason = String::new();
    let mut disconnected = false;
    if let Some(ev) = ev_handler.storage.take_request(DISCONNECTED_EV_ID) {
        warn!("disconnected from the server: {:?}", ev);
        reason = "Disconnected from the server".into();
        disconnected = true;
    }

    if let Some((_, ev)) = ev_handler
        .storage
        .take_request_if(|v| matches!(v, InboundEvent::PlayerDisconnected(_)))
    {
        let InboundEvent::PlayerDisconnected(pid) = ev else {
            unreachable!();
        };

        warn!("player {:?} disconnected from the server", pid);
        reason = "Opponent disconnected".into();
        disconnected = true;
    }

    if disconnected {
        commands.spawn((StateScoped(P2_CTX_STATE), DisconnectionInfo { reason }));
        state.set(P2State::Disconnected);
    }
}

fn disconnected(mut commands: Commands, mut disconnection_info: Single<&mut DisconnectionInfo>) {
    commands.trigger(SpawnPopupMessage {
        message: std::mem::take(&mut disconnection_info.reason),
        ..default()
    });
    commands.trigger(SetTimeout::new(1.0).with_state(AppState::Home));
}

fn recv_game_event(
    mut commands: Commands,
    mut ev_handler: GameEvHandler,
    mut state: ResMut<NextState<P2State>>,
) {
    let Some(ev) = ev_handler.recv_game_ev() else {
        return;
    };

    let mut delay = 0.0;

    match &ev {
        GameEvent::BoardChanged(board_change) => {
            commands.trigger(ApplyBoardChange(*board_change));
            delay += 0.5;
        }
        GameEvent::GameStarted(talon_view) => {
            commands.spawn((
                StateScoped(P2_CTX_STATE),
                Talon(talon_view.clone()),
                Name::new("Talon"),
            ));

            state.set(P2State::SetupTalon);
        }
        GameEvent::TurnOrderDetermined(_) => (),
        GameEvent::CardDistributed(_) => (),
        GameEvent::TurnStarted(pid) => commands.trigger(TurnStarted(*pid)),
        GameEvent::TurnPlayerDrewCard => (),
        GameEvent::NoCardsLeft => (),
        GameEvent::AttackTargetSelectionRequired { target_player } => {
            commands.trigger(AttackTargetSelectionRequired {
                target_player: *target_player,
            });
        }
        GameEvent::AttackTargetSelected { target_idx } => {
            commands.trigger(AttackTargetSelected {
                target_idx: *target_idx,
            });
            delay += 0.5;
        }
        GameEvent::NumberGuessRequired => {
            commands.trigger(NumberGuessRequired);
        }
        GameEvent::NumberGuessed(num) => {
            let message = format!("Guess: {}", num.0);
            display_info!(commands, "{}", message);
            commands.trigger(SpawnPopupMessage {
                duration_secs: 0.5,
                message,
            });
            delay += 0.5;
        }
        GameEvent::AttackSucceeded => {
            commands.trigger(InformAttackResult { succeeded: true });
            delay += 0.5;
        }
        GameEvent::AttackFailed => {
            commands.trigger(InformAttackResult { succeeded: false });
            delay += 0.5;
        }
        GameEvent::AttackedPlayerLost => {
            commands.trigger(GameSet);
            delay += 0.5;
        }
        GameEvent::GameEnded => (),
        GameEvent::AttackOrStayDecisionRequired => {
            commands.trigger(AttackOrStayDecisionRequired);
        }
        GameEvent::AttackOrStayDecided { attack } => {
            let message = if *attack { "Attack again!" } else { "Stay!" };
            display_info!(commands, "{}", message);
            commands.trigger(SpawnPopupMessage {
                duration_secs: 0.5,
                message: message.into(),
            });

            delay += 0.5;
        }
        GameEvent::TurnEnded => {}
        GameEvent::RespOk => unreachable!(),
    }

    // Respond with `RespOk`
    if !ev.is_decision_required() {
        commands.trigger(SetTimeout::new(delay).with_trigger(Resp::OK));
    }
}

fn setup_talon(talon: Single<(Entity, &Talon)>, mut commands: Commands) {
    let (talon_entity, talon) = *talon;

    // Spawn cards
    for (idx, color) in talon.cards.iter().enumerate() {
        let mut transform = Transform::from_translation(TALON_TRANSLATION);
        transform.translation.y += idx as f32 * CARD_DEPTH;

        commands.spawn((
            StateScoped(P2_CTX_STATE),
            transform,
            CardInstance::new(CardView::from_props(*color, None, false)),
            TalonCardIndex(idx as u32),
        ));
    }

    let top_idx = talon.cards.len() as u32;
    commands.spawn((
        StateScoped(P2_CTX_STATE),
        TalonTopCardIndex(top_idx),
        Name::new("TalonTopCardIndex"),
    ));

    // Cleanup temporary talon info
    commands.entity(talon_entity).despawn();

    commands.set_state(P2State::GameMain);
}

#[derive(Event)]
struct TurnStarted(PlayerId);

impl TurnStarted {
    fn turn_started(
        trigger: Trigger<Self>,
        mut commands: Commands,
        mut query: Single<(Entity, &mut TurnPlayer)>,
        my_player_field: Single<&CardFieldOwnedBy, With<MyCardField>>,
    ) {
        let (storage_entity, ref mut turn_player) = *query;
        let turn_player_id = trigger.0;
        turn_player.0 = Some(turn_player_id);

        let message = if turn_player_id == my_player_field.0 {
            commands.entity(storage_entity).insert(MyTurn);
            "Your turn!"
        } else {
            commands.entity(storage_entity).remove::<MyTurn>();
            "Opponent's turn!"
        };
        display_info!(commands, "{}", message);
        commands.trigger(SpawnPopupMessage {
            message: message.into(),
            ..default()
        });
    }
}

#[derive(Event)]
struct AttackTargetSelectionRequired {
    target_player: PlayerId,
}

impl AttackTargetSelectionRequired {
    fn attack_target_selection_required(
        trigger: Trigger<Self>,
        turn_player: Single<Has<MyTurn>, With<TurnPlayer>>,
        mut attack_target_player: Single<&mut AttackTargetPlayer>,
        fields: Query<(&CardField, &CardFieldOwnedBy)>,
        cards: Query<&CardInstance>,
        mut commands: Commands,
    ) {
        let target_player = trigger.event().target_player;
        attack_target_player.0 = Some(target_player);

        if !*turn_player {
            display_info!(commands, "Opponent is choosing a card to attack...");
            commands.trigger(Resp::OK);
            return;
        }
        display_info!(commands, "Choose a card to attack");

        // Find target player's field
        let (field, _) = fields
            .iter()
            .find(|(_, owned_by)| owned_by.0 == target_player)
            .unwrap();

        // Find valid attack target cards
        for attack_target in field
            .cards()
            .iter()
            .filter(|e| !cards.get(**e).unwrap().get().pub_info.revealed)
        {
            commands.entity(*attack_target).insert(PickableCard);
        }
    }
}

fn on_click_attack_target(
    trigger: Trigger<Pointer<Click>>,
    query: Query<&CardPosition>,
    mut ev_handler: GameEvHandler,
    pickable_cards: Query<Entity, With<PickableCard>>,
    mut commands: Commands,
) {
    let entity = trigger.entity();
    let target_idx = query.get(entity).unwrap().idx();

    ev_handler.send_game_ev(GameEvent::AttackTargetSelected { target_idx });

    for entity in &pickable_cards {
        commands.entity(entity).remove::<PickableCard>();
    }
}

#[derive(Event)]
struct AttackTargetSelected {
    target_idx: u32,
}

impl AttackTargetSelected {
    fn attack_target_selected(
        trigger: Trigger<Self>,
        attack_target_player: Single<&AttackTargetPlayer>,
        fields: Query<(&CardField, &CardFieldOwnedBy)>,
        mut attack_target_card: Single<&mut AttackTargetCard>,
        attacker: Single<Entity, With<Attacker>>,
        mut commands: Commands,
    ) {
        let attack_target_player = attack_target_player.0.unwrap();
        let (field, _) = fields
            .iter()
            .find(|(_, owned_by)| owned_by.0 == attack_target_player)
            .unwrap();

        let entity = field.cards()[trigger.target_idx as usize];
        attack_target_card.0 = Some(entity);

        commands.trigger_targets(
            AttackTo {
                target_card: entity,
            },
            *attacker,
        );
    }
}

#[derive(Event)]
struct NumberGuessRequired;

impl NumberGuessRequired {
    fn number_guess_required(
        _trigger: Trigger<Self>,
        turn_player: Single<Has<MyTurn>, With<TurnPlayer>>,
        mut commands: Commands,
        attack_target_card: Single<&AttackTargetCard>,
    ) {
        if !*turn_player {
            display_info!(commands, "Opponent is guessing a number...");
            commands.trigger(Resp::OK);
            return;
        }
        display_info!(commands, "Guess a card number");

        commands
            .entity(attack_target_card.0.unwrap())
            .trigger(SpawnNumSelector)
            .trigger(ObserveOnce::<NumSelected>::new(Observer::new(
                send_guessed_number,
            )));
    }
}

fn send_guessed_number(trigger: Trigger<NumSelected>, mut ev_handler: GameEvHandler) {
    ev_handler.send_game_ev(GameEvent::NumberGuessed(trigger.0));
}

#[derive(Event)]
struct InformAttackResult {
    succeeded: bool,
}

impl InformAttackResult {
    fn inform_attack_result(trigger: Trigger<Self>, mut commands: Commands) {
        let message = if trigger.succeeded {
            "Attack Succeeded!"
        } else {
            "Attack Failed!"
        }
        .into();
        display_info!(commands, "{}", message);
        commands.trigger(SpawnPopupMessage {
            duration_secs: 0.5,
            message,
        });
    }
}

#[derive(Event)]
struct AttackOrStayDecisionRequired;

impl AttackOrStayDecisionRequired {
    fn attack_or_stay_decision_required(
        _trigger: Trigger<Self>,
        turn_player: Single<Has<MyTurn>, With<TurnPlayer>>,
        mut commands: Commands,
    ) {
        if !*turn_player {
            display_info!(commands, "Opponent is choosing attack again or stay...");
            commands.trigger(Resp::OK);
            return;
        }
        display_info!(commands, "Choose attack again or stay");

        commands.trigger(SpawnQuestion {
            title: "Choose Next Action".into(),
            answers: ["Attack".into(), "Stay".into()],
        });
    }
}

fn chosen_attack_or_stay(trigger: Trigger<QuestionAnswered>, mut ev_handler: GameEvHandler) {
    let attack = match trigger.event().0 {
        0 => {
            // Attack
            true
        }
        1 => {
            // Stay
            false
        }
        _ => unreachable!(),
    };

    ev_handler.send_game_ev(GameEvent::AttackOrStayDecided { attack });
}

#[derive(Event)]
struct GameSet;

impl GameSet {
    fn game_set(
        _trigger: Trigger<Self>,
        turn_player: Single<Has<MyTurn>, With<TurnPlayer>>,
        mut commands: Commands,
    ) {
        let message = if *turn_player {
            "You Win!"
        } else {
            "You Lose!"
        }
        .into();
        display_info!(commands, "{}", message);

        commands.trigger(SpawnPopupMessage {
            duration_secs: 1.0,
            message,
        });
    }
}
