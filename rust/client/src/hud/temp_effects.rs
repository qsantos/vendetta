//! Temporary effect icons (`Afficher_Effets_Temporaires`): one per effect in `EffetsTemp`,
//! right of the 9 object shortcut slots, with its remaining time.

use super::object_tooltip::{EffectInfo, object_tooltip};
use super::{Hud, Input, status_bar};
use crate::assets::Textures;
use macroquad::prelude::*;
use sim::rules::Rules;
use sim::world::{Order, World};

/// `EffetTemporatireDureeEffacement`: below this many seconds left, the icon blinks.
const FADE: i64 = 5;

/// `Actualiser_Effets_Temporaires`: icon `i` sits after the 9 shortcut slots
/// (`EffetTemporatirePositionX + EffetTemporatireIntervalX * (i + NombreRaccourciObjets)`).
fn effect_pos(i: usize) -> (i32, i32) {
    // the touch interface has no shortcut slots before them
    let first = if crate::layout::touch() { 0 } else { 9 };
    (56 + 32 * (i as i32 + first), 0)
}

impl Hud {
    /// `Afficher_Effets_Temporaires`: row 1 of the object's image, translucent and pulsing in
    /// its last seconds, over a white bar of the remaining time.
    pub(super) fn draw_temp_effects(&self, world: &World, rules: &Rules, textures: &Textures, player: usize) {
        let p = &world.persos[player];
        for (i, e) in p.effects.iter().enumerate() {
            let (x, y) = effect_pos(i);
            let d = e.duration;
            // `EffetTemporatireTransparent`: the opacity follows a cosine of the time left.
            // `Compteur * 10` is the phase of the slow tick, 0..9.
            let alpha = if d < FADE && d != -1 && p.alive() {
                let t = d as f64 + world.slow_phase as f64;
                (((FADE as f64 - t + 0.1) / 1.5).cos() + 1.0) / 2.0
            } else {
                1.0
            };
            if let Some(tex) = textures.get(&format!("{}.bmp", rules.objects[e.object].image)) {
                let src = Rect::new(0.0, 32.0, 32.0, 32.0);
                let colour = Color::new(1.0, 1.0, 1.0, alpha as f32);
                tex.draw_ex(x as f32, y as f32, colour, DrawTextureParams { source: Some(src), ..Default::default() });
            }
            if d != -1 {
                // At the bottom of a 32×32 slot (`Hauteur - 7`, `Largeur - 4`).
                status_bar(d as f64, d as f64, e.duration_max as f64, x + 2, y + 25, 28, 5, Some(WHITE));
            }
        }
    }

    /// The icons' clicks and tooltip (`ClsInterface.cls:4171-4210`): right cancels the effect
    /// unless the object forbids it, left equips again the bag objects that cast it. Returns
    /// the tooltip when over an icon.
    pub(super) fn temp_effect_buttons(&self, world: &World, rules: &Rules, player: usize, input: &Input, orders: &mut Vec<Order>) -> Option<String> {
        let m = input.mouse;
        let p = &world.persos[player];
        // `Interieur_Bouton`, inclusive on both ends, with the object image's 32×32.
        let i = (0..p.effects.len()).find(|&i| {
            let (x, y) = effect_pos(i);
            m.0 >= x && m.0 <= x + 32 && m.1 >= y && m.1 <= y + 32
        })?;
        let object = p.effects[i].object;
        let perso = player;
        if input.right_pressed {
            if !rules.objects[object].effect_uncancellable {
                orders.push(Order::CancelEffect { perso, effect: i });
            }
        } else if input.left_pressed {
            for (slot, item) in p.inventory.iter().enumerate() {
                if item.as_ref().is_some_and(|it| it.object == object) {
                    orders.push(Order::EquipObject { perso, slot, right: false });
                }
            }
        }
        // The original shows no tooltip on the frame of a click.
        if input.left_pressed || input.right_pressed {
            return Some(String::new());
        }
        Some(object_tooltip(rules, &self.tooltip_labels, p, object, 0, Some(EffectInfo::new(rules, world, player, i)), None))
    }
}
