use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub trait AddObserverExt {
    fn add_state_scoped_observer<S, E, B, M, I>(&mut self, state: S, observer: I) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone;
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
}
