use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub trait IntoColor {
    fn into_color(self) -> Color;
}

impl IntoColor for [u8; 3] {
    fn into_color(self) -> Color {
        let [r, g, b] = self;

        Color::srgb_u8(r, g, b)
    }
}

impl IntoColor for [u8; 4] {
    fn into_color(self) -> Color {
        let [r, g, b, a] = self;

        Color::srgba_u8(r, g, b, a)
    }
}

impl IntoColor for Color {
    fn into_color(self) -> Color {
        self
    }
}

pub trait AddStateScopedObserver {
    fn add_state_scoped_observer<S, E, B, M, I>(&mut self, state: S, observer: I) -> &mut Self
    where
        S: States,
        E: Event,
        B: Bundle,
        I: IntoObserverSystem<E, B, M> + Sync + Clone;
}

impl AddStateScopedObserver for App {
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
