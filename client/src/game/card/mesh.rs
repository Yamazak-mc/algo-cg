use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use bevy_mod_outline::{GenerateOutlineNormalsSettings, OutlineMeshExt};

pub struct CardMeshPlugin {
    pub card_size: Vec3,
}

impl Plugin for CardMeshPlugin {
    fn build(&self, app: &mut App) {
        let card_size = self.card_size;

        app.add_systems(
            Startup,
            move |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                let card_mesh = meshes.add(create_card_mesh(card_size));
                commands.insert_resource(CardMesh(card_mesh));
            },
        );
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CardMesh(pub Handle<Mesh>);

pub fn create_card_mesh(size: Vec3) -> Mesh {
    let w = size.x / 2.0;
    let h = size.z / 2.0;
    let d = size.y / 2.0;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-w, d, -h],
            [w, d, -h],
            [w, d, h],
            [-w, d, h],
            [-w, -d, -h],
            [w, -d, -h],
            [w, -d, h],
            [-w, -d, h],
            [w, -d, -h],
            [w, -d, h],
            [w, d, h],
            [w, d, -h],
            [-w, -d, -h],
            [-w, -d, h],
            [-w, d, h],
            [-w, d, -h],
            [-w, -d, h],
            [-w, d, h],
            [w, d, h],
            [w, -d, h],
            [-w, -d, -h],
            [-w, d, -h],
            [w, d, -h],
            [w, -d, -h],
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ],
    )
    .with_inserted_indices(Indices::U32(vec![
        0, 3, 1, 1, 3, 2, 4, 5, 7, 5, 6, 7, 8, 11, 9, 9, 11, 10, 12, 13, 15, 13, 14, 15, 16, 19,
        17, 17, 19, 18, 20, 21, 23, 21, 22, 23,
    ]));

    mesh.generate_outline_normals(&GenerateOutlineNormalsSettings::default())
        .unwrap();
    mesh
}
