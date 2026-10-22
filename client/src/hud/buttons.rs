//! HUD buttons: build, move and info buttons, building placement, tooltips and shortcuts
//! (`ClsInterface.Interieur_Boutons`).

use super::{DEPLACEMENT, Hud, INFOS, RED, outline};
use crate::assets::{Sprite, Textures};
use crate::layout::{ax, screen_h, screen_w};
use macroquad::prelude::*;
use sim::rules::Rules;
use sim::world::{Order, World};

/// The mouse for one frame, in frame pixels, with the original's press/release edges.
#[derive(Clone, Copy)]
pub struct Input {
    pub mouse: (i32, i32),
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub ctrl: bool,
    pub shift: bool,
    /// The pointer is over the screen: with a mouse, unless off the game's canvas (on the rest
    /// of the page, or out of it); only while a finger is down with the touch interface (where
    /// the cursor is not shown otherwise).
    pub hover: bool,
    /// How far a finger dragged this frame, in interface pixels.
    pub drag: (f32, f32),
}

impl Input {
    /// With the touch interface: a tap clicks where the finger came off; nothing else does.
    /// `ui`: frame pixels per interface pixel.
    pub fn from_fingers(mouse: (i32, i32), fingers: &crate::touch::Frame, ui: f32) -> Input {
        Input {
            mouse,
            left_pressed: fingers.tap,
            left_released: fingers.tap,
            right_pressed: false,
            ctrl: false,
            shift: false,
            hover: fingers.down,
            drag: fingers.drag.map_or((0.0, 0.0), |(x, y)| (x / ui, y / ui)),
        }
    }

    pub fn poll(mouse: (i32, i32)) -> Input {
        Input {
            mouse,
            hover: !crate::storage::pointer_out(),
            drag: (0.0, 0.0),
            left_pressed: is_mouse_button_pressed(MouseButton::Left),
            left_released: is_mouse_button_released(MouseButton::Left),
            right_pressed: is_mouse_button_pressed(MouseButton::Right),
            ctrl: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl),
            shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
        }
    }
}

pub(super) fn button_image(rules: &Rules, kind: Option<usize>) -> String {
    let name = kind.map_or("_Demolir", |k| &rules.buildings[k].button_image);
    format!("Images/Boutons/Construire/{name}.bmp")
}

/// The build buttons: index 0 demolishes, then the playable buildings in `BatimentsTri` order.
pub(super) fn build_buttons(rules: &Rules) -> Vec<Option<usize>> {
    std::iter::once(None)
        .chain(rules.building_order.iter().copied().filter(|&k| rules.buildings[k].playable).map(Some))
        .collect()
}

pub fn images(rules: &Rules) -> Vec<String> {
    build_buttons(rules).into_iter().map(|k| button_image(rules, k)).collect()
}

/// A tooltip's text size (without its border), 7 pixels a character.
pub(super) fn tooltip_size(s: &str) -> (i32, i32) {
    let lines: Vec<&str> = s.split('#').collect();
    // Each line's length includes its separator, except the last one.
    let max_len = lines
        .iter()
        .enumerate()
        .map(|(i, l)| crate::text::columns(l) + (i + 1 < lines.len()) as usize)
        .max()
        .unwrap_or(0) as i32;
    (max_len * 7, lines.len() as i32 * 16)
}

/// `Interieur_Bouton`: inclusive on both ends.
pub(super) fn inside(m: (i32, i32), x: i32, y: i32, w: i32, h: i32) -> bool {
    m.0 >= x && m.0 <= x + w && m.1 >= y && m.1 <= y + h
}

