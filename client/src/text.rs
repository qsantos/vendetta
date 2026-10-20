//! Anti-aliased text, rasterised at the size it appears on screen so it stays sharp at any
//! interface scale. Glyphs are packed into a shared texture (an atlas), so that a line of text
//! is drawn in one batch rather than one draw call per letter.

use crate::atlas::Shelf;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The font for the characters the interface's font lacks (Chinese, Japanese: see `set_fallback`).
static FALLBACK: OnceLock<fontdue::Font> = OnceLock::new();

/// Sets the font for the characters the others lack (the language's `Polices/<folder>.otf`),
/// before any of them is drawn; a font that does not load is left out.
pub fn set_fallback(bytes: &[u8]) {
    match fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
        Ok(font) => {
            let _ = FALLBACK.set(font);
        }
        Err(e) => {
            warn!("fallback font: {e}");
        }
    }
}

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

/// A text's length in character cells, where the original sizes or times text by its number of
/// characters: a full-width character (Chinese, Japanese, Korean) is about two Latin letters
/// wide, and says about as much.
pub fn columns(s: &str) -> usize {
    s.chars()
        .map(|c| match c as u32 {
            0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 => 2,
            _ => 1,
        })
        .sum()
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
        let font = match FALLBACK.get() {
            Some(fallback) if self.font.lookup_glyph_index(c) == 0 => fallback,
            _ => &self.font,
        };
        let (m, alpha) = font.rasterize(c, self.px * self.scale.get());
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

    /// A line's width in interface pixels.
    pub fn width(&self, s: &str) -> i32 {
        let scale = self.scale.get();
        let mut glyphs = self.glyphs.borrow_mut();
        let w: f32 = s.chars().map(|c| glyphs.entry(c).or_insert_with(|| self.rasterise(c)).advance / scale).sum();
        w.ceil() as i32
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
