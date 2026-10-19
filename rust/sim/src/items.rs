//! Objects on characters: equipment, use, temporary effects, recharges and their bonuses
//! (`ClsJeuPerso` `Objet_*`, `Ajouter_Bonus_Objet`, `EffetTemp_*`, `EffetRecharge_*`).

use serde::{Deserialize, Serialize};
use crate::perso::{Item, Perso};
use crate::rules::{ObjectType, Rules};
use crate::stats::Trained;
use crate::vb::clng;
use crate::world::World;

/// `ClsJeuEffetTemp` in `EffetsTemp`: an object applied as a temporary effect.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempEffect {
    pub object: usize,
    /// `Duree` in slow ticks (-1 never expires) and `DureeMax`.
    pub duration: i64,
    pub duration_max: i64,
    /// `Proprietaire`: the character whose item skill scales the effect.
    pub owner: usize,
    /// The owner's item skill level when the effect was cast; the character's own level is
    /// read live instead when it is the owner.
    pub owner_skill: i64,
}

/// `ClsJeuEffetTemp` in `EffetsRecharge`: an object that is recharging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recharge {
    pub object: usize,
    pub duration: i64,
}

/// What `Objet_Equiper` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Equip {
    /// Nothing happened: a mount the race cannot ride, an empty slot, or a usable object that
    /// is recharging or cannot be used now.
    Refused,
    Equipped,
    /// A usable object was paid for; `used_object` waits for `Gestion_Utilisation_Speciale`.
    Used,
    /// A usable object could not be paid: the comment type to say (17 Vie, 18 Energie, 19 Magie,
    /// 20 Moral, 45 money, 47 resource).
    Missing(i64),
}

/// The duration setter's cap (`Definir_EffetTemp_Duree`).
pub const MAX_EFFECT_DURATION: i64 = 1_000_000;
/// `Jeu.ini Regle1` (`Regle_Invocations_Multiples`), in %.
const INVOCATION_RULE: f64 = 50.0;

/// The skill scaling of `Objet_Equipe_*Sup`: `(v*c/start - v)*k + v`.
fn scaled(rules: &Rules, c: i64, v: f64, k: f64) -> f64 {
    let v = v as f32 as f64;
    (v * c as f64 / rules.start_object as f64 - v) * k + v
}

/// `Objet_Equipe_BonusSup`: an object bonus `v` scaled by the item skill level `c`.
pub fn bonus_sup(rules: &Rules, c: i64, v: f64) -> i64 {
    scaled(rules, c, v, 1.0).floor() as i64
}

/// `Objet_Equipe_GainSup` (a Single, not rounded).
pub fn gain_sup(rules: &Rules, c: i64, v: f64) -> f64 {
    scaled(rules, c, v, 1.0) as f32 as f64
}

/// `Objet_Equipe_PorteeSup` and `Objet_Equipe_SurfaceSup`: a quarter of the skill effect.
pub fn range_sup(rules: &Rules, c: i64, v: f64) -> i64 {
    scaled(rules, c, v, 0.25).floor() as i64
}

/// `Objet_Equipe_ArmureSup`: the skill has no effect.
pub fn armour_sup(rules: &Rules, c: i64, v: f64) -> i64 {
    scaled(rules, c, v, 0.0).floor() as i64
}

/// `Objet_Equipe_InvocationSup`: at least 1.
pub fn invocation_sup(rules: &Rules, c: i64, v: f64) -> f64 {
    let t = scaled(rules, c, v, INVOCATION_RULE / 100.0) as f32 as f64;
    t.max(1.0)
}

fn set_duration(e: &mut TempEffect, v: i64) {
    let v = v.min(MAX_EFFECT_DURATION);
    e.duration = v;
    if v > e.duration_max {
        e.duration_max = v;
    }
}

impl Perso {
    fn active(&self, s: usize) -> bool {
        self.equipped.get(s).is_some_and(Option::is_some)
    }

    fn worn(&self, s: usize) -> Option<usize> {
        self.equipped.get(s).copied().flatten().map(|it| it.object)
    }

    /// `ClsJeuObjet.Categorie` of an equipment slot, 0 when empty.
    fn worn_category(&self, rules: &Rules, s: usize) -> i64 {
        self.worn(s).map_or(0, |o| rules.objects[o].category)
    }

