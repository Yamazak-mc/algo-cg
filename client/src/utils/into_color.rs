use bevy::color::Color;

pub trait IntoColor {
    fn into_color(self) -> Color;
}

impl IntoColor for [u8; 3] {
    fn into_color(self) -> Color {
        let [r, g, b] = self;

        Color::srgb_u8(r, g, b)
    }
}

impl IntoColor for [u8; 4] {
    fn into_color(self) -> Color {
        let [r, g, b, a] = self;

        Color::srgba_u8(r, g, b, a)
    }
}

impl IntoColor for Color {
    fn into_color(self) -> Color {
        self
    }
}
