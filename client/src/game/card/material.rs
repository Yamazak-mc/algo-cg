use algo_core::card::{CardColor, CardNumber};
use bevy::{
    asset::RenderAssetUsages,
    pbr::StandardMaterial,
    prelude::{App, Assets, Commands, Font, Handle, Image, ResMut, Resource, Startup, TextFont},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use client::utils::into_color::IntoColor as _;
use fontdue::layout::{CoordinateSystem, GlyphPosition, Layout, TextStyle};
use image::{Rgb, RgbaImage};
use std::collections::BTreeMap;

const CARD_IMG_PX_WIDTH: u32 = 500;
const CARD_IMG_PX_HEIGHT: u32 = 809;
const FONT_SIZE: f32 = 512.0;

/// The thickness of lines to be drawn under the numbers 6 or 9.
const UNDERLINE_PX_HEIGHT: u32 = 30;

pub fn card_material_plugin(app: &mut App) {
    app.add_systems(Startup, setup_resource);
}

fn setup_resource(mut commands: Commands, font_assets: ResMut<Assets<Font>>) {
    let id = TextFont::default().font.id();

    let Some(font) = font_assets.get(id) else {
        panic!("could not access font asset");
    };

    let Ok(res) = CardMaterials::from_bytes(&font.data[..]) else {
        panic!("failed to read font data");
    };

    commands.insert_resource(res)
}

#[derive(Resource)]
pub struct CardMaterials {
    font: fontdue::Font,
    font_size: f32,
    handles: BTreeMap<(CardColor, Option<CardNumber>), Handle<StandardMaterial>>,
}

impl CardMaterials {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &str> {
        fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).map(|font| Self {
            font,
            font_size: FONT_SIZE,
            handles: BTreeMap::default(),
        })
    }

    pub fn get_or_create_card_material(
        &mut self,
        color: CardColor,
        number: Option<CardNumber>,
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.handles.get(&(color, number)) {
            return handle.clone();
        }

        // Create a new material
        let img = number.map(|n| into_bevy_image(self.generate_card_inner(color, n)));

        let base_color = if img.is_none() {
            color.bg_color_rgb().into_color()
        } else {
            bevy::color::Color::WHITE
        };

        let handle = materials.add(StandardMaterial {
            base_color,
            base_color_texture: img.map(|v| images.add(v)),
            perceptual_roughness: 0.8,
            metallic: 1.0,
            ..Default::default()
        });

        let ret = handle.clone();
        self.handles.insert((color, number), handle);
        ret
    }

    // FIXME: Number 88 exceeds `CARD_WIDTH`.
    fn generate_card_inner(&self, color: CardColor, number: CardNumber) -> RgbaImage {
        // Create an image buffer
        let bg_color = color.bg_color_rgb().into();
        let card_width = CARD_IMG_PX_WIDTH;
        let card_height = CARD_IMG_PX_HEIGHT;
        let mut img_buf = filled_rgba_img_buf(card_width, card_height, bg_color);

        // Setup text processor
        let text_processor = self.setup_process_text(number.0.to_string());
        let (text_width, text_height) = text_processor.img_size();

        // Verify text size
        if text_width <= card_width && text_height <= card_height {
            // Draw text
            let x_offset = (card_width - text_width) / 2;
            let y_offset = (card_height - text_height) / 2;
            let text_color = color.text_color_rgb().into();

            let mut draw_method =
                common_draw_method(&mut img_buf, card_width, card_height, bg_color, text_color);
            text_processor.draw_text(|x, y, coverage| {
                (draw_method)(x + x_offset as usize, y + y_offset as usize, coverage);
            });

            // Add the underline to 6 or 9.
            if number == 6 || number == 9 {
                drop(draw_method);

                fill_rect(
                    &mut img_buf,
                    card_width,
                    card_height,
                    x_offset,
                    text_height + y_offset + UNDERLINE_PX_HEIGHT,
                    text_width,
                    UNDERLINE_PX_HEIGHT,
                    text_color,
                );
            }
        }

        RgbaImage::from_raw(card_width, card_height, img_buf)
            .expect("image buffer size should be correct")
    }

    fn setup_process_text(&self, text: impl AsRef<str>) -> TextProcessor<'_> {
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(
            &[&self.font],
            &TextStyle::new(text.as_ref(), self.font_size, 0),
        );

        let glyphs = layout.glyphs();
        let glyphs_bb = GlyphsBB::calculate(glyphs);

        TextProcessor {
            layout,
            glyphs_bb,
            font: &self.font,
        }
    }

    #[allow(unused)]
    fn process_text(
        &self,
        text: impl AsRef<str>,
        bg_color: Rgb<u8>,
        text_color: Rgb<u8>,
    ) -> RgbaImage {
        // Prepare text processor
        let text_processor = self.setup_process_text(text);

        // Create an image buffer
        let (img_width, img_height) = text_processor.img_size();
        let mut img_buf = filled_rgba_img_buf(img_width, img_height, bg_color);

        // Draw text
        text_processor.draw_text(common_draw_method(
            &mut img_buf,
            img_width,
            img_height,
            bg_color,
            text_color,
        ));

        RgbaImage::from_raw(img_width, img_height, img_buf)
            .expect("image buffer size should be correct")
    }

    #[allow(unused)]
    fn get_max_size(&self, numbers: impl IntoIterator<Item = u8>) -> (u32, u32) {
        let mut w_max = 0;
        let mut h_max = 0;

        for num in numbers {
            let (w, h) = self.get_text_size(num.to_string());

            w_max = w_max.max(w);
            h_max = h_max.max(h);
        }

        (w_max, h_max)
    }

    #[allow(unused)]
    fn get_text_size(&self, text: impl AsRef<str>) -> (u32, u32) {
        self.setup_process_text(text).img_size()
    }
}

