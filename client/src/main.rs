#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

use algo_core::player::PlayerId;
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
        .add_plugins((
            DefaultPlugins,
            TextInputPlugin,
            log_display_plugin,
            home::home_plugin,
            game::game_plugin,
            // DEBUG
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
            // bevy_remote_inspector::RemoteInspectorPlugins
        ))
        .insert_resource(args)
        .init_resource::<JoinedPlayers>()
        .insert_state(AppState::Home)
        .enable_state_scoped_entities::<AppState>()
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

#[derive(Debug, Default, Resource)]
struct JoinedPlayers {
    my_player: Option<PlayerId>,
    opponent_player: Option<PlayerId>,
}

impl JoinedPlayers {
    fn setup(mut this: ResMut<Self>) {
        *this = default();
    }

    fn set_my_player(&mut self, id: PlayerId) {
        self.my_player = Some(id);
    }

    fn set_opponent_player(&mut self, id: PlayerId) {
        self.opponent_player = Some(id);
    }
}
