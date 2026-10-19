//! The status bars' buttons: eat now, automatic restoring and its threshold
//! (`Interieur_Boutons`, `Actions_Automatiques`, `Afficher_Barre_Statut_Auto`).

use super::{Hud, Input, LIGHT_GREY, outline};
use crate::layout::ay;
use crate::storage;
use macroquad::prelude::*;
use sim::vb::clng;
use sim::world::{Order, World};

/// Top of each bar (Vie, Energie, Magie, Moral) at 1024×768, and their box.
pub const BAR_Y: [i32; 4] = [691, 709, 727, 745];
const BAR_X: i32 = 7;
const BAR_W: i32 = 136;
const BAR_H: i32 = 16;
const ORANGE: Color = Color::new(1.0, 128.0 / 255.0, 0.0, 1.0);

/// Per bar: automatic restoring on, and its threshold (fraction of the maximum). Kept in
/// local storage, like the original kept them in the character file.
pub struct AutoRestore {
    pub on: [bool; 4],
    pub threshold: [f32; 4],
}

impl AutoRestore {
    pub fn load() -> AutoRestore {
        // `ModSavPerso`: thresholds default to 100% for Vie and 50% for the others.
        let mut a = AutoRestore { on: [false; 4], threshold: [1.0, 0.5, 0.5, 0.5] };
        if let Some(text) = storage::load("autorestore").and_then(|b| String::from_utf8(b).ok()) {
            let v: Vec<f32> = text.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if v.len() == 8 {
                for i in 0..4 {
                    a.on[i] = v[i] != 0.0;
                    a.threshold[i] = v[4 + i].clamp(0.0, 1.0);
                }
            }
        }
        a
    }

    fn store(&self) {
        let v: Vec<String> = self.on.iter().map(|&b| (b as u8).to_string()).chain(self.threshold.iter().map(f32::to_string)).collect();
        storage::store("autorestore", v.join(" ").as_bytes());
    }
}

fn bar_rect(i: usize) -> (i32, i32) {
    (BAR_X, ay(BAR_Y[i]))
}

fn inside(m: (i32, i32), x: i32, y: i32, w: i32, h: i32) -> bool {
    m.0 >= x && m.0 <= x + w && m.1 >= y && m.1 <= y + h
}

impl Hud {
    /// The bars' outer contours: orange with a threshold marker when restoring
    /// automatically, grey otherwise (`Afficher_Contour_Barre_Statut`).
    pub(super) fn draw_bar_contours(&self) {
        for i in 0..4 {
            let (x, y) = bar_rect(i);
            let colour = if self.auto.on[i] { ORANGE } else { LIGHT_GREY };
            outline(x - 1, y - 1, x + BAR_W + 1, y + BAR_H + 1, colour);
            if self.auto.on[i] {
                let mx = x + clng(BAR_W as f64 * self.auto.threshold[i] as f64) as i32;
                outline(mx, y, mx + 1, y + BAR_H, colour);
            }
        }
    }

    pub fn over_status_bar(&self, m: (i32, i32)) -> bool {
        (0..4).any(|i| {
            let (x, y) = bar_rect(i);
            inside(m, x, y, BAR_W, BAR_H)
        })
    }

    /// Left click eats to restore the bar, right click toggles automatic restoring, the
    /// wheel sets its threshold (2.5% per notch, which also turns it on). Returns the
    /// tooltip when over a bar.
    pub(super) fn status_bar_buttons(&mut self, world: &World, player: usize, input: &Input, orders: &mut Vec<Order>) -> Option<String> {
        let m = input.mouse;
        let i = (0..4).find(|&i| {
            let (x, y) = bar_rect(i);
            inside(m, x, y, BAR_W, BAR_H)
        })?;
        let p = &world.persos[player];
        let max = p.bonus_max[i];
        if input.right_pressed {
            self.auto.on[i] = !self.auto.on[i];
            self.auto.store();
        } else if input.left_pressed && p.alive() && p.bar_total(i) < max as f64 {
            orders.push(Order::EatFor { perso: player, bar: i });
        }
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            let t = &mut self.auto.threshold[i];
            *t = ((*t + 0.025 * wheel.signum()) * 100.0).round() / 100.0;
            *t = t.clamp(0.0, 1.0);
            self.auto.on[i] = true;
            self.auto.store();
        }
        let (x, _) = bar_rect(i);
        let marker = x + clng(BAR_W as f64 * self.auto.threshold[i] as f64) as i32;
        Some(if self.auto.on[i] && (m.0 == marker || m.0 == marker + 1) {
            let t = self.auto.threshold[i];
            format!(
                "{}->{} / {max} ({}%){}",
                self.strings.threshold[0],
                (max as f32 * t).floor(),
                (t * 100.0).floor(),
                self.strings.threshold[1]
            )
        } else {
            format!("{:.2} / {max}", p.bars[i])
        })
    }

    /// `Actions_Automatiques`, once per tick: eat for every bar below its threshold.
    pub fn automatic_actions(&self, world: &World, player: usize, orders: &mut Vec<Order>) {
        let p = &world.persos[player];
        if !p.alive() {
            return;
        }
        for i in 0..4 {
            if self.auto.on[i] && p.bar_total(i) < p.bonus_max[i] as f64 * self.auto.threshold[i] as f64 {
                orders.push(Order::EatFor { perso: player, bar: i });
            }
        }
    }
}
