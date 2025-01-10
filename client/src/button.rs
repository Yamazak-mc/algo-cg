use bevy::prelude::*;

pub type QueryButtonClick<'w, 's, 'a, T> =
    Query<'w, 's, &'a Interaction, (With<T>, Changed<Interaction>)>;

pub fn is_button_pressed<T: Component>(query: QueryButtonClick<T>) -> bool {
    if let Ok(interaction) = query.get_single() {
        matches!(interaction, Interaction::Pressed)
    } else {
        false
    }
}
