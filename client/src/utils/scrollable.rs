use bevy::{input::mouse::MouseWheel, picking::focus::HoverMap, prelude::*};

// const LINE_HEIGHT: f32 = 20.0;

pub fn scrollable_plugin(app: &mut App) {
    app.add_systems(Update, update_scroll_position);
}

#[derive(Component)]
pub struct Scrollable;

// Ref: https://bevyengine.org/examples/ui-user-interface/ui/
fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrollable_query: Query<Entity, With<Scrollable>>,
    mut commands: Commands,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                if scrollable_query.get_mut(*entity).is_ok() {
                    commands
                        .entity(*entity)
                        .trigger(ScrollEvent(*mouse_wheel_event));
                }
            }
        }
    }
}

#[derive(Event)]
pub struct ScrollEvent(pub MouseWheel);
