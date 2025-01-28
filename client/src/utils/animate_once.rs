use super::add_observer_ext::AddStateScopedObserver as _;
use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    prelude::*,
};

pub struct AnimateOncePlugin<T> {
    pub target_state: Option<T>,
}

impl<T> AnimateOncePlugin<T> {
    pub fn from_state(target_state: T) -> Self {
        Self {
            target_state: Some(target_state),
        }
    }
}

impl<T: States + Clone> Plugin for AnimateOncePlugin<T> {
    fn build(&self, app: &mut App) {
        if let Some(ref state) = self.target_state {
            app.add_state_scoped_observer(state.clone(), AnimateOnce::handle_trigger)
                .add_state_scoped_observer(state.clone(), AnimateOnce::cleanup);
        } else {
            app.add_observer(AnimateOnce::handle_trigger)
                .add_observer(AnimateOnce::cleanup);
        }
    }
}

#[derive(Event)]
pub struct AnimateOnce(
    pub Box<dyn Fn(&mut AnimationClip, AnimationTargetId) + Send + Sync + 'static>,
);

#[derive(Clone, Event)]
struct AnimateOnceFinished;

impl AnimateOnce {
    pub fn translation(start: Vec3, end: Vec3, duration_secs: f32, ease_fn: EaseFunction) -> Self {
        Self(Box::new(move |clip, id| {
            let translation_curve = EasingCurve::new(start, end, ease_fn)
                .reparametrize_linear(Interval::new(0.0, duration_secs).unwrap())
                .unwrap();

            clip.add_curve_to_target(
                id,
                AnimatableCurve::new(animated_field!(Transform::translation), translation_curve),
            );
        }))
    }

    #[allow(unused)]
    pub fn rotation(start: Quat, end: Quat, duration_secs: f32, ease_fn: EaseFunction) -> Self {
        Self(Box::new(move |clip, id| {
            let rotation_curve = EasingCurve::new(start, end, ease_fn)
                .reparametrize_linear(Interval::new(0.0, duration_secs).unwrap())
                .unwrap();

            clip.add_curve_to_target(
                id,
                AnimatableCurve::new(animated_field!(Transform::rotation), rotation_curve),
            );
        }))
    }

    pub fn translation_and_rotation(
        start: Transform,
        end: Transform,
        duration_secs: f32,
        ease_fn: EaseFunction,
    ) -> Self {
        Self(Box::new(move |clip, id| {
            let translation_curve = EasingCurve::new(start.translation, end.translation, ease_fn)
                .reparametrize_linear(Interval::new(0.0, duration_secs).unwrap())
                .unwrap();
            let rotation_curve = EasingCurve::new(start.rotation, end.rotation, ease_fn)
                .reparametrize_linear(Interval::new(0.0, duration_secs).unwrap())
                .unwrap();

            clip.add_curve_to_target(
                id,
                AnimatableCurve::new(animated_field!(Transform::translation), translation_curve),
            );
            clip.add_curve_to_target(
                id,
                AnimatableCurve::new(animated_field!(Transform::rotation), rotation_curve),
            );
        }))
    }

    fn handle_trigger(
        trigger: Trigger<Self>,
        mut commands: Commands,
        mut clips: ResMut<Assets<AnimationClip>>,
        mut graphs: ResMut<Assets<AnimationGraph>>,
    ) {
        // AnimationTargetId
        let entity = trigger.entity();
        let animation_target_id = AnimationTargetId::from_name(&entity.to_string().into());

        // AnimationClip
        let mut clip = AnimationClip::default();
        (trigger.0)(&mut clip, animation_target_id);
        clip.add_event_to_target(animation_target_id, clip.duration(), AnimateOnceFinished);

        let clip_handle = clips.add(clip);

        // AnimationGraph
        let (graph, node_idx) = AnimationGraph::from_clip(clip_handle);
        let graph_handle = graphs.add(graph);

        // AnimationPlayer
        let mut animation_player = AnimationPlayer::default();
        animation_player.play(node_idx);

        commands.entity(entity).insert((
            AnimationTarget {
                id: animation_target_id,
                player: entity,
            },
            AnimationGraphHandle(graph_handle),
            animation_player,
        ));
    }

    /// This cleanup ensures that the animation system no longer accesses `&mut Transform`
    /// after the animation has completed.
    fn cleanup(trigger: Trigger<AnimateOnceFinished>, mut commands: Commands) {
        commands
            .entity(trigger.entity())
            .remove::<(AnimationTarget, AnimationGraphHandle, AnimationPlayer)>();
    }
}
