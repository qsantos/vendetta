//! Combat (`ModCombat`, the combat part of `ClsJeuPerso`): the attack orders, following and
//! hitting a target (`Deplacement1/2`), damage (`Perso_Attaquer_Perso`), kills and looting
//! (`Perso_Tuer_Perso`), buildings (`Perso_Attaquer_Batiment`), area attacks and trampling.

use crate::building::Building;
use crate::event::NumberKind;
use crate::perso::action;
use crate::rules::Rules;
use crate::treasure::TREASURE_SIZE;
use crate::vb::clng;
use crate::terrain::Terrain;
use crate::world::World;

/// `Attaquer_Perso` etc.: `Action` codes 300-303.
pub const ATTACK_PERSO: i32 = 300;
pub const ATTACK_HOUSE: i32 = 301;
pub const ATTACK_CASTLE: i32 = 302;
pub const ATTACK_GROUND: i32 = 303;

/// A building that can be hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    House(usize),
    Castle(usize),
}

/// Whether the box (x, y, w, h) is within `r` of the box of `(px, py, pw, ph)` (its copy nearest
/// to it on a torus).
fn within(t: &Terrain, r: f64, (px, py, pw, ph): (i64, i64, i32, i32), (x, y, w, h): (i64, i64, i32, i32)) -> bool {
    let (x, y) = t.near((px, py), (x, y));
    let (px, py, pw, ph) = (px as f64, py as f64, pw as f64, ph as f64);
    let (x, y, w, h) = (x as f64, y as f64, w as f64, h as f64);
    x + w >= px - r && y + h >= py - r && x <= px + pw + r && y <= py + ph + r
}

impl World {
    fn perso_box(&self, i: usize) -> (i64, i64, i32, i32) {
        let p = &self.persos[i];
        (p.x, p.y, p.w, p.h)
    }

