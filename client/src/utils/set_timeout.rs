use super::AddObserverExt as _;
use bevy::{ecs::observer::TriggerTargets, prelude::*, state::state::FreelyMutableState};

pub struct SetTimeoutPlugin<S> {
    pub ctx_state: S,
}

impl<S: States + Clone> Plugin for SetTimeoutPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<EventsOnTimedout>()
            .add_state_scoped_observer_named(self.ctx_state.clone(), SetTimeout::handle_trigger)
            .add_state_scoped_observer_named(
                self.ctx_state.clone(),
                NotifyTimedout::handle_trigger,
            );
    }
}

#[derive(Event)]
pub struct SetTimeout {
    duration_secs: f32,
    on_timedout: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync + 'static>>,
}

impl SetTimeout {
    /// Creates a new `SetTimeout` event.
    ///
    /// The caller must add a callback function to the return value for it to be valid.
    pub fn new(duration_secs: f32) -> Self {
        Self {
            duration_secs,
            on_timedout: None,
        }
    }

    pub fn with_fn(self, on_timedout: impl FnOnce(&mut Commands) + Send + Sync + 'static) -> Self {
        Self {
            on_timedout: Some(Box::new(on_timedout)),
            ..self
        }
    }

    pub fn with_trigger<E: Event>(self, event: E) -> Self {
        self.with_fn(|commands: &mut Commands| {
            commands.trigger(event);
        })
    }

    pub fn with_trigger_targets<E, T>(self, event: E, targets: T) -> Self
    where
        E: Event,
        T: TriggerTargets + Send + Sync + 'static,
    {
        self.with_fn(|commands: &mut Commands| {
            commands.trigger_targets(event, targets);
        })
    }

    pub fn with_state<S: FreelyMutableState>(self, state: S) -> Self {
        self.with_fn(|commands: &mut Commands| {
            commands.set_state(state);
        })
    }

    fn handle_trigger(
        mut trigger: Trigger<Self>,
        mut clips: ResMut<Assets<AnimationClip>>,
        mut graphs: ResMut<Assets<AnimationGraph>>,
        mut storage: ResMut<EventsOnTimedout>,
        mut commands: Commands,
    ) {
        let event = trigger.event_mut();
        let Some(on_timedout) = event.on_timedout.take() else {
            warn!("`SetTimeout` was triggered without a callback, so it does nothing");
            return;
        };

        if event.duration_secs <= 0.0 {
            (on_timedout)(&mut commands);
            return;
        }

        let entry = storage.vacant_entry();

        let animator_entity = {
            let mut clip = AnimationClip::default();
            clip.add_event(event.duration_secs, NotifyTimedout(entry.key()));

            let (graph, node_idx) = AnimationGraph::from_clip(clips.add(clip));

            let mut animation_player = AnimationPlayer::default();
            animation_player.play(node_idx);

            commands
                .spawn((animation_player, AnimationGraphHandle(graphs.add(graph))))
                .id()
        };

        entry.insert((animator_entity, on_timedout));
    }
}

#[derive(Deref, DerefMut, Default, Resource)]
struct EventsOnTimedout(
    slab::Slab<(
        Entity,
        Box<dyn FnOnce(&mut Commands) + Send + Sync + 'static>,
    )>,
);

#[derive(Clone, Event)]
struct NotifyTimedout(usize);

impl NotifyTimedout {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut storage: ResMut<EventsOnTimedout>,
        mut commands: Commands,
    ) {
        let (id, on_timedout) = storage.try_remove(trigger.event().0).unwrap();

        commands.entity(id).despawn();
        on_timedout(&mut commands);
    }
}
