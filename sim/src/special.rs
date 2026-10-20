//! What used objects do, and summons (`ModCapacitesSpe`): `Gestion_Utilisation_Speciale` runs
//! once per tick, after the moves, for the objects used (`NumeroObjetUtilise`) and the
//! projectiles launched (`NumeroProjectileLance`) during the tick.

use crate::building::Building;
use crate::combat::{ATTACK_CASTLE, ATTACK_HOUSE, ATTACK_PERSO, Target};
use crate::event::NumberKind;
use crate::items::{bonus_sup, gain_sup, invocation_sup, range_sup};
use crate::perso::{Perso, action};
use crate::rules::Rules;
use crate::vb::clng;
use crate::world::World;

/// The parameters of `Effets_Proches`.
struct Near {
    /// `CreerObjetAllie`, `CreerObjetEnnemi` of the used object.
    ally: i64,
    enemy: i64,
    zone: i64,
    /// Added to the Vie, Energie, Magie and Moral recovery pools.
    gains: [f64; 4],
    resurrection: i64,
    quantity: i64,
    effect: i64,
    /// The caster's item skill level.
    level: i64,
    /// The resistance that opposes the effect.
    resistance: i64,
}

impl World {
    /// `Gestion_Utilisation_Speciale`: for every perso (not those it creates), the projectiles
    /// it shot this tick fly off and the object it used takes effect.
    pub(crate) fn special_uses(&mut self, rules: &Rules) {
        for i in 0..self.persos.len() {
            let p = &self.persos[i];
            // Quirk: `Not ProjectileEnCours` is a bitwise Not, so only -1 (after a double
            // death) keeps it from shooting.
            if let Ok(race) = usize::try_from(p.projectile_launch)
                && p.projectiles != -1
            {
                let (count, effect) = (p.projectile_count as f32 as f64, p.attack_effect(rules));
                self.create_invocation(rules, race, count, i, effect, true, 0, "");
                self.persos[i].projectile_launch = -1;
            }
            if let Ok(n) = usize::try_from(self.persos[i].used_object) {
                self.use_special(rules, i, n);
                self.persos[i].used_object = -1;
            }
        }
    }

    /// The effect of object `n` used by perso `i`, scaled by its item skill: summons, then
    /// healing and effects on the nearest persos within its range of use (or on itself), a
    /// tax, and the object's effect on itself with the chance of its resistance.
    fn use_special(&mut self, rules: &Rules, i: usize, n: usize) {
        let o = &rules.objects[n];
        let c = self.persos[i].object_skill_level(rules, n);
        if let Ok(race) = usize::try_from(o.invocation) {
            let quantity = invocation_sup(rules, c, o.quantity);
            let max = invocation_sup(rules, c, o.quantity_max).floor() as i64;
            self.create_invocation(rules, race, quantity, i, o.attack_effect, false, max, &o.invocation_ai);
        }
        let p = &self.persos[i];
        let gains = [0, 1, 2, 3].map(|b| bonus_sup(rules, c, o.gain[b]) as f64 * p.eat_coef(b));
        if o.use_range > 0 {
            let near = Near {
                ally: o.create_object[1],
                enemy: o.create_object[2],
                zone: range_sup(rules, c, o.use_range as f64),
                gains,
                resurrection: o.resurrection,
                quantity: bonus_sup(rules, c, o.quantity),
                effect: o.attack_effect,
                level: c,
                resistance: o.attack_resistance,
            };
            self.effects_near(rules, i, &near);
        } else {
            if o.self_resurrection {
                self.resurrect(rules, i, o.attack_effect);
            }
            let p = &mut self.persos[i];
            for (b, g) in gains.into_iter().enumerate() {
                p.recovery[b] += g;
            }
            if gains[0] > 0.0 {
                let (x, y) = (p.x as f64 + p.w as f64 / 2.0, p.y as f64);
                self.number(x, y, gains[0], NumberKind::Heal, false);
            }
            if o.invocation < 0 {
                self.effect_on(o.attack_effect, i);
            }
        }
        if o.tax {
            let range = range_sup(rules, c, o.use_range as f64);
            self.tax(i, o.quantity, o.attack_effect, range);
        }
        let res = self.persos[i].resistance(o.attack_resistance);
        if res > rules.resistance_min && self.rng.next() < res {
            self.effect_on_perso(rules, i, n as i64, c, 0, i);
        }
    }