    /// `Objet_Equipes_2_Mains`: slot 0 holds a two-handed weapon.
    pub fn two_handed(&self, rules: &Rules) -> bool {
        self.worn_category(rules, 0) == 1
    }

    /// `Definir_Niveau_CompetenceObjets(Objet_Competence(object))`: the level scaling the object.
    pub fn object_skill_level(&self, rules: &Rules, object: usize) -> i64 {
        self.objects[rules.objects[object].skill].bonus
    }

    /// An object's value `v` (e.g. `bonus[k]`) as this character would get it, for tooltips.
    pub fn scaled_bonus(&self, rules: &Rules, object: usize, v: f64) -> i64 {
        bonus_sup(rules, self.object_skill_level(rules, object), v)
    }

    /// `Equipement_Libre`: there is room to equip `object` without displacing anything.
    pub fn slot_free(&self, rules: &Rules, object: usize) -> bool {
        match rules.objects[object].category {
            0 => !(self.active(0) && self.active(1)) && !(self.active(0) && self.two_handed(rules)),
            1 => !(self.active(0) || self.active(1)),
            6 | 7 => !(self.active(6) && self.active(7)),
            c => usize::try_from(c).is_ok_and(|c| c < self.equipped.len() && !self.active(c)),
        }
    }

    /// `Objet_Equipes_Detruire`: the equipped object disappears.
    pub fn destroy_equipped(&mut self, rules: &Rules, s: usize) {
        if self.worn(s).is_some_and(|o| rules.objects[o].change_look) {
            self.choose_look = false;
        }
        self.equipped[s] = None;
        self.update_bonus(rules);
    }

    /// `Objet_Ranger`: an equipped object goes back to the bag, unless the bag is full.
    pub fn store(&mut self, rules: &Rules, s: usize) {
        let Some(o) = self.worn(s) else { return };
        if self.inventory_full() {
            return;
        }
        self.add_object(rules, o);
        self.destroy_equipped(rules, s);
        self.update_bonus(rules);
        self.attack = self.attack.min(self.bonus_max[4] as f64);
        self.defense = self.defense.min(self.bonus_max[5] as f64);
    }

    /// `Objet_Equiper`: equips the object of inventory slot `slot` (or the held object with
    /// None), or uses it when it is usable. `right` (`CoteDroit`) aims at the second weapon or
    /// ring slot.
    pub fn equip(&mut self, rules: &Rules, slot: Option<usize>, right: bool) -> Equip {
        let t = match slot {
            Some(s) => self.inventory.get(s).copied().flatten().map(|it| it.object),
            None => usize::try_from(self.selected_object).ok(),
        };
        let Some(t) = t else { return Equip::Refused };
        let o = &rules.objects[t];
        if o.mount && !rules.races[self.race].mount {
            return Equip::Refused;
        }
        if o.usable {
            return self.use_object(rules, t, slot);
        }
        // The original raises an error for a category without a slot.
        let Some(cat) = usize::try_from(o.category).ok().filter(|&c| c < self.equipped.len()) else {
            return Equip::Refused;
        };
        // Weapons use slots 0/1 (two-handed: 1), rings 6/7 (two-handed: 7).
        let pair = |cat: usize| if cat == 0 { (0, 1, 1) } else { (6, 7, 7) };
        let (mut old1, mut old2) = (None, None);
        if !self.slot_free(rules, t) {
            match cat {
                0 | 6 | 7 => {
                    let (a, b, two) = pair(cat);
                    if right {
                        if self.active(a) && self.active(b) {
                            old1 = self.worn(b);
                            self.destroy_equipped(rules, b);
                        }
                    } else if self.active(b) || self.active(a) && self.worn_category(rules, a) == two {
                        old1 = self.worn(a);
                        self.destroy_equipped(rules, a);
                    }
                }
                1 => {
                    old1 = self.worn(0);
                    self.destroy_equipped(rules, 0);
                    if self.active(1) {
                        old2 = self.worn(1);
                        self.destroy_equipped(rules, 1);
                    }
                }
                c => {
                    old1 = self.worn(c);
                    self.destroy_equipped(rules, c);
                }
            }
        }
        if o.change_look {
            self.choose_look = true;
        }
        let load = |p: &mut Perso, s: usize, o: Option<usize>| p.equipped[s] = o.map(|object| Item { object, count: 1 });
        match cat {
            0 | 6 | 7 => {
                let (a, b, _) = pair(cat);
                if right {
                    if self.active(b) {
                        if !self.active(a) {
                            load(self, a, self.worn(b));
                        }
                        load(self, b, Some(t));
                    } else {
                        if cat == 0 && self.two_handed(rules) {
                            self.store(rules, 0);
                        }
                        load(self, b, Some(t));
                    }
                } else if self.active(a) {
                    if self.active(b) {
                        // When the bag is full the second object is overwritten (lost).
                        self.store(rules, b);
                        load(self, b, self.worn(a));
                        load(self, a, Some(t));
                    } else if slot.is_some() {
                        load(self, b, Some(t));
                    } else {
                        load(self, b, self.worn(a));
                        load(self, a, Some(t));
                    }
                } else {
                    load(self, a, Some(t));
                }
            }
            1 => load(self, 0, Some(t)),
            c => load(self, c, Some(t)),
        }
        match slot {
            Some(s) => self.unload_slot(s),
            None => self.selected_object = -1,
        }
        for o in [old1, old2].into_iter().flatten() {
            self.add_object(rules, o);
        }
        if self.two_handed(rules) && self.active(1) {
            self.equipped[1] = None;
        }
        self.update_bonus(rules);
        Equip::Equipped
    }

