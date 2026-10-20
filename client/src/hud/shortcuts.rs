//! Object shortcut keys (`TouchesRaccourciObjets`): each object type can be bound to one of
//! 27 keys — 1..9, Alt+1..9 and Shift+1..9 — shown as the 9 slots at the top left.

use super::{GREY, Hud, Input, OBJETS, status_bar};
use crate::assets::Textures;
use crate::storage;
use macroquad::prelude::*;
use sim::files::Files;
use sim::items::bonus_sup;
use sim::params::{Kind, Params};
use sim::perso::Perso;
use sim::rules::Rules;
use sim::world::{Order, World};

/// Keys per set (`NombreTouchesObjets`).
const KEYS: i32 = 9;

/// The key of each object type: `1 + i + 9 * set` for slot `i` of a set (0 plain, 1 Alt,
/// 2 Shift), 0 for none. Kept in local storage, like the original kept it per character.
pub struct Shortcuts {
    key: Vec<i32>,
}

impl Shortcuts {
    pub fn load(objects: usize) -> Shortcuts {
        let mut key = vec![0; objects];
        if let Some(text) = storage::load("shortcuts").and_then(|b| String::from_utf8(b).ok()) {
            for (k, v) in key.iter_mut().zip(text.split_whitespace()) {
                *k = v.parse().unwrap_or(0);
            }
        }
        Shortcuts { key }
    }

    fn store(&self) {
        let text: Vec<String> = self.key.iter().map(i32::to_string).collect();
        storage::store("shortcuts", text.join(" ").as_bytes());
    }

    fn bind(&mut self, object: usize, key: i32) {
        if let Some(k) = self.key.get_mut(object) {
            *k = key;
            self.store();
        }
    }

    /// The object types bound to `key`.
    fn bound(&self, key: i32) -> impl Iterator<Item = usize> + '_ {
        self.key.iter().enumerate().filter(move |&(_, &k)| k == key).map(|(o, _)| o)
    }

    /// The label drawn on an inventory or equipment slot holding `object`: `(n)` for a key of
    /// the displayed set, `(+)` for another set, nothing when unbound (`Afficher_TouchesObjets`).
    pub fn label(&self, object: usize, set: i32) -> Option<String> {
        let key = *self.key.get(object)?;
        let shown = key - set * KEYS;
        if shown > 0 && shown <= KEYS {
            Some(format!("({shown})"))
        } else if key > 0 {
            Some("(+)".to_owned())
        } else {
            None
        }
    }
}

/// An animation of `Parametres_Effets.ini` drawn still, one frame at a time.
pub struct Clock {
    image: String,
    w: f32,
    h: f32,
    /// `NombreEtapes`
    steps: i64,
}

/// `[Effets] EffetDureeRestante` and `EffetDureeRestante2`: the clocks over a recharging
/// object, red when no charge is left, white otherwise.
pub fn recharge_clocks(files: &Files) -> [Option<Clock>; 2] {
    let Ok(params) = Params::load(files, "Francais") else { return [None, None] };
    let num = |r: &sim::params::Record, k: &str| r.get(k).map_or(0.0, sim::ini::vb_val);
    ["EffetDureeRestante", "EffetDureeRestante2"].map(|k| {
        let i = sim::vb::clng(num(params.global(Kind::Effets), k) - 1.0);
        let r = params.list(Kind::Effets).get(usize::try_from(i).ok()?)?;
        Some(Clock {
            image: format!("Images/Effets/{}.bmp", r.get("Fichier").unwrap_or("").trim()),
            w: num(r, "Largeur") as f32,
            h: num(r, "Hauteur") as f32,
            steps: num(r, "NombreEtapes") as i64,
        })
    })
}

/// The set Alt or Shift selects (`BoutonRaccourciObjetDecalageY`).
pub fn current_set() -> i32 {
    if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
        1
    } else if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
        2
    } else {
        0
    }
}

fn worn(p: &Perso, object: usize) -> bool {
    p.equipped.iter().flatten().any(|it| it.object == object)
}

fn owned(p: &Perso, object: usize) -> i64 {
    p.inventory.iter().chain(&p.equipped).flatten().filter(|it| it.object == object).map(|it| it.count).sum()
}

impl Hud {
    /// Binds `object` to shortcut key `key` (1..27).
    pub fn bind_object(&mut self, object: usize, key: i32) {
        self.shortcuts.bind(object, key);
    }