    /// `Effets_Proches`: up to `quantity` times, the nearest perso (other than `l`, each once)
    /// that is an ally for `ally`, an enemy for `enemy`, or a wounded ally, attackable and
    /// failing its resistance roll (one `Rnd` per closer candidate) is affected if within
    /// `zone`: healed and given the ally or enemy effect. Resurrections cannot happen since the
    /// dead are not attackable.
    fn effects_near(&mut self, rules: &Rules, l: usize, near: &Near) {
        let diagonal = ((self.terrain.width as f64 * 16.0).powi(2) + (self.terrain.height as f64 * 16.0).powi(2)).sqrt();
        let mut affected: Vec<usize> = Vec::new();
        for _ in 0..near.quantity.max(1) {
            let (mut best, mut distance, mut friend) = (None, diagonal, false);
            for i in 0..self.persos.len() {
                let (p, q) = (&self.persos[l], &self.persos[i]);
                let same = p.team == q.team;
                let wounded = q.bar_total(0) < q.bonus_max[0] as f64 && q.alive() && near.resurrection == 0;
                let ghost = !q.alive() && near.resurrection > 0 && !q.invocation();
                let wanted = near.ally >= 0 && q.alive() && same
                    || near.enemy >= 0 && q.alive() && !same
                    || same && (wounded || ghost);
                if i == l || !wanted || !q.attackable() {
                    continue;
                }
                let d = self.terrain.dist((p.x, p.y), (q.x, q.y));
                let res = q.resistance(near.resistance);
                if distance > d && res > rules.resistance_min && self.rng.next() < res && !affected.contains(&i) {
                    (best, distance, friend) = (Some(i), d, same);
                }
            }
            let Some(t) = best.filter(|_| distance < near.zone as f64) else { break };
            if !self.persos[t].alive() && near.resurrection > 0 {
                self.resurrect(rules, t, near.effect);
            }
            let g = near.gains;
            if g.iter().sum::<f64>() > 0.0 {
                let q = &mut self.persos[t];
                q.recovery[0] += g[0];
                let (x, y) = (q.x as f64 + q.w as f64 / 2.0, q.y as f64);
                if g[0] > 0.0 {
                    self.number(x, y, g[0], NumberKind::Heal, false);
                }
                let q = &mut self.persos[t];
                for b in 1..4 {
                    q.recovery[b] += g[b];
                }
            }
            if friend {
                self.effect_on_perso(rules, t, near.ally, near.level, 1, l);
            } else {
                self.effect_on_perso(rules, t, near.enemy, near.level, 2, l);
            }
            affected.push(t);
            self.effect_on(near.effect, t);
        }
    }

    /// `Effet_Sur_Perso(t, n, C, k, caster)`: the object `CreerObjet`/`Allie`/`Ennemi` (`k` 0,
    /// 1, 2) of object `n` affects perso `t`: as a temporary effect cast by `caster` when `n`
    /// has a duration, else given (`Quantite` times). Quirk: the area effects pass the created
    /// object itself as `n`, so its own `CreerObjetAllie`/`Ennemi` is used.
    fn effect_on_perso(&mut self, rules: &Rules, t: usize, n: i64, level: i64, k: usize, caster: usize) {
        let Ok(n) = usize::try_from(n) else { return };
        let o = &rules.objects[n];
        let Ok(created) = usize::try_from(o.create_object[k]) else { return };
        if o.duration != 0 {
            let duration = if o.duration == -1 { -1 } else { clng(gain_sup(rules, level, o.duration as f64)) };
            let owner = (caster != t).then(|| (caster, self.persos[caster].object_skill_level(rules, created)));
            self.persos[t].add_effect(rules, created, duration, -1, owner);
        } else {
            // `For k = 1 To Quantite` with a Long counter.
            let count = if o.quantity > 1.0 { clng(o.quantity) } else { 1 };
            for _ in 0..count {
                self.persos[t].add_new_object(rules, created);
            }
        }
    }