    /// The usable branch of `Objet_Equiper`: pays and marks the object as used; its effect
    /// comes later in the tick (`Gestion_Utilisation_Speciale`).
    fn use_object(&mut self, rules: &Rules, t: usize, slot: Option<usize>) -> Equip {
        if !self.recharged(rules, t) {
            return Equip::Refused;
        }
        let o = &rules.objects[t];
        let alive = self.alive();
        if !((alive || o.use_when_dead) && self.used_object == -1 && !(alive && o.self_resurrection)) {
            return Equip::Refused;
        }
        let checks = [
            (self.bars[0] > o.cost[0] || !alive, 17),
            (self.bars[1] > o.cost[1], 18),
            (self.bars[2] >= o.cost[2], 19),
            (self.bars[3] > o.cost[3], 20),
            (self.money() >= o.cost_money as f64, 45),
            (self.res(o.ammo) >= o.ammo_quantity, 47),
        ];
        if let Some(&(_, m)) = checks.iter().find(|c| !c.0) {
            return Equip::Missing(m);
        }
        self.used_object = t as i64;
        self.train(rules, Trained::Object(o.skill), o.experience);
        self.set_life(rules, self.bars[0] - o.cost[0]);
        for i in 1..4 {
            self.set_bar(rules, i, self.bars[i] - o.cost[i]);
        }
        if o.single_use {
            match slot {
                Some(s) => self.unload_slot(s),
                None => self.selected_object = -1,
            }
        }
        self.set_money(self.money() - o.cost_money as f64);
        if o.ammo_quantity > 0.0
            && let Ok(r) = usize::try_from(o.ammo)
        {
            self.set_resource(r, self.resource(r) - o.ammo_quantity);
        }
        if o.recharge > 0 {
            self.add_recharge(t, o.recharge);
        }
        Equip::Used
    }

    /// `EffetRecharge_Pret`: not recharging, or a charge is left.
    pub fn recharged(&self, rules: &Rules, t: usize) -> bool {
        let o = &rules.objects[t];
        if o.recharge <= 0 {
            return true;
        }
        match self.recharges.iter().find(|r| r.object == t) {
            None => true,
            Some(_) if o.charges == 0.0 => false,
            Some(r) => {
                let charges = bonus_sup(rules, self.object_skill_level(rules, t), o.charges) as f64;
                (charges - r.duration as f64 / o.recharge as f64).floor() > 0.0
            }
        }
    }

    /// `EffetRecharge_Ajouter`: charges stack.
    pub fn add_recharge(&mut self, object: usize, duration: i64) {
        match self.recharges.iter_mut().find(|r| r.object == object) {
            Some(r) => r.duration += duration,
            None => self.recharges.push(Recharge { object, duration }),
        }
    }

