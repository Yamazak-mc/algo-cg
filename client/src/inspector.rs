use bevy::{input::common_conditions::input_just_pressed, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum InspectorVis {
    #[cfg_attr(debug_assertions, default)]
    Visible,
    #[cfg_attr(not(debug_assertions), default)]
    Hidden,
}

impl InspectorVis {
    fn toggle(&self) -> Self {
        match self {
            Self::Visible => Self::Hidden,
            Self::Hidden => Self::Visible,
        }
    }
}

pub fn inspector_plugin(app: &mut App) {
    app.init_state::<InspectorVis>()
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                .run_if(in_state(InspectorVis::Visible)),
        )
        .add_systems(
            Update,
            toggle_inspector.run_if(input_just_pressed(KeyCode::KeyI)),
        );
}

fn toggle_inspector(
    state: Res<State<InspectorVis>>,
    mut next_state: ResMut<NextState<InspectorVis>>,
) {
    next_state.set(state.toggle());
}
