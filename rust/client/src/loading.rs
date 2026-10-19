//! The loading screen (`FrmMoteur2D.Afficher_Image_Chargement`): a random artwork from
//! `Images/Chargement/`, a progress bar along its bottom band and a bubble saying what loads.
//!
//! Loading runs as a coroutine (which macroquad requires to be `Send`, hence the mutex) that reports through [`Progress`] while the main loop redraws
//! the screen every frame, so nothing flickers while files download.

use crate::assets::ROOT;
use crate::text::TextRenderer;
use macroquad::prelude::*;
use sim::ini::Ini;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    pct: f64,
    message: String,
    art: Option<Texture2D>,
    cursor: Option<Texture2D>,
    /// `Chargement.ini [Chargement_2]` labels by number.
    labels: Ini,
}

/// Shared between the loading coroutine and the screen.
#[derive(Clone, Default)]
pub struct Progress(Arc<Mutex<State>>);

impl Progress {
    pub fn set(&self, pct: f64, message: &str) {
        let mut s = self.0.lock().unwrap();
        s.pct = pct;
        if s.message != message {
            s.message = message.to_owned();
        }
    }

    /// `EtiquetteChargement<n>` of the game-loading section.
    pub fn label(&self, n: usize) -> String {
        self.0.lock().unwrap().labels.str_or("Chargement_2", &format!("EtiquetteChargement{n}"), "").to_owned()
    }

    /// Fetches the artwork, cursor and labels; any of them may fail, the bar still works.
    pub async fn load_decoration(&self, manifest: &str, lang: &str) {
        let arts: Vec<&str> = manifest.lines().filter(|p| p.starts_with("Images/Chargement/")).collect();
        if !arts.is_empty() {
            rand::srand((miniquad::date::now() * 1000.0) as u64);
            let pick = arts[rand::gen_range(0, arts.len())];
            let art = load_texture(&format!("{ROOT}/{pick}")).await.ok();
            self.0.lock().unwrap().art = art;
        }
        let cursor = load_texture(&format!("{ROOT}/Images/Curseurs/ArrowCursor.png")).await.ok();
        let labels = load_string(&format!("{ROOT}/Donnees/{lang}/Chargement.ini")).await.unwrap_or_default();
        let mut s = self.0.lock().unwrap();
        s.cursor = cursor;
        s.labels = Ini::parse(&labels);
        for t in s.art.iter().chain(&s.cursor) {
            t.set_filter(FilterMode::Nearest);
        }
    }
}

pub struct LoadingScreen {
    pub progress: Progress,
    font: TextRenderer,
}

impl LoadingScreen {
    pub fn new() -> LoadingScreen {
        LoadingScreen {
            progress: Progress::default(),
            font: TextRenderer::new(include_bytes!("../fonts/LiberationSans-Bold.ttf"), 14.0, 13.0),
        }
    }

    /// Draws one frame: the artwork fitted to the window (letterboxed, like the original's
    /// 4:3 screen), the bar behind its bottom band, the cursor and the message bubble.
    pub fn draw(&self) {
        let s = self.progress.0.lock().unwrap();
        set_default_camera();
        clear_background(BLACK);
        let (w, h) = (screen_width(), screen_height());
        let scale = (w / 1024.0).min(h / 768.0);
        let (iw, ih) = (1024.0 * scale, 768.0 * scale);
        let (ix, iy) = (((w - iw) / 2.0).floor(), ((h - ih) / 2.0).floor());
        // The original's fill colour is `CLng(Pourcentage * 2.5 * 255)` read as &HBBGGRR. It only
        // changed at a few milestones there; recomputed continuously it flickers through
        // red, green and black, so use its value at 100% (green).
        let v = (100.0f64 * 2.5 * 255.0).round() as u32;
        let colour = Color::from_rgba((v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8, 255);
        let band = 97.0 * scale;
        draw_rectangle(ix, iy + ih - band, iw * s.pct as f32 / 100.0, band, colour);
        if let Some(art) = &s.art {
            draw_texture_ex(art, ix, iy, WHITE, DrawTextureParams { dest_size: Some(vec2(iw, ih)), ..Default::default() });
        }
        let (mx, my) = mouse_position();
        if let Some(c) = &s.cursor {
            let src = Rect::new(0.0, 0.0, 24.0, 24.0);
            draw_texture_ex(c, mx, my, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
        }
        if !s.message.is_empty() {
            // Same box as the HUD tooltips; a line starting with '&' is red.
            let lines: Vec<&str> = s.message.split('#').collect();
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * 7.0;
            let height = lines.len() as f32 * 16.0;
            let x = (mx + 24.0).min(w - width - 2.0).max(0.0);
            let y = (my + 20.0).min(h - height - 2.0).max(0.0);
            draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, Color::from_rgba(255, 255, 192, 255));
            draw_rectangle_lines(x - 2.0, y - 2.0, width + 4.0, height + 4.0, 1.0, BLACK);
            for (i, l) in lines.iter().enumerate() {
                let (l, c) = match l.strip_prefix('&') {
                    Some(l) => (l, RED),
                    None => (*l, BLACK),
                };
                self.font.draw(l, x as i32, y as i32 + 11 + 16 * i as i32, c);
            }
        }
    }
}
