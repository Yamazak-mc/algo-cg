use super::P2_CTX_STATE;
use bevy::{prelude::*, ui::experimental::GhostNode};
use client::utils::{
    component_based::interaction_based,
    log_display::{LogDisplay, LogDisplaySettings},
    scrollable::{ScrollLineHeight, Scrollable},
};
use std::borrow::Cow;

mod image_handles;
use image_handles::ImageHandles;

pub mod popup;
use popup::PopupUiAnchor;

pub mod history;
use history::{CardSnapshot, HistoryUiAnchor};

const ICON_NOTES: &str = "tabler-icons/notes.png";
const ICON_NOTES_OFF: &str = "tabler-icons/notes-off.png";
const ICON_HISTORY: &str = "tabler-icons/history.png";
const ICON_HISTORY_OFF: &str = "tabler-icons/history-off.png";
const ICON_HELP: &str = "tabler-icons/help.png";
const ICON_ARROW_LEFT: &str = "tabler-icons/arrow-bar-to-left.png";
const ICON_ARROW_RIGHT: &str = "tabler-icons/arrow-bar-to-right.png";

pub fn ui_plugin(app: &mut App) {
    app.add_plugins((popup::popup_plugin, history::history_plugin))
        .insert_resource(ImageHandles::new([
            ICON_NOTES,
            ICON_NOTES_OFF,
            ICON_HISTORY,
            ICON_HISTORY_OFF,
            ICON_HELP,
            ICON_ARROW_LEFT,
            ICON_ARROW_RIGHT,
        ]))
        .add_systems(OnEnter(P2_CTX_STATE), setup);
}

#[derive(Default, Component)]
struct ToggleLogButton {
    enabled: bool,
}

#[derive(Default, Component)]
struct ToggleHistoryButton {
    enabled: bool,
}

#[derive(Default, Component)]
enum HistoryUiSide {
    Left,
    #[default]
    Right,
}

#[derive(Component)]
struct LogMain;

#[derive(Component)]
struct HistoryMain;