    /// `Definir_Vie = v` on perso `i`, dying (`Vivant = False`) when it reaches 0.
    pub fn set_life(&mut self, rules: &Rules, i: usize, v: f64) {
        if self.persos[i].set_life(rules, v) {
            self.kill(rules, i);
        }
    }

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
        let p = &self.persos[i];
        let r = p.bonus_range;
        let inside = p.in_house && self.houses[p.house_target].shoot && r > 0.0;
        let in_range = within(&self.terrain, r, self.perso_box(i), self.perso_box(k));
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
        p.orient(&self.terrain, x, y);
    }

    /// `Aller_Donner`: walk to perso `k` to give it the selection.
    pub fn go_give(&mut self, i: usize, k: usize) {
        let p = &mut self.persos[i];
        p.perso_target = k;
        p.action = action::GO_GIVE;
        p.set_visible(true);
    }

    // ---------------------------------------------------------------- Deplacement1/2

    /// The centre of the box (x, y, w, h) for perso `i`'s top-left corner.
    fn centre_for(&self, i: usize, x: i64, y: i64, w: i32, h: i32) -> (i64, i64) {
        let p = &self.persos[i];
        (
            clng(x as f64 + w as f64 / 2.0 - p.w as f64 / 2.0),
            clng(y as f64 + h as f64 / 2.0 - p.h as f64 / 2.0),
        )
    }

    /// `Deplacement1` after `Calculer_Vitalite`: an attacker or a giver heads for its target.
    pub(crate) fn track_target(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        match p.action {
            action::GO_GIVE => {
                let t = &self.persos[p.perso_target];
                if (t.visible() || t.in_own_house) && t.alive() {
                    let (x, y) = self.centre_for(i, t.x, t.y, t.w, t.h);
                    self.persos[i].dest = self.terrain.near((self.persos[i].x, self.persos[i].y), (x, y));
                } else {
                    self.stop(rules, i);
                }
            }
            ATTACK_PERSO => {
                let k = p.perso_target;
                let t = &self.persos[k];
                if t.visible() && t.alive() {
                    let (x, y) = self.centre_for(i, t.x, t.y, t.w, t.h);
                    self.persos[i].dest = self.terrain.near((self.persos[i].x, self.persos[i].y), (x, y));
                } else if t.alive() {
                    // The AIs go after a target hiding in a building.
                    if p.ai {
                        if t.in_own_house {
                            self.attack_house(rules, i, k);
                        } else if t.in_house {
                            self.attack_house(rules, i, t.house_target);
                        } else if t.in_own_castle {
                            self.attack_castle(rules, i, t.fief);
                        } else if t.in_castle {
                            self.attack_castle(rules, i, t.castle_target);
                        }
                    } else {
                        self.stop(rules, i);
                    }
                    if !self.houses[k].shoot {
                        self.say(rules, i, 15);
                    }
                } else {
                    self.stop(rules, i);
                    if self.persos[i].projectile {
                        self.kill(rules, i);
                    }
                }
            }
            ATTACK_HOUSE => {
                let m = &self.houses[p.house_target];
                if m.visible() {
                    let (x, y) = self.centre_for(i, m.x as i64, m.y as i64, m.w, m.h);
                    self.persos[i].dest = self.terrain.near((self.persos[i].x, self.persos[i].y), (x, y));
                } else {
                    self.stop(rules, i);
                    if self.persos[i].projectile {
                        self.kill(rules, i);
                    }
                }
            }
            ATTACK_CASTLE => {
                let c = &self.castles[p.castle_target];
                if c.visible() {
                    let (x, y) = self.centre_for(i, c.x as i64, c.y as i64, c.w, c.h);
                    self.persos[i].dest = self.terrain.near((self.persos[i].x, self.persos[i].y), (x, y));
                } else {
                    self.stop(rules, i);
                    if self.persos[i].projectile {
                        self.kill(rules, i);
                    }
                }
            }
            _ => return,
        }
        let p = &mut self.persos[i];
        let (x, y) = p.dest;
        p.orient(&self.terrain, x, y);
    }

    /// `Deplacement2`: give to or attack the target when it is in range, else walk
    /// (`Deplacement3`).
    pub(crate) fn act_or_move(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let r = p.bonus_range;
        let me = self.perso_box(i);
        let in_range = match p.action {
            action::GO_GIVE => within(&self.terrain, 0.0, me, self.perso_box(p.perso_target)),
            ATTACK_PERSO => within(&self.terrain, r, me, self.perso_box(p.perso_target)),
            ATTACK_HOUSE => {
                let m = &self.houses[p.house_target];
                within(&self.terrain, r, me, (m.x as i64, m.y as i64, m.w, m.h))
            }
            ATTACK_CASTLE => {
                let c = &self.castles[p.castle_target];
                within(&self.terrain, r, me, (c.x as i64, c.y as i64, c.w, c.h))
            }
            ATTACK_GROUND => true,
            _ => false,
        };
        if in_range {
            if p.action == action::GO_GIVE {
                self.give_selection(rules, i);
            } else if p.reload == 0 {
                self.attack_in_range(rules, i);
            }
            let p = &mut self.persos[i];
            if p.bonus_flying {
                p.frame = (p.frame + 1) % 4;
            }
        } else {
            self.move_step(rules, i);
            self.persos[i].advance_frame();
        }
    }

    /// The giving part of `Deplacement2`, with `Ressources_Donner`.
    fn give_selection(&mut self, rules: &Rules, i: usize) {
        let t = self.persos[i].perso_target;
        if let Ok(o) = usize::try_from(self.persos[i].selected_object)
            && !rules.objects[o].inseparable
        {
            self.persos[t].add_new_object(rules, o);
            self.persos[i].selected_object = -1;
        }
        self.persos[i].check_selection();
        let money = self.persos[i].selected_money;
        let q = &mut self.persos[t];
        q.set_money(q.money() + money);
        let p = &mut self.persos[i];
        p.set_money(p.money() - money);
        for r in 0..rules.resources.len() {
            let q = &self.persos[t];
            let n = self.persos[i].selected_resources[r].min(q.max_resource(r) - q.resource(r));
            let q = &mut self.persos[t];
            q.set_resource(r, q.resource(r) + n);
            let p = &mut self.persos[i];
            p.set_resource(r, p.resource(r) - n);
        }
        self.persos[i].clear_selection(rules);
        if self.persos[i].speaks {
            let line = self.comment_line(rules, i, 46);
            self.speak(t, line);
        }
        self.stop(rules, i);
    }

    /// The attacking part of `Deplacement2`, once the reload is over: hit when the attack bar
    /// is full, on the first hit after an order, when the blow can finish the target, or for
    /// automatic weapons; first pay what the weapons cost.
    fn attack_in_range(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let t = &self.persos[p.perso_target];
        let coef = rules.combat.damage_coef;
        let (launcher, area, act, k) = (p.launcher, p.bonus_area, p.action, p.perso_target);
        // A projectile already in flight.
        let flying = p.projectiles > 0 && p.launcher >= 0;
        let costly = p.equipped.iter().take(2).flatten().any(|it| rules.objects[it.object].cost_on_attack());
        let finish = if costly {
            p.attack >= t.bars[0] / coef + t.defense
        } else {
            p.attack > t.bars[0] + t.defense + 1.0
        };
        let full = p.attack == p.bonus_max[4] as f64;
        if !(p.attack >= 1.0 && !flying && full
            || p.force_attack
            || !flying && p.action == ATTACK_PERSO && finish
            || p.auto_attack)
        {
            return;
        }
        let missing = [
            (p.bars[0] > p.costs[0], 17),
            (p.bars[1] > p.costs[1], 18),
            (p.bars[2] >= p.costs[2], 19),
            (p.bars[3] > p.costs[3], 20),
            (p.money() >= p.cost_money, 45),
            (p.res(p.ammo) >= p.ammo_quantity, 34),
        ];
        if let Some(&(_, m)) = missing.iter().find(|c| !c.0) {
            self.say(rules, i, m);
            self.put_away_weapons(rules, i);
            return;
        }
        if launcher >= 0 {
            // The projectile is launched by `Gestion_Utilisation_Speciale` later in the tick.
            self.persos[i].projectile_launch = launcher;
            if act == ATTACK_GROUND {
                self.move_step(rules, i);
            }
        } else if area > 0 {
            let castle = if act == ATTACK_CASTLE { self.persos[i].castle_target as i64 } else { -1 };
            if (ATTACK_PERSO..=ATTACK_CASTLE).contains(&act) {
                self.hit_area(rules, i, area, castle);
            }
        } else if act == ATTACK_PERSO {
            self.hit_perso(rules, i, k, true);
        } else if act == ATTACK_HOUSE {
            let h = self.persos[i].house_target;
            self.hit_building(rules, i, Target::House(h), h, -1);
        } else if act == ATTACK_CASTLE {
            let c = self.persos[i].castle_target;
            self.hit_building(rules, i, Target::Castle(c), k, c as i64);
        }
        // The per-object costs are only paid with a main weapon.
        let p = &self.persos[i];
        if let Some(main) = p.equipped[0] {
            let objects: Vec<usize> = [Some(main), p.equipped[1]].into_iter().flatten().map(|it| it.object).collect();
            let cost = |k: usize| objects.iter().map(|&o| rules.objects[o].cost[k]).sum::<f64>();
            self.set_life(rules, i, p.bars[0] - cost(0));
            let p = &mut self.persos[i];
            for k in 1..4 {
                // Each setter reads its bar after the previous ones (a level-up rescales them).
                p.set_bar(rules, k, p.bars[k] - cost(k));
            }
            // `Decharger` without `Actualiser_Bonus`: the bonuses stay until the next update.
            for s in 0..2 {
                if p.equipped[s].is_some_and(|it| rules.objects[it.object].single_use) {
                    p.equipped[s] = None;
                }
            }
        }
        let p = &mut self.persos[i];
        p.force_attack = false;
        p.set_money(p.money() - p.cost_money);
        if let Ok(r) = usize::try_from(p.ammo) {
            p.set_resource(r, p.resource(r) - p.ammo_quantity);
        }
        p.reload = p.reload_object;
        p.reload_max = p.reload_object;
        if p.kamikaze {
            self.kill(rules, i);
        }
    }

    /// `Objet_Ranger_Armes`: stops and puts both weapons back in the bag (if there is room).
    fn put_away_weapons(&mut self, rules: &Rules, i: usize) {
        self.stop(rules, i);
        let p = &mut self.persos[i];
        for s in 0..2 {
            if p.equipped[s].is_some() {
                p.store(rules, s);
            }
        }
    }

    // ---------------------------------------------------------------- hitting a perso

    /// `Perso_Attaquer_Perso(a, d, Experience)`: one blow. The attack points spent are what
    /// the kill needed (or the whole bar), 20 on a critical failure; a projectile hits with
    /// at most its chef's attack and spends the chef's. With `experience` the spending trains
    /// the attack.
    pub fn hit_perso(&mut self, rules: &Rules, a: usize, d: usize, experience: bool) {
        let c = &rules.combat;
        let (projectile, chef) = (self.persos[a].projectile, self.persos[a].chef);
        // The one who aims: the chef for a projectile.
        let shooter = if projectile { chef as usize } else { a };
        let mut att = self.persos[a].attack;
        if projectile && att > self.persos[shooter].attack {
            att = self.persos[shooter].attack;
        }
        let def = self.persos[d].defense;
        let spent;
        if self.rng.next() > c.failure_chance || (self.persos[a].kamikaze && !projectile) {
            let (attacker, defender) = (&self.persos[a], &self.persos[d]);
            let coup = defender.emperor && attacker.fief == defender.fief;
            self.effect_on(attacker.attack_effect(rules), d);
            let q = &mut self.persos[d];
            q.stun = c.stun;
            q.step = 0;
            spent = q.bars[0] / c.damage_coef + q.defense - 1.0;
            let k = self.persos[a].bonus_attack_resistance;
            let res = self.persos[d].resistance(k);
            let (x, y) = (self.persos[d].x as f64 + self.persos[d].w as f64 / 2.0, self.persos[d].y as f64);
            if att > def {
                let critical = self.rng.next() <= c.critical_chance;
                let mut damage = if critical {
                    let level = self.persos[shooter].special[2].bonus as f64 / rules.start_special as f64;
                    (att * (level + c.critical_coef) - def) * c.damage_coef * res
                } else {
                    (att - def) * c.damage_coef * res
                };
                damage = damage.min(self.persos[d].bars[0]);
                let v = self.persos[d].bars[0] - damage;
                self.set_life(rules, d, v);
                self.number(x, y, damage, NumberKind::Damage, critical);
                if critical {
                    self.say(rules, shooter, 2);
                    if experience {
                        self.persos[shooter].train_special(rules, 2, damage);
                    }
                }
                if let Ok(k) = usize::try_from(k) {
                    self.persos[d].train_resistance(rules, k, damage);
                }
            } else {
                // A blow that does not pierce the defence does 1.
                let v = self.persos[d].bars[0] - 1.0;
                self.set_life(rules, d, v);
                self.number(x, y, 1.0, NumberKind::Damage, false);
            }
            self.persos[d].set_defense(rules, def - att);
            if !self.persos[d].alive() {
                self.credit_kill(rules, a, d, coup);
            } else {
                self.hit_effects(rules, a, d);
                self.hit_reaction(rules, a, d);
            }
        } else {
            // Critical failure.
            self.say(rules, shooter, 3);
            spent = c.failure_loss;
        }
        let p = &mut self.persos[shooter];
        if experience {
            p.set_attack(rules, p.attack - spent);
        } else {
            p.attack -= spent;
        }
    }

    /// The kill part of `Perso_Attaquer_Perso`: whoever killed a summon may turn on its chef,
    /// whose soldiers charge back; the kill, the loot and the bounty of a mercenary house go to
    /// the attacker, or to the chef of a summon.
    fn credit_kill(&mut self, rules: &Rules, a: usize, d: usize, coup: bool) {
        let (p, q) = (&self.persos[a], &self.persos[d]);
        if q.invocation() {
            let chef = q.chef as usize;
            if !p.ai_pacific && p.vitality == 1.0 && self.persos[chef].alive() {
                if p.ai {
                    self.attack_perso(rules, a, chef);
                    self.say(rules, a, 44);
                }
                if self.persos[chef].soldiers > 0 && self.persos[chef].ai {
                    // Ctrl, Shift and Alt (unused), in that order.
                    let ctrl = self.rng.next() > 0.5;
                    let shift = self.rng.next() > 0.5;
                    let _alt = self.rng.next() > 0.8;
                    self.order_charge(rules, chef, 0, a as i64, ctrl, shift, false);
                }
            }
        }
        let (p, q) = (&self.persos[a], &self.persos[d]);
        let race = q.race;
        // `Definir_MaxAttaque + Definir_MaxDefense + Definir_MaxVie` times the bounty rate.
        let bounty = (q.bonus_max[4] + q.bonus_max[5] + q.bonus_max[0]) as f64 * rules.services[2].coef;
        let start = rules.start_service as f64;
        let mercenary = |w: &World, i: usize| w.houses[i].service == 2 && w.houses[i].built;
        if p.invocation() {
            let chef = p.chef as usize;
            if p.kamikaze || p.projectile {
                self.persos[chef].bestiary[race] += 1;
                if mercenary(self, chef) {
                    let score = bounty * (self.persos[chef].services[1].bonus as f64 + start) / start;
                    let _ = self.use_service_score(rules, chef, chef, score, true);
                }
            } else {
                self.persos[a].bestiary[race] += 1;
            }
            // Summons offer their kills to their chef.
            let killer = if self.persos[chef].alive() { chef } else { a };
            self.kill_loot(rules, killer, d, coup);
            self.persos[chef].frags[0] += 1;
        } else {
            self.kill_loot(rules, a, d, coup);
            let p = &mut self.persos[a];
            p.bestiary[race] += 1;
            p.frags[0] += 1;
            if mercenary(self, a) {
                let level = self.persos[a].services[1].bonus as f64;
                let score = bounty * ((level - start + 1.0) / start / 10.0 + 1.0);
                let _ = self.use_service_score(rules, a, a, score, true);
            }
        }
    }

    /// `Perso_Tuer_Perso(a, d, CoupDEtat)`: `a` loots its victim `d` (into a treasure where
    /// `d` lies unless they touch): a share of its money and resources, and each object with
    /// some chance, either taken or destroyed. Killing the own emperor takes the crown; a tomb
    /// marks the place.
    pub fn kill_loot(&mut self, rules: &Rules, a: usize, d: usize, coup: bool) {
        let c = &rules.combat;
        let (p, q) = (&self.persos[a], &self.persos[d]);
        // Someone it was attacking thanks the killer.
        let t = q.perso_target;
        if self.persos[t].team == p.team && t != a && q.action == ATTACK_PERSO {
            self.say_now(rules, t, 14);
        }
        let (p, q) = (&self.persos[a], &self.persos[d]);
        let (qx, qy) = self.terrain.near((p.x, p.y), (q.x, q.y));
        let looted = qx + q.w as i64 >= p.x && qy + q.h as i64 >= p.y && qx <= p.x + p.w as i64 && qy <= p.y + p.h as i64;
        let tr = if looted {
            None
        } else {
            let half = TREASURE_SIZE as f64 / 2.0;
            let x = clng(q.x as f64 + q.w as f64 / 2.0 - half) as i32;
            let y = clng(q.y as f64 + q.h as f64 / 2.0 - half) as i32;
            Some(self.treasure_create(rules, x, y))
        };
        let money = self.persos[d].money() * c.loot_money;
        match tr {
            None => {
                let p = &mut self.persos[a];
                p.set_money(p.money() + money);
            }
            Some(t) => {
                let t = &mut self.treasures[t];
                t.set_money(rules, t.money() + money);
            }
        }
        let q = &mut self.persos[d];
        q.set_money(q.money() * (1.0 - c.loot_money));
        for r in 0..rules.resources.len() {
            let n = self.persos[d].resource(r) * c.loot_resources;
            match tr {
                None => {
                    let p = &mut self.persos[a];
                    p.set_resource(r, p.resource(r) + n);
                }
                Some(t) => {
                    let t = &mut self.treasures[t];
                    t.set_resource(rules, r, t.resource(r) + n);
                }
            }
            let q = &mut self.persos[d];
            q.set_resource(r, q.resource(r) * (1.0 - c.loot_resources));
        }
        // Each object is taken (or destroyed) with some chance; one draw per object even when
        // it is indestructible.
        let n = self.persos[d].equipped.len();
        for s in 0..n + self.persos[d].inventory.len() {
            let item = if s < n { self.persos[d].equipped[s] } else { self.persos[d].inventory[s - n] };
            let Some(item) = item else { continue };
            if self.rng.next() < c.loot_objects && !rules.objects[item.object].indestructible {
                if self.rng.next() > c.destroy_objects {
                    match tr {
                        None => self.persos[a].add_new_object(rules, item.object),
                        Some(t) => self.treasures[t].add_object(rules, item.object),
                    }
                }
                let q = &mut self.persos[d];
                if s < n {
                    q.destroy_equipped(rules, s);
                } else {
                    q.unload_slot(s - n);
                }
            }
        }
        if coup {
            let f = self.persos[a].fief;
            self.change_emperor(rules, f, a as i64, false);
            self.attack_allies(rules, a, false);
        }
        if let Some(t) = tr {
            if self.treasures[t].empty() {
                self.treasures.remove(t);
            } else {
                self.treasure_check(rules, t);
            }
        }
        let q = &self.persos[d];
        if let Ok(tomb) = usize::try_from(rules.races[q.race].tomb) {
            let level = q.level().floor();
            let killed_by = &rules.label_tomb[q.female as usize];
            let name = format!("&{} ({} {level}){killed_by}{}", q.name, rules.label_level, self.persos[a].name);
            let (x, y) = (q.x as i32, q.y as i32);
            self.decor_create(rules, tomb, x, y, name);
        }
    }

    /// `Attaquer_Allies(v)`: hostile to everybody (its team becomes `-Numero`, and it stops),
    /// or back in its fief's team; its summons follow and stop.
    pub fn attack_allies(&mut self, rules: &Rules, i: usize, v: bool) {
        if v {
            self.persos[i].team = -(i as i64);
            self.stop(rules, i);
        } else {
            let f = self.persos[i].fief;
            self.persos[i].team = self.fiefs[f].team;
        }
        if self.persos[i].soldiers > 0 {
            let team = self.persos[i].team;
            for j in 0..self.persos.len() {
                if self.persos[j].chef == i as i64 {
                    self.persos[j].team = team;
                    self.stop(rules, j);
                }
            }
        }
    }

    // ---------------------------------------------------------------- buildings

    pub fn building(&self, b: Target) -> &Building {
        match b {
            Target::House(h) => &self.houses[h],
            Target::Castle(c) => &self.castles[c],
        }
    }

    fn building_mut(&mut self, b: Target) -> &mut Building {
        match b {
            Target::House(h) => &mut self.houses[h],
            Target::Castle(c) => &mut self.castles[c],
        }
    }

    /// `Definir_Vie = v` of a building. Castles are of type 0, which gives and takes no
    /// object, and no market.
    fn set_building_life(&mut self, rules: &Rules, b: Target, v: f64) {
        match b {
            Target::House(h) => self.set_house_life(rules, h, v),
            Target::Castle(c) => {
                self.castles[c].set_life(v);
            }
        }
    }

    /// `Perso_Attaquer_Batiment(a, b, d, NoChateau)`: a blow of up to `MaxDegatBatiment`
    /// attack points (a kamikaze spends what it takes), scaled by the demolition skill; the
    /// attack bar (the chef's for a projectile) pays without limit and trains demolition.
    /// `d` is the owner (the attacker's target perso for a castle); `castle` is the fief of a
    /// castle, -1 for a house; a castle hit as a house (area attacks) is looted as one.
    pub fn hit_building(&mut self, rules: &Rules, a: usize, b: Target, d: usize, castle: i64) {
        let c = &rules.combat;
        let p = &self.persos[a];
        let shooter = if p.projectile { p.chef as usize } else { a };
        let att = self.persos[shooter].attack;
        let life = self.building(b).life;
        let mut spent = life;
        if spent > c.max_building_damage && (!p.kamikaze || p.projectile) {
            spent = c.max_building_damage;
        }
        if spent > att {
            spent = att;
        } else if spent < 1.0 {
            spent = 1.0;
        }
        // The projectile's own copy of its chef's demolition skill.
        let damage = spent * c.building_damage_coef * p.special[1].bonus as f64 / rules.start_special as f64;
        self.set_building_life(rules, b, life - damage);
        let m = self.building(b);
        let (bx, by, bw, bh) = (m.x as f64, m.y as f64, m.w as f64, m.h as f64);
        let x = bx + bw / 4.0 + self.rng.next() * bw / 2.0;
        self.number(x, by, damage, NumberKind::Damage, false);
        // No combat experience, but demolition.
        let s = &mut self.persos[shooter];
        s.attack -= spent;
        s.train_special(rules, 1, spent);
        let p = &self.persos[a];
        // The one credited: the chef of a summon.
        let credited = if p.invocation() { p.chef as usize } else { a };
        // The one blamed: the chef of a kamikaze summon.
        let culprit = if p.kamikaze && p.invocation() { p.chef as usize } else { a };
        if !self.building(b).visible() {
            self.stop(rules, a);
            self.building_mut(b).under_construction = false;
            if let Ok(f) = usize::try_from(castle) {
                self.effect(rules.effect_castle_destroyed, bx, by, bw, bh);
                for j in 0..self.persos.len() {
                    let q = &self.persos[j];
                    let lost = q.alive() && q.fief == f;
                    if self.rng.next() > 0.6 && lost {
                        self.say_now(rules, j, 12);
                    }
                }
                self.persos[credited].frags[2] += 1;
            } else {
                self.loot_building(rules, b);
                self.persos[credited].frags[1] += 1;
                self.say_now(rules, d, 5);
                self.effect(rules.effect_house_destroyed, bx, by, bw, bh);
                // An AI goes for the owner, who was inside.
                let chase = self.persos[a].ai && self.persos[d].in_own_house;
                if self.rng.next() < 0.9 && chase {
                    let owner = self.building(b).owner;
                    self.attack_perso(rules, a, owner);
                }
            }
        } else if let Ok(f) = usize::try_from(castle) {
            let x = bx + self.rng.next() * bw / 2.0 - bw / 4.0;
            let y = by + self.rng.next() * bh / 2.0 - bh / 4.0;
            let effect = self.persos[a].attack_effect(rules);
            self.effect(effect, x, y, bw, bh);
            // The castle's defenders may fight back, and complain.
            for j in 0..self.persos.len() {
                let q = &self.persos[j];
                if q.alive() && q.fief == f && !q.invocation() && !q.ai_selfish {
                    if self.rng.next() > 0.92 && self.persos[j].ai {
                        self.attack_perso(rules, j, culprit);
                    }
                    if self.rng.next() > 0.9 {
                        self.say(rules, j, 11);
                    }
                }
            }
            if self.rng.next() > 0.9 {
                self.call_for_help(rules, a);
            }
        } else {
            let x = bx + self.rng.next() * bw - bw / 2.0;
            let y = by + self.rng.next() * bh - bh / 2.0;
            let effect = self.persos[a].attack_effect(rules);
            self.effect(effect, x, y, bw, bh);
            let q = &self.persos[d];
            if q.alive() && q.ai && !q.invocation() {
                let busy = q.action == ATTACK_PERSO;
                if self.rng.next() > 0.75 && !busy {
                    self.attack_perso(rules, d, a);
                }
            }
            let m = self.building(b);
            let damaged = m.life_with_repair() < m.max_life as f64 / 2.0;
            if self.rng.next() > 0.65 && damaged {
                self.call_for_help(rules, d);
            }
            self.say(rules, d, 4);
        }
    }

    /// `Perso_Piller_Batiment`: a destroyed house leaves a treasure with half its till, half
    /// the stock of a shop, and each stocked object with a chance of one half. Quirk: the last
    /// object slot is skipped.
    fn loot_building(&mut self, rules: &Rules, b: Target) {
        let c = &rules.combat;
        let m = self.building(b);
        // The original uses the width for Y too.
        let half = TREASURE_SIZE as f64 / 2.0;
        let x = clng(m.x as f64 + m.w as f64 / 2.0 - half) as i32;
        let y = clng(m.y as f64 + m.w as f64 / 2.0 - half) as i32;
        let t = self.treasure_create(rules, x, y);
        let m = self.building(b);
        let (money, kind, stock) = (m.money, m.kind, m.stock());
        let product = rules.buildings[kind].product;
        let shop = m.shop && product > 0 && stock > 0.0;
        let stocks: Vec<f64> = if m.factory && stock > 0.0 {
            (0..m.object_slots().saturating_sub(1)).map(|s| m.object_stock(s)).collect()
        } else {
            Vec::new()
        };
        let tr = &mut self.treasures[t];
        tr.set_money(rules, tr.money() + money * c.house_loot_money);
        if shop {
            let r = product as usize;
            tr.set_resource(rules, r, tr.resource(r) + stock * c.house_loot_resources);
        }
        for (s, n) in stocks.into_iter().enumerate() {
            // `For j = 0 To Stock_Objets(i) - 1` with an Integer counter.
            for _ in 0..=clng(n - 1.0) {
                if c.house_loot_objects > self.rng.next() {
                    let o = rules.buildings[kind].objects[s];
                    self.treasures[t].add_object(rules, o);
                }
            }
        }
        if self.treasures[t].empty() {
            self.treasures.remove(t);
        } else {
            self.treasure_check(rules, t);
        }
    }

    // ---------------------------------------------------------------- area attacks

    /// `Perso_Attaquer_Surface(a, X, Y, Rayon, NoChateau)`: every enemy perso, house and castle
    /// within `radius` of the attacker (the given centre is ignored), and its targets, take a
    /// blow without experience, each with the whole attack bar; then the bar is empty (the
    /// minimum it keeps track of starts at 0). Castles are hit with `castle`, so outside an
    /// order on a castle they are looted like houses.
    pub fn hit_area(&mut self, rules: &Rules, a: usize, radius: i64, castle: i64) {
        let radius = radius as f64;
        let init = self.persos[a].attack;
        let mut min = 0.0;
        let dist = |t: &Terrain, p: (i64, i64), q: (f64, f64)| (t.dxf(p.0 as f64, q.0).powi(2) + t.dyf(p.1 as f64, q.1).powi(2)).sqrt();
        let after = |w: &mut World, min: &mut f64| {
            if *min > w.persos[a].attack {
                *min = w.persos[a].attack;
            }
            w.persos[a].attack = init;
        };
        for i in 0..self.persos.len() {
            let (p, q) = (&self.persos[a], &self.persos[i]);
            if q.team != p.team && q.attackable() && (radius > dist(&self.terrain, (p.x, p.y), (q.x as f64, q.y as f64)) || i == p.perso_target) {
                self.hit_perso(rules, a, i, false);
                after(self, &mut min);
            }
        }
        for i in 0..self.houses.len() {
            let (p, m) = (&self.persos[a], &self.houses[i]);
            // The original uses the width for Y too.
            let centre = (m.x as f64 + m.w as f64 / 2.0, m.y as f64 + m.w as f64 / 2.0);
            if self.persos[i].team != p.team && m.visible() && (radius > dist(&self.terrain, (p.x, p.y), centre) || i == p.house_target) {
                self.hit_building(rules, a, Target::House(i), i, -1);
                after(self, &mut min);
            }
        }
        for i in 0..self.castles.len() {
            let (p, c) = (&self.persos[a], &self.castles[i]);
            if self.fiefs[i].team != p.team && c.visible() && (radius > dist(&self.terrain, (p.x, p.y), (c.x as f64, c.y as f64)) || i == p.castle_target) {
                let d = p.perso_target;
                self.hit_building(rules, a, Target::Castle(i), d, castle);
                after(self, &mut min);
            }
        }
        self.persos[a].set_attack(rules, min);
    }

    /// `Perso_Pietinement`: a walking perso crushes the enemies under it that trample less,
    /// and the enemy buildings, its attack bar re-armed to its trampling strength each time.
    pub fn trample(&mut self, rules: &Rules, a: usize) {
        let p = &self.persos[a];
        let (me, team) = ((p.x as i32, p.y as i32, p.w, p.h), p.team);
        let rearm = |w: &mut World| {
            let p = &mut w.persos[a];
            if p.attack < p.trample as f64 {
                p.set_attack(rules, p.trample as f64);
            }
        };
        for i in 0..self.persos.len() {
            rearm(self);
            let (p, q) = (&self.persos[a], &self.persos[i]);
            if q.team != team && q.attackable() && self.terrain.overlaps(me, (q.x as i32, q.y as i32, q.w, q.h)) && p.trample > q.trample {
                self.hit_perso(rules, a, i, false);
            }
        }
        for i in 0..self.houses.len() {
            rearm(self);
            let m = &self.houses[i];
            if self.persos[i].team != team && m.visible() && self.terrain.overlaps(me, (m.x, m.y, m.w, m.h)) {
                self.hit_building(rules, a, Target::House(i), i, -1);
            }
        }
        for i in 0..self.castles.len() {
            let c = &self.castles[i];
            if self.terrain.overlaps(me, (c.x, c.y, c.w, c.h)) && c.visible() && self.fiefs[i].team != team {
                let d = self.persos[a].perso_target;
                self.hit_building(rules, a, Target::Castle(i), d, i as i64);
            }
        }
    }

    /// The temporary effects a blow causes (`BonusAttaqueCauseEffet`), each with the chance of
    /// the defender's resistance to it, cast by the attacker (the chef for a projectile).
    fn hit_effects(&mut self, rules: &Rules, a: usize, d: usize) {
        let effects = self.persos[a].bonus_attack_cause_effect.clone();
        let p = &self.persos[a];
        let owner = if p.projectile { p.chef as usize } else { a };
        for (n, &e) in effects.iter().enumerate() {
            let Ok(e) = usize::try_from(e) else { continue };
            let o = &rules.objects[e];
            let res = self.persos[d].resistance(o.attack_resistance);
            if res > rules.resistance_min && self.rng.next() < res {
                let duration = if o.duration == -1 {
                    -1
                } else {
                    let level = self.persos[a].object_skill_level(rules, e);
                    clng(crate::items::gain_sup(rules, level, o.duration as f64))
                };
                let skill = self.persos[owner].object_skill_level(rules, e);
                let q = &mut self.persos[d];
                q.add_effect(rules, e, duration, duration, Some((owner, skill)));
                let (x, y) = (q.x as f64 + q.w as f64 / 2.0, q.y as f64 - 9.0 * n as f64);
                let text = o.effect_name[q.female as usize].clone();
                self.text(x, y, text, crate::event::NumberKind::Damage);
            }
        }
    }

    /// How a defender that survived a blow reacts: an AI turns on the attacker (unless its
    /// current target is closer), may call for help and flee; others fight back when idle or
    /// harvesting.
    fn hit_reaction(&mut self, rules: &Rules, a: usize, d: usize) {
        let p = &self.persos[a];
        let q = &self.persos[d];
        // Kamikaze summons and projectiles make their chef the culprit.
        let culprit = if (p.kamikaze && p.invocation()) || p.projectile { p.chef as usize } else { a };
        if q.ai {
            if q.action == ATTACK_PERSO {
                let (c, t) = (&self.persos[culprit], &self.persos[q.perso_target]);
                let g = &self.terrain;
                if g.dx(q.x, c.x).abs() + g.dy(q.y, c.y).abs() < g.dx(q.x, t.x).abs() + g.dy(q.y, t.y).abs() {
                    self.attack_perso(rules, d, culprit);
                }
            } else {
                self.attack_perso(rules, d, culprit);
            }
            let (p, q) = (&self.persos[a], &self.persos[d]);
            let weak = q.bars[0] < q.bonus_max[0] as f64 / 2.0 && q.bars[0] < p.bars[0] && !q.invocation();
            if self.rng.next() > 0.2 && weak {
                self.call_for_help(rules, d);
                let (p, q) = (&self.persos[a], &self.persos[d]);
                let flee = q.step >= p.step && q.bars[0] < p.bars[0] && !q.ai_bold;
                if self.rng.next() > 0.9 && flee {
                    if self.houses[d].built {
                        self.go_home(rules, d, d, false);
                    } else if self.castles[q.fief].visible() {
                        self.go_own_castle(rules, d);
                    } else {
                        self.random_walk(rules, d);
                    }
                    self.say(rules, a, 13);
                }
            }
        } else if q.idle() || q.action == action::HARVEST {
            let target = if p.kamikaze && p.invocation() { p.chef as usize } else { a };
            self.attack_perso(rules, d, target);
        }
    }
}
