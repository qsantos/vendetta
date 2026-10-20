//! The touch interface (see `layout::touch`): instead of the trays that slide out under the
//! mouse, the info buttons and the keys, a bar of big buttons whose panels list the build, move
//! and window buttons, each with what its tooltip would tell first; one window at a time;
//! buildings dragged into place, then confirmed. Not in the original.

use super::buttons::{build_buttons, button_image, inside};
use super::{DEPLACEMENT, Hud, INFOS, Input, LIGHT_GREY, RED, outline};
use crate::assets::Textures;
use crate::layout::{BAR, bar_vertical, full_size, screen_h, screen_w};
use macroquad::prelude::*;
use sim::rules::Rules;
use sim::world::{Order, World};

const ACTION: &str = "Images/Boutons/Action.bmp";
/// A bar button's side.
const BUTTON: i32 = 56;
/// A panel's entries, and the panel's padding.
const CELL_W: i32 = 188;
const CELL_H: i32 = 48;
const PAD: i32 = 8;
const YELLOW: Color = Color::new(1.0, 0.85, 0.2, 1.0);
const GREEN: Color = Color::new(0.0, 0.75, 0.0, 1.0);
const BACKDROP: Color = Color::new(0.0, 0.0, 0.0, 0.92);

#[derive(Clone, Copy, PartialEq)]
pub enum Panel {
    Build,
    Move,
    Windows,
}

#[derive(Clone, Copy, PartialEq)]
enum BarAction {
    Open(Panel),
    Confirm,
    Cancel,
    Drop,
}

/// A cell of a sprite sheet, or a drawn tick.
enum Icon {
    Sheet { path: String, col: i32, row: i32, size: i32 },
    Tick,
}

/// A panel's entry: its button, what it is, and a second line (red after `&`).
struct Entry {
    icon: Icon,
    caption: String,
    detail: String,
    enabled: bool,
    /// Its window is shown, or the game paused.
    on: bool,
    tip: String,
}

/// The touch interface's state.
#[derive(Default)]
pub struct Touch {
    pub panel: Option<Panel>,
    /// The panel's first row shown.
    scroll: usize,
    /// Dragged since the panel last scrolled by a row.
    dragged: f32,
    /// The one window shown.
    pub front: Option<usize>,
    /// Dragged on the window since its list last scrolled by a row.
    pub window_dragged: f32,
    /// In a building in the previous frame: entering one shows its window.
    in_building: bool,
    /// The centre of the building being placed, in map pixels.
    ghost: Option<(f32, f32)>,
    /// Demolishing the house waits for a confirmation.
    demolishing: bool,
}

/// The top-left corner of the bar's `i`-th button.
fn bar_slot(i: usize) -> (i32, i32) {
    let (w, h) = full_size();
    let gap = (BAR - BUTTON) / 2;
    if bar_vertical() { (w - BAR + gap, gap + BAR * i as i32) } else { (gap + BAR * i as i32, h - BAR + gap) }
}

/// A tooltip without the key between brackets at its end ("Carte (Tab)").
fn without_key(s: &str) -> &str {
    match s.rfind(" (") {
        Some(i) if s.ends_with(')') => &s[..i],
        _ => s,
    }
}

/// The panel of `n` entries: its rectangle, columns and rows shown, next to the bar.
fn panel_rect(n: usize) -> (i32, i32, i32, i32, usize, usize) {
    let (fw, fh) = (screen_w(), screen_h());
    let cols = (((fw - 2 * PAD - 8) / CELL_W).max(1) as usize).min(n.max(1));
    let rows = n.div_ceil(cols);
    let shown = rows.min(((fh - 2 * PAD - 8) / CELL_H).max(1) as usize);
    let (w, h) = (cols as i32 * CELL_W + 2 * PAD, shown as i32 * CELL_H + 2 * PAD);
    let (x, y) = if bar_vertical() { (fw - 4 - w, 4) } else { (4, fh - 4 - h) };
    (x, y, w, h, cols, shown)
}

