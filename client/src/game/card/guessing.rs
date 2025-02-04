use crate::{game::CTX_STATE, AppState};
use algo_core::card::CardNumber;
use bevy::prelude::*;
use client::utils::AddObserverExt as _;

const PANEL_SIZE: Vec2 = Vec2::new(360.0, 200.0);
const PANEL_TRANSLATION: Vec3 = Vec3::new(0.0, -80.0, 0.0);

const ITEMS_PER_ROW: u8 = 6;
const ITEMS_PER_COL: u8 = 2;
const GAP: Vec2 = Vec2::new(4.0, 4.0);

pub fn card_guessing_plugin(app: &mut App) {
    app.add_sub_state::<NumSelectorState>()
        .enable_state_scoped_entities::<NumSelectorState>()
        .add_state_scoped_observer(CTX_STATE, SpawnNumSelector::handle_trigger)
        .add_systems(
            OnEnter(NumSelectorState::Selecting),
            SpawnNumSelector::setup_button_interaction,
        );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(AppState = CTX_STATE)]
pub(crate) enum NumSelectorState {
    #[default]
    Inactive,
    Selecting,
    #[allow(unused)]
    Confirming, // TODO
}

impl NumSelectorState {
    fn is_active(&self) -> bool {
        matches!(self, Self::Selecting | Self::Confirming)
    }
}

#[derive(Event)]
pub struct SpawnNumSelector;

impl SpawnNumSelector {
    fn handle_trigger(
        trigger: Trigger<Self>,
        state: Res<State<NumSelectorState>>,
        mut next_state: ResMut<NextState<NumSelectorState>>,
        mut commands: Commands,
    ) {
        if state.get().is_active() {
            warn!("the SpawnNumSelector was triggered while NumSelector was already active");
            return;
        }

        let card_entity = trigger.entity();
        commands.entity(card_entity).insert(NumSelectorTarget);

        let gap_p = GAP / PANEL_SIZE;
        let button_w_plus_gap_p = (1.0 - gap_p.x) / ITEMS_PER_ROW as f32;
        let button_h_plus_gap_p = (1.0 - gap_p.y) / ITEMS_PER_COL as f32;
        let button_size = Vec2::new(
            PANEL_SIZE.x * (button_w_plus_gap_p - gap_p.x),
            PANEL_SIZE.y * (button_h_plus_gap_p - gap_p.y),
        );

        commands
            .spawn((
                StateScoped(NumSelectorState::Selecting),
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.5), PANEL_SIZE),
                Transform::from_translation(PANEL_TRANSLATION),
                Name::new("NumSelector"),
            ))
            .with_children(|parent| {
                for row in 0..ITEMS_PER_COL {
                    for col in 0..ITEMS_PER_ROW {
                        let x = {
                            let i = col as i32 - ITEMS_PER_ROW as i32 / 2;
                            (PANEL_SIZE.x * button_w_plus_gap_p) * (i as f32 + 0.5)
                        };
                        let y = {
                            let j = row as i32 - ITEMS_PER_COL as i32 / 2;
                            -(PANEL_SIZE.y * button_h_plus_gap_p) * (j as f32 + 0.5)
                        };

                        let n = col + row * ITEMS_PER_ROW;

                        parent
                            .spawn((
                                NumSelectorButton {
                                    output: CardNumber(n),
                                },
                                Sprite::from_color(Color::srgba(0.3, 0.3, 0.3, 0.7), button_size),
                                Transform::from_xyz(x, y, 1.0),
                                Name::new(format!("NumSelectorButton[{col}, {row}]")),
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text2d(format!("{}", n)),
                                    TextFont::from_font_size(48.0),
                                    Transform::from_xyz(0.0, 0.0, 2.0),
                                ));
                            });
                    }
                }
            });

        // commands.trigger_targets(
        //     AddFollower {
        //         follower: selector_entity,
        //         offset_3d: Vec3::new(0.0, 0.0, CARD_HEIGHT * 3.0),
        //         offset_2d: Vec3::new(0.0, 0.0, 0.0),
        //     },
        //     card_entity,
        // );

        next_state.set(NumSelectorState::Selecting);
    }

    fn setup_button_interaction(
        mut commands: Commands,
        buttons: Query<Entity, With<NumSelectorButton>>,
    ) {
        let observers = [
            Observer::new(NumSelectorButton::pointer_over),
            Observer::new(NumSelectorButton::pointer_out),
            Observer::new(NumSelectorButton::pointer_click),
        ];

        for mut observer in observers {
            for button_entity in &buttons {
                observer.watch_entity(button_entity);
            }
            commands.spawn((StateScoped(NumSelectorState::Selecting), observer));
        }
    }
}

#[derive(Component)]
struct NumSelectorTarget;

#[derive(Event)]
#[allow(unused)]
pub struct DespawnNumSelector;

#[derive(Component)]
struct NumSelectorButton {
    output: CardNumber,
}

impl NumSelectorButton {
    fn pointer_over(trigger: Trigger<Pointer<Over>>, mut sprites: Query<&mut Sprite>) {
        sprites.get_mut(trigger.entity()).unwrap().color = Color::srgba(0.3, 0.3, 0.3, 1.0);
    }

    fn pointer_out(trigger: Trigger<Pointer<Out>>, mut sprites: Query<&mut Sprite>) {
        sprites.get_mut(trigger.entity()).unwrap().color = Color::srgba(0.3, 0.3, 0.3, 0.7);
    }

    fn pointer_click(
        trigger: Trigger<Pointer<Click>>,
        buttons: Query<&NumSelectorButton>,
        target: Single<Entity, With<NumSelectorTarget>>,
        mut next_state: ResMut<NextState<NumSelectorState>>,
        mut commands: Commands,
    ) {
        let target = *target;
        commands.entity(target).remove::<NumSelectorTarget>();
        commands.trigger_targets(
            NumSelected(buttons.get(trigger.entity()).unwrap().output),
            target,
        );
        // commands.trigger_targets(DespawnFollower, target);

        next_state.set(NumSelectorState::Inactive);
    }
}

#[derive(Debug, Clone, Event)]
pub struct NumSelected(pub CardNumber);
