use crate::{AppArgs, AppState};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_dev_tools::states::log_transitions;
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputSettings, TextInputTextColor,
    TextInputTextFont, TextInputValue,
};
use client::{
    button::{button_system, spawn_common_button, ButtonPressed},
    client::{
        client_connection_plugin, spawn_client, CancelSpawnClientEvent, InboundEvent,
        OutboundEvent, ReceivedRequest, ReceivedResponse, SpawnClientResult,
    },
    log_display::{LogDisplay, LogDisplaySettings, LogEvent, Message},
    util::{AddStateScopedObserver as _, IntoColor as _},
};
use protocol::server_to_client::JoinInfo;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(AppState = AppState::Home)]
enum HomeState {
    #[default]
    Menu,
    JoiningServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(HomeState = HomeState::JoiningServer)]
enum JoiningServerState {
    #[default]
    Setup,
    Connecting,
    Cancelling,
    Failed,
    Joining,
    WaitingForOtherPlayers,
    WaitingForGameToStart,
}

impl JoiningServerState {
    fn connected(&self) -> bool {
        matches!(
            self,
            Self::Joining | Self::WaitingForOtherPlayers | Self::WaitingForGameToStart
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConnectedToServer;

impl ComputedStates for ConnectedToServer {
    type SourceStates = JoiningServerState;

    fn compute(source: Self::SourceStates) -> Option<Self> {
        source.connected().then_some(Self)
    }
}

// TODO: use CONSTs instead of magic numbers for UI
const POPUP_HEIGHT_PERCENT: f32 = 60.0;
const POPUP_BG_COLOR_RGBA: [u8; 4] = [43, 43, 43, 240];

pub fn home_plugin(app: &mut App) {
    app.add_plugins(client_connection_plugin)
        .add_sub_state::<HomeState>()
        .add_sub_state::<JoiningServerState>()
        .add_computed_state::<ConnectedToServer>()
        .enable_state_scoped_entities::<HomeState>()
        // .add_systems(Update, log_transitions::<HomeState>) // DEBUG
        .add_systems(Update, log_transitions::<JoiningServerState>) // DEBUG
        // .add_systems(Update, log_transitions::<ConnectedToServer>) // DEBUG
        .add_systems(OnEnter(AppState::Home), setup_home)
        .add_systems(
            Update,
            (
                button_system::<JoinServerButton>,
                button_system::<QuitButton>,
                focus_text_input,
                unfocus_text_input.run_if(input_just_pressed(MouseButton::Left)),
            )
                .run_if(in_state(HomeState::Menu)),
        )
        .add_state_scoped_observer(HomeState::Menu, on_click_join_server_button)
        .add_state_scoped_observer(HomeState::Menu, on_click_quit_button)
        .add_systems(OnEnter(HomeState::JoiningServer), setup_join_server_ui)
        .add_systems(OnEnter(JoiningServerState::Setup), setup_join_server)
        .add_systems(
            Update,
            button_system::<PopupCenterButton>.run_if(in_state(HomeState::JoiningServer)),
        )
        .add_state_scoped_observer(JoiningServerState::Connecting, on_click_cancel_conn_button)
        .add_state_scoped_observer(
            JoiningServerState::Failed,
            on_click_acknowledge_conn_failure,
        )
        .add_systems(
            Update,
            wait_for_connection.run_if(in_state(JoiningServerState::Connecting)),
        )
        .add_systems(
            Update,
            wait_for_client_to_shutdown.run_if(in_state(JoiningServerState::Cancelling)),
        )
        .add_systems(OnEnter(JoiningServerState::Failed), modify_button_text)
        .add_state_scoped_observer(JoiningServerState::Joining, check_response_to_join)
        .add_state_scoped_observer(ConnectedToServer, check_if_disconnected)
        .add_state_scoped_observer(
            JoiningServerState::WaitingForOtherPlayers,
            check_new_players,
        );
}

#[derive(Component)]
struct HomeWidget;

#[derive(Component)]
struct IpAddrTextInput;

#[derive(Component)]
struct JoinServerButton;

#[derive(Component)]
struct PopupCenterButton;

#[derive(Component)]
struct QuitButton;

fn setup_home(mut commands: Commands, args: Res<AppArgs>) {
    // Setup camera
    commands.spawn((
        StateScoped(AppState::Home),
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(20, 20, 26)),
            ..default()
        },
    ));

    let server_ip_text = args.server_ip.clone().unwrap_or_default();

    commands
        .spawn((
            HomeWidget,
            StateScoped(AppState::Home),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                align_content: AlignContent::Center,
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(30.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    column_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        IpAddrTextInput,
                        Node {
                            width: Val::Px(480.0),
                            height: Val::Px(48.0),
                            border: UiRect::all(Val::Px(2.0)),
                            padding: UiRect::all(Val::Px(5.0)),
                            ..default()
                        },
                        BorderColor(Color::srgb_u8(200, 200, 200)),
                        BackgroundColor(Color::srgb_u8(200, 200, 200)),
                        TextInput,
                        TextInputValue(server_ip_text),
                        TextInputSettings {
                            retain_on_submit: true,
                            ..default()
                        },
                        TextInputInactive(true),
                        TextInputPlaceholder {
                            value: "Enter Server IP".into(),
                            text_color: Some(Color::srgb_u8(100, 100, 100).into()),
                            ..default()
                        },
                        TextInputTextFont(TextFont {
                            font_size: 36.0,
                            ..default()
                        }),
                        TextInputTextColor(Color::BLACK.into()),
                    ));

                    spawn_common_button(parent, "Join", JoinServerButton);
                });

