use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::focus::HoverMap,
    prelude::*,
};

pub fn scrollable_plugin(app: &mut App) {
    app.add_systems(Update, update_scroll_position).add_systems(
        PostUpdate,
        scroll_to_end.after(TransformSystem::TransformPropagate),
    );
}

#[derive(Component)]
pub struct Scrollable;

#[derive(Deref, DerefMut, Debug, Clone, Copy, Component)]
pub struct ScrollLineHeight(pub f32);

// Ref: https://bevyengine.org/examples/ui-user-interface/ui/
fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrollable_query: Query<(
        Option<&ScrollLineHeight>,
        Option<&mut ScrollPosition>,
        Has<Scrollable>,
    )>,
    mut commands: Commands,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                let (height, position, scrollable) = scrollable_query.get_mut(*entity).unwrap();

                if !scrollable {
                    continue;
                }

                commands
                    .entity(*entity)
                    .trigger(ScrollEvent(*mouse_wheel_event));

                let Some(mut position) = position else {
                    continue;
                };
                let Some(height) = height else {
                    continue;
                };

                let (dx, dy) = {
                    let (dx, dy) = (mouse_wheel_event.x, mouse_wheel_event.y);
                    match mouse_wheel_event.unit {
                        MouseScrollUnit::Line => (dx * height.0, dy * height.0),
                        MouseScrollUnit::Pixel => (dx, dy),
                    }
                };

                position.offset_x -= dx;
                position.offset_y -= dy;
            }
        }
    }
}

#[derive(Event)]
pub struct ScrollEvent(pub MouseWheel);

#[derive(Component)]
pub struct ScrollToEnd;

fn scroll_to_end(
    mut scrollable_query: Query<
        (Entity, &ComputedNode, &Children, &mut ScrollPosition),
        (With<Scrollable>, With<ScrollToEnd>),
    >,
    node_query: Query<&ComputedNode>,
    mut commands: Commands,
) {
    for (entity, node, children, mut position) in &mut scrollable_query {
        let node_height = node.outlined_node_size().y;
        let elem_height = children
            .iter()
            .flat_map(|e| node_query.get(*e).map(ComputedNode::outlined_node_size))
            .fold(0.0, |acc, val| acc + val.y);
        position.offset_y = elem_height - node_height;

        commands.entity(entity).remove::<ScrollToEnd>();
    }
}