    /// `Gestion_Effets_Recharge`: one slow tick; quirk: after the first expiry the rest wait.
    pub(crate) fn tick_recharges(&mut self) {
        for i in 0..self.recharges.len() {
            self.recharges[i].duration -= 1;
            if self.recharges[i].duration <= 0 {
                self.recharges.remove(i);
                break;
            }
        }
    }

    /// `EffetTemp_Ajouter`: `owner` is the caster and its item skill level, None for the
    /// character itself. The same effect from the same owner gets longer (capped at `max`
    /// unless it is -1).
    pub fn add_effect(&mut self, rules: &Rules, object: usize, duration: i64, max: i64, owner: Option<(usize, i64)>) {
        let (owner, owner_skill) = owner.unwrap_or((self.numero, 0));
        if let Some(e) = self.effects.iter_mut().find(|e| e.object == object && e.owner == owner) {
            set_duration(e, e.duration + duration);
            if max > -1 && e.duration > max {
                set_duration(e, max);
                e.duration_max = max;
            }
        } else {
            self.effects.push(TempEffect { object, duration, duration_max: duration, owner, owner_skill });
            self.update_bonus(rules);
        }
    }

    /// `EffetTemp_Effacer`: ends effect `i`, giving the object back if it says so.
    pub fn remove_effect(&mut self, rules: &Rules, i: usize) {
        let o = self.effects[i].object;
        if rules.objects[o].effect_restores {
            self.add_object(rules, o);
        }
        self.effects.remove(i);
        self.update_bonus(rules);
    }

    /// `Gestion_Effets_Temporaires`: one slow tick; quirk: after the first expiry the rest
    /// wait.
    pub fn tick_effects(&mut self, rules: &Rules) {
        for i in 0..self.effects.len() {
            if self.effects[i].duration != -1 {
                let d = self.effects[i].duration - 1;
                set_duration(&mut self.effects[i], d);
                if self.effects[i].duration <= 0 {
                    self.remove_effect(rules, i);
                    self.update_bonus(rules);
                    break;
                }
            }
        }
    }

    /// `EffetTemp_Effacer_Mort`
    pub(crate) fn effects_on_death(&mut self, rules: &Rules) {
        let mut j = 0;
        while let Some(i) = (j..self.effects.len()).find(|&i| rules.objects[self.effects[i].object].effect_ends_at_death) {
            self.remove_effect(rules, i);
            j = i;
        }
    }

    /// The item skill bonuses of `Actualiser_Bonus`: each equipped object, then each effect,
    /// adds its `BonusCompetencesObjets` scaled by the (partially updated) levels.
    pub(crate) fn add_skill_bonuses(&mut self, rules: &Rules) {
        let effects = self.effects.iter().map(|e| e.object);
        let sources: Vec<usize> = self.equipped.iter().flatten().map(|it| it.object).chain(effects).collect();
        for o in sources {
            let o = &rules.objects[o];
            for j in 0..self.objects.len() {
                let c = self.objects[o.skill].bonus;
                self.objects[j].bonus += bonus_sup(rules, c, o.bonus_object[j] as f64);
            }
        }
    }

    /// The `Ajouter_Bonus_Objet` part of `Actualiser_Bonus`: slots 2..10, then the weapons
    /// (so they win the "last value" fields), then the temporary effects.
    pub(crate) fn add_object_bonuses(&mut self, rules: &Rules) {
        let n = self.equipped.len();
        for s in (2..n).chain(0..2.min(n)) {
            if let Some(o) = self.worn(s) {
                let c = self.object_skill_level(rules, o);
                self.add_object_bonus(rules, o, false, -1, c);
            }
        }
        for i in 0..self.effects.len() {
            let e = self.effects[i];
            let c = if e.owner == self.numero { self.object_skill_level(rules, e.object) } else { e.owner_skill };
            self.add_object_bonus(rules, e.object, true, e.owner as i64, c);
        }
    }