            spawn_common_button(parent, "Quit", QuitButton);
        });
}

fn focus_text_input(
    mut query: Query<
        (&Interaction, &mut TextInputInactive),
        (With<IpAddrTextInput>, Changed<Interaction>),
    >,
) {
    // FIXME: use single
    for (interaction, mut inactive) in &mut query {
        if matches!(interaction, Interaction::Pressed) && inactive.0 {
            debug!("activating ipaddr text input");
            inactive.0 = false;
        }
    }
}

fn unfocus_text_input(
    mut query: Query<(&Interaction, &mut TextInputInactive), With<IpAddrTextInput>>,
) {
    // FIXME: use single
    for (interaction, mut inactive) in &mut query {
        if matches!(interaction, Interaction::None) && !inactive.0 {
            debug!("deactivating ipaddr text input");
            inactive.0 = true;
        }
    }
}

fn on_click_quit_button(_trigger: Trigger<ButtonPressed<QuitButton>>, mut commands: Commands) {
    commands.send_event(bevy::app::AppExit::Success);
}

fn on_click_join_server_button(
    _trigger: Trigger<ButtonPressed<JoinServerButton>>,
    mut home_state: ResMut<NextState<HomeState>>,
) {
    home_state.set(HomeState::JoiningServer);
}

fn setup_join_server_ui(mut commands: Commands) {
    commands
        .spawn((
            StateScoped(HomeState::JoiningServer),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(POPUP_HEIGHT_PERCENT),
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(POPUP_BG_COLOR_RGBA.into_color()),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(88.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(LogDisplay::new(LogDisplaySettings {
                        max_lines: 20,
                        ..default()
                    }));
                });

            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(12.0),
                    align_content: AlignContent::End,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|parent| {
                    spawn_common_button(parent, "Cancel", PopupCenterButton);
                });
        });
}

fn setup_join_server(
    mut commands: Commands,
    addr_input_value: Single<&TextInputValue, With<IpAddrTextInput>>,
    mut state: ResMut<NextState<JoiningServerState>>,
    app_args: Res<AppArgs>,
) {
    // Parse IP Address
    let addr: IpAddr = match addr_input_value.0.parse() {
        Ok(v) => v,
        Err(e) => {
            commands.send_event(LogEvent::Push(Message::error(e.to_string())));
            state.set(JoiningServerState::Failed);
            return;
        }
    };

    commands.send_event(LogEvent::Push(Message::from_text(format!(
        "joining the server...\nIP address: {}",
        addr
    ))));

    spawn_client(&mut commands, addr, app_args.server_port);
    state.set(JoiningServerState::Connecting);
}

fn wait_for_connection(
    mut commands: Commands,
    mut reader: EventReader<SpawnClientResult>,
    mut state: ResMut<NextState<JoiningServerState>>,
    ev_handler: Option<ResMut<client::EventHandler>>,
) {
    if reader.is_empty() {
        return;
    }
    let res = reader.read().next().unwrap();

    match res.0 {
        Ok(_) => {
            // Connection is established.
            commands.send_event(LogEvent::Push(Message::success("connected to the server")));

            // Now send Join request to the server.
            let id = match ev_handler
                .expect("event handler should be available at this point")
                .send_request(OutboundEvent::RequestJoin)
            {
                Ok(id) => id,
                Err(e) => {
                    // Possibly disconnected from the server.
                    commands.send_event(LogEvent::Push(Message::error(format!(
                        "failed to send join request: {}",
                        e
                    ))));
                    state.set(JoiningServerState::Failed);
                    return;
                }
            };
            commands.spawn((
                StateScoped(JoiningServerState::Joining),
                JoinRequestEventId(id),
            ));

            state.set(JoiningServerState::Joining);
        }
        Err(ref err_msg) => {
            // Connection is failed.
            commands.send_event(LogEvent::Push(Message::error(format!(
                "failed to join the server :(\n{}",
                err_msg
            ))));

            state.set(JoiningServerState::Failed);
        }
    }

    reader.clear();
}

