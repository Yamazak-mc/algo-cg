use super::AppState;
use bevy::prelude::*;

enum LobbyState {}

pub fn lobby_plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Lobby), setup_lobby);
}

#[derive(Component)]
struct LobbyWidget;

fn setup_lobby(mut commands: Commands) {
    commands.spawn((
        LobbyWidget,
        StateScoped(AppState::Lobby),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_items: JustifyItems::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(30.0),
            ..default()
        },
        Interaction::default(),
    ));
}
