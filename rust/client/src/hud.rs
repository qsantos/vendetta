//! The in-game HUD (`ClsInterface.Afficher`), at the original's 1024×768 positions.

mod bars;
mod bubbles;
mod buttons;
mod messages;
mod object_tooltip;
mod shortcuts;
mod temp_effects;
mod windows;

pub use buttons::Input;
pub use object_tooltip::TooltipLabels;
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
pub(crate) const GREY: Color = Color::new(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0);
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
    /// `BoutonRaccourciObjetEtiquette`: "Equiper Objets ".
    pub shortcut: String,
    pub closed: [String; 2],
    pub abandoned: [String; 2],
    pub percent: String,
    pub lives_left: [String; 2],
    /// `EtiquetteObjetVente` and `EtiquetteObjetVenteImpossible`, `EtiquetteObjetDepot` and
    /// `EtiquetteObjetDepotImpossible`: the bubble of a held object.
    pub sell: [String; 2],
    pub drop: [String; 2],
    pub move_tips: Vec<String>,
}

impl Strings {
    pub fn load(files: &Files, lang: &str) -> Strings {
        let ini = |name: &str| Ini::parse(files.text(&format!("Donnees/{lang}/{name}")).unwrap_or(""));
        let interface = ini("Interface.ini");
        let globals = ini("Parametres_Globales.ini");
        let n = interface.find("NombreMessagesActions").map(sim::ini::vb_val).unwrap_or(0.0) as usize;
        Strings {
            // The original's reader trims the trailing space ("FPS :12"); it is put back.
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
            sell: ["EtiquetteObjetVente", "EtiquetteObjetVenteImpossible"].map(|k| globals.find(k).unwrap_or("").to_owned()),
            drop: ["EtiquetteObjetDepot", "EtiquetteObjetDepotImpossible"].map(|k| globals.find(k).unwrap_or("").to_owned()),
            shortcut: interface.find("BoutonRaccourciObjetEtiquette").unwrap_or("").to_owned(),
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
    /// `FontDegats`: Arial Bold 8 pt, for slot labels, counters and damage numbers.
    small_font: TextRenderer,
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
    /// The build button hovered and since when, for its delayed details.
    build_hover: Option<(usize, f64)>,
    /// Labels of the object tooltips.
    tooltip_labels: object_tooltip::TooltipLabels,
    /// Object shortcut keys.
    shortcuts: shortcuts::Shortcuts,
    /// The message log at the top left.
    messages: messages::MessageLog,
    /// The recharge clocks over the shortcut slots.
    clocks: [Option<shortcuts::Clock>; 2],
    /// The bag or equipment object under the cursor, for binding it with a digit.
    hovered_object: Option<usize>,
    /// `FileFabricationObjets`: crafting slots queued in the own workshop.
    craft_queue: Vec<usize>,
    /// Automatic restoring of the status bars.
    auto: bars::AutoRestore,
    /// Build buttons' (state, image), see `refresh_build_buttons`.
    build_cache: Vec<(i32, String)>,
    /// The tick the per-tick caches (build buttons, fief list) were made at.
    cache_tick: Option<u64>,
    /// The fief list's text, made once per tick.
    statistics: String,
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
/// GDI `RoundRect(l, t, r, b, 10, 10)` with a 1-pixel pen: [`outline`] with corners of radius
/// 5 (the 10 is the size of the corners' ellipse).
pub(crate) fn rounded_outline(l: i32, t: i32, r: i32, b: i32, c: Color) {
    const R: f32 = 5.0;
    if r - l <= 2 * R as i32 || b - t <= 2 * R as i32 {
        return outline(l, t, r, b, c);
    }
    let (l, t, r, b) = (l as f32, t as f32, r as f32, b as f32);
    draw_rectangle(l + R, t, r - l - 2.0 * R, 1.0, c);
    draw_rectangle(l + R, b - 1.0, r - l - 2.0 * R, 1.0, c);
    draw_rectangle(l, t + R, 1.0, b - t - 2.0 * R, c);
    draw_rectangle(r - 1.0, t + R, 1.0, b - t - 2.0 * R, c);
    // quarter circles, angles clockwise from +x as y points down
    for (x, y, start) in [(l + R, t + R, 180.0), (r - R, t + R, 270.0), (r - R, b - R, 0.0), (l + R, b - R, 90.0)] {
        draw_arc(x, y, 8, R - 1.0, start, 1.0, 90.0, c);
    }
}

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
    status_bar_framed(min1, min2, max, x, y, w, h, colour, BLACK);
}

/// [`status_bar`] with its frame in GDI's current `ForeColor` `frame`, rather than black.
pub fn status_bar_framed(min1: f64, min2: f64, max: f64, x: i32, y: i32, w: i32, h: i32, colour: Option<Color>, frame: Color) {
    // The original clamps the bar's left and top edges to the screen, squeezing the bars of
    // buildings and characters partly off screen; here they simply slide off it.
    if w <= 0 || h <= 0 {
        return;
    }
    draw_rectangle(x as f32, y as f32, w as f32, h as f32, BLACK);
    if frame != BLACK {
        outline(x, y, x + w, y + h, frame);
    }
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
        self.small_font.set_scale(scale);
    }

    pub fn new(strings: Strings, window_strings: WindowStrings, tooltip_labels: object_tooltip::TooltipLabels, files: &Files, objects: usize) -> Hud {
        let font = TextRenderer::new(include_bytes!("../fonts/LiberationSans-Bold.ttf"), FONT_PX);
        let small_font = TextRenderer::new(include_bytes!("../fonts/LiberationSans-Bold.ttf"), 8.0 * 96.0 / 72.0);
        Hud {
            strings,
            font,
            small_font,
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
            cache_tick: None,
            statistics: String::new(),
            auto: bars::AutoRestore::load(),
            craft_queue: Vec::new(),
            shortcuts: shortcuts::Shortcuts::load(objects),
            clocks: shortcuts::recharge_clocks(files),
            messages: messages::MessageLog::load(files, "Francais"),
            hovered_object: None,
            tooltip_labels,
            build_hover: None,
            fps: 0,
            frames: 0,
            fps_timer: get_time(),
        }
    }

    /// Text in the small font (Arial Bold 8 pt).
    pub fn small_text(&self, x: i32, y: i32, s: &str, colour: Color) {
        self.small_font.draw(s, x, y + 9, colour);
    }

    /// `ClsIntMessage.Afficher`: `#` breaks lines (16 px apart); a line starting with `&` is red,
    /// with `$` blue. `(x, y)` is the top of the text cell, like GDI `DrawText`.
    /// The map's view (top-left and size in map pixels), for the minimap's rectangle.
    pub fn set_view(&mut self, cam: (i32, i32), view: (i32, i32)) {
        self.windows.view = (cam, view);
    }

    /// Where the minimap puts the view's top-left corner, when it does (the view then stops
    /// following the player until the map is closed).
    pub fn camera(&self) -> Option<(i32, i32)> {
        self.windows.camera
    }

    /// `ToucheChangerPersonnage`: the player now controls another character. What the interface
    /// remembered of the previous one goes (the original saved it per character).
    pub fn switch_character(&mut self) {
        self.messages.forget();
        self.cache_tick = None;
        self.craft_queue.clear();
        self.placing = None;
        self.radial = None;
        self.windows.forget_character();
    }

    /// Lines of text under the FPS counter, in white (the `#perf` timings).
    pub fn debug_lines(&self, lines: &[String]) {
        for (i, line) in lines.iter().enumerate() {
            self.text(crate::layout::screen_w() - 260, 18 + 16 * i as i32, line, WHITE);
        }
    }

    /// `Afficher_Commentaires`: what characters say, in white at their top-left plus (24, -16),
    /// at the interface's size. `to_screen` maps map pixels to interface pixels.
    pub fn draw_comments(&self, world: &World, to_screen: impl Fn(i64, i64) -> (i32, i32)) {
        // `Commentaires.ini`: DecalageX, DecalageY
        const OFFSET: (i64, i64) = (24, -16);
        for p in world.persos.iter().filter(|p| !p.comment.is_empty() && p.alive()) {
            let (x, y) = to_screen(p.x + OFFSET.0, p.y + OFFSET.1);
            if x > 0 && x < crate::layout::screen_w() && y > 0 && y < crate::layout::screen_h() {
                self.text(x, y, &p.comment, WHITE);
            }
        }
    }

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
        // The message log is under the interface.
        self.draw_messages();
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

        self.draw_statistics();
        self.draw_epoch(world, rules, p);
        self.draw_object_slots(world, rules, textures, player);
        self.draw_temp_effects(world, rules, textures, player);
        // What only changes with the world is made again once per tick.
        if self.cache_tick != Some(world.tick) {
            self.cache_tick = Some(world.tick);
            self.refresh_build_buttons(world, rules, player);
            self.statistics = self.statistics_text(world, rules, player);
        }
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
        // `Afficher_Objet_Curseur`: the held object under the cursor.
        let held = world.persos[player].selected_object;
        if held >= 0 {
            let path = format!("{}.bmp", rules.objects[held as usize].image);
            if let Some(tex) = textures.get(&path) {
                let src = Rect::new(0.0, 0.0, 32.0, 32.0);
                let (x, y) = (input.mouse.0 - 12, input.mouse.1 - 12);
                tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
            }
        }
        let cursor = placement.unwrap_or(match hover {
            Some((c, _)) if c <= 17 => c,
            _ => 0,
        });
        if let Some(tex) = textures.get(CURSOR) {
            tex.draw_ex(
                input.mouse.0 as f32,
                input.mouse.1 as f32,
                WHITE,
                DrawTextureParams { source: Some(Rect::new(24.0 * cursor as f32, 0.0, 24.0, 24.0)), ..Default::default() },
            );
        }
        if let (None, Some((code, hit))) = (placement, hover) {
            self.terrain_bubble(world, rules, player, code, hit, input.mouse);
        }
        if !crate::options::test_mode() {
            self.text(ax(940), 0, &format!("{} {:02}", self.strings.fps, self.fps), WHITE_);
        }
    }

