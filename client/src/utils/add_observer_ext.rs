use bevy::{ecs::system::IntoObserverSystem, prelude::*};

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
