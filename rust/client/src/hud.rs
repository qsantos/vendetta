//! The in-game HUD (`ClsInterface.Afficher`), at the original's 1024×768 positions.

mod bars;
mod bubbles;
mod buttons;
mod windows;

pub use buttons::Input;
pub use windows::WindowStrings;

use crate::assets::Textures;
use crate::layout::{ax, ay, cx};
use crate::text::TextRenderer;
use macroquad::prelude::*;
use sim::files::Files;
use sim::ini::Ini;
use sim::perso::{Perso, action};
use sim::rules::Rules;
use sim::vb::clng;
use sim::world::{Order, World};

pub const INFOS: &str = "Images/Boutons/Infos.bmp";
pub const DEPLACEMENT: &str = "Images/Boutons/Deplacement.bmp";
pub const OBJETS: &str = "Images/Boutons/Objets.bmp";
pub const CURSOR: &str = "Images/Curseurs/ArrowCursor.bmp";

pub(crate) const WHITE_: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const GREY: Color = Color::new(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0);
pub(crate) const LIGHT_GREY: Color = Color::new(192.0 / 255.0, 192.0 / 255.0, 192.0 / 255.0, 1.0);
pub(crate) const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
pub(crate) const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

/// Text and labels the HUD needs, from the language's ini files.
pub struct Strings {
    fps: String,
    pub actions: Vec<String>,
    bars: [String; 4],
    pub demolish: String,
    pub coins: String,
    pub emperor_only: String,
    pub info_tips: Vec<String>,
    /// `BarresStatutSeuilMessage1/2`, around the threshold tooltip.
    pub threshold: [String; 2],
    pub closed: [String; 2],
    pub abandoned: [String; 2],
    pub percent: String,
    pub lives_left: [String; 2],
    pub move_tips: Vec<String>,
}

impl Strings {
    pub fn load(files: &Files, lang: &str) -> Strings {
        let ini = |name: &str| Ini::parse(files.text(&format!("Donnees/{lang}/{name}")).unwrap_or(""));
        let interface = ini("Interface.ini");
        let globals = ini("Parametres_Globales.ini");
        let n = interface.find("NombreMessagesActions").map(sim::ini::vb_val).unwrap_or(0.0) as usize;
        Strings {
            // The original's reader trims the trailing space: "FPS :12".
            fps: interface.find("FPSMessage").unwrap_or("FPS :").trim().to_owned(),
            actions: (1..=n)
                .map(|i| interface.find(&format!("MessageAction{i}")).unwrap_or("").to_owned())
                .collect(),
            bars: ["EtiquetteVie", "EtiquetteEnergie", "EtiquetteMagie", "EtiquetteMoral"]
                .map(|k| globals.find(k).unwrap_or("").to_owned()),
            demolish: interface.find("BoutonDemolirEtiquette").unwrap_or("").to_owned(),
            coins: globals.find("EtiquettePieces").unwrap_or("").to_owned(),
            emperor_only: globals.find("EtiquetteBatimentEmpereur").unwrap_or("").to_owned(),
            closed: [globals.find("EtiquetteFermeM").unwrap_or("").to_owned(), globals.find("EtiquetteFermeF").unwrap_or("").to_owned()],
            abandoned: [
                globals.find("EtiquetteAbandonneM").unwrap_or("").to_owned(),
                globals.find("EtiquetteAbandonneF").unwrap_or("").to_owned(),
            ],
            percent: globals.find("EtiquettePourcent").unwrap_or("%").to_owned(),
            lives_left: [
                globals.find("EtiquetteVieRestante").unwrap_or("").to_owned(),
                globals.find("EtiquetteViesRestantes").unwrap_or("").to_owned(),
            ],
            threshold: ["BarresStatutSeuilMessage1", "BarresStatutSeuilMessage2"].map(|k| interface.find(k).unwrap_or("").to_owned()),
            info_tips: (1..=9).map(|i| interface.find(&format!("BoutonInfosInfoBulle{i}")).unwrap_or("").to_owned()).collect(),
            move_tips: (1..=23)
                .map(|i| interface.find(&format!("BoutonDeplacementInfoBulle{i}")).unwrap_or("").to_owned())
                .collect(),
        }
    }
}

