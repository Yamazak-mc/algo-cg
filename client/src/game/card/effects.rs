use crate::game::CTX_STATE;
use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    prelude::*,
};
use bevy_mod_outline::{OutlineMode, OutlineVolume};
use client::utils::observer_controller;

macro_rules! impl_pointer_observer {
    ($typ:ty, $method_name:ident, $ev_name:ident, $set_to:expr $(,)?) => {
        impl $typ {
            fn $method_name(trigger: Trigger<Pointer<$ev_name>>, mut query: Query<&mut Self>) {
                if let Ok(mut val) = query.get_mut(trigger.entity()) {
                    val.set_if_neq($set_to);
                }
            }
        }
    };
}

const OUTLINE_MODE: OutlineMode = OutlineMode::ExtrudeFlat;

pub fn card_effect_plugin(app: &mut App) {
    app.add_systems(Startup, setup_animation)
        .add_systems(
            Update,
            (
                CardPickingState::on_change,
                CardEffectState::update_state,
                CardEffectState::update,
                CardOutlineColorAlpha::update_outline_color_alpha,
            )
                .chain()
                .run_if(in_state(CTX_STATE)),
        )
        .add_systems(
            PostUpdate,
            PrevCardEffectState::post_update.run_if(in_state(CTX_STATE)),
        );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component)]
#[require(CardHoverObservers)]
pub enum CardPickingState {
    #[default]
    None,
    Pickable,
}

impl CardPickingState {
    fn on_change(
        mut query: Query<
            (Entity, &CardPickingState, &mut CardHoverObservers),
            Changed<CardPickingState>,
        >,
        mut commands: Commands,
    ) {
        for (entity, state, mut observers) in &mut query {
            match state {
                CardPickingState::None => {
                    commands.trigger_targets(
                        observer_controller::Pause::<Pointer<Click>>::new(),
                        entity,
                    );

                    if let Some((a, b)) = observers.0.take() {
                        commands.entity(a).despawn();
                        commands.entity(b).despawn();
                    }
                }
                CardPickingState::Pickable => {
                    commands.trigger_targets(
                        observer_controller::Activate::<Pointer<Click>>::new(),
                        entity,
                    );

                    if observers.0.is_none() {
                        let a = commands
                            .spawn((
                                StateScoped(CTX_STATE),
                                Observer::new(CardHoverState::pointer_over).with_entity(entity),
                                Name::new("Obs:CardHoverState::pointer_over"),
                            ))
                            .id();

                        let b = commands
                            .spawn((
                                StateScoped(CTX_STATE),
                                Observer::new(CardHoverState::pointer_out).with_entity(entity),
                                Name::new("Obs:CardHoverState::pointer_out"),
                            ))
                            .id();

                        observers.0 = Some((a, b));
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component, Reflect)]
pub enum CardHoverState {
    #[default]
    None,
    Hovered,
}

impl_pointer_observer!(CardHoverState, pointer_over, Over, Self::Hovered);
impl_pointer_observer!(CardHoverState, pointer_out, Out, Self::None);

#[derive(Default, Component)]
struct CardHoverObservers(Option<(Entity, Entity)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component, Reflect)]
pub enum CardMentionState {
    #[default]
    None,
    Mentioned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component, Reflect)]
#[require(
    CardPickingState,
    CardHoverState,
    CardMentionState,
    PrevCardEffectState,
    CardOutlineColorAlpha
)]
pub enum CardEffectState {
    #[default]
    None,
    Pickable,
    Mentioned,
    Hovered,
}

impl CardEffectState {
    fn update_state(
        mut query: Query<
            (
                &CardPickingState,
                &CardMentionState,
                &CardHoverState,
                &mut Self,
            ),
            Or<(
                Changed<CardPickingState>,
                Changed<CardMentionState>,
                Changed<CardHoverState>,
            )>,
        >,
    ) {
        for (picking_state, mention_state, pointer_state, mut effect_state) in &mut query {
            let new_state = Self::compute(*picking_state, *mention_state, *pointer_state);
            effect_state.set_if_neq(new_state);
        }
    }

    fn compute(
        picking_state: CardPickingState,
        mention_state: CardMentionState,
        hover_state: CardHoverState,
    ) -> Self {
        match (picking_state, mention_state, hover_state) {
            (CardPickingState::Pickable, _, CardHoverState::Hovered) => Self::Hovered,
            (_, CardMentionState::Mentioned, _) => Self::Mentioned,
            (CardPickingState::Pickable, _, _) => Self::Pickable,
            _ => Self::None,
        }
    }

