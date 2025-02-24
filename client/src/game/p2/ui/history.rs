use crate::game::{
    card::{
        effects::{CardMentionState, MENTION_OUTLINE_COLOR},
        instance::CardInstance,
        material::CardMaterials,
    },
    p2::P2_CTX_STATE,
    CARD_HEIGHT, CARD_WIDTH,
};
use algo_core::card::{CardNumber, CardView};
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};
use client::utils::{
    component_based::interaction_based, into_color::IntoColor, scrollable::ScrollToEnd,
    AddObserverExt,
};

const CARD_2D_HEIGHT: f32 = 64.0;
const CARD_2D_WIDTH: f32 = CARD_2D_HEIGHT * CARD_WIDTH / CARD_HEIGHT;

const CARD_2D_BORDER: f32 = 3.0;

pub fn history_plugin(app: &mut App) {
    app.add_systems(
        Update,
        update_card_2d_interaction.run_if(in_state(P2_CTX_STATE)),
    )
    .add_systems(
        PostUpdate,
        force_scroll_to_newest
            .run_if(in_state(P2_CTX_STATE))
            .before(TransformSystem::TransformPropagate),
    )
    .add_observer(CardSnapshot::on_add_card_snapshot)
    .add_observer(HistoryBgColor::set_bg_color)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::initial_cards)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::turn_started)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::draw)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::attack_target_selected)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::number_guessed)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::text_events)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::card_revealed)
    .add_state_scoped_observer_named(P2_CTX_STATE, PushHistory::attacker_inserted_to_field)
    .add_state_scoped_observer_named(
        P2_CTX_STATE,
        SpawnLabeledCardSnapshot::spawn_labeled_card_2d,
    )
    .add_state_scoped_observer_named(P2_CTX_STATE, SpawnMessage::spawn_message);
}

#[derive(Component)]
#[component(on_add = HistoryUiAnchor::on_add)]
// #[require(HistoryBgColor)]
pub struct HistoryUiAnchor;

impl HistoryUiAnchor {
    fn on_add(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        world.trigger_targets(HistoryBgColor::default(), entity);
    }
}

#[derive(Component)]
struct CurrentHistoryUiParent;

#[derive(Debug, Clone, Copy, Component)]
#[require(CardMentionToggle)]
pub struct CardMention {
    card_entity: Entity,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct CardMentionToggle {
    enabled: bool,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CardSnapshot(pub CardView);

impl CardSnapshot {
    fn on_add_card_snapshot(
        trigger: Trigger<OnAdd, Self>,
        query: Query<&Self>,
        mut card_materials: ResMut<CardMaterials>,
        mut images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,

        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let Ok(card_snapshot) = query.get(entity) else {
            return;
        };

        let card = card_snapshot.0;
        let card_color = card.pub_info.color;
        let img = card_materials.get_or_create_card_image(
            card_color,
            card.priv_info.map(|v| v.number),
            &mut images,
            &mut materials,
        );

        let bg_color = card_color.bg_color_rgb().into_color();
        let border_color = MENTION_OUTLINE_COLOR;

        commands
            .entity(entity)
            .insert((
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    width: Val::Px(CARD_2D_WIDTH + CARD_2D_BORDER * 2.0),
                    height: Val::Px(CARD_2D_HEIGHT + CARD_2D_BORDER * 2.0),
                    border: UiRect::all(Val::Px(CARD_2D_BORDER)),
                    ..default()
                },
                BorderColor(Color::NONE),
                PickingBehavior::IGNORE,
                Interaction::default(),
                interaction_based(
                    BorderColor(border_color),
                    border_color.into(),
                    Color::NONE.into(),
                ),
                Name::new("CardSnapshot"),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            width: Val::Px(CARD_2D_WIDTH),
                            height: Val::Px(CARD_2D_HEIGHT),
                            ..default()
                        },
                        BackgroundColor(bg_color),
                        PickingBehavior::IGNORE,
                    ))
                    .with_children(|parent| {
                        let Some(img) = img else {
                            return;
                        };

                        if card.pub_info.revealed {
                            parent.spawn((ImageNode::new(img), PickingBehavior::IGNORE));
                        } else {
                            let text_color = card_color.text_color_rgb().into_color();
                            let card_number = card.priv_info.unwrap().number.0;

                            // The card is not revealed but the player knows its number.
                            // Draw a number in small size.
                            parent.spawn((
                                Node {
                                    justify_self: JustifySelf::Center,
                                    align_self: AlignSelf::End,
                                    ..default()
                                },
                                Text(card_number.to_string()),
                                TextColor(text_color),
                                PickingBehavior::IGNORE,
                            ));
                        }
                    });
            });
    }
}