#[derive(Component)]
struct IconSet([Cow<'static, str>; 2]);

#[derive(Clone, Component)]
struct IconIndex(u8);

fn setup(mut commands: Commands, mut images: ResMut<ImageHandles>, asset_server: Res<AssetServer>) {
    // Log
    commands
        .spawn((
            StateScoped(P2_CTX_STATE),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            PickingBehavior::IGNORE,
            Name::new("UiRoot"),
        ))
        .with_children(|parent| {
            // Log
            parent
                .spawn((
                    Node {
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(60, 60, 60, 140)),
                    LogMain,
                    Visibility::Hidden,
                    Name::new("LogMain"),
                ))
                .with_child((
                    LogDisplay::new(LogDisplaySettings {
                        max_lines: 27,
                        ..default()
                    }),
                    Node {
                        height: Val::Percent(100.0),
                        overflow: Overflow {
                            x: OverflowAxis::Clip,
                            y: OverflowAxis::Scroll,
                        },
                        ..default()
                    },
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    Scrollable,
                    Name::new("LogImpl"),
                ));

            // Buttons
            parent
                .spawn((
                    Node {
                        justify_self: JustifySelf::End,
                        align_self: AlignSelf::FlexEnd,
                        ..default()
                    },
                    Name::new("ButtonContainer"),
                ))
                .with_children(|parent| {
                    // LogDisplay Toggle
                    parent
                        .spawn((
                            button_node_components("LogToggleButton"),
                            ToggleLogButton::default(),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn(togglable_icon_components(
                                    &mut images,
                                    &asset_server,
                                    [ICON_NOTES_OFF, ICON_NOTES],
                                    0,
                                ))
                                .observe(update_icon);
                        })
                        .observe(toggle_log);

                    // History Toggle
                    parent
                        .spawn((
                            button_node_components("HistoryToggleButton"),
                            ToggleHistoryButton::default(),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn(togglable_icon_components(
                                    &mut images,
                                    &asset_server,
                                    [ICON_HISTORY_OFF, ICON_HISTORY],
                                    0,
                                ))
                                .observe(update_icon);
                        })
                        .observe(toggle_history);

                    // Help
                    parent
                        .spawn(button_node_components("HelpButton"))
                        .with_children(|parent| {
                            parent.spawn((ImageNode::new(images.load(ICON_HELP, &asset_server)),));
                        });
                });
        });

    // Popup
    commands
        .spawn((GhostNode::new(), ZIndex(-1)))
        .with_children(|parent| {
            parent
                .spawn((
                    StateScoped(P2_CTX_STATE),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        // justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    PickingBehavior::IGNORE,
                    Name::new("PopupUiRoot"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            display: Display::Flex,
                            align_self: AlignSelf::Center,
                            justify_self: JustifySelf::Center,
                            flex_direction: FlexDirection::Column,
                            top: Val::Percent(60.0),
                            ..default()
                        },
                        PopupUiAnchor,
                        Name::new("PopupUiAnchor"),
                    ));
                });
        });

    // History UI
    commands
        .spawn((GhostNode::new(), ZIndex(-1)))
        .with_children(|parent| {
            parent
                .spawn((
                    StateScoped(P2_CTX_STATE),
                    Node {
                        display: Display::Flex,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    },
                    PickingBehavior::IGNORE,
                    HistoryUiSide::Right,
                    Name::new("HistoryUiRoot"),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                display: Display::Flex,
                                width: Val::Percent(15.0),
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                            PickingBehavior::IGNORE,
                        ))
                        .with_children(|parent| {
                            // Main
                            parent
                                .spawn((
                                    Node {
                                        display: Display::Flex,
                                        height: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        overflow: Overflow::scroll_y(),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba_u8(60, 60, 60, 140)),
                                    Visibility::Hidden,
                                    HistoryMain,
                                    Name::new("HistoryMain"),
                                ))
                                .with_children(|parent| {
                                    parent
                                        .spawn((
                                            Node {
                                                display: Display::Flex,
                                                height: Val::Percent(100.0),
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::scroll_y(),
                                                // padding: UiRect::vertical(Val::Px(10.0)),
                                                ..default()
                                            },
                                            HistoryUiAnchor,
                                            Scrollable,
                                            ScrollLineHeight(20.0),
                                            Name::new("HistoryUiAnchor"),
                                        ))
                                        .with_children(|_parent| {
                                            // DEBUG
                                            // for color in ["Black", "White"] {
                                            //     _parent.spawn((
                                            //         CardSnapshot(
                                            //             format!("{}-?", color).parse().unwrap(),
                                            //         ),
                                            //         Name::new("TestCard2d"),
                                            //     ));

                                            //     for i in 0..=11 {
                                            //         _parent.spawn((
                                            //             CardSnapshot(
                                            //                 format!("{}-{}", color, i)
                                            //                     .parse()
                                            //                     .unwrap(),
                                            //             ),
                                            //             Name::new("TestCard2d"),
                                            //         ));
                                            //     }
                                            // }
                                        });

                                    parent
                                        .spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Px(60.0),
                                                padding: UiRect {
                                                    right: Val::Vw(0.05),
                                                    bottom: Val::Vh(0.05),
                                                    ..default()
                                                },
                                                border: UiRect::all(Val::Px(4.0)),
                                                justify_content: JustifyContent::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgb_u8(0x36, 0x74, 0xB5)),
                                            BorderRadius::all(Val::Px(4.0)),
                                            BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                                            interaction_based(
                                                BorderColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                                                Color::srgba_u8(0, 233, 255, 255).into(),
                                                Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                                            ),
                                            Name::new("SlideHistoryUiButton"),
                                        ))
                                        .with_children(|parent| {
                                            parent
                                                .spawn(togglable_icon_components(
                                                    &mut images,
                                                    &asset_server,
                                                    [ICON_ARROW_LEFT, ICON_ARROW_RIGHT],
                                                    0,
                                                ))
                                                .observe(update_icon);
                                        })
                                        .observe(toggle_history_ui_side);
                                });

                            // Dummy node
                            parent.spawn((
                                Node {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
                                    padding: UiRect {
                                        right: Val::Vw(0.05),
                                        bottom: Val::Vh(0.05),
                                        ..default()
                                    },
                                    border: UiRect::all(Val::Px(4.0)),
                                    ..default()
                                },
                                PickingBehavior::IGNORE,
                                Name::new("UiHeightAlignment"),
                            ));
                        });
                });
        });
}