pub struct Hud {
    pub strings: Strings,
    font: TextRenderer,
    /// Horizontal offset of the auto-hidden move buttons: -52 (hidden) to 0 (shown).
    move_slide: i32,
    /// Same for the build buttons: -80 to 0.
    build_slide: i32,
    /// Building type being placed (`NoBatiment`), from a build button.
    pub placing: Option<usize>,
    /// Info buttons' `Enabled` (window open / paused).
    pub info_enabled: [bool; 9],
    windows: windows::Windows,
    /// Radial buttons: centre while active, category (0 info, 1 build) and angle per category.
    radial: Option<(i32, i32)>,
    radial_category: usize,
    radial_angle: [f32; 2],
    /// The left release that follows a click closing the radial buttons is not a map click.
    swallow_release: bool,
    /// Automatic restoring of the status bars.
    auto: bars::AutoRestore,
    /// Build buttons' (state, image), see `refresh_build_buttons`.
    build_cache: Vec<(i32, String)>,
    fps: u32,
    frames: u32,
    fps_timer: f64,
}

/// Images the HUD needs.
pub fn images(rules: &Rules) -> Vec<String> {
    let mut v: Vec<String> = [INFOS, DEPLACEMENT, OBJETS, CURSOR].map(String::from).to_vec();
    v.extend(buttons::images(rules));
    v.extend(windows::images(rules));
    v
}

/// GDI `Rectangle` outline: covers x∈[l, r-1], y∈[t, b-1].
pub(crate) fn outline(l: i32, t: i32, r: i32, b: i32, c: Color) {
    if r - l <= 0 || b - t <= 0 {
        return;
    }
    let (l, t, r, b) = (l as f32, t as f32, r as f32, b as f32);
    draw_rectangle(l, t, r - l, 1.0, c);
    draw_rectangle(l, b - 1.0, r - l, 1.0, c);
    draw_rectangle(l, t, 1.0, b - t, c);
    draw_rectangle(r - 1.0, t, 1.0, b - t, c);
}

/// GDI `Rectangle` with no pen: the fill covers x∈[l, r-2], y∈[t, b-2].
fn fill_no_pen(l: i32, t: i32, r: i32, b: i32, c: Color) {
    if r - l - 1 > 0 && b - t - 1 > 0 {
        draw_rectangle(l as f32, t as f32, (r - l - 1) as f32, (b - t - 1) as f32, c);
    }
}

/// `Afficher_Barre_Statut` (horizontal): black framed box, white `min2` part, coloured `min1`
/// part. `colour = None` picks red/orange/green from the ratio.
pub fn status_bar(min1: f64, min2: f64, max: f64, x: i32, y: i32, w: i32, h: i32, colour: Option<Color>) {
    let (mut x, mut y, mut w, mut h) = (x, y, w, h);
    if x < 0 {
        w += x;
        x = 0;
    }
    if y < 0 {
        h += y;
        y = 0;
    }
    if w <= 0 || h <= 0 {
        return;
    }
    draw_rectangle(x as f32, y as f32, w as f32, h as f32, BLACK);
    if max <= 0.0 {
        return;
    }
    let right = |v: f64| x + clng(w as f64 * v / max) as i32;
    if min2 > 0.0 {
        fill_no_pen(x + 1, y + 1, right(min2), y + h, WHITE_);
    }
    let r = min1 / max;
    let c = colour.unwrap_or(if r < 0.1 {
        rgb(128, 0, 0)
    } else if r < 0.25 {
        rgb(255, 0, 0)
    } else if r < 0.5 {
        rgb(255, 128, 0)
    } else {
        rgb(0, 192, 0)
    });
    fill_no_pen(x + 1, y + 1, right(min1), y + h, c);
}

/// `Afficher_Barre_Statut` with `Vertical`: the bar fills from the bottom up.
pub fn status_bar_v(min1: f64, min2: f64, max: f64, x: i32, y: i32, w: i32, h: i32, colour: Option<Color>) {
    if w <= 0 || h <= 0 {
        return;
    }
    draw_rectangle(x as f32, y as f32, w as f32, h as f32, BLACK);
    if max <= 0.0 {
        return;
    }
    let top = |v: f64| y + h - clng(h as f64 * v / max) as i32;
    if min2 > 0.0 {
        fill_no_pen(x, top(min2), x + w, y + h, WHITE_);
    }
    let c = colour.unwrap_or(rgb(0, 192, 0));
    fill_no_pen(x + 1, top(min1), x + w, y + h, c);
}