fn update_card_2d_interaction(
    mut query: Query<
        (&Interaction, Option<(&CardMention, &mut CardMentionToggle)>),
        (Changed<Interaction>, With<CardSnapshot>),
    >,
    mut card_target_query: Query<&mut CardMentionState>,
) {
    for (interaction, card_mention, mut card_mention_toggle) in query
        .iter_mut()
        .filter_map(|(a, b)| b.map(|(b1, b2)| (a, b1, b2)))
    {
        let Ok(mut target_mention_state) = card_target_query.get_mut(card_mention.card_entity)
        else {
            continue;
        };

        let (enabled, new_target_mention_state) = match interaction {
            Interaction::Pressed | Interaction::Hovered => (true, CardMentionState::Mentioned),
            Interaction::None => (false, CardMentionState::None),
        };
        card_mention_toggle.set_if_neq(CardMentionToggle { enabled });
        target_mention_state.set_if_neq(new_target_mention_state);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CardSnapshotBuilder {
    pub mention_target: Option<Entity>,
    pub card: CardView,
}

impl CardSnapshotBuilder {
    pub fn from_entity_with_query(entity: Entity, query: &Query<&CardInstance>) -> Self {
        Self {
            mention_target: Some(entity),
            card: *query.get(entity).unwrap().get(),
        }
    }

    fn spawn(&self, parent: &mut ChildBuilder<'_>) {
        let mut entity_cmds = parent.spawn((CardSnapshot(self.card), PickingBehavior::IGNORE));

        if let Some(mention_target) = self.mention_target {
            entity_cmds.insert(CardMention {
                card_entity: mention_target,
            });
        }
    }
}

#[derive(Debug, Clone, Event)]
pub enum PushHistory {
    InitialCards(Vec<CardSnapshotBuilder>),
    TurnStarted {
        message: String,
        color: HistoryBgColor,
    },
    Draw(CardSnapshotBuilder),
    AttackTargetSelected {
        target: CardSnapshotBuilder,
    },
    NumberGuessed(CardNumber),
    AttackSucceeded,
    AttackFailed,
    CardRevealed(CardSnapshotBuilder),
    AttackerInsertedToField(CardSnapshotBuilder),
}

type HistoryParentQuery<'w> = Single<'w, Entity, With<CurrentHistoryUiParent>>;

impl PushHistory {
    fn initial_cards(
        trigger: Trigger<Self>,
        mut commands: Commands,
        history_parent: HistoryParentQuery,
    ) {
        let Self::InitialCards(cards) = trigger.event() else {
            return;
        };
        commands.entity(*history_parent).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        ..history_bg_node()
                    },
                    PickingBehavior::IGNORE,
                    Name::new("InitialCards"),
                ))
                .with_children(|parent| {
                    for builder in cards {
                        builder.spawn(parent);
                    }
                });
        });
    }

    fn turn_started(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        anchor_entity: Single<Entity, With<HistoryUiAnchor>>,
    ) {
        let Self::TurnStarted { message, color } = trigger.event_mut() else {
            return;
        };
        commands.trigger_targets(*color, *anchor_entity);
        commands.trigger(SpawnMessage(std::mem::take(message)));
    }

    fn draw(trigger: Trigger<Self>, mut commands: Commands) {
        let Self::Draw(builder) = trigger.event() else {
            return;
        };
        commands.trigger(SpawnLabeledCardSnapshot {
            label: "Draw:".into(),
            builder: *builder,
        });
    }

    fn attack_target_selected(trigger: Trigger<Self>, mut commands: Commands) {
        let Self::AttackTargetSelected { target } = trigger.event() else {
            return;
        };
        // commands.trigger(SpawnMessage("Attack".into()));
        commands.trigger(SpawnLabeledCardSnapshot {
            label: "Attack\n- Target:".into(),
            builder: *target,
        });
    }

    fn number_guessed(trigger: Trigger<Self>, mut commands: Commands) {
        let Self::NumberGuessed(num) = trigger.event() else {
            return;
        };
        commands.trigger(SpawnMessage(format!("- Guess: {}", num.0)));
    }

    fn text_events(trigger: Trigger<Self>, mut commands: Commands) {
        let message = match trigger.event() {
            Self::AttackSucceeded => "- Succeeded!",
            Self::AttackFailed => "- Failed!",
            _ => return,
        };
        commands.trigger(SpawnMessage(message.into()));
    }

    fn card_revealed(trigger: Trigger<Self>, mut commands: Commands) {
        let Self::CardRevealed(builder) = trigger.event() else {
            return;
        };
        commands.trigger(SpawnLabeledCardSnapshot {
            label: "Revealed:".into(),
            builder: *builder,
        });
    }

    fn attacker_inserted_to_field(trigger: Trigger<Self>, mut commands: Commands) {
        let Self::AttackerInsertedToField(builder) = trigger.event() else {
            return;
        };
        if builder.card.pub_info.revealed {
            return;
        }
        commands.trigger(SpawnLabeledCardSnapshot {
            label: "Stay:".into(),
            builder: *builder,
        });
    }
}