    /// `Prelever_Impots`: every living citizen of the caster's fief within `range` pays the
    /// share `rate` of its money, and so does every house of theirs in range with at least
    /// `1 / rate`; the caster gets the total (Longs).
    fn tax(&mut self, l: usize, rate: f64, effect: i64, range: i64) {
        let (f, lx, ly) = (self.persos[l].fief, self.persos[l].x as f64, self.persos[l].y as f64);
        let range = range as f64;
        let mut total = 0;
        for i in 0..self.persos.len() {
            if self.persos[i].fief != f || i == l {
                continue;
            }
            let within = |t: &crate::terrain::Terrain, x: f64, y: f64| range >= (t.dxf(lx, x).powi(2) + t.dyf(ly, y).powi(2)).sqrt();
            let q = &mut self.persos[i];
            if q.alive() && within(&self.terrain, q.x as f64, q.y as f64) {
                let tax = clng(q.money() * rate);
                q.set_money(q.money() - tax as f64);
                total += tax;
                let (x, y) = (q.x as f64 + q.w as f64 / 2.0, q.y as f64);
                self.number(x, y, -tax as f64, NumberKind::Money, false);
            }
            let m = &mut self.houses[i];
            let min = (1.0 / rate as f32) as f64;
            if m.visible() && m.money >= min && within(&self.terrain, m.x as f64, m.y as f64) {
                let tax = clng(m.money * rate);
                m.money -= tax as f64;
                total += tax;
                let (x, y) = (m.x as f64 + m.w as f64 / 2.0, m.y as f64);
                self.number(x, y, -tax as f64, NumberKind::Money, false);
            }
        }
        let p = &mut self.persos[l];
        p.set_money(p.money() + total as f64);
        let (x, y) = (p.x as f64 + p.w as f64 / 2.0, p.y as f64);
        self.effect_on(effect, l);
        self.number(x, y, total as f64, NumberKind::Money, true);
    }

    /// `Collision_Projectile`: a projectile flying over the first obstacle of its kind (an
    /// enemy perso, house or castle, per `ProjectileCible`) hits it; it goes through persos
    /// unless it is a kamikaze or its chef's attack is spent, and stops at buildings.
    pub(crate) fn projectile_collision(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let (code, k) = self.obstacle_at(rules, i, p.x as i32, p.y as i32, p.w, p.h);
        let (kind, area, chef) = (p.ground_target_kind, p.bonus_area, p.chef as usize);
        match (code, kind) {
            (9, 0) => {
                if area > 0 {
                    self.hit_area(rules, i, area, -1);
                } else {
                    self.hit_perso(rules, i, k, true);
                }
                if self.persos[i].kamikaze || self.persos[chef].attack == 0.0 {
                    self.kill(rules, i);
                }
            }
            (10, 1) => {
                if area > 0 {
                    self.hit_area(rules, i, area, -1);
                } else {
                    self.hit_building(rules, i, Target::House(k), k, -1);
                }
                self.kill(rules, i);
            }
            (11, 2) => {
                if area > 0 {
                    self.hit_area(rules, i, area, k as i64);
                } else {
                    // The original passes the perso of the castle's index as its owner.
                    self.hit_building(rules, i, Target::Castle(k), k, k as i64);
                }
                self.kill(rules, i);
            }
            _ => {}
        }
    }

    /// `Perso_Ajouter_Nouveau`: a new perso slot (and its house) at the end.
    fn add_perso(&mut self, rules: &Rules) -> usize {
        let i = self.persos.len();
        self.houses.push(Building { owner: i, ..Building::new(rules) });
        let mut p = Perso::new(rules, &mut self.rng, 0, 0, false);
        p.numero = i;
        p.perso_target = i;
        self.persos.push(p);
        self.ai_resize(self.persos.len());
        i
    }

