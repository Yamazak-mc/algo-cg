use crate::game::{CARD_DEPTH, CARD_WIDTH};
use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    prelude::*,
};
use std::f32::consts::PI;

pub fn card_flip_plugin(app: &mut App) {
    app.add_systems(Startup, setup_animation)
        .add_observer(on_start_animation)
        .add_systems(Update, adjust_y)
        .add_observer(on_finish_animation);
}

#[derive(Resource)]
pub struct CardFlipAnimation {
    pub animation_target: AnimationTarget,
    pub node_idx: AnimationNodeIndex,
}

#[derive(Component)]
struct CardFlipAnimationTarget;

#[derive(Clone, Event)]
struct CardFlipAnimationStarted;

#[derive(Clone, Event)]
struct CardFlipAnimationFinished;

fn setup_animation(
    mut commands: Commands,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let duration = 0.5;

    let animation_target_id = AnimationTargetId::from_name(&"CardFlip".into());

    let mut clip = AnimationClip::default();
    clip.add_event_to_target(animation_target_id, 0.0, CardFlipAnimationStarted);
    clip.add_curve_to_target(
        animation_target_id,
        AnimatableCurve::new(
            animated_field!(Transform::rotation),
            EasingCurve::new(
                Quat::from_rotation_z(-PI),
                Quat::from_rotation_z(0.0),
                EaseFunction::CubicInOut,
            )
            .reparametrize_linear(Interval::new(0.0, duration).unwrap())
            .unwrap(),
        ),
    );
    clip.add_event_to_target(animation_target_id, duration, CardFlipAnimationFinished);

    let (graph, node_idx) = AnimationGraph::from_clip(clips.add(clip));
    let graph_handle = graphs.add(graph);

    let player = commands
        .spawn((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph_handle),
        ))
        .id();

    commands.insert_resource(CardFlipAnimation {
        animation_target: AnimationTarget {
            id: animation_target_id,
            player,
        },
        node_idx,
    });
}

fn on_start_animation(trigger: Trigger<CardFlipAnimationStarted>, mut commands: Commands) {
    commands
        .entity(trigger.entity())
        .insert(CardFlipAnimationTarget);
}

fn adjust_y(mut query: Query<&mut Transform, With<CardFlipAnimationTarget>>) {
    let max_y = ((CARD_WIDTH.powi(2) + CARD_DEPTH.powi(2)).sqrt() - CARD_DEPTH) / 2.0;

    for mut transform in &mut query {
        let z = transform.rotation.to_euler(EulerRot::XYZ).2;
        transform.translation.y = (PI - z).sin() * max_y;
    }
}

fn on_finish_animation(
    trigger: Trigger<CardFlipAnimationFinished>,
    mut commands: Commands,
    mut transform: Query<&mut Transform>,
) {
    let entity = trigger.entity();
    commands
        .entity(entity)
        .remove::<(AnimationTarget, CardFlipAnimationTarget)>();

    transform.get_mut(entity).unwrap().translation.y = 0.0;
}
