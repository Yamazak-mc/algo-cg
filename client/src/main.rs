#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;
use client::log_display::log_display_plugin;

mod home;

/// Arguments for launching client app.
#[derive(argh::FromArgs, Debug, Resource)]
struct AppArgs {
    /// server IP address
    #[argh(option)]
    server_ip: Option<String>,

    /// server port number
    #[argh(option, default = "client::DEFAULT_SERVER_PORT")]
    server_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum AppState {
    Home,
}

fn main() {
    let args: AppArgs = argh::from_env();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextInputPlugin)
        .add_plugins(log_display_plugin)
        .insert_resource(args)
        .insert_state(AppState::Home)
        .enable_state_scoped_entities::<AppState>()
        .add_plugins(home::home_plugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(20, 20, 26)),
            ..default()
        },
    ));
}

fn spawn_common_button<M: Component>(parent: &mut ChildBuilder, text: &str, marker: M) {
    parent
        .spawn((
            marker,
            Node {
                width: Val::Px(320.0),
                height: Val::Px(48.0),
                padding: UiRect::top(Val::Px(3.0)),
                align_content: AlignContent::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor(Color::WHITE),
            BackgroundColor(Color::srgb_u8(0x57, 0x7B, 0xC1)),
            Button,
        ))
        .with_child((
            Text::new(text),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb_u8(0xFF, 0xFA, 0xEC)),
        ));
}
