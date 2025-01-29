use anyhow::Context as _;
use bevy::{math::Vec3, render::camera::Camera, transform::components::GlobalTransform};

pub fn world_to_2d_pos(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    at: Vec3,
) -> anyhow::Result<Vec3> {
    let vpsize = camera
        .logical_viewport_size()
        .context("failed to get viewport size")?;

    let pos = camera
        .world_to_viewport(camera_transform, at)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let x = pos.x - vpsize.x / 2.0;
    let y = vpsize.y / 2.0 - pos.y;

    Ok(Vec3::new(x, y, 0.0))
}
