use crate::AppState;
use algo_core::{card::CardView, player::PlayerId};
use bevy::prelude::*;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use client::utils::{
    add_observer_ext::AddStateScopedObserver as _,
    animate_once::{AnimateOnce, AnimateOncePlugin},
};
use std::f32::consts::PI;

mod card;
use card::{material::CardMaterials, mesh::CardMesh, CardPlugins};

// DEBUG
mod sandbox;
use sandbox::game_sandbox_plugin;

const GAME_SCOPE: StateScoped<AppState> = StateScoped(AppState::Game);
const CAMERA_TRANSLATION: Vec3 = Vec3::new(0.0, 10.715912, 4.192171);
const CAMERA_ROTATION: Quat = Quat::from_xyzw(-0.5792431, 0.0, 0.0, 0.81516445);

const CARD_SIZE: Vec3 = Vec3::new(1.0, 0.03 /* 0.03 */, 1.618);
const CARD_WIDTH: f32 = CARD_SIZE.x;
const CARD_HEIGHT: f32 = CARD_SIZE.z;
const CARD_DEPTH: f32 = CARD_SIZE.y;

const CARD_X_GAP_RATIO: f32 = 0.2;
const CARD_Z_GAP_RATIO: f32 = 0.1;

const CARD_WIDTH_PLUS_GAP: f32 = CARD_WIDTH * (1.0 + CARD_X_GAP_RATIO);

const CARD_INSERTION_ANIMATION_SECS: f32 = 0.5;

const TALON_TRANSLATION: Vec3 = Vec3::new(0.0, CARD_DEPTH / 2.0, 0.0);

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
            CardPlugins,
            game_sandbox_plugin, // DEBUG
        ))
        .add_systems(OnEnter(AppState::Game), setup_game)
        .add_state_scoped_observer(AppState::Game, CardInstance::init)
        .add_state_scoped_observer(AppState::Game, CardPosition::init)
        .add_state_scoped_observer(AppState::Game, CardPosition::shift);
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

#[derive(Debug, Default, Component)]
#[require(Transform)]
struct CardField {
    cards: Vec<Entity>,
}

impl CardField {
    /// Inserts a pre-existing card into the field.
    fn insert_card(
        &mut self,
        self_entity: Entity,
        idx: u32,
        entity: Entity,
        commands: &mut Commands,
    ) {
        if !self.cards.is_empty() {
            commands.trigger_targets(OtherCardInserted { idx }, self.cards.clone());
        }

        commands.entity(entity).insert(CardPosition {
            origin: self_entity,
            idx,
            len: self.cards.len() as u32 + 1,
        });

        self.cards.insert(idx as usize, entity);
    }
}

#[derive(Debug, Component)]
#[require(CardField)]
struct CardFieldOwnedBy(PlayerId);

/// A marker component used with `CardField`.
///
/// To get an opponent's `CardField`, use `Without<MyCardField>` filter.
#[derive(Debug, Component)]
#[require(CardField)]
struct MyCardField;

/// The main card component.
#[derive(Debug, Component, Deref, DerefMut)]
struct CardInstance(CardView);

impl CardInstance {
    /// Initializes the card instance as a Mesh3d.
    fn init(
        trigger: Trigger<OnAdd, Self>,
        query: Query<(&Self, Option<&Transform>)>,

        // for card looks
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut card_textures: ResMut<CardMaterials>,
        card_mesh: Res<CardMesh>,
    ) {
        let entity = trigger.entity();
        let (card_instance, transform) = query.get(entity).unwrap();

        let transform = transform.cloned().unwrap_or_default();

        commands
            .entity(entity)
            .insert((GAME_SCOPE, transform, Visibility::Inherited))
            .with_children(|parent| {
                parent.spawn(Self::create_card_components(
                    &card_mesh,
                    &mut card_textures,
                    &mut images,
                    &mut materials,
                    card_instance.0,
                ));
            });
    }

    /// Creates the components for card graphics.
    fn create_card_components(
        // Resources for card looks
        card_mesh: &CardMesh,
        card_textures: &mut CardMaterials,

        // Assets
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,

        // Card info
        card_view: CardView,
    ) -> impl Bundle {
        let color = card_view.pub_info.color;
        let number = card_view.priv_info.map(|v| v.number);

        let material_handle =
            card_textures.get_or_create_card_material(color, number, images, materials);
        let mesh_handle = card_mesh.0.clone();

        let rotation = if !card_view.pub_info.revealed {
            // Face down
            Quat::from_rotation_z(PI)
        } else {
            default()
        };

        (
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_rotation(rotation),
        )
    }
}

#[derive(Debug, Clone, Copy, Component)]
struct CardPosition {
    origin: Entity,
    idx: u32,
    len: u32,
}

impl CardPosition {
    fn init(
        trigger: Trigger<OnAdd, Self>,
        mut commands: Commands,
        mut query: Query<(&Self, &Transform)>,
        transform_query: Query<&Transform>,
    ) {
        let entity = trigger.entity();
        let (Self { origin, idx, len }, transform) = query.get_mut(entity).unwrap();
        let origin_xf = transform_query.get(*origin).unwrap();

        // Translation
        let animation = AnimateOnce::translation_and_rotation(
            *transform,
            Transform {
                translation: calculate_card_translation(*origin_xf, *idx, *len),
                rotation: transform.rotation * origin_xf.rotation,
                ..*transform
            },
            CARD_INSERTION_ANIMATION_SECS,
            EaseFunction::QuarticOut,
        );
        commands.trigger_targets(animation, entity);
    }

    fn shift(
        trigger: Trigger<OtherCardInserted>,
        mut commands: Commands,
        mut query: Query<(&Transform, &mut Self)>,
        origin_transform: Query<&Transform, With<CardField>>,
    ) {
        let entity = trigger.entity();
        let (xf, mut card_pos) = query.get_mut(entity).unwrap();
        let origin_xf = origin_transform.get(card_pos.origin).unwrap();

        card_pos.sync_idx_for_insertion(trigger.idx);

        // The card is already inserted to the field, no need to modify its rotation.
        let new_translation = calculate_card_translation(*origin_xf, card_pos.idx, card_pos.len);
        let animation = AnimateOnce::translation(
            xf.translation,
            new_translation,
            CARD_INSERTION_ANIMATION_SECS,
            EaseFunction::QuarticOut,
        );
        commands.trigger_targets(animation, entity);
    }
}

impl CardPosition {
    fn sync_idx_for_insertion(&mut self, inserted_at: u32) {
        if self.idx >= inserted_at {
            self.idx += 1;
        }
        self.len += 1;
    }
}

//
//
// EVENTS
//
//

#[derive(Debug, Event)]
struct OtherCardInserted {
    idx: u32,
}

//
//
// UTILS
//
//

fn calculate_card_translation(origin: Transform, idx: u32, len: u32) -> Vec3 {
    let j = idx as i32 - len as i32 / 2;
    let offset = if len % 2 == 0 { 0.5 } else { 0.0 };
    let distance = (j as f32 + offset) * CARD_WIDTH_PLUS_GAP;

    origin.translation + distance * origin.right()
}
