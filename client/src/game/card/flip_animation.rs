use crate::game::{CARD_DEPTH, CARD_WIDTH, CTX_STATE};
use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    prelude::*,
};
use client::utils::AddObserverExt as _;
use std::f32::consts::PI;

const FLIP_ANIMATION_DURATION_SECS: f32 = 0.5;

pub fn card_flip_plugin(app: &mut App) {
    app.add_systems(Startup, setup_animation)
        .add_state_scoped_observer(CTX_STATE, FlipCard::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, CardFlipStarted::handle_trigger)
        .add_state_scoped_observer(CTX_STATE, CardFlipFinished::handle_trigger)
        .add_systems(Update, CardFlipTarget::adjust_y.run_if(in_state(CTX_STATE)));
}

#[derive(Resource)]
struct CardFlipAnimation {
    animation_target: AnimationTarget,
    node_idx: AnimationNodeIndex,
}

fn setup_animation(
    mut commands: Commands,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let duration = FLIP_ANIMATION_DURATION_SECS;

    let animation_target_id = AnimationTargetId::from_name(&"CardFlip".into());

    let mut clip = AnimationClip::default();
    clip.add_event_to_target(animation_target_id, 0.0, CardFlipStarted);
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
    clip.add_event_to_target(animation_target_id, duration, CardFlipFinished);

    let (graph, node_idx) = AnimationGraph::from_clip(clips.add(clip));
    let graph_handle = graphs.add(graph);

    let player = commands
        .spawn((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph_handle),
            Name::new("CardFlipAnimationPlayer"),
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

#[derive(Event)]
pub struct FlipCard;

impl FlipCard {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        children: Query<&Children>,
        animation: Res<CardFlipAnimation>,
        mut animation_player: Query<&mut AnimationPlayer>,
    ) {
        let card_entity = trigger.entity();
        let child_entity = children.get(card_entity).unwrap()[0];

        commands
            .entity(child_entity)
            .insert(animation.animation_target);
        animation_player
            .get_mut(animation.animation_target.player)
            .unwrap()
            .start(animation.node_idx);
    }
}

#[derive(Clone, Event)]
struct CardFlipStarted;

impl CardFlipStarted {
    fn handle_trigger(trigger: Trigger<Self>, mut commands: Commands) {
        commands.entity(trigger.entity()).insert(CardFlipTarget);
    }
}

#[derive(Clone, Event)]
struct CardFlipFinished;

impl CardFlipFinished {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        mut transform: Query<&mut Transform>,
    ) {
        let entity = trigger.entity();
        commands
            .entity(entity)
            .remove::<(AnimationTarget, CardFlipTarget)>();

        transform.get_mut(entity).unwrap().translation.y = 0.0;
    }
}

#[derive(Component)]
struct CardFlipTarget;

impl CardFlipTarget {
    fn adjust_y(mut query: Query<&mut Transform, With<Self>>) {
        let max_y = ((CARD_WIDTH.powi(2) + CARD_DEPTH.powi(2)).sqrt() - CARD_DEPTH) / 2.0;

        for mut transform in &mut query {
            let z = transform.rotation.to_euler(EulerRot::XYZ).2;
            transform.translation.y = (PI - z).sin() * max_y;
        }
    }
}
