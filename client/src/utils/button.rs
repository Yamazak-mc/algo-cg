use super::component_based::{
    interaction_based, parent_interaction_based, EnableComponentBased, EnableParentComponentBased,
};
use crate::utils::into_color::IntoColor;
use bevy::prelude::*;
use std::marker::PhantomData;

const BORDER_COLOR_DEFAULT: [u8; 3] = [0xFF, 0xFF, 0xFF];
const BORDER_COLOR_ON_HOVER: [u8; 3] = [0x48, 0xCF, 0xCB];

const BG_COLOR_DEFAULT: [u8; 3] = [0x57, 0x7B, 0xC1];
const BG_COLOR_ON_CLICK: [u8; 3] = [0x2E, 0x4E, 0x8C];

pub fn common_button_plugin(app: &mut App) {
    app.enable_component_based::<BorderColor, Interaction>()
        .enable_component_based::<BackgroundColor, Interaction>()
        .enable_parent_component_based::<TextColor, Interaction>();
}

pub fn button_system<M: Component>(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<CommonButton<M>>)>,
) {
    if query.iter().any(|v| matches!(v, Interaction::Pressed)) {
        commands.trigger(ButtonPressed::<M>(PhantomData));
    }
}

#[derive(Component)]
#[require(Button)]
pub struct CommonButton<M>(PhantomData<M>);

#[derive(Event)]
pub struct ButtonPressed<M>(PhantomData<M>);

pub fn spawn_common_button<M: Component>(parent: &mut ChildBuilder, text: &str, marker: M) {
    parent
        .spawn((
            marker,
            CommonButton::<M>(PhantomData),
            Node {
                width: Val::Px(320.0),
                height: Val::Px(48.0),
                padding: UiRect::top(Val::Px(1.0)),
                align_content: AlignContent::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BorderColor(BORDER_COLOR_DEFAULT.into_color()),
            interaction_based(
                BorderColor(BORDER_COLOR_ON_HOVER.into_color()),
                BORDER_COLOR_ON_HOVER.into_color().into(),
                BORDER_COLOR_DEFAULT.into_color().into(),
            ),
            BackgroundColor(BG_COLOR_DEFAULT.into_color()),
            interaction_based(
                BackgroundColor(BG_COLOR_ON_CLICK.into_color()),
                BG_COLOR_DEFAULT.into_color().into(),
                BG_COLOR_DEFAULT.into_color().into(),
            ),
            BorderRadius::all(Val::Percent(30.0)),
        ))
        .with_child((
            Text::new(text),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(BORDER_COLOR_DEFAULT.into_color()),
            parent_interaction_based(
                TextColor(BORDER_COLOR_ON_HOVER.into_color()),
                BORDER_COLOR_ON_HOVER.into_color().into(),
                BORDER_COLOR_DEFAULT.into_color().into(),
            ),
        ));
}
