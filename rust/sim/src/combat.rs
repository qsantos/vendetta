//! Combat (`ModCombat`). For now only the attack orders of `ClsJeuPerso`; they set the action
//! and the target, and nothing deals damage yet.
// TODO: Deplacement1/2 for actions 205 and 300-303, damage, deaths and loot (spec 03 §3.4-3.5, §6)

use crate::perso::action;
use crate::rules::Rules;
use crate::world::World;

/// `Attaquer_Perso` etc.: `Action` codes 300-303.
pub const ATTACK_PERSO: i32 = 300;
pub const ATTACK_HOUSE: i32 = 301;
pub const ATTACK_CASTLE: i32 = 302;
pub const ATTACK_GROUND: i32 = 303;

impl World {
    /// The end of every attack order: the cooldown restarts and the next hit is forced; soldiers
    /// follow.
    fn attack_order(&mut self, rules: &Rules, i: usize) {
        let p = &mut self.persos[i];
        p.reload = p.reload_object;
        p.reload_max = p.reload_object;
        p.force_attack = true;
        if p.soldiers > 0 {
            self.order_charge(rules, i, -1, -1, false, false, false);
        }
    }

    /// `Attaquer_Perso`: from a house with `Tir`, a perso stays inside while the target is in range.
    pub fn attack_perso(&mut self, rules: &Rules, i: usize, k: usize) {
        let (p, t) = (&self.persos[i], &self.persos[k]);
        let r = p.bonus_range;
        let inside = p.in_house && self.houses[p.house_target].shoot && r > 0.0;
        let in_range = t.x as f64 + t.w as f64 >= p.x as f64 - r
            && t.y as f64 + t.h as f64 >= p.y as f64 - r
            && t.x as f64 <= p.x as f64 + p.w as f64 + r
            && t.y as f64 <= p.y as f64 + p.h as f64 + r;
        let p = &mut self.persos[i];
        p.action = ATTACK_PERSO;
        p.perso_target = k;
        if !(inside && in_range) {
            p.set_visible(true);
        }
        self.attack_order(rules, i);
    }

    /// `Attaquer_Maison`
    pub fn attack_house(&mut self, rules: &Rules, i: usize, h: usize) {
        let p = &mut self.persos[i];
        p.action = ATTACK_HOUSE;
        p.house_target = h;
        p.set_visible(true);
        self.attack_order(rules, i);
    }

    /// `Attaquer_Chateau`
    pub fn attack_castle(&mut self, rules: &Rules, i: usize, c: usize) {
        let p = &mut self.persos[i];
        p.action = ATTACK_CASTLE;
        p.castle_target = c;
        p.set_visible(true);
        self.attack_order(rules, i);
    }

    /// `Attaquer_Terrain`: shoot at (x, y) while walking on; only with a projectile weapon.
    pub fn attack_ground(&mut self, i: usize, x: i64, y: i64, kind: i64) {
        let p = &self.persos[i];
        if p.launcher < 0 {
            return;
        }
        let shoot = self.houses[p.house_target].shoot;
        let p = &mut self.persos[i];
        p.ground_target_kind = kind;
        p.action = ATTACK_GROUND;
        if !((p.in_own_house || p.in_house) && shoot) {
            p.set_visible(true);
        }
        p.reload = p.reload_object;
        p.reload_max = p.reload_object;
        p.force_attack = true;
        p.ground_target = (x, y);
        p.orient(x, y);
    }

    /// `Aller_Donner`: walk to perso `k` to give it the selection.
    pub fn go_give(&mut self, i: usize, k: usize) {
        let p = &mut self.persos[i];
        p.perso_target = k;
        p.action = action::GO_GIVE;
        p.set_visible(true);
    }
}
