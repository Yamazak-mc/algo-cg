use super::P2_CTX_STATE;
use bevy::{prelude::*, ui::experimental::GhostNode};
use client::utils::{
    component_based::interaction_based,
    log_display::{LogDisplay, LogDisplaySettings},
    scrollable::Scrollable,
};
use std::borrow::Cow;

mod image_handles;
use image_handles::ImageHandles;

pub mod popup;
use popup::PopupUiAnchor;

const ICON_NOTES: &str = "tabler-icons/notes.png";
const ICON_NOTES_OFF: &str = "tabler-icons/notes-off.png";
const ICON_HISTORY: &str = "tabler-icons/history.png";
const ICON_HISTORY_OFF: &str = "tabler-icons/history-off.png";
const ICON_HELP: &str = "tabler-icons/help.png";

pub fn ui_plugin(app: &mut App) {
    app.add_plugins(popup::popup_plugin)
        .insert_resource(ImageHandles::new([
            ICON_NOTES,
            ICON_NOTES_OFF,
            ICON_HISTORY,
            ICON_HISTORY_OFF,
            ICON_HELP,
        ]))
        .add_systems(OnEnter(P2_CTX_STATE), setup);
}

#[derive(Default, Component)]
struct ToggleLogButton {
    enabled: bool,
}

#[derive(Component)]
struct LogMain;

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
                                .spawn((
                                    ImageNode::new(images.load(ICON_NOTES_OFF, &asset_server)),
                                    IconSet([ICON_NOTES_OFF.into(), ICON_NOTES.into()]),
                                    IconIndex(0),
                                ))
                                .observe(update_icon);
                        })
                        .observe(toggle_log);

                    // History Toggle
                    parent
                        .spawn(button_node_components("HistoryToggleButton"))
                        .with_children(|parent| {
                            parent
                                .spawn((
                                    ImageNode::new(images.load(ICON_HISTORY_OFF, &asset_server)),
                                    IconSet([ICON_HISTORY_OFF.into(), ICON_HISTORY.into()]),
                                    IconIndex(0),
                                ))
                                .observe(update_icon);
                        });

                    // Help
                    parent
                        .spawn(button_node_components("HelpButton"))
                        .with_children(|parent| {
                            parent.spawn((ImageNode::new(images.load(ICON_HELP, &asset_server)),));
                        });
                });
        });

    // DEBUG
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