#[derive(Event)]
struct SpawnLabeledCardSnapshot {
    label: String,
    builder: CardSnapshotBuilder,
}

impl SpawnLabeledCardSnapshot {
    fn spawn_labeled_card_2d(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        history_parent: HistoryParentQuery,
    ) {
        let label = std::mem::take(&mut trigger.event_mut().label);
        let builder = trigger.event().builder;

        commands.entity(*history_parent).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        ..history_bg_node()
                    },
                    PickingBehavior::IGNORE,
                ))
                .with_children(|parent| {
                    let name = format!("LabeledCard({})", label);
                    parent.spawn((Text(label), PickingBehavior::IGNORE, Name::new(name)));
                    builder.spawn(parent);
                });
        });
    }
}

#[derive(Event)]
struct SpawnMessage(String);

impl SpawnMessage {
    fn spawn_message(
        mut trigger: Trigger<Self>,
        mut commands: Commands,
        history_parent: HistoryParentQuery,
    ) {
        let message = std::mem::take(&mut trigger.event_mut().0);

        commands.entity(*history_parent).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        ..history_bg_node()
                    },
                    PickingBehavior::IGNORE,
                ))
                .with_children(|parent| {
                    let name = format!("HistoryEntryMessage({})", message);
                    parent.spawn((Text(message), PickingBehavior::IGNORE, Name::new(name)));
                });
        });
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Event)]
pub enum HistoryBgColor {
    OpponentTurn,
    MyTurn,
    #[default]
    None,
}

impl HistoryBgColor {
    pub fn from_bool(is_my_turn: bool) -> Self {
        if is_my_turn {
            Self::MyTurn
        } else {
            Self::OpponentTurn
        }
    }

    fn as_bg_color(&self) -> BackgroundColor {
        match self {
            Self::OpponentTurn => Color::srgba(1.0, 0.0, 0.0, 0.5),
            Self::MyTurn => Color::srgba(0.0, 0.0, 1.0, 0.5),
            Self::None => Color::srgba(0.0, 0.0, 0.0, 0.0),
        }
        .into()
    }

    fn set_bg_color(
        trigger: Trigger<Self>,
        prev_parent: Option<Single<Entity, With<CurrentHistoryUiParent>>>,
        mut commands: Commands,
    ) {
        // Remove marker from the previous UI parent.
        if let Some(prev_parent) = prev_parent {
            commands
                .entity(*prev_parent)
                .remove::<CurrentHistoryUiParent>();
        }

        let anchor_entity = trigger.entity();

        let bg_color = trigger.event().as_bg_color();

        commands.entity(anchor_entity).with_child((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                // flex_wrap: FlexWrap::Wrap,
                width: Val::Percent(100.0),
                padding: UiRect::vertical(Val::Px(22.0)),
                ..default()
            },
            bg_color,
            CurrentHistoryUiParent,
            PickingBehavior::IGNORE,
            Name::new("HistoryUiParent"),
        ));
    }
}

fn history_bg_node() -> Node {
    Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        width: Val::Percent(100.0),
        ..default()
    }
}

fn force_scroll_to_newest(
    parent_of_ui_parent: Option<Single<&Parent, (With<CurrentHistoryUiParent>, Changed<Children>)>>,
    mut commands: Commands,
) {
    let Some(anchor) = parent_of_ui_parent else {
        return;
    };
    commands.entity(anchor.get()).insert(ScrollToEnd);
}
