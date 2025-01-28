use super::CARD_SIZE;
use bevy::app::{PluginGroup, PluginGroupBuilder};

pub mod flip_animation;
pub mod material;
pub mod mesh;
pub mod name;
pub mod picking;
pub mod tag;

pub struct CardPlugins;

impl PluginGroup for CardPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(mesh::CardMeshPlugin::new(CARD_SIZE))
            .add(material::card_material_plugin)
            .add(tag::card_tag_plugin)
            .add(picking::card_picking_plugin)
            .add(flip_animation::card_flip_plugin)
            .add(name::card_name_plugin)
    }
}
