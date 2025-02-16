use bevy::prelude::*;
use client::utils::{component_based::interaction_based, set_timeout::SetTimeout, AddObserverExt};

use crate::game::p2::P2_CTX_STATE;

pub fn popup_plugin(app: &mut App) {
    app.add_state_scoped_observer_named(P2_CTX_STATE, SpawnPopupMessage::spawn_popup_message)
        .add_state_scoped_observer_named(P2_CTX_STATE, SpawnQuestion::spawn_question);
}

const POPUP_TITLE_FONT_SIZE: f32 = 48.0;

#[derive(Component)]
pub(super) struct PopupUiAnchor;

#[derive(Event)]
pub struct SpawnPopupMessage {
    pub duration_secs: f32,
    pub message: String,
}

impl Default for SpawnPopupMessage {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            message: String::default(),
        }
    }
}

impl SpawnPopupMessage {
    fn spawn_popup_message(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        anchor: Single<Entity, With<PopupUiAnchor>>,
    ) {
        let message = std::mem::take(&mut trigger.message);

        let mut entity = Entity::PLACEHOLDER;
        commands.entity(*anchor).with_children(|parent| {
            entity = parent
                .spawn((
                    Node {
                        justify_self: JustifySelf::Center,
                        ..default()
                    },
                    Text(message),
                    TextFont::from_font_size(POPUP_TITLE_FONT_SIZE),
                    Name::new("PopupText"),
                    TextLayout {
                        justify: JustifyText::Center,
                        ..default()
                    },
                ))
                .observe(DespawnPopup::despawn_popup)
                .id();
        });

        commands.trigger(
            SetTimeout::new(trigger.duration_secs).with_trigger_targets(DespawnPopup, entity),
        );
    }
}

#[derive(Clone, Event)]
struct DespawnPopup;

impl DespawnPopup {
    fn despawn_popup(trigger: Trigger<Self>, mut commands: Commands) {
        commands.entity(trigger.entity()).despawn_recursive();
    }
}

#[derive(Debug, Clone, Event)]
pub struct SpawnQuestion {
    pub title: String,
    pub answers: [String; 2],
}

impl SpawnQuestion {
    fn spawn_question(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        anchor: Single<Entity, With<PopupUiAnchor>>,
    ) {
        let title = std::mem::take(&mut trigger.title);
        let answer1 = std::mem::take(&mut trigger.answers[0]);
        let answer2 = std::mem::take(&mut trigger.answers[1]);

        commands.entity(*anchor).with_children(|parent| {
            parent
                .spawn((
                    QuestionPopup,
                    Node {
                        display: Display::Flex,
                        justify_self: JustifySelf::Center,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect {
                            left: Val::Px(25.0),
                            right: Val::Px(25.0),
                            top: Val::Px(5.0),
                            bottom: Val::Px(5.0),
                        },
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(60, 60, 60, 140)),
                    Name::new("QuestionNodeRoot"),
                ))
                .with_children(|parent| {
                    // Title (question)
                    parent.spawn((
                        Text(title),
                        TextFont::from_font_size(POPUP_TITLE_FONT_SIZE),
                        Name::new("QuestionTitle"),
                    ));

                    // Buttons
                    parent
                        .spawn((
                            Node {
                                display: Display::Flex,
                                justify_content: JustifyContent::SpaceEvenly,
                                ..default()
                            },
                            Name::new("AnswerButtonContainer"),
                        ))
                        .with_children(|parent| {
                            for (idx, (text, style)) in [
                                (
                                    answer1,
                                    (
                                        BackgroundColor(Color::srgb_u8(180, 36, 47)),
                                        interaction_based(
                                            BorderColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                                            Color::srgba_u8(255, 113, 180, 255).into(),
                                            Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                                        ),
                                    ),
                                ),
                                (
                                    answer2,
                                    (
                                        BackgroundColor(Color::srgb_u8(0x36, 0x74, 0xB5)),
                                        interaction_based(
                                            BorderColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                                            Color::srgba_u8(0, 233, 255, 255).into(),
                                            Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                                        ),
                                    ),
                                ),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                spawn_answer_button(parent, idx as u32, text, style);
                            }
                        });
                });
        });
    }
}

fn spawn_answer_button(
    parent: &mut ChildBuilder<'_>,
    idx: u32,
    text: String,
    components: impl Bundle,
) {
    parent
        .spawn((
            Node {
                border: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                width: Val::Percent(50.0),
                ..default()
            },
            Interaction::default(),
            BorderRadius::all(Val::Px(4.0)),
            BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            QuestionAnswerButton(idx),
            Name::new(format!("AnswerButton[{}]", idx)),
            components,
        ))
        .with_child((
            Text(text),
            TextFont::from_font_size(POPUP_TITLE_FONT_SIZE),
            PickingBehavior::IGNORE,
        ))
        .observe(on_click_answer_button);
}

#[derive(Component)]
struct QuestionPopup;

#[derive(Deref, DerefMut, Debug, Component)]
#[require(Interaction)]
struct QuestionAnswerButton(u32);

#[derive(Deref, DerefMut, Debug, Event)]
pub struct QuestionAnswered(pub u32);

fn on_click_answer_button(
    trigger: Trigger<Pointer<Click>>,
    query: Query<&QuestionAnswerButton>,
    root_entity: Single<Entity, With<QuestionPopup>>,
    mut commands: Commands,
) {
    info!("answer button clicked!");
    commands.entity(*root_entity).despawn_recursive();

    let idx = query.get(trigger.entity()).unwrap().0;
    commands.trigger(QuestionAnswered(idx));
}
