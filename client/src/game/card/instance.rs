use super::{material::CardMaterials, mesh::CardMesh};
use crate::game::CTX_STATE;
use algo_core::card::CardView;
use bevy::prelude::*;
use client::utils::add_observer_ext::AddStateScopedObserver as _;
use std::f32::consts::PI;

pub fn card_instance_plugin(app: &mut App) {
    app.add_state_scoped_observer(CTX_STATE, CardInstance::init);
}

/// The main card component.
#[derive(Debug, Component, Deref, DerefMut)]
pub struct CardInstance(pub CardView);

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
            .insert((StateScoped(CTX_STATE), transform, Visibility::Inherited))
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
