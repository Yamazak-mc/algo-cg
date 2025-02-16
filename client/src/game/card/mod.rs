use bevy::{
    app::{PluginGroup, PluginGroupBuilder},
    math::Vec3,
};

pub mod attacker;
pub mod flip_animation;
pub mod guessing;
pub mod instance;
pub mod material;
pub mod mesh;
pub mod name;
pub mod picking;
pub mod tag;

pub struct CardPlugins {
    pub card_size: Vec3,
}

impl PluginGroup for CardPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(instance::card_instance_plugin)
            .add(mesh::CardMeshPlugin {
                card_size: self.card_size,
            })
            .add(material::card_material_plugin)
            .add(tag::card_tag_plugin)
            .add(picking::card_picking_plugin)
            .add(guessing::card_guessing_plugin)
            .add(flip_animation::card_flip_plugin)
            .add(name::card_name_plugin)
            .add(attacker::attacker_plugin)
    }
}