#[derive(Component)]
struct JoinRequestEventId(protocol::EventId);

fn wait_for_client_to_shutdown(
    mut reader: EventReader<SpawnClientResult>,
    mut home_state: ResMut<NextState<HomeState>>,
) {
    if reader.is_empty() {
        return;
    }

    let res = reader.read().next().unwrap();
    info!("connection is cancelled: res={:?}", res);

    home_state.set(HomeState::Menu);
}

#[allow(clippy::never_loop)]
fn check_response_to_join(
    response: Trigger<ReceivedResponse>,
    mut ev_handler: ResMut<client::EventHandler>,
    query: Single<(Entity, &JoinRequestEventId)>,
    mut state: ResMut<NextState<JoiningServerState>>,
    mut commands: Commands,
) {
    let (entity, ev_id) = *query;
    let ev_id = ev_id.0;

    if response.event().id() != ev_id {
        return;
    }

    commands.entity(entity).despawn();

    let response = ev_handler
        .storage
        .take_response(ev_id)
        .expect("response should be available");

    match response {
        InboundEvent::RequestJoinAccepted(JoinInfo {
            // TODO: Store PlayerId somewhere
            player_id,
            join_position,
            room_size,
        }) => {
            commands.send_event(LogEvent::Push(Message::success(format!(
                "joined the lobby ( {} / {} )",
                join_position, room_size
            ))));
            commands.send_event(LogEvent::Push(Message::debug(format!("{:?}", player_id)))); // DEBUG

            let next_state = if join_position == room_size {
                JoiningServerState::WaitingForGameToStart
            } else {
                if join_position > room_size {
                    warn!(
                        "invalid join info state: join_position={}, room_size={}",
                        join_position, room_size
                    );
                }
                JoiningServerState::WaitingForOtherPlayers
            };
            state.set(next_state);
        }
        InboundEvent::Error(e) => {
            commands.send_event(LogEvent::Push(Message::error(e)));
            state.set(JoiningServerState::Failed);
        }
        unexp => {
            panic!("unexpected response to RequestJoin: {:?}", unexp);
        }
    }
}

fn on_click_cancel_conn_button(
    _trigger: Trigger<ButtonPressed<PopupCenterButton>>,
    mut commands: Commands,
    mut state: ResMut<NextState<JoiningServerState>>,
) {
    commands.send_event(LogEvent::Push(Message::warn(
        "cancelling the connection...",
    )));
    commands.send_event(CancelSpawnClientEvent);
    state.set(JoiningServerState::Cancelling);
}

fn on_click_acknowledge_conn_failure(
    _trigger: Trigger<ButtonPressed<PopupCenterButton>>,
    mut home_state: ResMut<NextState<HomeState>>,
) {
    // Go back to home menu
    home_state.set(HomeState::Menu);
}

fn modify_button_text(
    mut commands: Commands,
    children: Single<&Children, With<PopupCenterButton>>,
) {
    commands.entity(children[0]).insert(Text::new("Go Back"));
}

fn check_if_disconnected(
    trigger: Trigger<ReceivedRequest>,
    mut ev_handler: ResMut<client::EventHandler>,
    mut state: ResMut<NextState<JoiningServerState>>,
    mut commands: Commands,
) {
    let id = trigger.event().id();

    info!("event triggered! id={:?}", id);

    if let Some(InboundEvent::ServerShutdown) = ev_handler.storage.get_request(id) {
        // Consume this event
        ev_handler.storage.take_request(id);

        commands.send_event(LogEvent::Push(Message::error("disconnected")));
        state.set(JoiningServerState::Failed);
    }
}

fn check_new_players(
    trigger: Trigger<ReceivedRequest>,
    mut ev_handler: ResMut<client::EventHandler>,
    mut state: ResMut<NextState<JoiningServerState>>,
    mut commands: Commands,
) {
    let id = trigger.event().id();

    if let Some(InboundEvent::PlayerJoined(join_info)) = ev_handler.storage.get_request(id) {
        let JoinInfo {
            // TODO: Store PlayerId somewhere?
            player_id,
            join_position,
            room_size,
        } = *join_info;

        // Consume this event
        ev_handler.storage.take_request(id);

        commands.send_event(LogEvent::Push(Message::from_text(format!(
            "new player joined the lobby ( {} / {} )",
            join_position, room_size
        ))));
        commands.send_event(LogEvent::Push(Message::debug(format!("{:?}", player_id)))); // DEBUG

        if join_position == room_size {
            state.set(JoiningServerState::WaitingForGameToStart);
        }
    }
}
