use bevy::prelude::*;
use std::marker::PhantomData;

use super::AddObserverExt as _;

macro_rules! impl_marker_newtype {
    ($ty:ident $(,)?) => {
        impl<E, B> $ty<E, B>
        where
            E: Event,
            B: Bundle,
        {
            pub fn new() -> Self {
                Self(PhantomData)
            }
        }

        impl<E, B> Default for $ty<E, B>
        where
            E: Event,
            B: Bundle,
        {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub struct ObserverControllerSettings {
    pub removable: bool,
    pub pausable: bool,
    pub once: bool,
}

impl ObserverControllerSettings {
    pub fn once() -> Self {
        Self {
            removable: false,
            pausable: false,
            once: true,
        }
    }
}

impl Default for ObserverControllerSettings {
    fn default() -> Self {
        Self {
            removable: true,
            pausable: true,
            once: true,
        }
    }
}

pub struct ObserverControllerPlugin<E, B = ()> {
    settings: ObserverControllerSettings,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> Default for ObserverControllerPlugin<E, B> {
    fn default() -> Self {
        Self::new(ObserverControllerSettings::default())
    }
}

impl<E, B> ObserverControllerPlugin<E, B> {
    pub fn new(settings: ObserverControllerSettings) -> Self {
        Self {
            settings,
            _marker: PhantomData,
        }
    }

    pub fn state_scoped<S: States>(self, state: S) -> ObserverControllerPluginStateScoped<E, B, S> {
        ObserverControllerPluginStateScoped {
            plugin: self,
            state,
        }
    }
}

impl<E, B> Plugin for ObserverControllerPlugin<E, B>
where
    E: Event,
    B: Bundle,
{
    fn build(&self, app: &mut App) {
        let ObserverControllerSettings {
            removable,
            pausable,
            once,
        } = self.settings;

        if removable {
            ObserverController::<E, B>::minimal_plugin(app);
            if pausable {
                ObserverController::<E, B>::pausable_plugin(app);
            }
        }
        if once {
            ObserveOnce::<E, B>::plugin(app);
        }
    }
}

pub struct ObserverControllerPluginStateScoped<E, B, S>
where
    S: States,
{
    plugin: ObserverControllerPlugin<E, B>,
    state: S,
}

impl<E, B, S> Plugin for ObserverControllerPluginStateScoped<E, B, S>
where
    E: Event,
    B: Bundle,
    S: States + Clone,
{
    fn build(&self, app: &mut App) {
        let ObserverControllerSettings {
            removable,
            pausable,
            once,
        } = self.plugin.settings;
        let state = &self.state;

        if removable {
            ObserverController::<E, B>::minimal_plugin_state_scoped(app, state.clone());
            if pausable {
                ObserverController::<E, B>::pausable_plugin_state_scoped(app, state.clone());
            }
        }
        if once {
            ObserveOnce::<E, B>::plugin_state_scoped(app, state.clone());
        }
    }
}

#[derive(Component)]
struct ObserverController<E, B> {
    observer_fn: Box<dyn Fn() -> Observer + Send + Sync + 'static>,
    observer_entity: Option<Entity>,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> ObserverController<E, B>
where
    E: Event,
    B: Bundle,
{
    fn minimal_plugin(app: &mut App) {
        app.add_observer(Self::insert).add_observer(Self::remove);
    }

    fn pausable_plugin(app: &mut App) {
        app.add_observer(Self::activate).add_observer(Self::pause);
    }

    fn minimal_plugin_state_scoped<S: States + Clone>(app: &mut App, state: S) {
        app.add_state_scoped_observer(state.clone(), Self::insert)
            .add_state_scoped_observer(state, Self::remove);
    }

    fn pausable_plugin_state_scoped<S: States + Clone>(app: &mut App, state: S) {
        app.add_state_scoped_observer(state.clone(), Self::activate)
            .add_state_scoped_observer(state, Self::remove);
    }

    fn insert(mut trigger: Trigger<Insert<E, B>>, mut commands: Commands) {
        let entity = trigger.entity();
        let event = trigger.event_mut();
        let observer_fn = event.observer_fn.take().unwrap();
        let state = event.state;

        let observer_entity = match state {
            ObsConState::Active => Some(commands.spawn(observer_fn().with_entity(entity)).id()),
            ObsConState::Paused => None,
        };

        commands.entity(entity).insert(Self {
            observer_fn,
            observer_entity,
            _marker: PhantomData,
        });
    }

    fn remove(trigger: Trigger<Remove<E, B>>, mut query: Query<&mut Self>, mut commands: Commands) {
        let entity = trigger.entity();

        if let Some(observer_entity) = query.get_mut(entity).unwrap().observer_entity.take() {
            debug!(
                "removing observer: observer={}, target={}",
                observer_entity, entity
            );
            commands.entity(observer_entity).despawn();
        }

        commands.entity(entity).remove::<Self>();
    }

    fn activate(
        trigger: Trigger<Activate<E, B>>,
        mut query: Query<&mut Self>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let mut this = query.get_mut(entity).unwrap();

        if this.observer_entity.is_none() {
            let observer_entity = commands
                .spawn((this.observer_fn)().with_entity(entity))
                .id();
            debug!(
                "activating observer: observer={}, target={}",
                observer_entity, entity
            );
            this.observer_entity = Some(observer_entity);
        }
    }

    fn pause(trigger: Trigger<Pause<E, B>>, mut query: Query<&mut Self>, mut commands: Commands) {
        let entity = trigger.entity();
        let mut this = query.get_mut(entity).unwrap();

        if let Some(observer_entity) = this.observer_entity.take() {
            debug!(
                "pausing observer: observer={}, target={}",
                observer_entity, entity
            );
            commands.entity(observer_entity).despawn();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObsConState {
    Paused,
    Active,
}

#[derive(Event)]
pub struct Insert<E, B = ()> {
    observer_fn: Option<Box<dyn Fn() -> Observer + Send + Sync + 'static>>,
    state: ObsConState,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> Insert<E, B>
where
    E: Event,
    B: Bundle,
{
    pub fn new_active<F>(observer_fn: F) -> Self
    where
        F: Fn() -> Observer + Send + Sync + 'static,
    {
        Self::new(observer_fn, ObsConState::Active)
    }

    pub fn new_paused<F>(observer_fn: F) -> Self
    where
        F: Fn() -> Observer + Send + Sync + 'static,
    {
        Self::new(observer_fn, ObsConState::Paused)
    }

    fn new<F>(observer_fn: F, state: ObsConState) -> Self
    where
        F: Fn() -> Observer + Send + Sync + 'static,
    {
        Self {
            observer_fn: Some(Box::new(observer_fn)),
            state,
            _marker: PhantomData,
        }
    }
}

#[derive(Event)]
pub struct Remove<E, B = ()>(PhantomData<fn(&E, &B)>);

impl_marker_newtype! { Remove }

#[derive(Event)]
pub struct Activate<E, B = ()>(PhantomData<fn(&E, &B)>);

impl_marker_newtype! { Activate }

#[derive(Event)]
pub struct Pause<E, B = ()>(PhantomData<fn(&E, &B)>);

impl_marker_newtype! { Pause }

#[derive(Event)]
pub struct ObserveOnce<E, B = ()> {
    observer: Option<Observer>,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> ObserveOnce<E, B>
where
    E: Event,
    B: Bundle,
{
    pub fn new(observer: Observer) -> Self {
        Self {
            observer: Some(observer),
            _marker: PhantomData,
        }
    }

    fn plugin(app: &mut App) {
        app.add_observer(Self::handle_trigger)
            .add_observer(Self::cleanup);
    }

    fn plugin_state_scoped<S: States + Clone>(app: &mut App, state: S) {
        app.add_state_scoped_observer(state.clone(), Self::handle_trigger)
            .add_state_scoped_observer(state, Self::cleanup);
    }

    fn handle_trigger(mut trigger: Trigger<Self>, mut commands: Commands) {
        let entity = trigger.entity();
        let observer = trigger.event_mut().observer.take().unwrap();

        let observer_entity = commands.spawn(observer.with_entity(entity)).id();
        commands
            .entity(entity)
            .insert(ObservedByOnce::<E, B>::new(observer_entity));
    }

    fn cleanup(
        trigger: Trigger<E, B>,
        query: Query<&ObservedByOnce<E, B>>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();

        if let Ok(observed) = query.get(entity) {
            commands.entity(observed.observer_entity).despawn();
            commands.entity(entity).remove::<ObservedByOnce<E, B>>();
        }
    }
}

#[derive(Component)]
struct ObservedByOnce<E, B> {
    observer_entity: Entity,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> ObservedByOnce<E, B> {
    fn new(observer_entity: Entity) -> Self {
        Self {
            observer_entity,
            _marker: PhantomData,
        }
    }
}