    /// `Ajouter_Bonus_Objet`: the bonuses of object `n` at item skill level `c`. `owner` is
    /// the effect's caster, -1 for equipment.
    fn add_object_bonus(&mut self, rules: &Rules, n: usize, effect: bool, owner: i64, c: i64) {
        let o = &rules.objects[n];
        let sup = |v: f64| bonus_sup(rules, c, v);
        // The last object with a movement bonus wins; flyers get none.
        if !self.bonus_flying && sup(o.bonus_move) >= 1 {
            self.bonus_move = self.speed + sup(o.bonus_move);
        }
        if sup(o.bonus_move) < 0 {
            self.bonus_move = clng(self.speed as f64 + o.bonus_move);
        }
        self.bonus_speed += sup(o.bonus_speed);
        self.bonus_vitality += o.bonus_vitality;
        for k in 0..6 {
            self.bonus_max[k] += sup(o.bonus[k] as f64);
        }
        if o.attack_resistance > 0 && !effect {
            self.bonus_attack_resistance = o.attack_resistance;
        }
        self.bonus_range += range_sup(rules, c, o.range_bonus) as f64;
        if o.launcher > 0 {
            self.launcher = o.launcher;
        }
        self.bonus_area += range_sup(rules, c, o.area as f64);
        if o.attack_cause_effect >= 0 {
            self.bonus_attack_cause_effect.push(o.attack_cause_effect);
        }
        self.auto_attack |= o.auto_attack;
        if o.berserk > 0 {
            self.bonus_berserk = o.berserk;
        }
        if !effect {
            for k in 0..4 {
                self.costs[k] += o.cost[k];
            }
            self.cost_money += o.cost_money as f64;
            if o.ammo > 0 {
                self.ammo = o.ammo;
            }
            self.ammo_quantity += o.ammo_quantity;
        }
        if o.poison > 0.0 {
            self.poison += sup(o.poison).max(1);
            if owner >= 0 {
                self.poisoner = owner;
            }
        }
        self.reload_object += o.attack_recharge;
        self.projectile_count += sup(o.extra_projectiles);
        for (s, &b) in self.special.iter_mut().zip(&o.bonus_special) {
            s.bonus += sup(b as f64);
        }
        for (s, &b) in self.resource_skills.iter_mut().zip(&o.bonus_resource) {
            s.bonus += sup(b as f64);
        }
        for (s, &b) in self.services.iter_mut().zip(&o.bonus_service) {
            s.bonus += sup(b as f64);
        }
        if o.gain_money > 0.0 {
            self.gain_money = (self.gain_money + gain_sup(rules, c, o.gain_money)) as f32 as f64;
            self.generating = true;
        }
        for (g, &b) in self.gain_resources.iter_mut().zip(&o.gain_resources) {
            if b > 0.0 {
                *g = (*g + gain_sup(rules, c, b)) as f32 as f64;
                self.generating = true;
            }
        }
        self.bonus_armour += armour_sup(rules, c, o.bonus_armour as f64);
        self.bonus_max_resources += sup(o.bonus_max_resources as f64);
        for k in 0..4 {
            self.bonus_regen[k] = (self.bonus_regen[k] + o.bonus_regen[k]) as f32 as f64;
        }
        for (r, &b) in self.bonus_resistances.iter_mut().zip(&o.bonus_resistances) {
            if b != 0.0 {
                *r = ((*r - b) as f32 as f64).clamp(rules.resistance_min, rules.resistance_max);
            }
        }
        if (!(self.choose_look || self.look_locked) || o.mount) && o.has_overlay() {
            self.overlays.push(n);
            if o.mount {
                self.mount = self.overlays.len();
            }
        }
        if o.flying {
            self.bonus_flying = true;
        }
    }

    /// The `Gestion_Niveau` loops over the equipped objects: those for which `boosts` holds
    /// train their item skill by `gain`.
    pub(crate) fn train_objects(&mut self, rules: &Rules, boosts: impl Fn(&ObjectType) -> bool, gain: f64) {
        for s in 0..self.equipped.len() {
            if let Some(o) = self.worn(s).map(|o| &rules.objects[o])
                && boosts(o)
            {
                self.train(rules, Trained::Object(o.skill), gain);
            }
        }
    }

    /// `Gestion_Niveau_Prime`: XP of the bounty service skill (and of the objects boosting it).
    pub fn train_bounty(&mut self, rules: &Rules, score: f64) {
        let gain = score * rules.services[2].xp_coef;
        self.train(rules, Trained::Service(1), gain);
        if self.service_boosted(rules, 1) {
            self.train_objects(rules, |o| o.bonus_service.get(1).is_some_and(|&b| b > 0), gain);
        }
    }

