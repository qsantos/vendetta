//! Aliased text like GDI's on the original's 16-bit DirectDraw surfaces: glyphs are
//! rasterised once and thresholded to fully opaque pixels.

use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

struct Glyph {
    texture: Option<Texture2D>,
    xmin: i32,
    ymin: i32,
    height: i32,
    advance: f32,
}

pub struct TextRenderer {
    font: fontdue::Font,
    px: f32,
    /// Size used for advances; GDI's hinted Arial is taller than its spacing suggests.
    advance_px: f32,
    glyphs: RefCell<HashMap<char, Glyph>>,
}

impl TextRenderer {
    pub fn new(ttf: &[u8], px: f32, advance_px: f32) -> TextRenderer {
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).unwrap();
        TextRenderer { font, px, advance_px, glyphs: RefCell::new(HashMap::new()) }
    }

    fn rasterise(&self, c: char) -> Glyph {
        let (m, alpha) = self.font.rasterize(c, self.px);
        let texture = (m.width > 0 && m.height > 0).then(|| {
            let rgba: Vec<u8> = alpha
                .iter()
                .flat_map(|&a| if a >= 96 { [255, 255, 255, 255] } else { [0, 0, 0, 0] })
                .collect();
            let t = Texture2D::from_rgba8(m.width as u16, m.height as u16, &rgba);
            t.set_filter(FilterMode::Nearest);
            t
        });
        let advance = self.font.metrics(c, self.advance_px).advance_width.round();
        Glyph { texture, xmin: m.xmin, ymin: m.ymin, height: m.height as i32, advance }
    }

    /// Draws one line with its baseline at `baseline`.
    pub fn draw(&self, s: &str, x: i32, baseline: i32, colour: Color) {
        let mut glyphs = self.glyphs.borrow_mut();
        let mut pen = x as f32;
        for c in s.chars() {
            let g = glyphs.entry(c).or_insert_with(|| self.rasterise(c));
            if let Some(t) = &g.texture {
                let gx = (pen + g.xmin as f32).round();
                let gy = (baseline - g.ymin - g.height) as f32;
                draw_texture(t, gx, gy, colour);
            }
            pen += g.advance;
        }
    }
}
