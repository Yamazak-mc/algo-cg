use std::net::{IpAddr, Ipv4Addr};

use crate::{spawn_common_button, AppState, ServerPort};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputSettings, TextInputTextColor,
    TextInputTextFont, TextInputValue,
};
use client::{
    button::{is_button_pressed, QueryButtonClick},
    client::{client_connection_plugin, spawn_client, ConnectionResult},
    util::IntoColor as _,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(AppState = AppState::Home)]
enum HomeState {
    #[default]
    Menu,
    JoiningServer,
    FailedToJoinServer,
}

// TODO: use CONSTs instead of magic numbers for UI
const POPUP_HEIGHT_PERCENT: f32 = 60.0;
const POPUP_BG_COLOR_RGBA: [u8; 4] = [43, 43, 43, 240];

pub fn home_plugin(app: &mut App) {
    app.add_plugins(client_connection_plugin)
        .add_sub_state::<HomeState>()
        .enable_state_scoped_entities::<HomeState>()
        .add_systems(Update, bevy_dev_tools::states::log_transitions::<HomeState>) // DEBUG
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
        .add_systems(OnEnter(HomeState::JoiningServer), setup_join_server)
        .add_systems(
            Update,
            wait_for_connection.run_if(in_state(HomeState::JoiningServer)),
        )
        .add_systems(OnEnter(HomeState::FailedToJoinServer), setup_error_popup)
        .add_systems(
            Update,
            on_click_close_popup_button.run_if(
                in_state(HomeState::FailedToJoinServer).and(is_button_pressed::<ClosePopupButton>),
            ),
        );
}

#[derive(Component)]
struct HomeWidget;

#[derive(Component)]
struct IpAddrTextInput;

#[derive(Component)]
struct JoinServerButton;

#[derive(Component)]
struct QuitButton;

fn setup_home(mut commands: Commands) {
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

fn on_click_join_server_button(
    // mut app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut home_state: ResMut<NextState<HomeState>>,
    addr_input_value: Query<&TextInputValue, With<IpAddrTextInput>>,
) {
    fn parse_ip_v4_addr(addr: &str) -> anyhow::Result<Ipv4Addr> {
        use anyhow::Context;

        match addr.parse() {
            Ok(IpAddr::V4(v4_addr)) => Ok(v4_addr),
            Ok(IpAddr::V6(v6_addr)) => v6_addr
                .to_ipv4()
                .context("given address is not IPv4-compatible"),
            Err(e) => Err(e.into()),
        }
    }
    let v4_addr = match parse_ip_v4_addr(&addr_input_value.single().0) {
        Ok(v) => v,
        Err(e) => {
            commands.spawn((
                StateScoped(HomeState::FailedToJoinServer),
                JoinServerError(e.into()),
            ));
            home_state.set(HomeState::FailedToJoinServer);
            return;
        }
    };

    commands.spawn((
        StateScoped(HomeState::JoiningServer),
        JoinServerAttempt { addr: v4_addr },
    ));
    home_state.set(HomeState::JoiningServer);
}

fn popup_root_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(POPUP_HEIGHT_PERCENT),
        align_self: AlignSelf::Center,
        justify_self: JustifySelf::Center,
        ..default()
    }
}

fn popup_bg_color() -> BackgroundColor {
    BackgroundColor(POPUP_BG_COLOR_RGBA.into_color())
}

#[derive(Component)]
struct JoinServerError(Box<dyn std::error::Error + Send + Sync + 'static>);

fn setup_error_popup(mut commands: Commands, err: Query<&JoinServerError>) {
    let err_msg = err.single().0.to_string();

    commands
        .spawn((
            StateScoped(HomeState::FailedToJoinServer),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..popup_root_node()
            },
            popup_bg_color(),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(88.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text(format!("Failed to join server :(\n{}", err_msg)),
                        TextColor(Color::srgb_u8(255, 0, 0)),
                    ));
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
                    spawn_common_button(parent, "OK", ClosePopupButton);
                });
        });
}

#[derive(Component)]
struct ClosePopupButton;

fn on_click_close_popup_button(mut home_state: ResMut<NextState<HomeState>>) {
    // Go back to home menu
    home_state.set(HomeState::Menu);
}

fn setup_join_server(
    attempt: Query<&JoinServerAttempt>,
    port: Res<ServerPort>,
    mut commands: Commands,
) {
    let addr = attempt.single().addr;
    let port = port.0;

    let text = format!("Joining server...\nIP: {}", addr);

    commands
        .spawn((
            StateScoped(HomeState::JoiningServer),
            popup_root_node(),
            popup_bg_color(),
        ))
        .with_children(|parent| {
            parent.spawn(Text(text));
        });

    spawn_client(&mut commands, addr.into(), port);
}

#[derive(Component)]
struct JoinServerAttempt {
    addr: Ipv4Addr,
}

fn wait_for_connection(mut res: EventReader<ConnectionResult>) {
    if res.is_empty() {
        return;
    }
    let res = res.read().next().unwrap();

    match res.0 {
        Ok(_) => {
            info!("Connection suceeded!");
        }
        Err(ref e) => {
            error!("Connection failed!\n{}", e);
        }
    }
}
