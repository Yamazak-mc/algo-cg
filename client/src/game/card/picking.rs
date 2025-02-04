use crate::game::CTX_STATE;
use bevy::prelude::*;
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use client::utils::AddObserverExt as _;

pub fn card_picking_plugin(app: &mut App) {
    app.add_plugins((MeshPickingPlugin, OutlinePlugin))
        .add_state_scoped_observer(CTX_STATE, PickableCard::init)
        .add_state_scoped_observer(CTX_STATE, PickableCard::cleanup);
}

#[derive(Component)]
struct PickingObservers {
    over: Entity,
    out: Entity,
}

#[derive(Component)]
pub struct PickableCard;

impl PickableCard {
    fn init(trigger: Trigger<OnAdd, Self>, mut commands: Commands, children: Query<&Children>) {
        let child = children.get(trigger.entity()).unwrap()[0];

        let over = commands
            .spawn(Observer::new(PickableCard__::pointer_over).with_entity(child))
            .id();
        let out = commands
            .spawn(Observer::new(PickableCard__::pointer_out).with_entity(child))
            .id();

        commands
            .entity(child)
            .insert((PickableCard__, PickingObservers { over, out }));
    }

    fn cleanup(
        trigger: Trigger<OnRemove, Self>,
        mut commands: Commands,
        children: Query<&Children>,
        observers_query: Query<&PickingObservers>,
    ) {
        let child = children.get(trigger.entity()).unwrap()[0];

        if let Ok(observers) = observers_query.get(child) {
            commands.entity(observers.over).despawn();
            commands.entity(observers.out).despawn();

            commands.entity(child).remove::<(
                PickableCard__,
                RayCastPickable,
                PickableCardSettings,
                OutlineMode,
                OutlineVolume,
                PickingObservers,
            )>();
        }
    }
}

#[derive(Component)]
#[require(RayCastPickable, OutlineMode(|| OutlineMode::FloodFlat), PickableCardSettings)]
struct PickableCard__;

#[derive(Clone, Component)]
pub struct PickableCardSettings {
    pub outline_vol: OutlineVolume,
}

impl Default for PickableCardSettings {
    fn default() -> Self {
        let outline_vol = OutlineVolume {
            visible: true,
            colour: Color::srgb(1.0, 1.0, 0.0),
            width: 4.0,
        };

        Self { outline_vol }
    }
}

impl PickableCard__ {
    fn pointer_over(
        trigger: Trigger<Pointer<Over>>,
        mut volume: Query<&mut OutlineVolume>,
        settings: Query<&PickableCardSettings>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let Ok(settings) = settings.get(entity) else {
            return;
        };

        if let Ok(mut vol) = volume.get_mut(entity) {
            // Setting the color back.
            vol.colour = settings.outline_vol.colour;
        } else {
            let outline_vol = settings.outline_vol.clone();
            commands.entity(entity).insert(outline_vol);
        }
    }

    fn pointer_out(trigger: Trigger<Pointer<Out>>, mut volume: Query<&mut OutlineVolume>) {
        let entity = trigger.entity();
        let volume: &mut Query<&mut OutlineVolume> = &mut volume;
        if let Ok(mut vol) = volume.get_mut(entity) {
            // HACK: Setting `OutlineVolume::visible` doesn't seem to work.
            //       For now, we set alpha to 0 instead.
            vol.colour.set_alpha(0.0);
        }
    }
}
