//! Anti-aliased text, rasterised at the size it appears on screen so it stays sharp at any
//! interface scale.

use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

struct Glyph {
    texture: Option<Texture2D>,
    /// Placement and advance in screen pixels (of the rasterisation).
    xmin: f32,
    ymin: f32,
    width: f32,
    height: f32,
    advance: f32,
}

pub struct TextRenderer {
    font: fontdue::Font,
    /// Font size in interface pixels.
    px: f32,
    /// Screen pixels per interface pixel; glyphs are rasterised at `px * scale`.
    scale: Cell<f32>,
    glyphs: RefCell<HashMap<char, Glyph>>,
}

impl TextRenderer {
    pub fn new(ttf: &[u8], px: f32) -> TextRenderer {
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).unwrap();
        TextRenderer { font, px, scale: Cell::new(1.0), glyphs: RefCell::new(HashMap::new()) }
    }

    /// Sets how many screen pixels an interface pixel covers (re-rasterising on change).
    pub fn set_scale(&self, scale: f32) {
        if (scale - self.scale.get()).abs() > 1e-4 {
            self.scale.set(scale);
            self.glyphs.borrow_mut().clear();
        }
    }

    fn rasterise(&self, c: char) -> Glyph {
        let (m, alpha) = self.font.rasterize(c, self.px * self.scale.get());
        let texture = (m.width > 0 && m.height > 0).then(|| {
            let rgba: Vec<u8> = alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            Texture2D::from_rgba8(m.width as u16, m.height as u16, &rgba)
        });
        Glyph {
            texture,
            xmin: m.xmin as f32,
            ymin: m.ymin as f32,
            width: m.width as f32,
            height: m.height as f32,
            advance: m.advance_width,
        }
    }

    /// Draws one line with its baseline at `baseline` (interface pixels).
    pub fn draw(&self, s: &str, x: i32, baseline: i32, colour: Color) {
        let scale = self.scale.get();
        let mut glyphs = self.glyphs.borrow_mut();
        let mut pen = x as f32;
        for c in s.chars() {
            let g = glyphs.entry(c).or_insert_with(|| self.rasterise(c));
            if let Some(t) = &g.texture {
                // Snap to screen pixels so glyphs stay crisp.
                let gx = ((pen + g.xmin / scale) * scale).round() / scale;
                let gy = ((baseline as f32 - (g.ymin + g.height) / scale) * scale).round() / scale;
                let size = vec2(g.width / scale, g.height / scale);
                draw_texture_ex(t, gx, gy, colour, DrawTextureParams { dest_size: Some(size), ..Default::default() });
            }
            pen += g.advance / scale;
        }
    }
}
