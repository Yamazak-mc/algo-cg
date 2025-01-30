use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Default)]
pub struct ObserveOncePlugin<E, B = ()>(PhantomData<fn(&E, &B)>);

impl<E, B> ObserveOncePlugin<E, B> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E, B> Plugin for ObserveOncePlugin<E, B>
where
    E: Event + 'static,
    B: Bundle,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<ObserveOnceObservers>()
            .add_observer(ObserveOnce::<E, B>::handle_trigger);
    }
}

#[derive(Event)]
pub struct ObserveOnce<E, B = ()> {
    observer: Option<Observer>,
    _marker: PhantomData<fn(&E, &B)>,
}

impl<E, B> ObserveOnce<E, B>
where
    E: Event + 'static,
    B: Bundle,
{
    pub fn new(observer: Observer) -> Self {
        Self {
            observer: Some(observer),
            _marker: PhantomData,
        }
    }

    fn handle_trigger(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        mut observers: ResMut<ObserveOnceObservers>,
    ) {
        let entity = trigger.entity();

        // Main observer
        let mut observer_1 = trigger.event_mut().observer.take().unwrap();
        observer_1.watch_entity(entity);
        let obs_id_1 = commands.spawn(observer_1).id();

        // Observer for cleanup
        let entry = observers.vacant_entry();
        let obs_id_2 = commands
            .spawn(Self::cleanup_observer(entry.key()).with_entity(entity))
            .id();

        entry.insert([obs_id_1, obs_id_2]);
    }

    fn cleanup_observer(key: usize) -> Observer {
        Observer::new(
            move |_trigger: Trigger<E, B>,
                  mut commands: Commands,
                  mut observers: ResMut<ObserveOnceObservers>| {
                for id in observers.remove(key) {
                    commands.entity(id).despawn();
                }
            },
        )
    }
}

#[derive(Default, Deref, DerefMut, Resource)]
struct ObserveOnceObservers(slab::Slab<[Entity; 2]>);
