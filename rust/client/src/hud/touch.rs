//! The touch interface (see `layout::touch`): instead of the trays that slide out under the
//! mouse, the info buttons and the keys, a bar of big buttons whose panels list the build, move
//! and window buttons, each with what its tooltip would tell first; one window at a time;
//! buildings dragged into place, then confirmed. Not in the original.

use super::buttons::{build_buttons, button_image, inside};
use super::bars::{BAR_H, BAR_W, BAR_X, BAR_Y, ORANGE};
use super::{DEPLACEMENT, Hud, INFOS, Input, LIGHT_GREY, RED, outline};
use crate::assets::Textures;
use crate::layout::{BAR, ay, bar_vertical, full_size, screen_h, screen_w};
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
    /// Eating automatically, per bar: on or off, and below what.
    Auto,
}

#[derive(Clone, Copy, PartialEq)]
enum BarAction {
    Open(Panel),
    Drop,
    /// The page's in-game menu, at the bar's far end.
    Menu,
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
    /// Where the buttons that confirm or cancel the building or demolishing are, top-left.
    confirm: Option<[(i32, i32); 2]>,
}

/// The automatic eating's button, right of the status bars, level with the last one, and
/// the rows of its panel.
const AUTO_W: i32 = 36;
const AUTO_H: i32 = 36;
const AUTO_ROW: i32 = 36;

fn auto_button() -> (i32, i32) {
    (BAR_X + BAR_W + 4, ay(BAR_Y[3]) + BAR_H + 1 - AUTO_H)
}

/// The buttons that confirm or cancel a building or demolishing: their side.
const CONFIRM: i32 = 40;

