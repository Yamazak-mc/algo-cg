use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use std::borrow::Cow;

pub struct AddObserverExtPlugin;

impl Plugin for AddObserverExtPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ObserverSysName>();
    }
}

#[derive(Clone, Component, Reflect)]
struct ObserverSysName(Cow<'static, str>);

fn name_components<T>() -> impl Bundle + Clone {
    let type_name = std::any::type_name::<T>();
    let entity_name = type_name.split("::").last().unwrap();
    (
        ObserverSysName(Cow::Borrowed(type_name)),
        Name::new(format!("Observer({})", entity_name)),
    )
}

pub trait AddObserverExt {
    fn add_state_scoped_observer<S, E, B, M, I>(&mut self, state: S, observer: I) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone;

    fn add_state_scoped_observer_with<S, E, B, M, I, W>(
        &mut self,
        state: S,
        observer: I,
        with: W,
    ) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone,
        W: Bundle + Clone;

    fn add_state_scoped_observer_named<S, E, B, M, I>(&mut self, state: S, observer: I) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone,
    {
        self.add_state_scoped_observer_with(state, observer, name_components::<I>())
    }
}

impl AddObserverExt for App {
    fn add_state_scoped_observer<S, E, B, M, I>(&mut self, state: S, observer: I) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone,
    {
        self.add_systems(OnEnter(state.clone()), move |mut commands: Commands| {
            commands.spawn((StateScoped(state.clone()), Observer::new(observer.clone())));
        })
    }

    fn add_state_scoped_observer_with<S, E, B, M, I, W>(
        &mut self,
        state: S,
        observer: I,
        with: W,
    ) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone,
        W: Bundle + Clone,
    {
        self.add_systems(OnEnter(state.clone()), move |mut commands: Commands| {
            commands.spawn((
                StateScoped(state.clone()),
                Observer::new(observer.clone()),
                with.clone(),
            ));
        })
    }
}
