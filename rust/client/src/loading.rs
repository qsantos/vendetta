//! The loading screen (`FrmMoteur2D.Afficher_Image_Chargement`): a random artwork from
//! `Images/Chargement/`, a progress bar along the bottom and a bubble saying what loads.

use crate::assets::ROOT;
use crate::text::TextRenderer;
use macroquad::prelude::*;
use sim::ini::Ini;

pub struct LoadingScreen {
    art: Option<Texture2D>,
    cursor: Option<Texture2D>,
    font: TextRenderer,
    /// `Chargement.ini [Chargement_2]` labels by number.
    labels: Ini,
    last_draw: f64,
}

impl LoadingScreen {
    /// Shows the empty bar right away, then fetches the artwork, cursor and labels; any of
    /// them may fail, the bar still works.
    pub async fn new(manifest: &str, lang: &str) -> LoadingScreen {
        let mut screen = LoadingScreen {
            art: None,
            cursor: None,
            font: TextRenderer::new(include_bytes!("../fonts/LiberationSans-Bold.ttf"), 14.0, 13.0),
            labels: Ini::default(),
            last_draw: 0.0,
        };
        screen.progress(0.0, "").await;
        let arts: Vec<&str> = manifest.lines().filter(|p| p.starts_with("Images/Chargement/")).collect();
        if !arts.is_empty() {
            rand::srand((miniquad::date::now() * 1000.0) as u64);
            let pick = arts[rand::gen_range(0, arts.len())];
            screen.art = load_texture(&format!("{ROOT}/{pick}")).await.ok();
        }
        screen.cursor = load_texture(&format!("{ROOT}/Images/Curseurs/ArrowCursor.png")).await.ok();
        for t in screen.art.iter().chain(&screen.cursor) {
            t.set_filter(FilterMode::Nearest);
        }
        let labels = load_string(&format!("{ROOT}/Donnees/{lang}/Chargement.ini")).await.unwrap_or_default();
        screen.labels = Ini::parse(&labels);
        screen
    }

    /// Shows a fatal error instead of the game, forever.
    pub async fn fail(&mut self, error: &str) -> ! {
        loop {
            self.last_draw = 0.0;
            self.progress(0.0, &format!("&Erreur#{error}")).await;
        }
    }

    /// `EtiquetteChargement<n>` of the game-loading section.
    pub fn label(&self, n: usize) -> String {
        self.labels.str_or("Chargement_2", &format!("EtiquetteChargement{n}"), "").to_owned()
    }

    /// Shows `pct` (0..100) and `message`, at most 30 times a second so loading stays fast.
    pub async fn progress(&mut self, pct: f64, message: &str) {
        if get_time() - self.last_draw < 1.0 / 30.0 {
            return;
        }
        self.last_draw = get_time();
        set_default_camera();
        clear_background(BLACK);
        let (w, h) = (screen_width(), screen_height());
        let scale = w / 1024.0;
        let top = (h - 768.0 * scale) / 2.0;
        // The original's fill colour: `CLng(Pourcentage * 2.5 * 255)` read as &HBBGGRR.
        let v = (pct * 2.5 * 255.0).round() as u32;
        let colour = Color::from_rgba((v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8, 255);
        let bar_top = h - 97.0 / 768.0 * h - top;
        draw_rectangle(0.0, bar_top, w * pct as f32 / 100.0, h - bar_top, colour);
        if let Some(art) = &self.art {
            draw_texture_ex(art, 0.0, top, WHITE, DrawTextureParams { dest_size: Some(vec2(w, 768.0 * scale)), ..Default::default() });
        }
        let (mx, my) = mouse_position();
        if let Some(c) = &self.cursor {
            let src = Rect::new(0.0, 0.0, 24.0, 24.0);
            draw_texture_ex(c, mx, my, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
        }
        if !message.is_empty() {
            // Same box as the HUD tooltips.
            let lines: Vec<&str> = message.split('#').collect();
            let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * 7.0;
            let height = lines.len() as f32 * 16.0;
            let x = (mx + 24.0).min(w - width - 2.0).max(0.0);
            let y = (my + 20.0).min(h - height - 2.0).max(0.0);
            draw_rectangle(x - 2.0, y - 2.0, width + 4.0, height + 4.0, Color::from_rgba(255, 255, 192, 255));
            draw_rectangle_lines(x - 2.0, y - 2.0, width + 4.0, height + 4.0, 1.0, BLACK);
            for (i, l) in lines.iter().enumerate() {
                self.font.draw(l, x as i32, y as i32 + 11 + 16 * i as i32, BLACK);
            }
        }
        next_frame().await;
    }
}
