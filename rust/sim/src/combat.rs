//! Combat (`ModCombat`, the combat part of `ClsJeuPerso`): the attack orders, following and
//! hitting a target (`Deplacement1/2`), damage (`Perso_Attaquer_Perso`), kills and looting
//! (`Perso_Tuer_Perso`), buildings (`Perso_Attaquer_Batiment`), area attacks and trampling.

use crate::event::NumberKind;
use crate::perso::action;
use crate::rules::Rules;
use crate::vb::clng;
use crate::world::World;

/// `Attaquer_Perso` etc.: `Action` codes 300-303.
pub const ATTACK_PERSO: i32 = 300;
pub const ATTACK_HOUSE: i32 = 301;
pub const ATTACK_CASTLE: i32 = 302;
pub const ATTACK_GROUND: i32 = 303;

/// Whether the box (x, y, w, h) is within `r` of the box of `(px, py, pw, ph)`.
fn within(r: f64, (px, py, pw, ph): (i64, i64, i32, i32), (x, y, w, h): (i64, i64, i32, i32)) -> bool {
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
        let in_range = within(r, self.perso_box(i), self.perso_box(k));
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
                    self.persos[i].dest = (x, y);
                } else {
                    self.stop(rules, i);
                }
            }
            ATTACK_PERSO => {
                let k = p.perso_target;
                let t = &self.persos[k];
                if t.visible() && t.alive() {
                    let (x, y) = self.centre_for(i, t.x, t.y, t.w, t.h);
                    self.persos[i].dest = (x, y);
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
                    self.persos[i].dest = (x, y);
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
                    self.persos[i].dest = (x, y);
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
        p.orient(x, y);
    }

    /// `Deplacement2`: give to or attack the target when it is in range, else walk
    /// (`Deplacement3`).
    pub(crate) fn act_or_move(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let r = p.bonus_range;
        let me = self.perso_box(i);
        let in_range = match p.action {
            action::GO_GIVE => within(0.0, me, self.perso_box(p.perso_target)),
            ATTACK_PERSO => within(r, me, self.perso_box(p.perso_target)),
            ATTACK_HOUSE => {
                let m = &self.houses[p.house_target];
                within(r, me, (m.x as i64, m.y as i64, m.w, m.h))
            }
            ATTACK_CASTLE => {
                let c = &self.castles[p.castle_target];
                within(r, me, (c.x as i64, c.y as i64, c.w, c.h))
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
            let tick = self.tick;
            self.persos[t].set_comment(line, tick);
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
            // TODO: area attacks (`Perso_Attaquer_Surface`)
        } else if act == ATTACK_PERSO {
            self.hit_perso(rules, i, k, true);
        } else {
            // TODO: attacking buildings (`Perso_Attaquer_Batiment`)
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
                let _ = coup;
                // TODO: the kill's credit and loot (`Perso_Tuer_Perso`)
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
                self.text(x, y, text);
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
                if (c.x - q.x).abs() + (c.y - q.y).abs() < (t.x - q.x).abs() + (t.y - q.y).abs() {
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
                        self.go_home(d, d, false);
                    } else if self.castles[q.fief].visible() {
                        self.go_own_castle(d);
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