    /// `Creer_Invocation`: `quantity` summons of race `race` (the fraction is a chance of one
    /// more) for perso `chef`, in a dead summon's slot whose house is gone, or a new one. Beyond
    /// `max` (when above 1), the oldest of that race dies. Soldiers get the AI file `ai_file`
    /// (or none), stand in a star around their chef, copy its order and line up in its
    /// formation; projectiles start from the chef's centre and fly to twice its range toward
    /// its target. Returns the last one.
    #[allow(clippy::too_many_arguments)]
    pub fn create_invocation(
        &mut self,
        rules: &Rules,
        race: usize,
        quantity: f64,
        chef: usize,
        effect: i64,
        projectile: bool,
        max: i64,
        ai_file: &str,
    ) -> usize {
        // `Quantite_Invocations_Aleatoire`, in Singles.
        let whole = quantity.floor();
        let fraction = (quantity as f32 - whole as f32) as f64;
        let n = whole as i64 + i64::from(self.rng.next() < fraction);
        let mut k = 0;
        // The aim of the first projectile, kept for the others: X is a Variant (not rounded),
        // Y and the distance are Longs.
        let (mut aim_x, mut aim_y, mut l) = (0.0, 0i64, 1i64);
        for i in 0..n {
            if max > 1 {
                let mut count = 0;
                for j in 0..self.persos.len() {
                    if self.persos[j].chef == chef as i64 && self.persos[j].race == race {
                        count += 1;
                        if count >= max {
                            // Dead or alive.
                            self.kill(rules, j);
                            break;
                        }
                    }
                }
            }
            let reuse = (0..self.persos.len())
                .find(|&j| self.persos[j].invocation() && !self.persos[j].alive() && !self.houses[j].visible());
            k = match reuse {
                Some(j) => {
                    // `Init`
                    let (fief, team) = (self.persos[j].fief, self.persos[j].team);
                    let mut p = Perso::new(rules, &mut self.rng, fief, team, false);
                    p.numero = j;
                    p.perso_target = j;
                    let (x, y) = (p.x, p.y);
                    self.persos[j] = p;
                    self.spatial.persos.moved_to(j, x, y, 0);
                    j
                }
                None => self.add_perso(rules),
            };
            let (fief, team) = (self.persos[chef].fief, self.persos[chef].team);
            let p = &mut self.persos[k];
            p.fief = fief;
            p.team = team;
            p.projectile = projectile;
            p.change_race(rules, &mut self.rng, race, false);
            self.give_talents(rules, k);
            let p = &mut self.persos[k];
            p.ai = !projectile;
            p.choose_ai = !ai_file.is_empty();
            if p.choose_ai {
                p.ai_file = ai_file.to_owned();
                p.ai_index = crate::ia_file::find(&rules.ais, ai_file);
                p.choose_ai = p.ai_index >= 0;
                self.ai_init_perso(k);
            }
            let p = &mut self.persos[k];
            p.ai_order = 0;
            p.chef = chef as i64;
            if projectile {
                // It hits with its chef's characteristics.
                let c = &self.persos[chef];
                let skills: Vec<(usize, i64)> = (c.bonus_attack_cause_effect.iter())
                    .filter_map(|&e| usize::try_from(e).ok())
                    .map(|e| (rules.objects[e].skill, c.objects[rules.objects[e].skill].bonus))
                    .collect();
                let (max_attack, demolition) = (c.bonus_max[4], c.special[1].bonus);
                let p = &mut self.persos[k];
                p.max[4] = max_attack;
                p.special[1].carac = demolition;
                for (s, level) in skills {
                    p.objects[s].carac = level;
                }
            }
            self.persos[k].update_bonus(rules);
            self.revive(rules, k);
            let p = &mut self.persos[k];
            for b in 1..4 {
                p.bars[b] = p.bonus_max[b] as f64;
            }
            p.pick_look(rules, &mut self.rng);
            let c = &self.persos[chef];
            let (cx, cy, cw, ch, cdest, caction) = (c.x, c.y, c.w as i64, c.h as i64, c.dest, c.action);
            let targets = (c.perso_target, c.house_target, c.castle_target);
            if projectile {
                let range = c.bonus_range;
                let kind = match caction {
                    ATTACK_PERSO => 0,
                    ATTACK_HOUSE => 1,
                    ATTACK_CASTLE => 2,
                    _ => c.ground_target_kind,
                };
                let ground = c.ground_target;
                self.persos[chef].projectiles += 1;
                let p = &mut self.persos[k];
                p.x = clng(cx as f64 + cw as f64 / 2.0 - p.w as f64 / 2.0);
                p.y = clng(cy as f64 + ch as f64 / 2.0 - p.h as f64 / 2.0);
                (p.x, p.y) = self.terrain.wrap((p.x, p.y));
                p.ground_target_kind = kind;
                self.spatial.persos.moved_to(k, p.x, p.y, 0);
                let (px, py) = (p.x, p.y);
                if i == 0 {
                    let (x, y, w, h) = match caction {
                        ATTACK_PERSO => {
                            let t = &self.persos[targets.0];
                            (t.x, t.y, t.w as i64, t.h as i64)
                        }
                        ATTACK_HOUSE => {
                            let m = &self.houses[targets.1];
                            (m.x as i64, m.y as i64, m.w as i64, m.h as i64)
                        }
                        ATTACK_CASTLE => {
                            let m = &self.castles[targets.2];
                            (m.x as i64, m.y as i64, m.w as i64, m.h as i64)
                        }
                        _ => {
                            // Shooting at the ground: the chef keeps walking.
                            self.persos[chef].action = action::WALK;
                            (ground.0, ground.1, 0, 0)
                        }
                    };
                    // (the target's copy nearest to the projectile on a torus)
                    let (x, y) = self.terrain.near((px, py), (x, y));
                    aim_x = x as f64 + w as f64 / 2.0;
                    aim_y = clng(y as f64 + h as f64 / 2.0);
                    l = clng(((aim_x - px as f64).powi(2) + ((aim_y - py) as f64).powi(2)).sqrt());
                    if l == 0 {
                        l = 1;
                    }
                }
                // The extra projectiles spread out.
                let spread = if i % 2 == 0 { i * 32 } else { -i * 32 };
                let to = |a: f64, p: i64| clng(range * 2.0 / l as f64 * (a - p as f64) + p as f64 + spread as f64);
                let (x, y) = (to(aim_x, px), to(aim_y as f64, py));
                self.walk_to(rules, k, x, y);
                // As far as it was sent, even past half the map on a torus.
                self.persos[k].dest = (x, y);
            } else {
                // A star of 8 branches around the chef.
                let m = i / 8 + 1;
                let (w, h) = (cw * m, ch * m);
                let offset = [(w, 0), (-w, 0), (0, h), (0, -h), (-w, -h), (w, -h), (-w, h), (w, h)][(i % 8) as usize];
                let p = &mut self.persos[k];
                p.chef_offset = offset;
                (p.x, p.y) = self.terrain.wrap((cx + offset.0, cy + offset.1));
                self.spatial.persos.moved_to(k, p.x, p.y, 0);
                match caction {
                    ATTACK_PERSO => self.attack_perso(rules, k, targets.0),
                    ATTACK_HOUSE => self.attack_house(rules, k, targets.1),
                    ATTACK_CASTLE => self.attack_castle(rules, k, targets.2),
                    _ => self.walk_to(rules, k, cdest.0 + offset.0, cdest.1 + offset.1),
                }
                self.effect_on(effect, k);
                self.say(rules, k, 33);
            }
        }
        if !projectile {
            self.order_formation(rules, chef, false, true);
        }
        k
    }
}
