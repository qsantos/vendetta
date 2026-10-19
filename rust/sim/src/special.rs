//! What used objects do, and summons (`ModCapacitesSpe`): `Gestion_Utilisation_Speciale` runs
//! once per tick, after the moves, for the objects used (`NumeroObjetUtilise`) and the
//! projectiles launched (`NumeroProjectileLance`) during the tick.

use crate::building::Building;
use crate::combat::{ATTACK_CASTLE, ATTACK_HOUSE, ATTACK_PERSO, Target};
use crate::items::invocation_sup;
use crate::perso::{Perso, action};
use crate::rules::Rules;
use crate::vb::clng;
use crate::world::World;

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

    /// The effect of object `n` used by perso `i`, scaled by its item skill.
    fn use_special(&mut self, rules: &Rules, i: usize, n: usize) {
        let o = &rules.objects[n];
        let c = self.persos[i].object_skill_level(rules, n);
        if let Ok(race) = usize::try_from(o.invocation) {
            let quantity = invocation_sup(rules, c, o.quantity);
            let max = invocation_sup(rules, c, o.quantity_max).floor() as i64;
            self.create_invocation(rules, race, quantity, i, o.attack_effect, false, max, &o.invocation_ai);
        }
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
                    self.persos[j] = p;
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
                p.ground_target_kind = kind;
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
                self.persos[k].go_to(x, y);
            } else {
                // A star of 8 branches around the chef.
                let m = i / 8 + 1;
                let (w, h) = (cw * m, ch * m);
                let offset = [(w, 0), (-w, 0), (0, h), (0, -h), (-w, -h), (w, -h), (-w, h), (w, h)][(i % 8) as usize];
                let p = &mut self.persos[k];
                p.chef_offset = offset;
                p.x = cx + offset.0;
                p.y = cy + offset.1;
                match caction {
                    ATTACK_PERSO => self.attack_perso(rules, k, targets.0),
                    ATTACK_HOUSE => self.attack_house(rules, k, targets.1),
                    ATTACK_CASTLE => self.attack_castle(rules, k, targets.2),
                    _ => self.persos[k].go_to(cdest.0 + offset.0, cdest.1 + offset.1),
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
