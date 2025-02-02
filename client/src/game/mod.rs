use crate::AppState;
use bevy::prelude::*;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use client::utils::{
    animate_once::AnimateOncePlugin, observer_controller::ObserverControllerPlugin,
    world_to_2d::world_to_2d_plugin,
};

mod card;
use card::CardPlugins;

mod card_field;
use card_field::card_field_plugin;

// DEBUG
mod sandbox;
use sandbox::game_sandbox_plugin;

const CTX_STATE: AppState = AppState::Game;

const CAMERA_TRANSLATION: Vec3 = Vec3::new(0.0, 10.715912, 4.192171);
const CAMERA_ROTATION: Quat = Quat::from_xyzw(-0.5792431, 0.0, 0.0, 0.81516445);

const CARD_SIZE: Vec3 = Vec3::new(1.0, 0.03, 1.618);
const CARD_WIDTH: f32 = CARD_SIZE.x;
const CARD_HEIGHT: f32 = CARD_SIZE.z;
const CARD_DEPTH: f32 = CARD_SIZE.y;

const CARD_X_GAP_RATIO: f32 = 0.2;
const CARD_Z_GAP_RATIO: f32 = 0.1;

const CARD_WIDTH_PLUS_GAP: f32 = CARD_WIDTH * (1.0 + CARD_X_GAP_RATIO);

const TALON_TRANSLATION: Vec3 = Vec3::new(2.0, CARD_DEPTH / 2.0, 0.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(AppState = AppState::Game)]
enum GameMode {
    #[default]
    TwoPlayers,
    Sandbox,
}

pub fn game_plugin(app: &mut App) {
    app.add_sub_state::<GameMode>()
        .enable_state_scoped_entities::<GameMode>()
        .add_plugins((
            InfiniteGridPlugin,
            AnimateOncePlugin::from_state(AppState::Game),
            ObserverControllerPlugin::<Pointer<Click>>::default().state_scoped(AppState::Game),
            world_to_2d_plugin,
            CardPlugins {
                card_size: CARD_SIZE,
            },
            card_field_plugin,
            game_sandbox_plugin, // DEBUG
        ))
        .add_systems(OnEnter(AppState::Game), setup_game);
}

fn setup_game(mut commands: Commands) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        Transform {
            translation: CAMERA_TRANSLATION,
            rotation: CAMERA_ROTATION,
            ..default()
        },
        Msaa::Sample4,
    ));

    // grid
    commands.spawn(InfiniteGridBundle {
        settings: InfiniteGridSettings { ..default() },
        ..default()
    });

    // lights
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 250.0,
    });
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
}
