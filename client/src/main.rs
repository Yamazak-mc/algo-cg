#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;
use client::utils::log_display::log_display_plugin;

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
