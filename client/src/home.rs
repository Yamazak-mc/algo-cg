use crate::{spawn_common_button, AppArgs, AppState};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_dev_tools::states::log_transitions;
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputSettings, TextInputTextColor,
    TextInputTextFont, TextInputValue,
};
use client::{
    button::is_button_pressed,
    client::{
        client_connection_plugin, spawn_client, CancelSpawnClientEvent, InboundEvent,
        OutboundEvent, ReceivedRequest, ReceivedResponse, SpawnClientResult,
    },
    log_display::{LogDisplay, LogDisplaySettings, LogEvent, Message},
    util::IntoColor as _,
};
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
    Joined,
}

impl JoiningServerState {
    fn connected(&self) -> bool {
        matches!(self, Self::Joining | Self::Joined)
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
        .add_systems(Update, log_transitions::<HomeState>) // DEBUG
        .add_systems(Update, log_transitions::<JoiningServerState>) // DEBUG
        .add_systems(Update, log_transitions::<ConnectedToServer>) // DEBUG
        .add_systems(OnEnter(AppState::Home), setup_home)
        .add_systems(Update, focus_text_input.run_if(in_state(HomeState::Menu)))
        .add_systems(
            Update,
            on_click_join_server_button
                .run_if(in_state(HomeState::Menu).and(is_button_pressed::<JoinServerButton>)),
        )
        .add_systems(
            Update,
            on_click_quit_button
                .run_if(in_state(HomeState::Menu).and(is_button_pressed::<QuitButton>)),
        )
        .add_systems(
            Update,
            unfocus_text_input
                .run_if(in_state(HomeState::Menu).and(input_just_pressed(MouseButton::Left))),
        )
        .add_systems(OnEnter(HomeState::JoiningServer), setup_join_server_ui)
        .add_systems(OnEnter(JoiningServerState::Setup), setup_join_server)
        .add_systems(
            Update,
            wait_for_connection.run_if(in_state(JoiningServerState::Connecting)),
        )
        .add_systems(
            Update,
            on_click_cancel_conn_button.run_if(
                in_state(JoiningServerState::Connecting).and(is_button_pressed::<ClosePopupButton>),
            ),
        )
        .add_systems(
            Update,
            check_response_to_join
                .run_if(in_state(JoiningServerState::Joining).and(on_event::<ReceivedResponse>)),
        )
        .add_systems(
            Update,
            wait_for_client_to_shutdown.run_if(in_state(JoiningServerState::Cancelling)),
        )
        .add_systems(OnEnter(JoiningServerState::Failed), modify_button_text)
        .add_systems(
            Update,
            on_click_acknowledge_conn_failure.run_if(
                in_state(JoiningServerState::Failed).and(is_button_pressed::<ClosePopupButton>),
            ),
        )
        .add_systems(
            Update,
            check_if_disconnected
                .run_if(in_state(ConnectedToServer).and(on_event::<ReceivedRequest>)),
        );
}

#[derive(Component)]
struct HomeWidget;

#[derive(Component)]
struct IpAddrTextInput;

#[derive(Component)]
struct JoinServerButton;

#[derive(Component)]
struct ClosePopupButton;

#[derive(Component)]
struct QuitButton;

fn setup_home(mut commands: Commands, args: Res<AppArgs>) {
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

fn on_click_quit_button(mut commands: Commands) {
    commands.send_event(bevy::app::AppExit::Success);
}

fn on_click_join_server_button(mut home_state: ResMut<NextState<HomeState>>) {
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
                        font: TextFont::default(),
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
                    spawn_common_button(parent, "Cancel", ClosePopupButton);
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
            let id = ev_handler
                .expect("event handler should be available at this point")
                .send_request(OutboundEvent::RequestJoin)
                .unwrap();
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
    mut commands: Commands,
    ev_id: Single<&JoinRequestEventId>,
    mut responses: EventReader<ReceivedResponse>,
    mut ev_handler: ResMut<client::EventHandler>,
    mut state: ResMut<NextState<JoiningServerState>>,
) {
    let ev_id = ev_id.0;
    if !responses.read().any(|ev| ev.id() == ev_id) {
        return;
    }

    let response = ev_handler
        .storage
        .take_response(ev_id)
        .expect("response should be available");

    match response {
        InboundEvent::RequestJoinAccepted => {
            commands.send_event(LogEvent::Push(Message::success("joined the lobby")));
            state.set(JoiningServerState::Joined);
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
    mut commands: Commands,
    mut state: ResMut<NextState<JoiningServerState>>,
) {
    commands.send_event(LogEvent::Push(Message::warn(
        "cancelling the connection...",
    )));
    commands.send_event(CancelSpawnClientEvent);
    state.set(JoiningServerState::Cancelling);
}

fn on_click_acknowledge_conn_failure(mut home_state: ResMut<NextState<HomeState>>) {
    // Go back to home menu
    home_state.set(HomeState::Menu);
}

fn modify_button_text(mut commands: Commands, children: Single<&Children, With<ClosePopupButton>>) {
    commands.entity(children[0]).insert(Text::new("Go Back"));
}

fn check_if_disconnected(
    mut evr: EventReader<ReceivedRequest>,
    mut ev_handler: ResMut<client::EventHandler>,
    mut state: ResMut<NextState<JoiningServerState>>,
    mut commands: Commands,
) {
    for req in evr.read() {
        let id = req.id();
        if let Some(InboundEvent::ServerShutdown) = ev_handler.storage.get_request(id) {
            // Consume this event
            ev_handler.storage.take_request(id);

            commands.send_event(LogEvent::Push(Message::error("disconnected")));
            state.set(JoiningServerState::Failed);
        }
    }
}