    /// `Afficher_Raccourcis_Objets`: the 9 slots of the current set with their objects' icons
    /// (the worn one on top), attack/defence bars on worn weapons, and counters. Only the
    /// sprite's frame is drawn: its labels are low-resolution pixels, written with the font.
    pub(super) fn draw_object_slots(&self, world: &World, rules: &Rules, textures: &Textures, player: usize) {
        let p = &world.persos[player];
        let set = current_set();
        // GDI's `ForeColor`, which the charge counters are written in: the epoch box's grey,
        // then black once a weapon's bars are drawn, white once a slot's counter is.
        let mut fore = GREY;
        for i in 0..KEYS {
            let key = i + 1 + set * KEYS;
            let (x, y) = (56 + 32 * i, 0);
            // `Enabled`: an object of this key is worn.
            let enabled = self.shortcuts.bound(key).any(|o| worn(p, o));
            if let Some(tex) = textures.get(OBJETS) {
                let row = enabled as i32 + 2 * set;
                for (sx, sy, w, h) in [(0, 0, 32, 3), (0, 29, 32, 3), (0, 3, 3, 26), (29, 3, 3, 26)] {
                    let src = Rect::new((32 * i + sx) as f32, (32 * row + sy) as f32, w as f32, h as f32);
                    tex.draw_ex((x + sx) as f32, (y + sy) as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
                }
            }
            // Owned objects first, then the first worn one again on top.
            let mut icons: Vec<usize> = self.shortcuts.bound(key).filter(|&o| owned(p, o) > 0).collect();
            let mut message = String::new();
            if !enabled {
                for &o in &icons {
                    let t = &rules.objects[o];
                    if owned(p, o) > 1 && t.usable && t.single_use {
                        message = format!("x{}", owned(p, o));
                    } else if t.ammo_quantity > 0.0 && t.ammo >= 0 {
                        message = ammo_message(p, t.ammo as usize, t.ammo_quantity);
                    }
                }
            }
            if let Some(o) = self.shortcuts.bound(key).find(|&o| worn(p, o)) {
                icons.push(o);
                let t = &rules.objects[o];
                if t.ammo_quantity > 0.0 && t.ammo >= 0 {
                    message = ammo_message(p, t.ammo as usize, t.ammo_quantity);
                }
            }
            for &o in &icons {
                if let Some(tex) = textures.get(&format!("{}.bmp", rules.objects[o].image)) {
                    let src = Rect::new(0.0, 0.0, 32.0, 32.0);
                    tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
                }
                let weapon = rules.objects[o].category <= 1;
                let (max_attack, max_defense) = (p.bonus_max[4] as f64, p.bonus_max[5] as f64);
                if enabled && weapon && (p.attack < max_attack || p.defense < max_defense || p.reload > 0) {
                    fore = BLACK;
                    if p.reload == 0 {
                        status_bar(p.attack, p.attack, max_attack, x, y + 22, 32, 5, Some(Color::from_rgba(255, 0, 0, 255)));
                    } else {
                        status_bar(0.0, (p.reload_max - p.reload) as f64, p.reload_max as f64, x, y + 22, 32, 5, None);
                    }
                    status_bar(p.defense, p.defense, max_defense, x, y + 27, 32, 5, Some(Color::from_rgba(0, 0, 255, 255)));
                }
                self.draw_recharge(rules, textures, p, o, x, y, fore);
            }
            let label = match set {
                1 => format!("Alt {}", i + 1),
                2 => format!("Maj{}", i + 1),
                _ => (i + 1).to_string(),
            };
            let colour = if enabled { Color::from_rgba(24, 24, 24, 255) } else { WHITE };
            // The sprite's labels have their cap tops 4 px into the cell.
            self.small_font.draw(&label, x + 4, y + 12, colour);
            if !message.is_empty() {
                let (text, colour) = match message.strip_prefix('&') {
                    Some(t) => (t, Color::from_rgba(255, 0, 0, 255)),
                    None => (message.as_str(), WHITE),
                };
                self.small_text(x + 3, y + 18, text, colour);
                fore = WHITE;
            }
        }
    }

    /// `Afficher_Duree_Recharge`: over a recharging object, a clock at the point of the current
    /// charge's recharge, red when no charge is left; and the charges left out of the
    /// total ("1/3") for objects with several.
    fn draw_recharge(&self, rules: &Rules, textures: &Textures, p: &Perso, o: usize, x: i32, y: i32, fore: Color) {
        let t = &rules.objects[o];
        if t.recharge <= 0 {
            return;
        }
        let total = bonus_sup(rules, p.object_skill_level(rules, o), t.charges);
        // `EffetRecharge_TempRestant`
        let left = p.recharges.iter().find(|r| r.object == o).map_or(0, |r| r.duration);
        let ready = (total as f64 - left as f64 / t.recharge as f64).floor() as i64;
        if left > 0 {
            let clock = if ready >= total || ready <= 0 { &self.clocks[0] } else { &self.clocks[1] };
            if let Some(c) = clock
                && let Some(tex) = textures.get(&c.image)
            {
                let frame = ((left % t.recharge) as f64 / t.recharge as f64 * c.steps as f64).floor() as f32;
                let src = Rect::new(frame * c.w, 0.0, c.w, c.h);
                tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
            }
        }
        if t.charges > 0.0 && total > 1 {
            let message = format!("{ready}/{total}");
            self.small_text(x + 15 - message.len() as i32 * 2, y + 9, &message, fore);
        }
    }

    /// Clicks and tooltip on the slots: left equips the key's objects, or binds the held
    /// object to it; right clears the key. Returns the tooltip when over a slot.
    pub(super) fn object_slot_buttons(&mut self, world: &World, rules: &Rules, player: usize, input: &Input, orders: &mut Vec<Order>) -> Option<String> {
        let m = input.mouse;
        let i = (0..KEYS).find(|&i| m.0 >= 56 + 32 * i && m.0 <= 56 + 32 * i + 32 && m.1 >= 0 && m.1 <= 32)?;
        let set = current_set();
        let key = i + 1 + set * KEYS;
        let p = &world.persos[player];
        if input.right_pressed {
            for o in self.shortcuts.bound(key).collect::<Vec<_>>() {
                self.shortcuts.bind(o, 0);
            }
        } else if input.left_pressed {
            if p.selected_object >= 0 {
                self.shortcuts.bind(p.selected_object as usize, key);
                orders.push(Order::Unselect { perso: player });
            } else {
                self.equip_key(world, rules, player, key, orders);
            }
        }
        let prefix = ["", "Alt+", "Maj+"][set as usize];
        let mut tip = format!("{}({prefix}{})", self.strings.shortcut, i + 1);
        for o in self.shortcuts.bound(key) {
            let red = if owned(p, o) > 0 { "" } else { "&" };
            tip += &format!("#{red}- {}", rules.objects[o].name);
        }
        Some(tip)
    }

    /// `Raccourcis_Objets_Equiper`: equips every bag object bound to `key`; a second one-hand
    /// weapon (or ring) first puts away what the second hand (or ring) holds.
    pub(super) fn equip_key(&self, world: &World, rules: &Rules, player: usize, key: i32, orders: &mut Vec<Order>) {
        let p = &world.persos[player];
        let perso = player;
        let (mut hand, mut ring) = (false, false);
        for (slot, item) in p.inventory.iter().enumerate() {
            let Some(item) = item else { continue };
            if self.shortcuts.key.get(item.object) != Some(&key) {
                continue;
            }
            match rules.objects[item.object].category {
                0 if hand && p.equipped.get(1).is_some_and(Option::is_some) => orders.push(Order::StoreEquipped { perso, slot: 1 }),
                0 => hand = true,
                6 if ring && p.equipped.get(6).is_some_and(Option::is_some) => orders.push(Order::StoreEquipped { perso, slot: 6 }),
                6 => ring = true,
                _ => {}
            }
            orders.push(Order::EquipObject { perso, slot, right: false });
        }
    }

    /// The digit keys: equip a key's objects; Ctrl+digit binds everything worn to it; over a
    /// bag or equipment object, a digit binds that object (0 unbinds it).
    pub(super) fn shortcut_keys(&mut self, world: &World, rules: &Rules, player: usize, orders: &mut Vec<Order>) {
        const DIGITS: [KeyCode; 10] = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
            KeyCode::Key0,
        ];
        let set = current_set();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        for (d, &code) in DIGITS.iter().enumerate() {
            if !is_key_pressed(code) {
                continue;
            }
            let digit = d as i32 + 1; // 10 for the 0 key
            if let Some(o) = self.hovered_object {
                self.shortcuts.bind(o, if digit > KEYS { 0 } else { digit + set * KEYS });
            } else if digit <= KEYS && ctrl {
                let p = &world.persos[player];
                for it in p.equipped.iter().flatten() {
                    self.shortcuts.bind(it.object, digit + set * KEYS);
                }
            } else if digit <= KEYS {
                self.equip_key(world, rules, player, digit + set * KEYS, orders);
            }
        }
    }
}

/// `"(NN)"` shots left for an ammunition-using weapon, red `&` when none.
fn ammo_message(p: &Perso, ammo: usize, per_shot: f64) -> String {
    let shots = (p.resource(ammo) / per_shot).floor();
    format!("{}({shots:02})", if shots == 0.0 { "&" } else { "" })
}
