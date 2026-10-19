//! Anti-aliased text, rasterised at the size it appears on screen so it stays sharp at any
//! interface scale. Glyphs are packed into a shared texture (an atlas), so that a line of text
//! is drawn in one batch rather than one draw call per letter.

use crate::atlas::Shelf;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// Side of an atlas texture, in pixels.
const ATLAS: u16 = 1024;

/// A texture glyphs are packed into.
struct Atlas {
    texture: Texture2D,
    shelf: Shelf,
}

impl Atlas {
    fn new() -> Atlas {
        let texture = Texture2D::from_rgba8(ATLAS, ATLAS, &vec![0; ATLAS as usize * ATLAS as usize * 4]);
        Atlas { texture, shelf: Shelf::new(ATLAS) }
    }
}

struct Glyph {
    /// The atlas holding it and where, None for blank glyphs.
    texture: Option<(Texture2D, Rect)>,
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
    /// Where new glyphs go; a full one is replaced, its glyphs keeping it.
    atlas: RefCell<Atlas>,
}

impl TextRenderer {
    pub fn new(ttf: &[u8], px: f32) -> TextRenderer {
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).unwrap();
        let (glyphs, atlas) = (RefCell::new(HashMap::new()), RefCell::new(Atlas::new()));
        TextRenderer { font, px, scale: Cell::new(1.0), glyphs, atlas }
    }

    /// Sets how many screen pixels an interface pixel covers (re-rasterising on change).
    pub fn set_scale(&self, scale: f32) {
        if (scale - self.scale.get()).abs() > 1e-4 {
            self.scale.set(scale);
            self.glyphs.borrow_mut().clear();
            *self.atlas.borrow_mut() = Atlas::new();
        }
    }

    fn rasterise(&self, c: char) -> Glyph {
        let (m, alpha) = self.font.rasterize(c, self.px * self.scale.get());
        let texture = (m.width > 0 && m.height > 0).then(|| {
            let (w, h) = (m.width as u16, m.height as u16);
            let mut atlas = self.atlas.borrow_mut();
            let (x, y) = atlas.shelf.place(w, h).unwrap_or_else(|| {
                *atlas = Atlas::new();
                atlas.shelf.place(w, h).expect("a glyph bigger than an atlas")
            });
            let bytes: Vec<u8> = alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            let image = Image { bytes, width: w, height: h };
            atlas.texture.update_part(&image, x as i32, y as i32, w as i32, h as i32);
            (atlas.texture.clone(), Rect::new(x as f32, y as f32, w as f32, h as f32))
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
            if let Some((t, source)) = &g.texture {
                // Snap to screen pixels so glyphs stay crisp.
                let gx = ((pen + g.xmin / scale) * scale).round() / scale;
                let gy = ((baseline as f32 - (g.ymin + g.height) / scale) * scale).round() / scale;
                let size = vec2(g.width / scale, g.height / scale);
                let params = DrawTextureParams { dest_size: Some(size), source: Some(*source), ..Default::default() };
                draw_texture_ex(t, gx, gy, colour, params);
            }
            pen += g.advance / scale;
        }
    }
}