struct TextProcessor<'a> {
    layout: Layout,
    glyphs_bb: GlyphsBB,
    font: &'a fontdue::Font,
}

impl TextProcessor<'_> {
    fn img_size(&self) -> (u32, u32) {
        (self.glyphs_bb.width, self.glyphs_bb.height)
    }

    fn draw_text(&self, mut f: impl FnMut(usize, usize, u8)) {
        let y_min = self.glyphs_bb.y_min;
        let mut next_x0 = 0;

        for g in self.layout.glyphs() {
            let (metrics, char_data) = self.font.rasterize_config(g.key);

            let char_width = metrics.width;
            let char_height = metrics.height;

            let y0 = g.y as usize - y_min as usize;
            let x0 = next_x0;
            next_x0 += char_width;

            for y_offset in 0..char_height {
                for x_offset in 0..char_width {
                    let coverage = char_data[x_offset + y_offset * char_width];

                    (f)(x0 + x_offset, y0 + y_offset, coverage)
                }
            }
        }
    }
}

/// Glyphs' Bounding Box
#[derive(Debug, Clone, Copy)]
struct GlyphsBB {
    _x_min: u32,
    y_min: u32,
    width: u32,
    height: u32,
}

impl GlyphsBB {
    fn calculate<'a>(glyphs: impl IntoIterator<Item = &'a GlyphPosition>) -> Self {
        let mut x_min = u32::MAX;
        let mut y_min = u32::MAX;
        let mut x_max = 0;
        let mut y_max = 0;

        let mut prev_x2 = None;
        let mut x_gaps = 0;

        for glyph in glyphs {
            let x1 = glyph.x as u32;
            let y1 = glyph.y as u32;
            let x2 = x1 + glyph.width as u32;
            let y2 = y1 + glyph.height as u32;

            if let Some(prev_x2) = prev_x2 {
                let gap = x1 - prev_x2;
                x_gaps += gap;
            }
            prev_x2 = Some(x2);

            x_min = x_min.min(x1);
            y_min = y_min.min(y1);
            x_max = x_max.max(x2);
            y_max = y_max.max(y2);
        }

        let width = x_max - x_min + 1 - x_gaps;
        let height = y_max - y_min + 1;

        Self {
            _x_min: x_min,
            y_min,
            width,
            height,
        }
    }
}

fn filled_rgba_img_buf(width: u32, height: u32, color: Rgb<u8>) -> Vec<u8> {
    let mut buf = vec![u8::MAX; width as usize * height as usize * 4];

    for pixel in buf.chunks_exact_mut(4) {
        pixel[0..3].copy_from_slice(&color.0);
    }

    buf
}

fn common_draw_method(
    rgba_img_buf: &mut [u8],
    width: u32,
    _height: u32,
    bg_color: Rgb<u8>,
    text_color: Rgb<u8>,
) -> impl FnMut(usize, usize, u8) + '_ {
    move |x, y, coverage| {
        if coverage == 0 {
            // Assuming that the buffer is pre-filled with background color.
            return;
        }

        let idx = caluclate_rgba_img_idx(width, x, y);

        for ((v, bg_color), text_color) in (&mut rgba_img_buf[idx..idx + 3])
            .iter_mut()
            .zip(bg_color.0)
            .zip(text_color.0)
        {
            *v = blend_color(bg_color, text_color, coverage);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    rgba_img_buf: &mut [u8],
    img_width: u32,
    img_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgb<u8>,
) {
    if x >= img_width || y >= img_height {
        return;
    }

    for y in y..(y + height).min(img_height) {
        for x in x..(x + width).min(img_width) {
            let idx = caluclate_rgba_img_idx(img_width, x as usize, y as usize);
            rgba_img_buf[idx..idx + 3].copy_from_slice(&color.0[..]);
        }
    }
}

fn caluclate_rgba_img_idx(width: u32, x: usize, y: usize) -> usize {
    let idx = x + y * width as usize;
    idx * 4
}

fn blend_color(bg_color: u8, text_color: u8, ratio: u8) -> u8 {
    let (bg_color, text_color, alpha) = (bg_color as f32, text_color as f32, ratio as f32);

    let text_ratio = alpha / u8::MAX as f32;
    let bg_ratio = 1.0 - text_ratio;

    let color = bg_color * bg_ratio + text_color * text_ratio;
    color as u8
}

fn into_bevy_image(image: RgbaImage) -> Image {
    Image::new_fill(
        Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &image.into_vec(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    )
}
