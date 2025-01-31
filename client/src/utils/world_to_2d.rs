use anyhow::Context as _;
use bevy::{prelude::*, window::WindowResized};

pub fn world_to_2d_plugin(app: &mut App) {
    app.register_type::<(Followed, FollowOffsets)>()
        .add_systems(Update, Followed::update)
        .add_observer(UpdatePos::handle_trigger)
        .add_observer(AddFollower::handle_trigger)
        .add_observer(DespawnFollower::handle_trigger)
        .add_systems(Update, handle_camera_movement)
        .add_systems(PostUpdate, handle_window_resize);
}

#[derive(Clone, Copy, Event)]
pub struct AddFollower {
    pub follower: Entity,
    pub offset_3d: Vec3,
    pub offset_2d: Vec3,
}

impl AddFollower {
    fn handle_trigger(trigger: Trigger<Self>, mut commands: Commands) {
        let target = trigger.entity();
        let AddFollower {
            follower,
            offset_3d,
            offset_2d,
        } = *trigger.event();

        commands.entity(target).insert((
            Followed { follower },
            FollowOffsets {
                offset_2d,
                offset_3d,
            },
        ));

        commands
            .entity(follower)
            .insert(Follower { _target: target });

        commands.trigger_targets(UpdatePos, target);
    }
}

#[derive(Event)]
pub struct DespawnFollower;

impl DespawnFollower {
    fn handle_trigger(trigger: Trigger<Self>, query: Query<&Followed>, mut commands: Commands) {
        let target = trigger.entity();

        let Ok(Followed { follower }) = query.get(target) else {
            return;
        };

        commands
            .entity(target)
            .remove::<(Followed, FollowOffsets)>();

        commands.entity(*follower).despawn_recursive();
    }
}

#[derive(Component, Reflect)]
#[require(Transform)]
pub struct Followed {
    follower: Entity,
}

#[derive(Debug, Clone, Default, Component, Reflect)]
pub struct FollowOffsets {
    pub offset_3d: Vec3,
    pub offset_2d: Vec3,
}

impl Followed {
    fn update(
        mut commands: Commands,
        targets: Query<
            Entity,
            (
                With<Followed>,
                Or<(Changed<Transform>, Changed<FollowOffsets>)>,
            ),
        >,
    ) {
        for target in &targets {
            commands.trigger_targets(UpdatePos, target);
        }
    }
}

#[derive(Event)]
struct UpdatePos;

impl UpdatePos {
    fn handle_trigger(
        trigger: Trigger<Self>,
        query: Query<(&Followed, &FollowOffsets, &Transform)>,
        camera: Option<Single<(&Camera, &GlobalTransform), With<Camera3d>>>,
        mut followers: Query<&mut Transform, Without<Followed>>,
    ) {
        let Some(camera) = camera else {
            return;
        };
        let (camera, camera_transform) = *camera;

        let entity = trigger.entity();
        let (target, offsets, target_transform) = query.get(entity).unwrap();

        let Ok(pos2d) = world_to_2d_pos(
            camera,
            camera_transform,
            target_transform.translation + offsets.offset_3d,
        ) else {
            return;
        };

        followers.get_mut(target.follower).unwrap().translation = pos2d + offsets.offset_2d;
    }
}

#[derive(Component)]
#[require(Transform, FollowOffsets)]
pub struct Follower {
    _target: Entity,
}

fn handle_camera_movement(
    mut params: ParamSet<(
        Option<Single<Entity, (With<Camera3d>, Changed<Transform>)>>,
        Query<&mut Transform, With<Followed>>,
    )>,
) {
    if params.p0().is_none() {
        return;
    }

    flag_transforms(params.p1());
}

fn handle_window_resize(
    events: EventReader<WindowResized>,
    owners: Query<&mut Transform, With<Followed>>,
) {
    if !events.is_empty() {
        flag_transforms(owners);
    }
}

fn flag_transforms(mut transforms: Query<&mut Transform, With<Followed>>) {
    for mut owner in &mut transforms {
        // Perform `DerefMut` to trigger the `Changed<Transform>` query filter.
        let _: &mut Transform = &mut owner;
    }
}

fn world_to_2d_pos(
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
