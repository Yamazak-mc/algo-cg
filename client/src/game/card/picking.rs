use crate::game::CTX_STATE;
use bevy::prelude::*;
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use client::utils::add_observer_ext::AddStateScopedObserver as _;

pub fn card_picking_plugin(app: &mut App) {
    app.add_plugins((MeshPickingPlugin, OutlinePlugin))
        .add_state_scoped_observer(CTX_STATE, PickableCard::init)
        .add_state_scoped_observer(CTX_STATE, PickableCard::cleanup);
}

#[derive(Component)]
#[require(RayCastPickable, OutlineMode(|| OutlineMode::FloodFlat), PickableCardSettings)]
pub struct PickableCard;

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

impl PickableCard {
    fn init(trigger: Trigger<OnAdd, Self>, mut commands: Commands) {
        commands
            .entity(trigger.entity())
            .observe(Self::pointer_over)
            .observe(Self::pointer_out);
    }

    fn cleanup(trigger: Trigger<OnRemove, Self>, mut commands: Commands) {
        commands.entity(trigger.entity()).remove::<(
            RayCastPickable,
            PickableCardSettings,
            OutlineMode,
            OutlineVolume,
        )>();
    }

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
