use crate::{
    game::{CardInstance, CARD_HEIGHT},
    AppState,
};
use anyhow::Context as _;
use bevy::prelude::*;
use client::utils::add_observer_ext::AddStateScopedObserver as _;

const FONT_SIZE: f32 = 32.0;

pub fn card_tag_plugin(app: &mut App) {
    let ctx_state = AppState::Game;

    app.add_state_scoped_observer(ctx_state, AddCardTag::handle_trigger)
        .add_state_scoped_observer(ctx_state, RemoveCardTag::handle_trigger)
        .add_state_scoped_observer(ctx_state, CardTag::init)
        .add_systems(Update, CardTagOwner::update.run_if(in_state(ctx_state)))
        .add_systems(Update, on_camera_movement.run_if(in_state(ctx_state)));
}

#[derive(Event)]
pub struct AddCardTag;

impl AddCardTag {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        query: Query<&CardInstance, Without<CardTagOwner>>,
    ) {
        let card_entity = trigger.entity();
        let Ok(card) = query.get(card_entity) else {
            return;
        };
        if card.pub_info.revealed || card.priv_info.is_none() {
            return;
        }

        let tag_entity = commands
            .spawn((
                CardTag { owner: card_entity },
                Text2d(format!("{}", card.priv_info.unwrap().number.0)),
                TextFont::from_font_size(FONT_SIZE),
                // DEBUG
                Name::new("CardTag"),
            ))
            .id();

        commands
            .entity(card_entity)
            .insert(CardTagOwner { tag_entity });
    }
}

#[derive(Event)]
pub struct RemoveCardTag;

impl RemoveCardTag {
    fn handle_trigger(trigger: Trigger<Self>, query: Query<&CardTagOwner>, mut commands: Commands) {
        let card_entity = trigger.entity();
        let Ok(tag_owner) = query.get(card_entity) else {
            return;
        };

        commands.entity(card_entity).remove::<CardTagOwner>();
        commands.entity(tag_owner.tag_entity).despawn();
    }
}

#[derive(Component)]
struct CardTagOwner {
    tag_entity: Entity,
}

impl CardTagOwner {
    fn update(
        query: Query<(&CardTagOwner, &Transform), Changed<Transform>>,
        camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
        mut tag_transforms: Query<&mut Transform, Without<CardTagOwner>>,
    ) {
        let (camera, camera_transform) = *camera;

        for (owner, card_transform) in &query {
            let Ok(text_pos) = calc_text_pos(camera, camera_transform, card_transform.translation)
            else {
                return;
            };

            tag_transforms
                .get_mut(owner.tag_entity)
                .unwrap()
                .translation = text_pos;
        }
    }
}

#[derive(Component)]
#[require(Text2d)]
struct CardTag {
    owner: Entity,
}

impl CardTag {
    fn init(
        trigger: Trigger<OnAdd, Self>,
        card_tag: Query<&CardTag>,
        camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
        mut transform: Query<&mut Transform>,
    ) {
        let tag_entity = trigger.entity();
        let owner_entity = card_tag.get(tag_entity).unwrap().owner;

        // World to viewport
        let (camera, camera_transform) = *camera;
        let Ok(text_pos) = calc_text_pos(
            camera,
            camera_transform,
            transform.get(owner_entity).unwrap().translation,
        ) else {
            return;
        };

        transform.get_mut(tag_entity).unwrap().translation = text_pos;
    }
}

fn calc_text_pos(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    at: Vec3,
) -> anyhow::Result<Vec3> {
    let vpsize = camera
        .logical_viewport_size()
        .context("failed to get viewport size")?;

    let pos = camera
        .world_to_viewport(
            camera_transform,
            at + Vec3::new(0.0, 0.0, -CARD_HEIGHT / 2.0),
        )
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let x = pos.x - vpsize.x / 2.0;
    let y = vpsize.y / 2.0 - pos.y + FONT_SIZE * 0.75;

    Ok(Vec3::new(x, y, 0.0))
}

fn on_camera_movement(
    mut params: ParamSet<(
        Option<Single<Entity, (With<Camera3d>, Changed<Transform>)>>,
        Query<&mut Transform, With<CardTagOwner>>,
    )>,
) {
    if params.p0().is_none() {
        return;
    }

    for mut owner in &mut params.p1() {
        // Perform `DerefMut` to trigger the `Changed<Transform>` query filter.
        let _: &mut Transform = &mut owner;
    }
}