fn blit(tex: &Sprite, x: i32, y: i32, src: Rect) {
    // Sprites are clipped at the screen's left edge by shrinking the source rectangle.
    let mut src = src;
    let mut x = x as f32;
    if x < 0.0 {
        src.x -= x;
        src.w += x;
        x = 0.0;
    }
    if src.w > 0.0 {
        tex.draw_ex(x, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
    }
}

fn move_button_pos(i: usize, slide: i32) -> (i32, i32) {
    if i < 18 {
        (28 * (i as i32 % 2) + slide, 28 * (i as i32 / 2))
    } else {
        (56 + slide, 112 + 28 * (i as i32 - 18))
    }
}

/// `Trouver_Maison`: the first visible house of type `kind` owned by a teammate, cycling from
/// the one after the house the character last went to.
fn next_allied_house(world: &World, player: usize, kind: usize) -> Option<usize> {
    let p = &world.persos[player];
    let n = world.houses.len();
    (1..=n).map(|d| (p.house_target + d) % n).find(|&j| {
        let m = &world.houses[j];
        world.persos[j].team == p.team && m.kind == kind && m.visible()
    })
}

fn build_button_pos(i: usize, slide: i32) -> (i32, i32) {
    (28 * (i as i32 % 3) + slide, 232 + 28 * (i as i32 / 3 + 1))
}

fn info_button_pos(i: usize) -> (i32, i32) {
    (ax(888 - 30 * i as i32), 0)
}

impl Hud {
    /// Which move buttons are shown (`Afficher_BoutonsDeplacement`).
    pub(super) fn move_visible(&self, world: &World, player: usize, i: usize) -> bool {
        let p = &world.persos[player];
        match i {
            0 => world.castles[p.fief].visible(),
            1 => world.houses[player].visible(),
            // TODO: soldiers (17), army selected (18..22)
            17.. => false,
            _ => true,
        }
    }

    /// Build button state (`Etat`): 0 disabled, 1 enabled, 2 epoch not reached.
    fn build_state(&self, world: &World, rules: &Rules, player: usize, kind: Option<usize>) -> i32 {
        let p = &world.persos[player];
        match kind {
            None => (world.houses[player].visible() || p.in_house) as i32,
            Some(k) => {
                if world.fiefs[p.fief].epoch as i64 >= rules.buildings[k].epoch - 1 {
                    (p.alive() && world.can_build_replacing(rules, player, k)) as i32
                } else {
                    2
                }
            }
        }
    }

    /// `Deplacer_BoutonsConstruire` / `Deplacer_BoutonsDeplacement`: 4 px per frame toward
    /// shown while hovered, hidden otherwise.
    pub(super) fn slide_buttons(&mut self, world: &World, rules: &Rules, player: usize, m: (i32, i32)) {
        let n = build_buttons(rules).len();
        let over_build = (0..n).any(|i| {
            let (x, y) = build_button_pos(i, self.build_slide);
            inside(m, x, y, 28, 28)
        });
        if over_build && self.build_slide < 0 {
            self.build_slide += 4;
        } else if !over_build && self.build_slide > -80 {
            self.build_slide -= 4;
        }
        let over_move = (0..18).filter(|&i| self.move_visible(world, player, i)).any(|i| {
            let (x, y) = move_button_pos(i, self.move_slide);
            inside(m, x, y, 28, 28)
        });
        if over_move && self.move_slide < 0 {
            self.move_slide += 4;
        } else if !over_move && self.move_slide > -52 {
            self.move_slide -= 4;
        }
    }

    pub(super) fn draw_move_buttons(&self, textures: &Textures, world: &World, player: usize) {
        let Some(tex) = textures.get(DEPLACEMENT) else { return };
        let p = &world.persos[player];
        for i in (0..18).filter(|&i| self.move_visible(world, player, i)) {
            let (x, y) = move_button_pos(i, self.move_slide);
            let row = if i == 14 { p.ai as i32 } else { 0 };
            // opaque blit: black is drawn, not keyed
            let w = 28 + x.min(0);
            if w > 0 {
                draw_rectangle(x.max(0) as f32, y as f32, w as f32, 28.0, BLACK);
            }
            blit(tex, x, y, Rect::new(28.0 * i as f32, 28.0 * row as f32, 28.0, 28.0));
        }
    }

    pub(super) fn draw_build_buttons(&self, textures: &Textures) {
        for (i, (state, image)) in self.build_cache.iter().enumerate() {
            if let Some(tex) = textures.get(image) {
                let (x, y) = build_button_pos(i, self.build_slide);
                blit(tex, x, y, Rect::new(0.0, 28.0 * *state as f32, 28.0, 28.0));
            }
        }
    }

    pub(super) fn draw_info_buttons(&self, textures: &Textures) {
        let Some(tex) = textures.get(INFOS) else { return };
        // Button 8 (objectives) only shows once an objective exists.
        for i in 0..8 {
            let (x, y) = info_button_pos(i);
            let row = self.info_enabled[i] as i32;
            blit(tex, x, y, Rect::new(24.0 * i as f32, 24.0 * row as f32, 24.0, 24.0));
        }
    }

    /// Refreshes the build buttons' states and images (`ActualiserLent` does it once per second;
    /// doing it every frame only removes that delay).
    pub(super) fn refresh_build_buttons(&mut self, world: &World, rules: &Rules, player: usize) {
        self.build_cache = build_buttons(rules)
            .into_iter()
            .map(|k| (self.build_state(world, rules, player, k), button_image(rules, k)))
            .collect();
    }

    /// `Definir_InfoBulle_Batiment`
    /// `details` counts the workshop's objects to list (`(seconds hovered - 2) / 0.3`).
    pub(super) fn build_tooltip(&self, world: &World, rules: &Rules, player: usize, kind: Option<usize>, state: i32, details: f64) -> String {
        let Some(k) = kind else {
            return format!("{}{}", if state == 1 { "" } else { "&" }, self.strings.demolish);
        };
        let (b, p) = (&rules.buildings[k], &world.persos[player]);
        let mut lines = vec![b.name.clone()];
        if state == 2 {
            lines.push(format!("&({})", rules.epochs[(b.epoch - 1).max(0) as usize].name));
        }
        if b.price_money > 0 {
            if p.money() >= b.price_money as f64 {
                lines.push(format!("({} {})", b.price_money, self.strings.coins));
            } else {
                lines.push(format!("&({} / {} {})", p.money().floor(), b.price_money, self.strings.coins));
            }
        }
        for (r, &q) in b.price_resources.iter().enumerate().filter(|(_, q)| **q > 0) {
            let name = rules.resources[r].name.to_lowercase();
            if p.resource(r) >= q as f64 {
                lines.push(format!("({q} {name})"));
            } else {
                lines.push(format!("&({} / {q} {name})", p.resource(r).floor()));
            }
        }
        if b.emperor && !p.emperor {
            lines.push(format!("&{}", self.strings.emperor_only));
        }
        if !b.description.is_empty() {
            lines.push(b.description.clone());
        }
        // After 2 s of hovering, a workshop lists what it makes, one more line every 0.3 s:
        // "- <plural> (<costs>) (<epoch>)", red when unaffordable or too early.
        if details > 0.0 && b.factory {
            let epoch = world.fiefs[p.fief].epoch as i64;
            let shown = (details.floor() as usize).max(1);
            for &o in b.objects.iter().take(shown) {
                let t = &rules.objects[o];
                let costs: Vec<String> = t
                    .price_resources
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| **q > 0)
                    .map(|(r, &q)| {
                        let name = &rules.resources[r].name;
                        if q as f64 > p.resource(r) { format!("{}/{q} {name}", p.resource(r).floor()) } else { format!("{q} {name}") }
                    })
                    .collect();
                let short = t.price_resources.iter().enumerate().any(|(r, &q)| q as f64 > p.resource(r));
                let mut line = format!("- {}", t.plural);
                if !costs.is_empty() {
                    line += &format!(" ({})", costs.join(", "));
                }
                let early = t.epoch - 1 > epoch;
                if early {
                    line += &format!(" ({})", rules.epochs[(t.epoch - 1) as usize].name);
                }
                if short || early {
                    line.insert(0, '&');
                }
                lines.push(line);
            }
        }
        lines.join("#")
    }

    /// `ClsIntBouton.Afficher_Info_Bulle`: pale yellow box at the mouse + (24, 20).
    pub(crate) fn tooltip(&self, m: (i32, i32), s: &str) {
        if s.is_empty() {
            return;
        }
        let (w, h) = tooltip_size(s);
        let (mut x, mut y) = (m.0 + 24, m.1 + 20);
        if x < 0 {
            x = 0;
        } else if x + w + 2 > screen_w() {
            x = screen_w() - w - 2;
        }
        if y < 0 {
            y = 0;
        } else if y + h + 2 > screen_h() {
            y = screen_h() - h - 2;
        }
        draw_rectangle((x - 2) as f32, (y - 2) as f32, (w + 4) as f32, (h + 4) as f32, Color::from_rgba(255, 255, 192, 255));
        outline(x - 2, y - 2, x + w + 2, y + h + 2, BLACK);
        self.text(x, y, s, BLACK);
    }

    /// `Interieur_Boutons`: button clicks, keyboard shortcuts and the hovered tooltip. Returns
    /// whether the mouse is on the interface.
    pub(super) fn interact(
        &mut self,
        world: &World,
        rules: &Rules,
        player: usize,
        input: &Input,
        orders: &mut Vec<Order>,
        on_window: bool,
        mut tip: String,
    ) -> bool {
        let m = input.mouse;
        let perso = player;
        let mut on_ui = on_window;

        // Build buttons (the hover timer restarts when none is hovered); the touch interface
        // has its own buttons instead of these, the move, info and shortcut ones.
        let touch = crate::layout::touch();
        let kinds = if touch { Vec::new() } else { build_buttons(rules) };
        if !kinds.iter().enumerate().any(|(i, _)| {
            let (x, y) = build_button_pos(i, self.build_slide);
            inside(m, x, y, 28, 28)
        }) {
            self.build_hover = None;
        }
        for (i, &kind) in kinds.iter().enumerate().filter(|_| !on_window) {
            let (x, y) = build_button_pos(i, self.build_slide);
            if !inside(m, x, y, 28, 28) {
                continue;
            }
            on_ui = true;
            let state = self.build_cache.get(i).map_or(0, |c| c.0);
            let details = self.hover_details(i);
            tip = self.build_tooltip(world, rules, player, kind, state, details);
            if input.left_pressed && state == 1 {
                match kind {
                    None => orders.push(Order::Demolish { perso }),
                    Some(k) => self.placing = Some(k),
                }
            }
            if input.right_pressed
                && let Some(k) = kind
            {
                let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                orders.extend(next_allied_house(world, player, k).map(|house| Order::GoToHouse { perso, house, repair: ctrl }));
            }
            break;
        }

        // Move buttons
        if !on_ui && !touch {
            for i in (0..18).filter(|&i| self.move_visible(world, player, i)) {
                let (x, y) = move_button_pos(i, self.move_slide);
                if !inside(m, x, y, 28, 28) {
                    continue;
                }
                on_ui = true;
                tip = self.strings.move_tips[i].clone();
                if (2..=9).contains(&i) {
                    let r = rules.terrain_resources[i - 2].resource;
                    if r >= 0 {
                        let p = &world.persos[player];
                        let r = r as usize;
                        tip += &format!("#{} : {} / {}", rules.resources[r].name, p.resource(r).floor(), p.max_resource(r));
                    }
                }
                let click = if i <= 1 { input.left_released } else { input.left_pressed };
                // Right clicks: the attack buttons also send the soldiers.
                let right = input.right_pressed && (10..=14).contains(&i);
                if click || right {
                    if let Some(o) = self.move_order(world, player, i, right) {
                        orders.push(o);
                    }
                }
                break;
            }
        }

        // Info buttons
        if !on_ui && !touch {
            for i in 0..8 {
                let (x, y) = info_button_pos(i);
                if inside(m, x, y, 24, 24) {
                    on_ui = true;
                    tip = self.strings.info_tips[i].clone();
                    if input.left_pressed {
                        self.toggle_info(i);
                    }
                    break;
                }
            }
        }

        // Object shortcut slots
        if !on_ui && !touch {
            if let Some(t) = self.object_slot_buttons(world, rules, player, input, orders) {
                on_ui = true;
                tip = t;
            }
        }
        // Temporary effect icons
        if !on_ui {
            if let Some(t) = self.temp_effect_buttons(world, rules, player, input, orders) {
                on_ui = true;
                tip = t;
            }
        }
        // Status bars
        if !on_ui {
            if let Some(t) = self.status_bar_buttons(world, player, input, orders) {
                on_ui = true;
                tip = t;
            }
        }
        // The screen border counts as interface.
        on_ui |= m.0 <= 0 || m.1 <= 0 || m.0 >= screen_w() || m.1 >= screen_h();

        self.keyboard(world, player, orders);
        self.shortcut_keys(world, rules, player, orders);
        self.tooltip(m, &tip);
        on_ui
    }

    /// The command of move button `i` or its key (`ClsInterface.cls:3955-4050`,
    /// `ClsIntTouches.cls:540-610`). With `right` (a right click) or a modifier, the attack
    /// buttons also send the soldiers; Ctrl (or Shift) on the home button enters without repairing.
    pub(super) fn move_order(&mut self, world: &World, player: usize, i: usize, right: bool) -> Option<Order> {
        let perso = player;
        let p = &world.persos[player];
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        let alive = p.alive();
        Some(match i {
            0 => Order::GoOwnCastle { perso },
            // Shift too, since browsers keep Ctrl+M (mute the tab).
            1 => Order::GoHome { perso, enter: (ctrl || shift) && world.houses[player].built },
            2..=9 if alive => Order::FindResource { perso, kind: i - 2 },
            10..=12 if alive => Order::AttackNearest { perso, kind: (i - 10) as u8, send: right || ctrl || shift || alt, ctrl, shift },
            13 if alive => Order::FindTreasure { perso },
            14 => Order::SetAi { perso, ai: !p.ai },
            16 if !right => Order::Stop { perso },
            17 if alive => Order::Formation { perso, change: true },
            // TODO: 15 attack allies, after the original's confirmation window
            _ => return None,
        })
    }

    /// The keyboard shortcuts of the buttons (`Touches.ini`).
    fn keyboard(&mut self, world: &World, player: usize, orders: &mut Vec<Order>) {
        // `Touches.ini` holds DirectInput scancodes, physical positions like macroquad's key
        // codes, which are named after QWERTY: AZERTY's M is `Semicolon`, W is `Z`, A is `Q`.
        const MOVE_KEYS: [(KeyCode, usize); 17] = [
            (KeyCode::G, 0),
            (KeyCode::Semicolon, 1),
            (KeyCode::F1, 2),
            (KeyCode::F2, 3),
            (KeyCode::F3, 4),
            (KeyCode::F4, 5),
            (KeyCode::F5, 6),
            (KeyCode::F6, 7),
            (KeyCode::F7, 8),
            (KeyCode::F8, 9),
            (KeyCode::Z, 10),
            (KeyCode::X, 11),
            (KeyCode::C, 12),
            (KeyCode::V, 13),
            (KeyCode::Q, 14),
            (KeyCode::S, 16),
            (KeyCode::F, 17),
        ];
        for (key, i) in MOVE_KEYS {
            if is_key_pressed(key) && self.move_visible(world, player, i) {
                if let Some(o) = self.move_order(world, player, i, false) {
                    orders.push(o);
                }
            }
        }
        const INFO_KEYS: [(KeyCode, usize); 8] = [
            (KeyCode::B, 0),
            (KeyCode::R, 1),
            (KeyCode::P, 2),
            (KeyCode::T, 3),
            (KeyCode::I, 4),
            (KeyCode::U, 5),
            (KeyCode::Tab, 6),
            (KeyCode::F10, 7),
        ];
        for (key, i) in INFO_KEYS {
            if is_key_pressed(key) {
                self.toggle_info(i);
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            self.menu_requested = true;
        }
        if is_key_pressed(KeyCode::Delete) {
            orders.push(Order::Demolish { perso: player });
        }
        // `ToucheAfficherConseil` (F9): a tip; always the first one in the rendering tests.
        if is_key_pressed(KeyCode::F9) {
            self.add_tip(if crate::options::test_mode() { 0.0 } else { rand::gen_range(0.0, 1.0) });
        }
    }

    /// Top-left corners of the radial buttons of the current category.
    fn radial_positions(&self, rules: &Rules) -> Vec<(i32, i32, i32)> {
        let Some((cx, cy)) = self.radial else { return Vec::new() };
        let a0 = self.radial_angle[self.radial_category];
        let at = |angle: f32, r: f32, size: i32| {
            let x = cx as f64 + (angle.cos() * r) as f64 - size as f64 / 2.0;
            let y = cy as f64 + (angle.sin() * r) as f64 - size as f64 / 2.0;
            (sim::vb::clng(x) as i32, sim::vb::clng(y) as i32, size)
        };
        if self.radial_category == 0 {
            let step = 3.14 / 7.0 * 2.0;
            (0..7).map(|i| at(step * i as f32 + a0, 40.0, 24)).collect()
        } else {
            // Rings of 7, 14, 21… buttons at radius 40, 80, 120…
            let n = build_buttons(rules).len();
            let (mut ring, mut k) = (0, 0);
            (0..n)
                .map(|_| {
                    if k > 7 + 7 * ring - 1 {
                        ring += 1;
                        k = 0;
                    }
                    let step = 3.14 / (7 + 7 * ring) as f32 * 2.0;
                    let pos = at(step * k as f32 + a0, 40.0 + 40.0 * ring as f32, 28);
                    k += 1;
                    pos
                })
                .collect()
        }
    }

    /// `Activer_BoutonsRadiaux`: a right click on the map opens the ring at the mouse; while
    /// open, another right click switches between the info and build rings.
    pub fn open_radial(&mut self, m: (i32, i32)) {
        if self.radial.is_some() {
            self.radial_category = (self.radial_category + 1) % 2;
        }
        self.radial = Some(m);
    }

    /// The open radial buttons (modal): draw, hover, rotate with the wheel, click.
    pub(super) fn radial_buttons(&mut self, world: &World, rules: &Rules, textures: &Textures, player: usize, input: &Input) {
        let m = input.mouse;
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            // `Pivoter_BoutonsRadiaux` with DirectInput's 120 per notch, literally.
            let a = &mut self.radial_angle[self.radial_category];
            *a += 3.14 * 120.0 * wheel.signum() / 2.0;
            if *a < 0.0 {
                *a += 6.28;
            } else if *a > 6.28 {
                *a -= 6.28;
            }
        }
        let positions = self.radial_positions(rules);
        let mut hovered = None;
        let white = Color::from_rgba(255, 255, 255, 255);
        for (i, &(x, y, size)) in positions.iter().enumerate() {
            if self.radial_category == 0 {
                if let Some(tex) = textures.get(INFOS) {
                    let row = self.info_enabled[i] as i32;
                    blit(tex, x, y, Rect::new(24.0 * i as f32, 24.0 * row as f32, 24.0, 24.0));
                }
            } else if let Some((state, image)) = self.build_cache.get(i) {
                if let Some(tex) = textures.get(image) {
                    blit(tex, x, y, Rect::new(0.0, 28.0 * *state as f32, 28.0, 28.0));
                }
            }
            if inside(m, x, y, size, size) {
                hovered = Some(i);
            }
        }
        if let Some(i) = hovered {
            let (x, y, size) = positions[i];
            draw_rectangle_lines((x - 1) as f32, (y - 1) as f32, (size + 2) as f32, (size + 2) as f32, 3.0, white);
        }
        let (cx, cy) = self.radial.unwrap();
        draw_circle_lines(cx as f32, cy as f32, 2.0, 3.0, white);
        draw_line(cx as f32, cy as f32, m.0 as f32, m.1 as f32, 3.0, white);

        let kinds = build_buttons(rules);
        let tip = match hovered {
            Some(i) if self.radial_category == 0 => self.strings.info_tips[i].clone(),
            Some(i) => {
                let state = self.build_cache.get(i).map_or(0, |c| c.0);
                let details = self.hover_details(1000 + i);
                self.build_tooltip(world, rules, player, kinds[i], state, details)
            }
            None => {
                self.build_hover = None;
                String::new()
            }
        };
        self.tooltip(m, &tip);

        if input.right_pressed {
            self.open_radial(m);
        } else if input.left_pressed {
            match hovered {
                Some(i) if self.radial_category == 0 => self.toggle_info(i),
                Some(i) => {
                    if self.build_cache.get(i).map_or(0, |c| c.0) == 1 {
                        if let Some(k) = kinds[i] {
                            self.placing = Some(k);
                        }
                    }
                }
                None => {}
            }
            self.radial = None;
            self.swallow_release = true;
        }
    }

    pub fn radial_open(&self) -> bool {
        self.radial.is_some()
    }

    /// `(seconds hovering button `id` - 2) / 0.3`, restarting when another button is hovered.
    fn hover_details(&mut self, id: usize) -> f64 {
        let now = get_time();
        match self.build_hover {
            Some((h, _)) if h == id => {}
            _ => self.build_hover = Some((id, now)),
        }
        let start = self.build_hover.map_or(now, |(_, t)| t);
        (now - start - 2.0) / 0.3
    }

    /// Info button `i`: shows or hides its window; the menu/pause one (F10) asks for the menu.
    pub(super) fn toggle_info(&mut self, i: usize) {
        if i == 7 {
            self.menu_requested = true;
        } else {
            self.info_enabled[i] = !self.info_enabled[i];
        }
    }

    /// The menu/pause button (F10) or Escape was pressed since the last call: the page's menu
    /// should open.
    pub fn take_menu_request(&mut self) -> bool {
        std::mem::take(&mut self.menu_requested)
    }

    /// The menu is open (and, in solo, the game paused): the menu/pause button is lit.
    pub fn set_menu_open(&mut self, open: bool) {
        self.info_enabled[7] = open;
    }

    pub fn paused(&self) -> bool {
        self.info_enabled[7]
    }

    /// Building placement on the map; returns whether the click was used.
    /// `centre` is the cursor's centre in world pixels.
    pub fn map_input(&mut self, world: &World, rules: &Rules, player: usize, input: &Input, centre: (i32, i32), orders: &mut Vec<Order>) -> bool {
        let Some(kind) = self.placing else { return false };
        // With the touch interface the building goes where the finger drags it, and the bar
        // confirms it: taps on the map are only for that.
        if crate::layout::touch() {
            return input.left_released;
        }
        if input.right_pressed {
            self.placing = None;
            return true;
        }
        if input.left_released {
            let (x, y, ok) = self.footprint(world, rules, player, kind, centre);
            if ok {
                orders.push(Order::BuildHouse { perso: player, x, y, kind });
                if !(input.ctrl || input.shift) {
                    self.placing = None;
                }
            }
            return true;
        }
        false
    }

    /// The building's world rectangle under the cursor and whether it can be placed there.
    /// Centred on the cursor's centre (`mouse + 12`: the cursor is 24×24, hot spot top-left).
    pub(super) fn footprint(&self, world: &World, rules: &Rules, player: usize, kind: usize, centre: (i32, i32)) -> (i32, i32, bool) {
        let (w, h) = rules.buildings[kind].size;
        let x = centre.0 - w / 2;
        let y = centre.1 - h / 2;
        // (a torus has no edge)
        let inside_map = world.terrain.torus
            || x >= 0
            && y >= 0
            && x + w <= world.terrain.width as i32 * 16
            && y + h <= world.terrain.height as i32 * 16;
        let obstacle = if self.rebuild_elsewhere { world.obstacle(rules, player, x, y, w, h) } else { world.obstacle_rebuilding(rules, player, x, y, w, h) };
        let ok = inside_map && obstacle == 0;
        (x, y, ok)
    }

    /// `Afficher_Surface_A_Batir`: the hatched footprint, green when free, red when blocked.
    /// Drawn with the map's camera; `centre` is the cursor's centre in world pixels. Returns
    /// the cursor type (2 or 3) while placing.
    pub fn draw_placement(&self, world: &World, rules: &Rules, player: usize, centre: (i32, i32), cam: (i32, i32)) -> Option<i32> {
        let kind = self.placing?;
        let (w, h) = rules.buildings[kind].size;
        let (x, y, ok) = self.footprint(world, rules, player, kind, centre);
        let (x, y) = (x - cam.0, y - cam.1);
        let c = if ok { Color::from_rgba(0, 192, 0, 255) } else { RED };
        super::rounded_outline(x, y, x + w, y + h, c);
        let mut d = 0;
        while d < w + h {
            // upward diagonal hatch, clipped to the rectangle
            let (x0, y0) = (x + (d - h).max(0), y + h.min(d));
            let (x1, y1) = (x + d.min(w), y + (d - w).max(0));
            draw_line(x0 as f32, y0 as f32, x1 as f32, y1 as f32, 1.0, c);
            d += 8;
        }
        Some(if ok { 2 } else { 3 })
    }
}
