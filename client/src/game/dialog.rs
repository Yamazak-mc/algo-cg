use super::CTX_STATE;
use bevy::prelude::*;
use client::utils::{set_timeout::SetTimeout, AddObserverExt};

pub fn dialog_plugin(app: &mut App) {
    app.add_state_scoped_observer(CTX_STATE, Dialog::on_insert)
        .add_state_scoped_observer(CTX_STATE, Dialog::on_remove);
}

pub struct DialogButton {
    title: String,
    action: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync + 'static>>,
    style: DialogButtonStyle,
}

impl DialogButton {
    pub fn new(
        title: impl Into<String>,
        action: impl FnOnce(&mut Commands) + Send + Sync + 'static,
        style: DialogButtonStyle,
    ) -> Self {
        Self {
            title: title.into(),
            action: Some(Box::new(action)),
            style,
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct DialogButtonStyle {
    pub bg_color: Color,
    pub bg_color_on_hover: Option<Color>,
    pub text_color: Color,
    pub text_color_on_hover: Option<Color>,
    pub size: Vec2,
}

impl Default for DialogButtonStyle {
    fn default() -> Self {
        Self {
            bg_color: Color::srgba(0.3, 0.3, 0.3, 0.7),
            bg_color_on_hover: Some(Color::srgba(0.3, 0.3, 0.3, 1.0)),
            text_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            text_color_on_hover: None,
            size: Vec2::new(160.0, 100.0),
        }
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct Dialog {
    // TODO: Support title message
    _title: Option<String>,
    buttons: Vec<DialogButton>,
    gap: Vec2,
}

impl Dialog {
    pub fn new(title: Option<String>, buttons: impl IntoIterator<Item = DialogButton>) -> Self {
        Self {
            _title: title,
            buttons: buttons.into_iter().collect(),
            gap: Vec2::new(10.0, 10.0),
        }
    }

    fn on_insert(trigger: Trigger<OnInsert, Self>, mut commands: Commands, dialog: Query<&Dialog>) {
        let entity = trigger.entity();
        let dialog = dialog.get(entity).unwrap();

        let observer_entity = commands
            .spawn(Observer::new(DialogButtonPressed::handle_trigger).with_entity(entity))
            .id();

        let size = {
            let mut size = dialog.calculate_size();
            size.x += dialog.gap.x * (dialog.buttons.len() + 1) as f32;
            size.y += dialog.gap.y * 2.0;

            size
        };

        commands
            .entity(entity)
            .insert((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.5), size),
                DialogButtonPressObservedBy(observer_entity),
            ))
            .with_children(|parent| {
                let x_gap = dialog.gap.x;
                let mut x = -size.x / 2.0 + x_gap;

                for (i, button) in dialog.buttons.iter().enumerate() {
                    parent
                        .spawn((
                            Sprite::from_color(button.style.bg_color, button.style.size),
                            Transform::from_xyz(x + button.style.size.x / 2.0, 0.0, 1.0),
                            button.style.clone(),
                            DialogButtonIndex(i as u32),
                        ))
                        .with_child((
                            Text2d(button.title.clone()),
                            TextFont::from_font_size(32.0),
                            Transform::from_xyz(0.0, 0.0, 1.0),
                        ))
                        .observe(button_pointer_over)
                        .observe(button_pointer_out)
                        .observe(button_pointer_click);

                    x += button.style.size.x + x_gap;
                }
            });
    }

    fn on_remove(
        trigger: Trigger<OnRemove, Self>,
        mut commands: Commands,
        query: Query<(&Children, &DialogButtonPressObservedBy)>,
        is_button: Query<Has<DialogButtonIndex>>,
    ) {
        let entity = trigger.entity();

        let (children, observed_by) = query.get(entity).unwrap();

        for child in children {
            if is_button.get(*child).unwrap() {
                commands.entity(*child).despawn_recursive();
            }
        }

        commands.entity(observed_by.0).despawn();
        // TODO: Allow users choose whether to despawn or not
        commands.entity(entity).despawn();
        // .remove::<(Sprite, DialogButtonPressObservedBy)>();
    }

    fn calculate_size(&self) -> Vec2 {
        self.buttons
            .iter()
            .fold(Vec2::new(0.0, 0.0), |ret, val| Vec2 {
                x: ret.x + val.style.size.x,
                y: ret.y.max(val.style.size.y),
            })
    }
}

#[derive(Component)]
struct DialogButtonIndex(u32);

#[derive(Component)]
struct DialogButtonPressed {
    idx: u32,
}

impl Event for DialogButtonPressed {
    type Traversal = &'static Parent;
    const AUTO_PROPAGATE: bool = true;
}

impl DialogButtonPressed {
    fn handle_trigger(
        trigger: Trigger<Self>,
        mut query: Query<&mut Dialog>,
        mut commands: Commands,
    ) {
        let entity = trigger.entity();
        let idx = trigger.event().idx;
        let mut dialog = query.get_mut(entity).unwrap();

        commands.entity(entity).remove::<Dialog>();

        (dialog.buttons[idx as usize].action.take().unwrap())(&mut commands);
    }
}

#[derive(Event)]
struct DialogButtonPressObservedBy(Entity);

fn button_pointer_over(
    trigger: Trigger<Pointer<Over>>,
    mut query: Query<(&mut Sprite, &DialogButtonStyle, &Children)>,
    mut text_colors: Query<&mut TextColor>,
) {
    let entity = trigger.entity();
    let (mut sprite, style, children) = query.get_mut(entity).unwrap();

    if let Some(bg_color) = style.bg_color_on_hover {
        sprite.color = bg_color;
    }
    if let Some(text_color) = style.text_color_on_hover {
        text_colors.get_mut(children[0]).unwrap().0 = text_color;
    }
}

fn button_pointer_out(
    trigger: Trigger<Pointer<Out>>,
    mut query: Query<(&mut Sprite, &DialogButtonStyle, &Children)>,
    mut text_colors: Query<&mut TextColor>,
) {
    let entity = trigger.entity();
    let (mut sprite, style, children) = query.get_mut(entity).unwrap();

    sprite.color = style.bg_color;
    text_colors.get_mut(children[0]).unwrap().0 = style.text_color;
}

fn button_pointer_click(
    trigger: Trigger<Pointer<Click>>,
    query: Query<&DialogButtonIndex>,
    mut commands: Commands,
) {
    let entity = trigger.entity();
    let idx = query.get(entity).unwrap().0;
    commands.entity(entity).trigger(DialogButtonPressed { idx });
}

pub trait PopupMessageExt {
    /// Spawns a popup message on the entity.
    ///
    /// This function requires [`SetTimeoutPlugin`] and [`dialog_plugin`].
    ///
    /// [`SetTimeoutPlugin`]: `client::utils::set_timeout::SetTimeoutPlugin`
    fn insert_popup_message(&mut self, message: impl Into<String>, duration_secs: f32)
        -> &mut Self;
}

impl PopupMessageExt for EntityCommands<'_> {
    fn insert_popup_message(
        &mut self,
        message: impl Into<String>,
        duration_secs: f32,
    ) -> &mut Self {
        let entity = self.id();

        self.insert(Dialog::new(
            None,
            [DialogButton::new(message, |_| (), default())],
        ))
        .trigger(SetTimeout::new(duration_secs).with_fn(move |commands| {
            commands.entity(entity).remove::<Dialog>();
        }));

        self
    }
}