    /// `Gestion_Niveau_Resistances`: objects improving resistance `k` gain XP from damage.
    pub fn train_resistance(&mut self, rules: &Rules, k: usize, score: f64) {
        if self.bonus_resistances[k] < self.resistances[k] {
            for s in 0..self.equipped.len() {
                if let Some(o) = self.worn(s).map(|o| &rules.objects[o])
                    && o.bonus_resistances[k] > 0.0
                {
                    let gain = score * self.max[0] as f64 / self.bonus_max[0] as f64 * 0.2 * o.bonus_resistances[k];
                    self.train(rules, Trained::Object(o.skill), gain);
                }
            }
        }
    }

    /// `Definir_Resistance`: the effective resistance `k`, 1 for -1.
    pub fn resistance(&self, k: i64) -> f64 {
        usize::try_from(k).ok().and_then(|k| self.bonus_resistances.get(k)).copied().unwrap_or(1.0)
    }

    /// The `ObjetInventaireCharger` / `ObjetInventaireNombre` commands of an inventory click
    /// while holding an object: stack it on slot `slot`, swap it with a single object there, or
    /// put it in the empty slot. A full stack or a stack of another object is left alone.
    pub fn place_held(&mut self, rules: &Rules, slot: usize) {
        let Ok(held) = usize::try_from(self.selected_object) else { return };
        match self.inventory[slot] {
            Some(ref mut it) if it.object == held && it.count < rules.objects[held].stack => {
                it.count += 1;
                self.selected_object = -1;
            }
            Some(it) if it.count < 2 => {
                self.inventory[slot] = Some(Item { object: held, count: 1 });
                self.selected_object = it.object as i64;
            }
            Some(_) => {}
            None => {
                self.inventory[slot] = Some(Item { object: held, count: 1 });
                self.selected_object = -1;
            }
        }
    }

    /// `Objet_Ajouter_Inventaire(..., IA_Equiper > 0)`, used for race objects.
    pub(crate) fn add_race_object(&mut self, rules: &Rules, o: usize) {
        if self.ai_equip > 0 {
            self.add_new_object(rules, o);
        } else {
            self.add_object(rules, o);
        }
    }
}

impl World {
    /// `Objet_Equiper` on inventory slot `slot` (None: the held object), saying why a usable
    /// object could not be used.
    pub fn equip_object_at(&mut self, rules: &Rules, i: usize, slot: Option<usize>, right: bool) -> Equip {
        let r = self.persos[i].equip(rules, slot, right);
        if let Equip::Missing(m) = r {
            self.say(rules, i, m);
        }
        r
    }

    /// `Objets_Utilisation_Automatique`: perso `i` uses every inventory object marked for it,
    /// each followed by `Gestion_Utilisation_Speciale` for everybody.
    pub(crate) fn auto_use(&mut self, rules: &Rules, i: usize) {
        for s in 0..self.persos[i].inventory.len() {
            if let Some(it) = self.persos[i].inventory[s]
                && rules.objects[it.object].auto_use
            {
                self.equip_object_at(rules, i, Some(s), false);
                self.special_uses(rules);
            }
        }
    }

    /// `Changer_Race_Personnage_Equipement`: the race's free talent objects, then their
    /// automatic use.
    pub fn give_talents(&mut self, rules: &Rules, i: usize) {
        let p = &mut self.persos[i];
        for &o in &rules.races[p.race].objects {
            if rules.objects[o].talent_price <= 0 {
                if rules.objects[o].change_look {
                    p.choose_look = false;
                }
                p.add_race_object(rules, o);
            }
        }
        self.auto_use(rules, i);
    }

    /// `Charger_Sauvegarde_Tresor` with the free talents a new character preselects: each one
    /// not owned yet is added, then the automatic uses run.
    pub fn load_talents(&mut self, rules: &Rules, i: usize) {
        for &o in &rules.races[self.persos[i].race].objects {
            if rules.objects[o].talent_price <= 0 && self.persos[i].object_count(o) == 0 {
                self.persos[i].add_race_object(rules, o);
                self.auto_use(rules, i);
            }
        }
    }
}
