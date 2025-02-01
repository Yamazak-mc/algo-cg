#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;
use client::utils::log_display::log_display_plugin;

mod game;
mod home;

/// Arguments for launching client app.
#[derive(argh::FromArgs, Debug, Resource)]
struct AppArgs {
    /// server IP address
    #[argh(option)]
    server_ip: Option<String>,

    /// server port number
    #[argh(option, default = "protocol::DEFAULT_SERVER_PORT")]
    server_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum AppState {
    Home,
    Game,
}

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "FULL");

    let args: AppArgs = argh::from_env();

    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins(bevy_remote_inspector::RemoteInspectorPlugins)
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugins(TextInputPlugin)
        .add_plugins(log_display_plugin)
        .insert_resource(args)
        .insert_state(AppState::Home)
        .enable_state_scoped_entities::<AppState>()
        .add_plugins(home::home_plugin)
        .add_plugins(game::game_plugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(20, 20, 26)),
            order: 1,
            ..default()
        },
    ));
}