/// The original's font: Arial Bold 10 pt at 96 dpi (Liberation Sans Bold has its metrics).
const FONT_PX: f32 = 10.0 * 96.0 / 72.0;

impl Hud {
    /// Screen pixels per interface pixel, for sharp text.
    pub fn set_scale(&self, scale: f32) {
        self.font.set_scale(scale);
    }

    pub fn new(strings: Strings, window_strings: WindowStrings) -> Hud {
        let font = TextRenderer::new(include_bytes!("../fonts/LiberationSans-Bold.ttf"), FONT_PX);
        Hud {
            strings,
            font,
            move_slide: -52,
            build_slide: -80,
            placing: None,
            // The building window starts enabled (`Actionner_Boutons_Infos 0`).
            info_enabled: [true, false, false, false, false, false, false, false, false],
            windows: windows::Windows::new(window_strings),
            radial: None,
            radial_category: 0,
            radial_angle: [-1.57; 2],
            swallow_release: false,
            build_cache: Vec::new(),
            auto: bars::AutoRestore::load(),
            fps: 0,
            frames: 0,
            fps_timer: get_time(),
        }
    }

    /// `ClsIntMessage.Afficher`: `#` breaks lines (16 px apart); a line starting with `&` is red,
    /// with `$` blue. `(x, y)` is the top of the text cell, like GDI `DrawText`.
    pub fn text(&self, x: i32, y: i32, s: &str, colour: Color) {
        for (i, line) in s.split('#').enumerate() {
            let (line, c) = if let Some(l) = line.strip_prefix('&') {
                (l, RED)
            } else if let Some(l) = line.strip_prefix('$') {
                (l, BLUE)
            } else {
                (line, colour)
            };
            // GDI places the baseline 11 px below the top of the text cell.
            self.font.draw(line, x, y + 11 + 16 * i as i32, c);
        }
    }

    /// The HUD for one rendered frame, after the world: draws it, handles its buttons (adding
    /// orders) and draws the hovered tooltip. Returns whether the mouse is on the interface.
    pub fn frame(&mut self, world: &World, rules: &Rules, textures: &Textures, player: usize, input: &Input, orders: &mut Vec<Order>) -> bool {
        self.frames += 1;
        if get_time() - self.fps_timer >= 1.0 {
            self.fps = self.frames;
            self.frames = 0;
            self.fps_timer = get_time();
        }
        let p = &world.persos[player];

        // Status bars: Moral, Magie, Energie, Vie (bottom to top), then their contours.
        use bars::BAR_Y;
        for i in (0..4).rev() {
            let (px, py) = (10, ay(BAR_Y[i]));
            let max = p.bonus_max[i] as f64;
            let v = p.bars[i];
            status_bar(v.max(0.0), v, max, px - 3, py, 136, 16, None);
            self.text(px, py, &format!("{} :", self.strings.bars[i]), WHITE_);
            let score = format!("{} / {}", v.floor(), p.bonus_max[i]);
            self.text(px - 3 + 108 - score.chars().count() as i32 * 4, py, &score, WHITE_);
        }
        self.draw_bar_contours();

        self.draw_statistics(world, rules);
        self.draw_epoch(world, rules, p);
        self.draw_object_slots(textures);
        self.refresh_build_buttons(world, rules, player);
        self.slide_buttons(world, rules, player, input.mouse);
        self.draw_move_buttons(textures, world, player);
        self.draw_build_buttons(textures);
        self.text(152, ay(745), &self.action_message(rules, p), WHITE_);
        self.draw_info_buttons(textures);
        let mut tip = String::new();
        if self.swallow_release && input.left_released {
            self.swallow_release = false;
            return true;
        }
        if self.radial.is_some() {
            self.windows(world, rules, textures, player, &Input { left_pressed: false, right_pressed: false, ..*input }, orders, &mut tip);
            self.radial_buttons(world, rules, textures, player, input);
            return true;
        }
        let on_window = self.windows(world, rules, textures, player, input, orders, &mut tip);
        self.interact(world, rules, player, input, orders, on_window, tip)
    }

