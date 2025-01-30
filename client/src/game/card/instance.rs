use super::{
    flip_animation::FlipCard,
    material::CardMaterials,
    mesh::CardMesh,
    tag::{DespawnCardTag, SpawnCardTag},
};
use crate::game::CTX_STATE;
use algo_core::card::{CardPrivInfo, CardView};
use bevy::prelude::*;
use client::utils::add_observer_ext::AddStateScopedObserver as _;
use std::f32::consts::PI;

pub fn card_instance_plugin(app: &mut App) {
    app.add_systems(Update, CardInstance::on_change.run_if(in_state(CTX_STATE)))
        .add_state_scoped_observer(CTX_STATE, CardInstance::init)
        .add_state_scoped_observer(CTX_STATE, AddPrivInfo::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, Reveal::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, RevealWith::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, UpdateMaterial::handle_trigger);
}

/// The main card component.
#[derive(Debug, Component)]
pub struct CardInstance(CardView);

impl CardInstance {
    pub fn new(card: CardView) -> Self {
        Self(card)
    }

    pub fn get(&self) -> &CardView {
        &self.0
    }

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

        let card_state = ComputedCardInstState::compute(&card_instance.0).unwrap();

        let transform = transform.cloned().unwrap_or_default();

        commands
            .entity(entity)
            .insert((
                StateScoped(CTX_STATE),
                card_state,
                transform,
                Visibility::Inherited,
            ))
            .with_children(|parent| {
                parent.spawn(Self::create_card_components(
                    &card_mesh,
                    &mut card_textures,
                    &mut images,
                    &mut materials,
                    &card_instance.0,
                ));
            });
    }

    /// Creates the components for card graphics.
    fn create_card_components(
        // Resources for card looks
        card_mesh: &CardMesh,
        card_materials: &mut CardMaterials,

        // Assets
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,

        // Card info
        card_view: &CardView,
    ) -> impl Bundle {
        let mesh_handle = card_mesh.0.clone();

        let material = Self::create_material(card_materials, images, materials, card_view);

        let rotation = if !card_view.pub_info.revealed {
            // Face down
            Quat::from_rotation_z(PI)
        } else {
            default()
        };

        (
            Mesh3d(mesh_handle),
            material,
            Transform::from_rotation(rotation),
        )
    }

    fn create_material(
        card_materials: &mut CardMaterials,
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,
        card_view: &CardView,
    ) -> impl Bundle {
        let color = card_view.pub_info.color;
        let number = card_view.priv_info.map(|v| v.number);

        let material_handle =
            card_materials.get_or_create_card_material(color, number, images, materials);

        MeshMaterial3d(material_handle)
    }

    fn on_change(mut commands: Commands, cards: Query<(Entity, &Self), Changed<Self>>) {
        for (entity, card) in &cards {
            if card.0.priv_info.is_some() {
                if card.0.pub_info.revealed {
                    commands.trigger_targets(DespawnCardTag, entity);
                } else {
                    commands.trigger_targets(SpawnCardTag, entity);
                }
            }
        }
    }
}

/// State transitions:
/// - _ -> 1: When the card is spawned.
/// - 1 -> 2: When the player (self) draws the card.
/// - 2 -> 3: When the player (self) flips their own card.
/// - 1 -> 3: When the opponent's card is flipped.
#[derive(Clone, Copy, PartialEq, Eq, Component)]
enum ComputedCardInstState {
    /// - revealed = `false`
    /// - priv_info = `None`
    Spawned,
    /// - revealed = `false`
    /// - priv_info = `Some`
    Private,
    /// - revealed = `true`
    /// - priv_info = `Some`
    Public,
}

impl ComputedCardInstState {
    fn compute(card: &CardView) -> Option<Self> {
        let ret = match (card.pub_info.revealed, card.priv_info) {
            (false, None) => Self::Spawned,
            (false, Some(_)) => Self::Private,
            (true, Some(_)) => Self::Public,
            _ => return None,
        };

        Some(ret)
    }

    fn is_spawned(&self) -> bool {
        matches!(self, Self::Spawned)
    }

    fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }
}

/// `Spawned` --> `Private`
#[derive(Clone, Event)]
pub struct AddPrivInfo(pub CardPrivInfo);

impl AddPrivInfo {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut query: Query<&mut CardInstance>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let card = &mut query.get_mut(entity).unwrap().0;

        if !ComputedCardInstState::compute(&card).unwrap().is_spawned() {
            warn!("invalid card instance state for an event `RevealWith`");
            return;
        }

        card.priv_info = Some(trigger.event().0.clone());

        commands.trigger_targets(UpdateMaterial(*card), entity);
    }
}

/// `Private` --> `Public`
#[derive(Clone, Event)]
pub struct Reveal;

impl Reveal {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut query: Query<&mut CardInstance>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let card = &mut query.get_mut(entity).unwrap().0;

        if !ComputedCardInstState::compute(&card).unwrap().is_private() {
            warn!("invalid card instance state for an event `Reveal`");
            return;
        }

        card.pub_info.revealed = true;

        commands.trigger_targets(FlipCard, entity);
    }
}

/// `Spawned` --> `Public`
#[derive(Clone, Event)]
pub struct RevealWith(pub CardPrivInfo);

impl RevealWith {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut query: Query<&mut CardInstance>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let card = &mut query.get_mut(entity).unwrap().0;

        if !ComputedCardInstState::compute(&card).unwrap().is_spawned() {
            warn!("invalid card instance state for an event `RevealWith`");
            return;
        }

        card.pub_info.revealed = true;
        card.priv_info = Some(trigger.event().0.clone());

        commands.trigger_targets(UpdateMaterial(*card), entity);
        commands.trigger_targets(FlipCard, entity);
    }
}

#[derive(Event)]
struct UpdateMaterial(CardView);

impl UpdateMaterial {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        mut card_materials: ResMut<CardMaterials>,
        mut images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        children: Query<&Children>,
    ) {
        commands
            .entity(children.get(trigger.entity()).unwrap()[0])
            .insert(CardInstance::create_material(
                &mut card_materials,
                &mut images,
                &mut materials,
                &trigger.event().0,
            ));
    }
}