    /// `Afficher_Stastistiques`: one line per active fief, bottom-aligned at y=745.
    /// Beyond the original's 16 fiefs, only 16 lines: the player's fief and the first others,
    /// then how many are left out.
    fn statistics_text(&self, world: &World, rules: &Rules, player: usize) -> String {
        const MAX_LINES: usize = 16;
        let own = world.persos[player].fief;
        // living citizens of every fief, in one pass
        let mut alive_by_fief = vec![0; world.fiefs.len()];
        for p in world.persos.iter().filter(|p| p.alive()) {
            if let Some(n) = alive_by_fief.get_mut(p.fief) {
                *n += 1;
            }
        }
        let mut lines = Vec::new();
        let mut hidden = 0;
        for (i, (f, c)) in world.fiefs.iter().zip(&world.castles).enumerate() {
            let alive = alive_by_fief[i];
            if alive == 0 && !c.visible() {
                continue;
            }
            if lines.len() >= MAX_LINES - 2 && i != own {
                hidden += 1;
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
        if hidden > 0 {
            lines.push(format!("(+{hidden})"));
        }
        format!("#{}", lines.join("#"))
    }

    /// The fief list at the bottom right, bottom-aligned on the action line.
    fn draw_statistics(&self) {
        let lines = self.statistics.matches('#').count() as i32;
        self.text(ax(816), ay(745) - 16 * lines, &self.statistics, WHITE_);
    }

    /// `Afficher_Epoques`: grey box with the fief's epoch name, centred on x=528; while the
    /// epoch changes, a white bar of its progress instead of the black box. Under it, the
    /// emperor's life bar and title.
    fn draw_epoch(&self, world: &World, rules: &Rules, p: &Perso) {
        // `EpoqueHauteur`, `EpoqueLargeur`, `EpoqueDecalageX/Y` around (528, 0)
        const H: i32 = 22;
        const W: i32 = 180;
        const DY: i32 = -4;
        let left = cx(438);
        let f = &world.fiefs[p.fief];
        let centred = |s: &str| cx(528) - clng(s.chars().count() as f64 * 3.25) as i32;
        match rules.epochs.get(f.epoch + 1).filter(|_| f.changing_epoch) {
            Some(next) => {
                // Drawn from y=-4, the bar is clipped to the screen: 18 high from y=0.
                let d = next.duration as f64 * world.epoch_speed;
                status_bar_framed(d - f.epoch_timer as f64, 0.0, d, left, 0, W, H + DY, Some(WHITE_), GREY);
            }
            None => {
                draw_rectangle(left as f32, 0.0, W as f32, (H + DY) as f32, BLACK);
                outline(left, 0, left + W, H + DY, GREY);
            }
        }
        let name = &rules.epochs[f.epoch].name;
        self.text(centred(name), 0, name, GREY);
        if let Some(e) = usize::try_from(f.emperor).ok().and_then(|e| world.persos.get(e)) {
            // `ForeColor` is still the epoch's grey.
            let y = H + DY - 1;
            let total = e.bar_total(0);
            status_bar_framed(e.bars[0].clamp(0.0, total.max(0.0)), total, e.bonus_max[0] as f64, left, y, W, H - 4, None, GREY);
            let title = format!("{} {}", rules.epochs[f.epoch].emperor_title[e.female as usize], e.name);
            self.text(centred(&title), y, &title, GREY);
        }
    }

    /// The 9 object shortcut slots ("1".."9") at the top left.
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