/// Where those two buttons go beside a box (left, top, right, bottom): right of it, else left
/// of it, side by side or one above the other, on the screen.
fn confirm_beside((l, t, r, b): (i32, i32, i32, i32), side_by_side: bool) -> [(i32, i32); 2] {
    let (span_w, span_h) = if side_by_side { (2 * CONFIRM + 4, CONFIRM) } else { (CONFIRM, 2 * CONFIRM + 4) };
    let x = if r + 6 + span_w <= screen_w() { r + 6 } else { (l - 6 - span_w).max(0) };
    let y = (if side_by_side { t } else { (t + b) / 2 - span_h / 2 }).clamp(0, (screen_h() - span_h).max(0));
    if side_by_side { [(x, y), (x + CONFIRM + 4, y)] } else { [(x, y), (x, y + CONFIRM + 4)] }
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
        if Self::has_selection(world, player) {
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
        v.push((Icon::Sheet { path: INFOS.to_owned(), col: 7, row: self.paused() as i32, size: 24 }, "Menu", BarAction::Menu));
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
            Panel::Auto => Vec::new(),
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
        m.0 >= fw
            || m.1 >= fh
            || self.touch.panel.is_some()
            || self.touch.confirm.is_some_and(|b| b.iter().any(|&(x, y)| inside(m, x, y, CONFIRM, CONFIRM)))
            || self.over_window(world, rules, player, m).is_some()
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

        // What is being placed or demolished, at the top: its tooltip; buttons to confirm or
        // cancel next to the building's footprint (which they follow), or to the question.
        self.touch.confirm = None;
        if let Some(kind) = self.placing {
            let state = self.build_cache.iter().zip(build_buttons(rules)).find(|(_, k)| *k == Some(kind)).map_or(1, |(c, _)| c.0);
            let text = self.build_tooltip(world, rules, player, Some(kind), state, 0.0);
            self.banner(&text);
            if let Some(centre) = self.touch.ghost.map(|(x, y)| (x.round() as i32, y.round() as i32)) {
                let (x, y, _) = self.footprint(world, rules, player, kind, centre);
                let (w, h) = rules.buildings[kind].size;
                // map pixels to interface pixels: the map's view covers the whole screen
                let ((cx, cy), (vw, _)) = self.windows.view;
                let k = full_size().0 as f32 / vw.max(1) as f32;
                let at = |v: i32, c: i32| ((v - c) as f32 * k).round() as i32;
                self.touch.confirm = Some(confirm_beside((at(x, cx), at(y, cy), at(x + w, cx), at(y + h, cy)), false));
            }
        } else if self.touch.demolishing {
            let text = format!("&{} ?", without_key(&self.strings.demolish));
            let (l, t, r, b) = self.banner(&text);
            self.touch.confirm = Some(confirm_beside((l, t, r, b), true));
        }
        if let Some(buttons) = self.touch.confirm {
            for (k, &(x, y)) in buttons.iter().enumerate() {
                draw_rectangle(x as f32, y as f32, CONFIRM as f32, CONFIRM as f32, Color::new(0.15, 0.15, 0.15, 0.9));
                outline(x, y, x + CONFIRM, y + CONFIRM, LIGHT_GREY);
                let icon = if k == 0 { Icon::Tick } else { Icon::Sheet { path: ACTION.to_owned(), col: 6, row: 0, size: 24 } };
                self.draw_icon(textures, &icon, x + (CONFIRM - 32) / 2, y + (CONFIRM - 32) / 2, 32.0 / 24.0);
                if !inside(m, x, y, CONFIRM, CONFIRM) {
                    continue;
                }
                on_ui = true;
                if !input.left_pressed {
                    continue;
                }
                if k == 1 {
                    self.placing = None;
                    self.touch.demolishing = false;
                } else if let (Some(kind), Some(centre)) = (self.placing, self.touch.ghost.map(|(x, y)| (x.round() as i32, y.round() as i32))) {
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
        }

        // The automatic eating's button, right of the status bars (a tap on a bar eats).
        let (ax_, ay_) = auto_button();
        let open = self.touch.panel == Some(Panel::Auto);
        draw_rectangle(ax_ as f32, ay_ as f32, AUTO_W as f32, AUTO_H as f32, Color::new(0.15, 0.15, 0.15, 1.0));
        let any = self.auto.on.iter().any(|&on| on);
        outline(ax_, ay_, ax_ + AUTO_W, ay_ + AUTO_H, if open { YELLOW } else if any { ORANGE } else { LIGHT_GREY });
        self.draw_icon(textures, &Icon::Sheet { path: ACTION.to_owned(), col: 0, row: 0, size: 24 }, ax_ + (AUTO_W - 20) / 2, ay_ + 2, 20.0 / 24.0);
        let w = self.small_font.width("Auto");
        self.small_text(ax_ + (AUTO_W - w) / 2, ay_ + AUTO_H - 14, "Auto", WHITE);
        if inside(m, ax_, ay_, AUTO_W, AUTO_H) {
            on_ui = true;
            if input.left_pressed {
                self.touch.panel = if open { None } else { Some(Panel::Auto) };
                self.touch.scroll = 0;
                return (true, None);
            }
        }

        // The panel, over the map and the window.
        if self.touch.panel == Some(Panel::Auto) {
            self.auto_panel(input);
            on_ui = true;
        } else if let Some(panel) = self.touch.panel {
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
        let last = (if bar_vertical() { fh } else { fw } / BAR - 1).max(0) as usize;
        for (i, (icon, caption, action)) in self.bar_buttons(world, rules, player).into_iter().enumerate() {
            let (x, y) = bar_slot(if action == BarAction::Menu { last } else { i });
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
                BarAction::Drop => orders.push(Order::Unselect { perso }),
                BarAction::Menu => self.toggle_info(7),
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
            Panel::Auto => {}
            Panel::Windows if !enabled => {}
            Panel::Windows => {
                if k == 7 {
                    self.toggle_info(7);
                } else {
                    self.touch.front = if self.touch.front == Some(k) { None } else { Some(k) };
                }
                self.touch.panel = None;
            }
        }
    }

    /// The automatic eating's panel, above the bars: per bar, on or off, and its threshold
    /// 5 % lower or higher (which also turns it on, as the wheel does on a bar).
    fn auto_panel(&mut self, input: &Input) {
        let m = input.mouse;
        let (w, h) = (2 * PAD + 300, 2 * PAD + 4 * AUTO_ROW);
        let (px, py) = (4, ay(BAR_Y[0]) - 8 - h);
        draw_rectangle(px as f32, py as f32, w as f32, h as f32, BACKDROP);
        outline(px, py, px + w, py + h, LIGHT_GREY);
        let mut hit = false;
        for i in 0..4 {
            let (x, y) = (px + PAD, py + PAD + AUTO_ROW * i as i32);
            let on = self.auto.on[i];
            self.text(x, y + 10, &self.strings.bars[i], WHITE);
            // [Auto : oui] [-] 50 % [+]
            let buttons = [(x + 84, 104), (x + 196, 34), (x + 266, 34)];
            for (k, &(bx, bw)) in buttons.iter().enumerate() {
                let active = k == 0 && on;
                draw_rectangle(bx as f32, (y + 2) as f32, bw as f32, (AUTO_ROW - 6) as f32, if active { Color::new(0.4, 0.2, 0.0, 1.0) } else { Color::new(0.15, 0.15, 0.15, 1.0) });
                outline(bx, y + 2, bx + bw, y + AUTO_ROW - 4, if active { ORANGE } else { LIGHT_GREY });
                let label = match k {
                    0 if on => "Auto : oui",
                    0 => "Auto : non",
                    1 => "-",
                    _ => "+",
                };
                let lw = self.font.width(label);
                self.text(bx + (bw - lw) / 2, y + 10, label, WHITE);
                if inside(m, bx, y + 2, bw, AUTO_ROW - 6) {
                    hit = true;
                    if input.left_pressed {
                        let t = &mut self.auto.threshold[i];
                        match k {
                            0 => self.auto.on[i] = !on,
                            1 => *t = ((*t - 0.05) * 100.0).round().max(0.0) / 100.0,
                            _ => *t = ((*t + 0.05) * 100.0).round().min(100.0) / 100.0,
                        }
                        if k > 0 {
                            self.auto.on[i] = true;
                        }
                        self.auto.store();
                    }
                }
            }
            let pct = format!("{} %", (self.auto.threshold[i] * 100.0).round());
            let pw = self.font.width(&pct);
            self.text(x + 248 - pw / 2, y + 10, &pct, if on { ORANGE } else { LIGHT_GREY });
        }
        // a tap elsewhere closes it
        if input.left_pressed && !hit && !inside(m, px, py, w, h) && m.0 < screen_w() && m.1 < screen_h() {
            self.touch.panel = None;
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

    /// A tooltip at the top of the screen, centred; returns its box (left, top, right, bottom).
    fn banner(&self, text: &str) -> (i32, i32, i32, i32) {
        let (w, h) = super::buttons::tooltip_size(text);
        let (x, y) = ((screen_w() - w) / 2, 28);
        self.tooltip((x - 24, y - 20), text);
        (x - 2, y - 2, x + w + 2, y + h + 2)
    }
}