fn button_node_components(name: &'static str) -> impl Bundle {
    (
        Node {
            width: Val::Px(60.0),
            height: Val::Px(60.0),
            padding: UiRect {
                right: Val::Vw(0.05),
                bottom: Val::Vh(0.05),
                ..default()
            },
            border: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        Interaction::default(),
        BackgroundColor(Color::srgb_u8(0x36, 0x74, 0xB5)),
        BorderRadius::all(Val::Px(4.0)),
        BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        interaction_based(
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
            Color::srgba_u8(0, 233, 255, 255).into(),
            Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
        ),
        Name::new(name),
    )
}

fn togglable_icon_components(
    images: &mut ImageHandles,
    asset_server: &AssetServer,
    icons: [&'static str; 2],
    initial_idx: u8,
) -> impl Bundle {
    (
        ImageNode::new(images.load(icons[initial_idx as usize], asset_server)),
        IconSet([icons[0].into(), icons[1].into()]),
        IconIndex(initial_idx),
    )
}

fn update_icon(
    trigger: Trigger<Pointer<Click>>,
    mut query: Query<(&mut ImageNode, &mut IconIndex, &IconSet)>,
    mut images: ResMut<ImageHandles>,
    asset_server: Res<AssetServer>,
) {
    let (mut image_node, mut icon_idx, icon_set) = query.get_mut(trigger.entity()).unwrap();

    icon_idx.0 = match icon_idx.0 {
        0 => 1,
        1 => 0,
        _ => unreachable!(),
    };

    image_node.image = images.load(icon_set.0[icon_idx.0 as usize].clone(), &asset_server);
}

fn toggle_log(
    trigger: Trigger<Pointer<Click>>,
    mut query: Query<&mut ToggleLogButton>,
    mut vis: Single<&mut Visibility, With<LogMain>>,
) {
    let entity = trigger.entity();
    let mut toggle = query.get_mut(entity).unwrap();
    toggle.enabled = match toggle.enabled {
        true => false,
        false => true,
    };

    **vis = match toggle.enabled {
        true => Visibility::Visible,
        false => Visibility::Hidden,
    };
}

fn toggle_history(
    trigger: Trigger<Pointer<Click>>,
    mut query: Query<&mut ToggleHistoryButton>,
    mut vis: Single<&mut Visibility, With<HistoryMain>>,
) {
    let entity = trigger.entity();
    let mut toggle = query.get_mut(entity).unwrap();
    toggle.enabled = match toggle.enabled {
        true => false,
        false => true,
    };

    **vis = match toggle.enabled {
        true => Visibility::Visible,
        false => Visibility::Hidden,
    };
}

fn toggle_history_ui_side(
    _trigger: Trigger<Pointer<Click>>,
    mut query: Single<(&mut Node, &mut HistoryUiSide)>,
) {
    let (ref mut node, ref mut side) = *query;

    **side = match **side {
        HistoryUiSide::Left => HistoryUiSide::Right,
        HistoryUiSide::Right => HistoryUiSide::Left,
    };

    node.justify_content = match **side {
        HistoryUiSide::Left => JustifyContent::FlexStart,
        HistoryUiSide::Right => JustifyContent::FlexEnd,
    };
}