    /// What is drawn over everything: the building footprint, the cursor and the FPS.
    /// `placement` is the cursor type from [`Hud::draw_placement`].
    pub fn overlay(&mut self, world: &World, rules: &Rules, textures: &Textures, player: usize, input: &Input, placement: Option<i32>, hover: Option<(i32, usize)>) {
        let cursor = placement.unwrap_or(match hover {
            Some((c, _)) if c <= 17 => c,
            _ => 0,
        });
        if let Some(tex) = textures.get(CURSOR) {
            draw_texture_ex(
                tex,
                input.mouse.0 as f32,
                input.mouse.1 as f32,
                WHITE,
                DrawTextureParams { source: Some(Rect::new(24.0 * cursor as f32, 0.0, 24.0, 24.0)), ..Default::default() },
            );
        }
        if let (None, Some((code, hit))) = (placement, hover) {
            self.terrain_bubble(world, rules, player, code, hit, input.mouse);
        }
        self.text(ax(940), 0, &format!("{}{:02}", self.strings.fps, self.fps), WHITE_);
    }

    /// `Afficher_Stastistiques`: one line per active fief, bottom-aligned at y=745.
    fn draw_statistics(&self, world: &World, rules: &Rules) {
        let mut lines = Vec::new();
        for (i, (f, c)) in world.fiefs.iter().zip(&world.castles).enumerate() {
            let alive = world.alive_citizens(i);
            if alive == 0 && !c.visible() {
                continue;
            }
            let mut line = String::new();
            if alive == 0 {
                line.push('&');
            }
            line += &format!("({}) {} {}/{}", f.team, rules.peoples[f.people].name, alive, f.citizens);
            if c.visible() {
                line += &format!(" - {:02.0}%", (c.life / c.max_life as f64 * 100.0).round_ties_even());
            }
            lines.push(line);
        }
        let y0 = ay(745) - 16 * lines.len() as i32;
        self.text(ax(816), y0, &format!("#{}", lines.join("#")), WHITE_);
    }

    /// `Afficher_Epoques`: grey box with the fief's epoch name, centred on x=528.
    fn draw_epoch(&self, world: &World, rules: &Rules, p: &Perso) {
        // TODO: progress bar while changing epoch, emperor life bar
        draw_rectangle(cx(438) as f32, 0.0, 180.0, 18.0, BLACK);
        outline(cx(438), 0, cx(618), 18, GREY);
        let name = &rules.epochs[world.fiefs[p.fief].epoch].name;
        let x = cx(528) - clng(name.chars().count() as f64 * 3.25) as i32;
        self.text(x, 0, name, GREY);
    }

    /// The 9 object shortcut slots ("1".."9") at the top left.
    fn draw_object_slots(&self, textures: &Textures) {
        let Some(tex) = textures.get(OBJETS) else { return };
        for i in 0..9 {
            // TODO: bound objects, Alt/Shift rows, enabled state
            let src = Rect::new(32.0 * i as f32, 0.0, 32.0, 32.0);
            draw_texture_ex(tex, (56 + 32 * i) as f32, 0.0, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
        }
    }

    /// `Afficher_Action`: the player's current action.
    fn action_message(&self, rules: &Rules, p: &Perso) -> String {
        // Harvesting shows the skill and the carried quantity instead.
        if (p.action == 2 || p.action == 102) && p.current_resource >= 0 {
            let r = p.current_resource as usize;
            return format!(
                "{} : {} / {}",
                rules.resources[r].skill_name,
                p.resource(r).floor(),
                p.max_resource(r)
            );
        }
        let index = match p.action {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            6 => 4,
            7 => 5,
            action::AT_HOME => 6,
            101 => 7,
            102 => 8,
            201 => 9,
            202 => 10,
            203 => 11,
            204 => 12,
            205 => 13,
            206 | 207 => 14,
            300 => 15,
            301 => 16,
            302 => 17,
            400 => 18,
            500 => 19,
            600 => 20,
            _ => 21,
        };
        // TODO: per-action suffixes
        self.strings.actions.get(index).cloned().unwrap_or_default()
    }
}