impl Hud {
    fn draw_icon(&self, textures: &Textures, icon: &Icon, x: i32, y: i32, scale: f32) {
        match icon {
            Icon::Sheet { path, col, row, size } => {
                if let Some(tex) = textures.get(path) {
                    let s = *size as f32;
                    let src = Rect::new(s * *col as f32, s * *row as f32, s, s);
                    let dest = Some(vec2(s * scale, s * scale));
                    tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), dest_size: dest, ..Default::default() });
                }
            }
            Icon::Tick => {
                let (x, y, s) = (x as f32, y as f32, 24.0 * scale);
                draw_line(x + 0.15 * s, y + 0.55 * s, x + 0.4 * s, y + 0.8 * s, 0.14 * s, GREEN);
                draw_line(x + 0.4 * s, y + 0.8 * s, x + 0.88 * s, y + 0.2 * s, 0.14 * s, GREEN);
            }
        }
    }

    fn icon_size(icon: &Icon) -> i32 {
        match icon {
            Icon::Sheet { size, .. } => *size,
            Icon::Tick => 24,
        }
    }

    /// Whether anything is selected: an object held, money or resources.
    fn has_selection(world: &World, player: usize) -> bool {
        let p = &world.persos[player];
        p.selected_object >= 0 || p.selected_money > 0.0 || p.selected_resources.iter().any(|&r| r > 0.0)
    }

    fn bar_buttons(&self, world: &World, rules: &Rules, player: usize) -> Vec<(Icon, &'static str, BarAction)> {
        let sheet = |path: &str, col, size| Icon::Sheet { path: path.to_owned(), col, row: 0, size };
        let mut v = vec![
            (sheet(ACTION, 1, 24), "Construire", BarAction::Open(Panel::Build)),
            (sheet(DEPLACEMENT, 4, 28), "Déplacer", BarAction::Open(Panel::Move)),
            (sheet(INFOS, 4, 24), "Fenêtres", BarAction::Open(Panel::Windows)),
        ];
        if self.placing.is_some() || self.touch.demolishing {
            v.push((Icon::Tick, "Valider", BarAction::Confirm));
            v.push((sheet(ACTION, 6, 24), "Annuler", BarAction::Cancel));
        } else if Self::has_selection(world, player) {
            let p = &world.persos[player];
            let icon = if p.selected_object >= 0 {
                sheet(&format!("{}.bmp", rules.objects[p.selected_object as usize].image), 0, 32)
            } else if let Some(r) = p.selected_resources.iter().position(|&r| r > 0.0) {
                sheet(&format!("Images/Ressources/{}.bmp", rules.resources[r].image), 0, 32)
            } else {
                sheet(ACTION, 2, 24)
            };
            v.push((icon, "Lâcher", BarAction::Drop));
        }
        v
    }

    fn panel_entries(&self, world: &World, rules: &Rules, player: usize, panel: Panel) -> Vec<Entry> {
        let p = &world.persos[player];
        match panel {
            Panel::Build => build_buttons(rules)
                .into_iter()
                .enumerate()
                .map(|(i, kind)| {
                    let state = self.build_cache.get(i).map_or(0, |c| c.0);
                    let tip = self.build_tooltip(world, rules, player, kind, state, 0.0);
                    // the tooltip's name, then its first line in red, or its costs
                    let mut lines = tip.split('#');
                    let caption = without_key(lines.next().unwrap_or("").trim_start_matches('&')).to_owned();
                    let rest: Vec<&str> = lines.collect();
                    let detail = match rest.iter().find(|l| l.starts_with('&')) {
                        Some(red) => format!("&{}", red[1..].trim_matches(['(', ')'])),
                        None => rest.iter().filter(|l| l.starts_with('(')).map(|l| l.trim_matches(['(', ')'])).collect::<Vec<_>>().join(", "),
                    };
                    let icon = Icon::Sheet { path: button_image(rules, kind), col: 0, row: state, size: 28 };
                    Entry { icon, caption, detail, enabled: state == 1, on: false, tip }
                })
                .collect(),
            Panel::Move => (0..18)
                .filter(|&i| self.move_visible(world, player, i))
                .map(|i| {
                    let mut tip = self.strings.move_tips[i].clone();
                    let mut detail = String::new();
                    if let Some(r) = (2..=9).contains(&i).then(|| rules.terrain_resources[i - 2].resource).filter(|&r| r >= 0) {
                        let r = r as usize;
                        detail = format!("{} : {} / {}", rules.resources[r].name, p.resource(r).floor(), p.max_resource(r));
                        tip += &format!("#{detail}");
                    }
                    let row = if i == 14 { p.ai as i32 } else { 0 };
                    let icon = Icon::Sheet { path: DEPLACEMENT.to_owned(), col: i as i32, row, size: 28 };
                    Entry { icon, caption: without_key(&self.strings.move_tips[i]).to_owned(), detail, enabled: true, on: false, tip }
                })
                .collect(),
            Panel::Windows => (0..8)
                .map(|i| {
                    let on = if i == 7 { self.paused() } else { self.touch.front == Some(i) };
                    let icon = Icon::Sheet { path: INFOS.to_owned(), col: i as i32, row: on as i32, size: 24 };
                    let tip = self.strings.info_tips[i].clone();
                    let enabled = i != 0 || self.in_building(world, player);
                    Entry { icon, caption: without_key(&tip).to_owned(), detail: String::new(), enabled, on, tip }
                })
                .collect(),
        }
    }

    /// The building window's condition (`Afficher_Fenetres`).
    pub(super) fn in_building(&self, world: &World, player: usize) -> bool {
        let p = &world.persos[player];
        (p.alive() && (p.in_house || p.in_own_house)) || p.in_castle || p.in_own_castle
    }

    /// Entering a building shows its window, as the original does with its window enabled;
    /// leaving it hides it.
    pub(super) fn update_front(&mut self, world: &World, player: usize) {
        let inside = self.in_building(world, player);
        if inside && !self.touch.in_building {
            self.touch.front = Some(0);
        } else if !inside && self.touch.front == Some(0) {
            self.touch.front = None;
        }
        self.touch.in_building = inside;
    }

    /// Whether `m` is on the touch interface: its bar, panel or window.
    pub fn over_touch_ui(&self, world: &World, rules: &Rules, player: usize, m: (i32, i32)) -> bool {
        let (fw, fh) = (screen_w(), screen_h());
        m.0 >= fw || m.1 >= fh || self.touch.panel.is_some() || self.over_window(world, rules, player, m).is_some()
    }

    /// Where the building being placed is, in map pixels: first `default`, then moved as far
    /// as a finger drags on the map (so that the finger does not hide it), or to where it taps.
    pub fn touch_ghost(&mut self, drag: (f32, f32), tap: Option<(i32, i32)>, default: (i32, i32)) -> (i32, i32) {
        if self.placing.is_none() {
            self.touch.ghost = None;
            return default;
        }
        let ghost = self.touch.ghost.get_or_insert((default.0 as f32, default.1 as f32));
        if let Some(t) = tap {
            *ghost = (t.0 as f32, t.1 as f32);
        }
        ghost.0 += drag.0;
        ghost.1 += drag.1;
        (ghost.0.round() as i32, ghost.1.round() as i32)
    }

    /// Draws the bar, the open panel and what is being placed, and handles their taps. Returns
    /// whether the finger is on them, and the panel's tooltip.
    pub(super) fn touch_controls(&mut self, world: &World, rules: &Rules, textures: &Textures, player: usize, input: &Input, orders: &mut Vec<Order>) -> (bool, Option<String>) {
        let m = input.mouse;
        let perso = player;
        let mut on_ui = false;
        let mut tip = None;

        // What is being placed or demolished, at the top: its tooltip.
        if let Some(kind) = self.placing {
            let state = self.build_cache.iter().zip(build_buttons(rules)).find(|(_, k)| *k == Some(kind)).map_or(1, |(c, _)| c.0);
            let text = self.build_tooltip(world, rules, player, Some(kind), state, 0.0);
            self.banner(&text);
        } else if self.touch.demolishing {
            self.banner(&format!("&{} ?", self.strings.demolish));
        }

        // The panel, over the map and the window.
        if let Some(panel) = self.touch.panel {
            let entries = self.panel_entries(world, rules, player, panel);
            let (px, py, pw, ph, cols, shown) = panel_rect(entries.len());
            let rows = entries.len().div_ceil(cols);
            let over = inside(m, px, py, pw, ph);
            // dragging scrolls it by rows
            if over && input.drag.1 != 0.0 {
                self.touch.dragged += input.drag.1;
                while self.touch.dragged <= -(CELL_H as f32) / 2.0 && self.touch.scroll + shown < rows {
                    self.touch.scroll += 1;
                    self.touch.dragged += CELL_H as f32 / 2.0;
                }
                while self.touch.dragged >= CELL_H as f32 / 2.0 && self.touch.scroll > 0 {
                    self.touch.scroll -= 1;
                    self.touch.dragged -= CELL_H as f32 / 2.0;
                }
            }
            self.touch.scroll = self.touch.scroll.min(rows - shown);
            draw_rectangle(px as f32, py as f32, pw as f32, ph as f32, BACKDROP);
            outline(px, py, px + pw, py + ph, LIGHT_GREY);
            let mut hit = None;
            for (k, e) in entries.iter().enumerate().skip(self.touch.scroll * cols).take(shown * cols) {
                let (col, row) = ((k % cols) as i32, (k / cols - self.touch.scroll) as i32);
                let (x, y) = (px + PAD + col * CELL_W, py + PAD + row * CELL_H);
                let size = Self::icon_size(&e.icon);
                let scale = 40.0 / size as f32;
                self.draw_icon(textures, &e.icon, x + 2, y + 4, scale);
                let colour = if e.enabled { WHITE } else { Color::new(0.6, 0.6, 0.6, 1.0) };
                let ty = if e.detail.is_empty() { y + 16 } else { y + 6 };
                let room = CELL_W - 48 - 6;
                self.text(x + 48, ty, &self.fit(&self.font, &e.caption, room), colour);
                if !e.detail.is_empty() {
                    let (detail, c) = match e.detail.strip_prefix('&') {
                        Some(d) => (d, RED),
                        None => (e.detail.as_str(), LIGHT_GREY),
                    };
                    self.small_text(x + 48, y + 26, &self.fit(&self.small_font, detail, room), c);
                }
                if e.on {
                    outline(x, y, x + CELL_W - 4, y + CELL_H, GREEN);
                }
                if inside(m, x, y, CELL_W - 4, CELL_H - 1) {
                    outline(x, y, x + CELL_W - 4, y + CELL_H, YELLOW);
                    hit = Some(k);
                }
            }
            // more rows above or below: a mark at the panel's edge
            if self.touch.scroll > 0 {
                draw_triangle(vec2((px + pw / 2 - 8) as f32, (py + 6) as f32), vec2((px + pw / 2 + 8) as f32, (py + 6) as f32), vec2((px + pw / 2) as f32, (py - 2) as f32), YELLOW);
            }
            if self.touch.scroll + shown < rows {
                let b = py + ph;
                draw_triangle(vec2((px + pw / 2 - 8) as f32, (b - 6) as f32), vec2((px + pw / 2 + 8) as f32, (b - 6) as f32), vec2((px + pw / 2) as f32, (b + 2) as f32), YELLOW);
            }
            if let Some(k) = hit {
                tip = Some(entries[k].tip.clone());
                if input.left_pressed {
                    self.panel_tap(world, rules, player, panel, k, entries[k].enabled, orders);
                }
            } else if input.left_pressed && m.0 < screen_w() && m.1 < screen_h() {
                // a tap elsewhere closes it
                self.touch.panel = None;
            }
            on_ui = true;
        }

        // The bar.
        let (fw, fh) = full_size();
        let (bx, by) = if bar_vertical() { (screen_w(), 0) } else { (0, screen_h()) };
        draw_rectangle(bx as f32, by as f32, (fw - bx) as f32, (fh - by) as f32, Color::new(0.0, 0.0, 0.0, 0.6));
        for (i, (icon, caption, action)) in self.bar_buttons(world, rules, player).into_iter().enumerate() {
            let (x, y) = bar_slot(i);
            let open = matches!(action, BarAction::Open(p) if self.touch.panel == Some(p));
            draw_rectangle(x as f32, y as f32, BUTTON as f32, BUTTON as f32, Color::new(0.15, 0.15, 0.15, 1.0));
            outline(x, y, x + BUTTON, y + BUTTON, if open { YELLOW } else { LIGHT_GREY });
            let size = Self::icon_size(&icon);
            let scale = 32.0 / size as f32;
            self.draw_icon(textures, &icon, x + (BUTTON - 32) / 2, y + 4, scale);
            let w = self.small_font.width(caption);
            self.small_text(x + (BUTTON - w) / 2, y + BUTTON - 16, caption, WHITE);
            if action == BarAction::Drop {
                // what is held, next to its button
                let lines = self.selection_lines(world, rules, player).join("#");
                if !lines.is_empty() {
                    let (tw, th) = super::buttons::tooltip_size(&lines);
                    let (tx, ty) = if bar_vertical() { (x - 8 - tw, y) } else { (x, y - 12 - th) };
                    self.tooltip((tx - 24, ty - 20), &lines);
                }
            }
            if !inside(m, x, y, BUTTON, BUTTON) {
                continue;
            }
            on_ui = true;
            if !input.left_pressed {
                continue;
            }
            match action {
                BarAction::Open(p) => {
                    self.touch.panel = if self.touch.panel == Some(p) { None } else { Some(p) };
                    self.touch.scroll = 0;
                }
                BarAction::Confirm => {
                    if let (Some(kind), Some(centre)) = (self.placing, self.touch.ghost.map(|(x, y)| (x.round() as i32, y.round() as i32))) {
                        let (x, y, ok) = self.footprint(world, rules, player, kind, centre);
                        if ok {
                            orders.push(Order::BuildHouse { perso, x, y, kind });
                            self.placing = None;
                        }
                    } else if self.touch.demolishing {
                        orders.push(Order::Demolish { perso });
                        self.touch.demolishing = false;
                    }
                }
                BarAction::Cancel => {
                    self.placing = None;
                    self.touch.demolishing = false;
                }
                BarAction::Drop => orders.push(Order::Unselect { perso }),
            }
        }
        (on_ui, tip)
    }

    /// A tap on the panel's `k`-th entry.
    fn panel_tap(&mut self, world: &World, rules: &Rules, player: usize, panel: Panel, k: usize, enabled: bool, orders: &mut Vec<Order>) {
        match panel {
            // A disabled building only shows its tooltip (why it is disabled).
            Panel::Build if enabled => {
                match build_buttons(rules)[k] {
                    None => self.touch.demolishing = true,
                    Some(kind) => self.placing = Some(kind),
                }
                // the map is where it goes
                self.touch.panel = None;
                self.touch.front = None;
            }
            Panel::Build => {}
            Panel::Move => {
                let i = (0..18).filter(|&i| self.move_visible(world, player, i)).nth(k).unwrap_or(0);
                orders.extend(self.move_order(world, player, i, false));
                self.touch.panel = None;
            }
            Panel::Windows if !enabled => {}
            Panel::Windows => {
                if k == 7 {
                    self.info_enabled[7] = !self.info_enabled[7];
                } else {
                    self.touch.front = if self.touch.front == Some(k) { None } else { Some(k) };
                }
                self.touch.panel = None;
            }
        }
    }

    /// `s`, cut short with an ellipsis to fit in `width`.
    fn fit(&self, font: &crate::text::TextRenderer, s: &str, width: i32) -> String {
        if font.width(s) <= width {
            return s.to_owned();
        }
        let mut cut: String = s.to_owned();
        while !cut.is_empty() && font.width(&format!("{cut}…")) > width {
            cut.pop();
        }
        format!("{}…", cut.trim_end())
    }

    /// A tooltip at the top of the screen, centred.
    fn banner(&self, text: &str) {
        let (w, _) = super::buttons::tooltip_size(text);
        self.tooltip(((screen_w() - w) / 2 - 24, 28 - 20), text);
    }
}
