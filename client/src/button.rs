use crate::util::IntoColor;
use bevy::prelude::*;
use std::marker::PhantomData;

const BORDER_COLOR_DEFAULT: [u8; 3] = [0xFF, 0xFF, 0xFF];
const BORDER_COLOR_ON_HOVER: [u8; 3] = [0x48, 0xCF, 0xCB];

const BG_COLOR_DEFAULT: [u8; 3] = [0x57, 0x7B, 0xC1];
const BG_COLOR_ON_CLICK: [u8; 3] = [0x2E, 0x4E, 0x8C];

pub fn button_system<M: Component>(
    mut commands: Commands,
    mut query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<CommonButton<M>>),
    >,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, mut bg_color, mut border_color, children) in &mut query {
        let mut text_color = text_query.get_mut(children[0]).unwrap();
        match interaction {
            Interaction::Pressed => {
                bg_color.0 = BG_COLOR_ON_CLICK.into_color();
                border_color.0 = BORDER_COLOR_ON_HOVER.into_color();
                text_color.0 = BORDER_COLOR_ON_HOVER.into_color();

                commands.trigger(ButtonPressed::<M>(PhantomData));
            }
            Interaction::Hovered => {
                bg_color.0 = BG_COLOR_DEFAULT.into_color();
                border_color.0 = BORDER_COLOR_ON_HOVER.into_color();
                text_color.0 = BORDER_COLOR_ON_HOVER.into_color();
            }
            Interaction::None => {
                bg_color.0 = BG_COLOR_DEFAULT.into_color();
                border_color.0 = BORDER_COLOR_DEFAULT.into_color();
                text_color.0 = BORDER_COLOR_DEFAULT.into_color();
            }
        }
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
            BorderRadius::all(Val::Percent(30.0)),
            BackgroundColor(BG_COLOR_DEFAULT.into_color()),
        ))
        .with_child((
            Text::new(text),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb_u8(0xFF, 0xFA, 0xEC)),
        ));
}