    fn update(
        query: Query<(Entity, &PrevCardEffectState, &Self, &Children), Changed<Self>>,
        mut commands: Commands,
        res: Res<PickableCardBlinkAnimation>,
        mut player: Single<&mut AnimationPlayer, With<PickableCardBlinkAnimationPlayer>>,
    ) {
        let mut rewind = false;
        for (entity, prev_state, state, children) in &query {
            if state == &**prev_state {
                continue;
            }

            let Some(child) = children.first().cloned() else {
                warn!("no child entity found for {}", entity);
                continue;
            };

            match (**prev_state, state) {
                (_, Self::None) => {
                    commands
                        .entity(child)
                        .remove::<(OutlineVolume, AnimationTarget)>();
                }
                (_, Self::Hovered) => {
                    let color = Self::Hovered.outline_color();
                    commands.entity(child).remove::<AnimationTarget>().insert((
                        OUTLINE_MODE,
                        CardOutlineColorAlpha { alpha: color.alpha },
                        outline_components(6.0, color),
                    ));
                }
                (Self::None, state) => {
                    let color = state.outline_color();
                    commands.entity(child).insert((
                        OUTLINE_MODE,
                        CardOutlineColorAlpha { alpha: color.alpha },
                        outline_components(1.0, color),
                        res.animation_target,
                    ));
                    rewind = true;
                }
                (prev_state, state) => {
                    commands
                        .entity(child)
                        .insert(outline_components(1.0, state.outline_color()));

                    if prev_state == Self::Hovered {
                        commands.entity(child).insert(res.animation_target);
                    }
                }
            }
        }

        if rewind {
            player.rewind_all();
        }
    }

    fn outline_color(&self) -> LinearRgba {
        let (r, g, b) = match self {
            Self::None => return LinearRgba::NONE,
            Self::Pickable => (1.0, 1.0, 0.0),
            Self::Mentioned => (0.0, 0.0, 1.0),
            Self::Hovered => (0.0, 1.0, 1.0),
        };
        LinearRgba::rgb(r, g, b)
    }
}

#[derive(Deref, DerefMut, Debug, Clone, Copy, PartialEq, Eq, Default, Component, Reflect)]
pub struct PrevCardEffectState(CardEffectState);

impl PrevCardEffectState {
    fn post_update(mut query: Query<(&CardEffectState, &mut Self), Changed<CardEffectState>>) {
        for (now, mut prev) in &mut query {
            prev.0 = *now;
        }
    }
}

#[derive(Resource)]
struct PickableCardBlinkAnimation {
    animation_target: AnimationTarget,
    _node_idx: AnimationNodeIndex,
}

fn setup_animation(
    mut commands: Commands,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let duration = 1.0;

    let animation_target_id = AnimationTargetId::from_name(&"PickaleCardBlink".into());

    let mut clip = AnimationClip::default();
    clip.add_curve_to_target(
        animation_target_id,
        AnimatableCurve::new(
            animated_field!(OutlineVolume::width),
            EasingCurve::new(4.0, 1.0, EaseFunction::CubicIn)
                .reparametrize_linear(Interval::new(0.0, duration).unwrap())
                .unwrap(),
        ),
    );
    clip.add_curve_to_target(
        animation_target_id,
        AnimatableCurve::new(
            animated_field!(CardOutlineColorAlpha::alpha),
            EasingCurve::new(1.0, 0.0, EaseFunction::CubicIn)
                .reparametrize_linear(Interval::new(0.0, duration).unwrap())
                .unwrap(),
        ),
    );

    let (graph, node_idx) = AnimationGraph::from_clip(clips.add(clip));
    let graph_handle = graphs.add(graph);

    let mut animation_player = AnimationPlayer::default();
    animation_player.play(node_idx).repeat();

    let player = commands
        .spawn((
            PickableCardBlinkAnimationPlayer,
            animation_player,
            AnimationGraphHandle(graph_handle),
            Name::new("PickableCardBlinkAnimationPlayer"),
        ))
        .id();

    commands.insert_resource(PickableCardBlinkAnimation {
        animation_target: AnimationTarget {
            id: animation_target_id,
            player,
        },
        _node_idx: node_idx,
    });
}

#[derive(Component)]
struct PickableCardBlinkAnimationPlayer;

#[derive(Debug, Clone, Copy, Default, Reflect, Component)]
struct CardOutlineColorAlpha {
    alpha: f32,
}

impl CardOutlineColorAlpha {
    fn update_outline_color_alpha(mut query: Query<(&Self, &mut OutlineVolume), Changed<Self>>) {
        for (this, mut volume) in &mut query {
            volume.colour.set_alpha(this.alpha);
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, Default, Reflect, Component)]
struct CardOutlineColor {
    color: LinearRgba,
}

impl CardOutlineColor {
    #[allow(unused)]
    fn update_outline_color(mut query: Query<(&Self, &mut OutlineVolume), Changed<Self>>) {
        for (color, mut volume) in &mut query {
            volume.colour = color.color.into();
        }
    }
}

fn outline_components(width: f32, color: LinearRgba) -> impl Bundle {
    (
        OutlineVolume {
            visible: true,
            width,
            colour: color.into(),
        },
        CardOutlineColor { color },
    )
}
