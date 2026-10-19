//! Character bars, skills, experience, money, resources and inventory (`ClsJeuPerso`).

use crate::perso::{Item, Perso, Xp, action};
use crate::rules::Rules;
use crate::vb::clng;

// Constants of `ClsJeuPerso.cls`, kept as the original expressions (`VitesseAction / 400 = 1`,
// `IntervalesActions = 10`).
pub const BUILD_SPEED: f64 = 3.0;
pub const CRAFT_SPEED: f64 = 0.1;
const FATIGUE_WALK: f64 = 0.001 / 1.0 * 10.0;
const DEMORALISATION_WALK: f64 = 0.0 / 1.0 * 10.0;
const FATIGUE_WORK: f64 = 10.0 / (400.0 * 4.0) / 20.0 * 10.0 * 0.8;
const DEMORALISATION_WORK: f64 = 2.0 / (400.0 * 4.0) / 20.0 * 10.0 * 0.8;
const FATIGUE_COMBAT: f64 = 0.0125 / 1.0 * 10.0;
const DEMORALISATION_COMBAT: f64 = 0.0025 / 1.0 * 10.0;
/// `VitesseRecuperationX`: share of the max drained from the food pool per slow tick.
const RECOVERY: [f64; 4] = [0.075 / 20.0 / 1.0 * 10.0, 0.1 / 20.0 / 1.0 * 10.0, 0.2 / 20.0 / 2.0 / 1.0 * 10.0, 0.05 / 20.0 / 1.0 * 10.0];
const RECOVERY_ATTACK: f64 = 1.25;
const RECOVERY_DEFENSE: f64 = 0.75;
/// `VitesseRegenerationXEnAttente` (idle outside) and `EnRepos` (resting at home).
const REGEN_IDLE: [f64; 4] = [0.001 / 20.0 / 1.0 * 10.0, 0.001 / 20.0 / 1.0 * 10.0, 0.0, 0.0 / 20.0 / 1.0 * 10.0];
const REGEN_REST: [f64; 4] = [0.002 / 20.0 / 1.0 * 10.0, 0.002 / 20.0 / 1.0 * 10.0, 0.0, 0.0005 / 20.0 / 1.0 * 10.0];
/// `CoefExp*`: XP per level step, for the 6 bars and the skill families.
pub const XP_COEFS: [f64; 6] = [20.0, 20.0, 20.0, 20.0, 400.0, 250.0];
pub const XP_SPECIAL: f64 = 20.0;
pub const XP_RESOURCE: f64 = 15.0;
pub const XP_SERVICE: f64 = 20.0;
pub const XP_OBJECT: f64 = 20.0;
/// `CoefExpCompConstruction`, `Destruction`, `CoupCritique`, `Marchandage`.
const XP_SPECIAL_COEFS: [f64; 4] = [0.00075, 0.0025, 0.00125 * 4.0 * 4.0, 0.02 / 20.0];
/// `CoefExpComp*Objet`: share of that XP for the objects boosting the skill.
const XP_SPECIAL_OBJECT_COEFS: [f64; 4] = [0.05, 1.0, 1.0, 1.0];
/// `CoefExpCompObjetsVie/Energie/Magie/Moral/Attaque/Defense`: object XP per bar loss.
const XP_BAR_OBJECT_COEFS: [f64; 6] = [0.2, 2.0, 0.2, 10.0, 0.1, 0.16];
pub const XP_CRAFT: f64 = 0.0025;
/// Money is clamped to `[0, MAX_MONEY]`.
pub const MAX_MONEY: f64 = 10_000_000.0;

/// What `Gestion_Niveau` trains.
#[derive(Debug, Clone, Copy)]
pub enum Trained {
    Bar(usize),
    Special(usize),
    Resource(usize),
    Service(usize),
    Object(usize),
}

impl Perso {
    fn xp_mut(&mut self, t: Trained) -> (&mut i64, &mut Xp, f64) {
        let s = match t {
            Trained::Bar(i) => return (&mut self.max[i], &mut self.bar_xp[i], XP_COEFS[i]),
            Trained::Special(i) => (&mut self.special[i], XP_SPECIAL),
            Trained::Resource(i) => (&mut self.resource_skills[i], XP_RESOURCE),
            Trained::Service(i) => (&mut self.services[i], XP_SERVICE),
            Trained::Object(i) => (&mut self.objects[i], XP_OBJECT),
        };
        (&mut s.0.carac, &mut s.0.xp, s.1)
    }

    /// `Gestion_Niveau`: adds XP, and levels up (`Carac + 1`) as long as it is enough.
    pub fn train(&mut self, rules: &Rules, t: Trained, gain: f64) {
        if !self.gain_experience {
            return;
        }
        self.xp_mut(t).1.exp += gain;
        loop {
            let (carac, xp, k) = self.xp_mut(t);
            if xp.exp < xp.exp_next {
                break;
            }
            *carac += 1;
            xp.level += 1;
            xp.exp_level = xp.exp_next;
            xp.exp_next += (xp.level + 1) as f64 * k;
            self.update_bonus(rules);
        }
    }

    /// `Gestion_Niveau_Comp_Speciale`
    pub fn train_special(&mut self, rules: &Rules, i: usize, score: f64) {
        let score = if i == 2 { score.min(self.attack) } else { score };
        let gain = score * XP_SPECIAL_COEFS[i] * XP_SPECIAL;
        self.train(rules, Trained::Special(i), gain);
        if self.special_boosted(rules, i) {
            self.train_objects(rules, |o| o.bonus_special[i] > 0, gain * XP_SPECIAL_OBJECT_COEFS[i]);
        }
    }

    /// `Bonus_CompetenceSpeciales`: an item boosts special skill `i`.
    pub fn special_boosted(&self, rules: &Rules, i: usize) -> bool {
        let s = &self.special[i];
        s.bonus as f64 > s.carac as f64 * rules.races[self.race].coef_special[i]
    }

    /// `Bonus_CompetenceServices`: an item boosts service skill `i` (service `i + 1`).
    pub fn service_boosted(&self, rules: &Rules, i: usize) -> bool {
        let s = &self.services[i];
        s.bonus as f64 > s.carac as f64 * rules.races[self.race].coef_service[i]
    }

    /// `Bonus_CompetenceObjets`: an item boosts item skill `i`.
    pub fn object_boosted(&self, rules: &Rules, i: usize) -> bool {
        let s = &self.objects[i];
        s.bonus as f64 > s.carac as f64 * rules.races[self.race].coef_object[i]
    }

    /// `Definir_BonusVie` etc.: the items' share of maximum `i`.
    fn item_bonus(&self, rules: &Rules, i: usize) -> f64 {
        (self.bonus_max[i] as f64 - self.max[i] as f64 * rules.races[self.race].coef[i] as f32 as f64).floor()
    }

    /// The object loop of the bar setters: objects boosting bar `i` train with the loss, read
    /// again each time since levelling up rescales the bars.
    fn train_bar_objects(&mut self, rules: &Rules, i: usize, gain: impl Fn(&Perso) -> f64) {
        if self.item_bonus(rules, i) > 0.0 {
            for s in 0..self.equipped.len() {
                if let Some(it) = self.equipped[s]
                    && rules.objects[it.object].bonus[i] > 0
                {
                    let g = gain(self);
                    self.train(rules, Trained::Object(rules.objects[it.object].skill), g);
                }
            }
        }
    }

    /// `Ajuster_Bonus`: effective values reset to the base ones.
    pub fn adjust_bonus(&mut self, rules: &Rules) {
        let r = &rules.races[self.race];
        self.bonus_regen = r.regen;
        self.bonus_move = self.speed;
        self.bonus_speed = 0;
        self.bonus_max = self.max;
        self.bonus_range = self.range;
        self.launcher = -1;
        for s in &mut self.special {
            s.bonus = s.carac;
        }
        for s in &mut self.resource_skills {
            s.bonus = s.xp.level;
        }
        for s in self.services.iter_mut().chain(&mut self.objects) {
            s.bonus = s.carac;
        }
        self.bonus_resistances.clone_from(&self.resistances);
        self.bonus_max_resources = r.bonus_max_resources;
        self.bonus_attack_resistance = self.attack_resistance;
        self.bonus_area = self.area;
        self.bonus_armour = self.armour;
        self.mount = 0;
        self.bonus_flying = self.flying;
    }

    /// `Actualiser_Bonus`: recomputes every effective value and rescales the bars.
    pub fn update_bonus(&mut self, rules: &Rules) {
        let r = &rules.races[self.race];
        self.overlays.clear();
        self.bonus_vitality = 1.0;
        self.bonus_berserk = self.berserk;
        self.generating = false;
        self.auto_attack = false;
        self.bonus_attack_cause_effect.clear();
        if self.attack_cause_effect >= 0 {
            self.bonus_attack_cause_effect.push(self.attack_cause_effect);
        }
        self.gain_money = 0.0;
        self.gain_resources.iter_mut().for_each(|g| *g = 0.0);
        let old = self.bonus_max;
        self.costs = [0.0; 4];
        self.cost_money = 0.0;
        self.ammo = 0;
        self.ammo_quantity = 0.0;
        self.poison = 0;
        self.poisoner = -1;
        self.reload_object = 0;
        self.projectile_count = 1;
        self.adjust_bonus(rules);
        self.add_skill_bonuses(rules);
        for (s, c) in self.objects.iter_mut().zip(&r.coef_object) {
            s.bonus = clng(s.bonus as f64 * c);
        }
        self.add_object_bonuses(rules);
        for i in 0..6 {
            self.bonus_max[i] = clng(self.bonus_max[i] as f64 * r.coef[i] as f32 as f64);
        }
        for (s, c) in self.special.iter_mut().zip(&r.coef_special) {
            s.bonus = clng(s.bonus as f64 * c);
        }
        let base = rules.start_max_resources as f64;
        for (s, c) in self.resource_skills.iter_mut().zip(&r.coef_resource) {
            s.bonus = clng(s.bonus as f64 + base * c - base);
        }
        for (s, c) in self.services.iter_mut().zip(&r.coef_service) {
            s.bonus = clng(s.bonus as f64 * c);
        }
        if self.bonus_move <= 1 {
            self.bonus_move = if self.speed == 0 { 0 } else { 1 };
        }
        for m in &mut self.bonus_max {
            if *m <= 0 {
                *m = 1;
            }
        }
        self.bonus_move += self.bonus_speed;
        for i in 0..4 {
            self.bars[i] = self.bars[i] * self.bonus_max[i] as f64 / old[i] as f64;
            self.recovery[i] = self.recovery[i] * self.bonus_max[i] as f64 / old[i] as f64;
        }
        self.attack = self.attack * self.bonus_max[4] as f64 / old[4] as f64;
        self.defense = self.defense * self.bonus_max[5] as f64 / old[5] as f64;
        self.attack = self.attack.min(self.bonus_max[4] as f64);
        if self.armour > self.bonus_max[5] {
            self.bonus_max[5] = self.armour;
        }
        self.defense = self.defense.min(self.bonus_max[5] as f64);
        for r in 0..self.resources.len() {
            self.resources[r] = self.resources[r].min(self.max_resource(r));
        }
    }

    /// `Definir_X` getter: the bar including its pending recovery, capped at the max.
    pub fn bar_total(&self, i: usize) -> f64 {
        (self.bars[i] + self.recovery[i]).min(self.bonus_max[i] as f64)
    }

    /// `Definir_Vie` setter. Returns true, leaving the bar as it is, when the character dies:
    /// the caller must then run `World::kill` (`Vivant = False`), which stops it while it is
    /// still alive, like the original, then empties the bar.
    pub fn set_life(&mut self, rules: &Rules, v: f64) -> bool {
        if self.alive() && v <= 0.0 {
            return true;
        }
        if v < self.bars[0] {
            let gain = (self.bars[0] - v) * self.max[0] as f64 / self.bonus_max[0] as f64;
            self.train(rules, Trained::Bar(0), gain);
            self.train_bar_objects(rules, 0, |p| {
                (p.bars[0] - v) * p.max[0] as f64 / p.bonus_max[0] as f64 * XP_BAR_OBJECT_COEFS[0]
            });
        }
        self.bars[0] = v.min(self.bonus_max[0] as f64);
        false
    }

    /// `Definir_Energie`, `Definir_Magie` and `Definir_Moral` setters: clamped, losses give XP.
    pub fn set_bar(&mut self, rules: &Rules, i: usize, v: f64) {
        if v < self.bars[i] {
            let loss = self.bars[i] - v;
            let gain = if i == 2 { loss * self.max[2] as f64 / self.bonus_max[2] as f64 } else { loss };
            self.train(rules, Trained::Bar(i), gain);
            self.train_bar_objects(rules, i, |p| {
                let loss = p.bars[i] - v;
                let loss = if i == 2 { loss * p.max[2] as f64 / p.bonus_max[2] as f64 } else { loss };
                loss * XP_BAR_OBJECT_COEFS[i]
            });
        }
        if v < 0.0 {
            self.bars[i] = 0.0;
        } else if v > self.bonus_max[i] as f64 {
            self.bars[i] = self.bonus_max[i] as f64;
        } else {
            self.bars[i] = v;
        }
    }

    /// `Definir_Attaque` setter.
    pub fn set_attack(&mut self, rules: &Rules, v: f64) {
        let v = if v < 0.0 { 0.0 } else { v.min(self.bonus_max[4] as f64) };
        if v < self.attack {
            let gain = (self.attack - v) * self.max[4] as f64 / self.bonus_max[4] as f64;
            self.train(rules, Trained::Bar(4), gain);
            self.train_bar_objects(rules, 4, |p| {
                (p.attack - v) * p.max[4] as f64 / p.bonus_max[4] as f64 * XP_BAR_OBJECT_COEFS[4]
            });
        }
        self.attack = v;
    }

    /// `Definir_Defense` setter: floored at the armour.
    pub fn set_defense(&mut self, rules: &Rules, v: f64) {
        let v = v.max(self.bonus_armour as f64).min(self.bonus_max[5] as f64);
        if v < self.defense {
            let gain = (self.defense - v) * self.max[5] as f64 / self.bonus_max[5] as f64;
            self.train(rules, Trained::Bar(5), gain);
            self.train_bar_objects(rules, 5, |p| {
                (p.defense - v) * p.max[5] as f64 / p.bonus_max[5] as f64 * XP_BAR_OBJECT_COEFS[5]
            });
        }
        self.defense = v;
    }

    /// `Niveau`: a tenth of the bar levels and a hundredth of the skill levels, plus 1.
    pub fn level(&self) -> f64 {
        let mut v = self.bar_xp.iter().map(|x| x.level).sum::<i64>() as f64 * 0.1 + 1.0;
        for s in self.special.iter().chain(&self.resource_skills).chain(&self.services).chain(&self.objects) {
            v += s.xp.level as f64 * 0.01;
        }
        v
    }

    pub fn money(&self) -> f64 {
        self.money
    }

    /// `Definir_Argent` setter.
    pub fn set_money(&mut self, v: f64) {
        self.money = v.clamp(0.0, MAX_MONEY);
    }

    pub fn resource(&self, r: usize) -> f64 {
        self.resources[r]
    }

    /// `Definir_MaxRessources`: carrying capacity.
    pub fn max_resource(&self, r: usize) -> f64 {
        (self.resource_skills[r].carac + self.bonus_max_resources) as f64
    }

    /// `Definir_ressources` setter: clamped to `[0, max_resource]`.
    pub fn set_resource(&mut self, r: usize, v: f64) {
        let max = self.max_resource(r);
        self.resources[r] = if v > max { max } else if v < 0.0 { 0.0 } else { v };
    }

    /// `Definir_Niveau_CompetenceRessources`: the effective harvest level (an Integer).
    pub fn harvest_level(&self, r: usize) -> i64 {
        self.resource_skills[r].bonus
    }

    /// `Bonus_CompetenceRessources`: an item boosts this skill (also true from race rounding).
    pub fn resource_boosted(&self, rules: &Rules, r: usize) -> bool {
        let base = rules.start_max_resources as f64;
        let s = &self.resource_skills[r];
        (s.bonus as f64 + base) > (s.xp.level as f64 + base) * rules.races[self.race].coef_resource[r]
    }

    /// `Prix_Marchandee`: the price after bargaining (an Integer).
    pub fn bargained(&self, rules: &Rules, price: f64) -> i64 {
        clng(price / 2.0 * (1.0 + rules.start_special as f64 / self.special[3].bonus as f64))
    }

    /// `RecuperationRapide`, every tick: the combat bars regenerate.
    pub fn quick_recovery(&mut self, rules: &Rules) {
        if !self.alive() {
            return;
        }
        if self.attack < self.bonus_max[4] as f64 {
            let v = self.attack + RECOVERY_ATTACK * self.vitality * self.bonus_max[4] as f64 / self.max[4] as f64;
            self.set_attack(rules, v);
        }
        if self.defense < self.bonus_max[5] as f64 {
            let v = self.defense + RECOVERY_DEFENSE * self.vitality * self.bonus_max[5] as f64 / self.max[5] as f64;
            self.set_defense(rules, v);
        }
        if self.reload > 0 {
            self.reload = (self.reload - 1).max(0);
        }
    }

    /// The start of the living branch of `RecuperationLente`, every 10th tick: walking and
    /// working tire (the caller then tramples, `Perso_Pietinement`). `craft_skill` is the item
    /// skill of the object being crafted at home.
    pub(crate) fn slow_fatigue(&mut self, rules: &Rules, craft_skill: Option<usize>) {
        let base = rules.start_max_resources as f64;
        if self.moving {
            self.set_bar(rules, 1, self.bars[1] - FATIGUE_WALK);
            self.set_bar(rules, 3, self.bars[3] - DEMORALISATION_WALK);
        } else {
            let work = |p: &mut Perso, level: f64| {
                p.set_bar(rules, 1, p.bars[1] - FATIGUE_WORK * level);
                p.set_bar(rules, 3, p.bars[3] - DEMORALISATION_WORK * level);
            };
            match self.action {
                action::IDLE | action::AT_HOME => {
                    let regen = if self.action == action::IDLE { REGEN_IDLE } else { REGEN_REST };
                    let v = self.bars[0] + regen[0] * self.bonus_max[0] as f64;
                    self.set_life_dying(rules, v);
                    for i in 1..4 {
                        self.set_bar(rules, i, self.bars[i] + regen[i] * self.bonus_max[i] as f64);
                    }
                }
                action::HARVEST | action::WORK_HOME if self.current_resource >= 0 => {
                    let level = self.resource_skills[self.current_resource as usize].xp.level as f64;
                    work(self, level + base);
                }
                action::WORK_HOME => {
                    if let Some(s) = self.current_service.checked_sub(1).and_then(|s| self.services.get(s)) {
                        work(self, s.carac as f64);
                    }
                }
                action::BUILD => work(self, self.special[0].carac as f64),
                action::CRAFT => {
                    if let Some(c) = craft_skill {
                        work(self, self.objects[c].carac as f64);
                    }
                }
                _ => {}
            }
        }
    }

    /// The rest of the living branch of `RecuperationLente`: food, combat fatigue,
    /// regeneration, income, effects and recharges.
    pub(crate) fn slow_recovery(&mut self, rules: &Rules) {
        for i in 0..4 {
            if self.recovery[i] > 0.0 {
                let s = RECOVERY[i] * self.bonus_max[i] as f64;
                let gain = if self.recovery[i] >= s { s } else { self.recovery[i] };
                let v = self.bars[i] + gain;
                if i == 0 {
                    self.set_life_dying(rules, v);
                } else {
                    self.set_bar(rules, i, v);
                }
                if self.recovery[i] >= s {
                    self.recovery[i] -= s;
                } else {
                    self.recovery[i] = 0.0;
                }
            }
        }
        let fight = |p: &mut Perso| {
            p.set_bar(rules, 1, p.bars[1] - FATIGUE_COMBAT * p.vitality);
            p.set_bar(rules, 3, p.bars[3] - DEMORALISATION_COMBAT * p.vitality);
        };
        if self.attack < self.bonus_max[4] as f64 {
            fight(self);
        }
        if self.defense < self.bonus_max[5] as f64 {
            fight(self);
        }
        let v = self.bars[0] + self.bonus_regen[0] * self.bonus_max[0] as f64;
        self.set_life_dying(rules, v);
        for i in 1..4 {
            self.set_bar(rules, i, self.bars[i] + self.bonus_regen[i] * self.bonus_max[i] as f64);
        }
        // Generer_Ressource
        if self.generating {
            self.set_money(self.money + self.gain_money);
            for r in 0..self.resources.len() {
                self.set_resource(r, self.resources[r] + self.gain_resources[r]);
            }
        }
        self.tick_effects(rules);
        self.tick_recharges();
    }

    /// `Definir_Vie` where the death cannot be handled at once (a negative regeneration): the
    /// bar empties and `World::slow_tick` kills afterwards.
    fn set_life_dying(&mut self, rules: &Rules, v: f64) {
        if self.set_life(rules, v) {
            self.bars[0] = 0.0;
        }
    }

    /// `Manger_Ressources_Coef`
    fn eat_coef(&self, i: usize) -> f64 {
        self.bonus_max[i] as f64 / self.max[i] as f64
    }

    /// `Manger_Ressources`: eats one unit of `r` into the recovery pools.
    pub fn eat(&mut self, rules: &Rules, r: usize) {
        if self.resources[r] >= 1.0 {
            self.resources[r] -= 1.0;
            for i in 0..4 {
                self.recovery[i] += rules.resources[r].gain[i] * self.eat_coef(i);
            }
        }
    }

    /// `Recuperer_Vie/Energie/Magie/Moral`: eats the first suitable food until the bar is
    /// nearly full or the stock runs out.
    pub fn eat_for(&mut self, rules: &Rules, bar: usize) {
        for (r, res) in rules.resources.iter().enumerate() {
            if res.edible && res.gain[bar] > 0.0 && self.resources[r] >= 1.0 {
                loop {
                    self.eat(rules, r);
                    let target = self.bonus_max[bar] as f64 - res.gain[bar] * self.eat_coef(bar);
                    if self.bar_total(bar) >= target || self.resources[r] < 1.0 {
                        break;
                    }
                }
                return;
            }
        }
    }

    /// `Recuperer_AutoVie` etc.: eats when Vie is below full, or another bar below half.
    pub fn eat_auto(&mut self, rules: &Rules, bar: usize) {
        let threshold = if bar == 0 { 1.0 } else { 0.5 };
        if self.bar_total(bar) < self.bonus_max[bar] as f64 * threshold {
            self.eat_for(rules, bar);
        }
    }

    /// `Objet_Ajouter_Inventaire_Fast`: the first free slot, or stacking on an earlier slot.
    pub fn add_object(&mut self, rules: &Rules, object: usize) {
        for slot in &mut self.inventory {
            match slot {
                None => {
                    *slot = Some(Item { object, count: 1 });
                    return;
                }
                Some(item) if item.object == object && item.count < rules.objects[object].stack => {
                    item.count += 1;
                    return;
                }
                _ => {}
            }
        }
    }

    /// `Objet_Ajouter_Inventaire` with `NouveauObjet`: like `add_object`, but a character with
    /// `IA_Equiper` equips a new non-usable object at once when its slot is free.
    pub fn add_new_object(&mut self, rules: &Rules, object: usize) {
        for s in 0..self.inventory.len() {
            match &mut self.inventory[s] {
                None => {
                    self.inventory[s] = Some(Item { object, count: 1 });
                    if self.ai_equip != 0 && self.slot_free(rules, object) && !rules.objects[object].usable {
                        self.equip(rules, Some(s), false);
                    }
                    return;
                }
                Some(item) if item.object == object && item.count < rules.objects[object].stack => {
                    item.count += 1;
                    return;
                }
                _ => {}
            }
        }
    }

    /// `Decharger` on an inventory slot: one fewer, or empty.
    pub fn unload_slot(&mut self, slot: usize) {
        if let Some(Some(item)) = self.inventory.get_mut(slot) {
            if item.count > 1 {
                item.count -= 1;
            } else {
                self.inventory[slot] = None;
            }
        }
    }

    /// `Objet_Prendre_Premier`: holds an object to give. Quirk: every inventory slot loses one
    /// object, and only the last one's type is held.
    pub fn take_first_object(&mut self) {
        for s in 0..self.inventory.len() {
            if let Some(it) = self.inventory[s] {
                self.selected_object = it.object as i64;
                self.unload_slot(s);
            }
        }
    }

    /// `Objet_Nombre_Total(True)`: occupied inventory slots.
    pub fn inventory_count(&self) -> usize {
        self.inventory.iter().flatten().count()
    }

    /// `Inventaire_Plein`
    pub fn inventory_full(&self) -> bool {
        self.inventory.iter().all(Option::is_some)
    }

    /// `Objet_Supprimer`: destroys one object of this type, equipped first.
    pub fn remove_object(&mut self, rules: &Rules, object: usize) {
        if let Some(i) = self.equipped.iter().position(|s| s.is_some_and(|it| it.object == object)) {
            self.destroy_equipped(rules, i);
            return;
        }
        if let Some(i) = self.inventory.iter().position(|s| s.is_some_and(|it| it.object == object)) {
            self.unload_slot(i);
        }
    }

    /// `Objet_NombreType`
    pub fn object_count(&self, object: usize) -> i64 {
        let equipped = self.equipped.iter().flatten().filter(|it| it.object == object).count() as i64;
        equipped + self.inventory.iter().flatten().filter(|it| it.object == object).map(|it| it.count).sum::<i64>()
    }

    /// `Ressources_TotalPrix` (a Long, re-rounded at each step).
    pub fn resources_value(&self, rules: &Rules) -> i64 {
        self.resources.iter().zip(&rules.resources).fold(0, |t, (q, r)| clng(t as f64 + q * r.price))
    }

    /// `Objets_TotalPrix`
    pub fn objects_value(&self, rules: &Rules) -> i64 {
        self.equipped
            .iter()
            .chain(&self.inventory)
            .flatten()
            .filter(|it| !rules.objects[it.object].inseparable)
            .map(|it| rules.objects[it.object].price)
            .sum()
    }

    /// `Definir_ObjetSelectionne`: true when holding anything.
    pub fn holding(&self) -> bool {
        self.selected_money > 0.0 || self.selected_object >= 0 || self.resources_selected
    }

    /// `Definir_ObjetSelectionne = False`: drops the selection, the held object back in the bag.
    pub fn clear_selection(&mut self, rules: &Rules) {
        self.selected_money = 0.0;
        self.selected_resources.iter_mut().for_each(|r| *r = 0.0);
        self.resources_selected = false;
        if self.selected_object >= 0 {
            self.add_object(rules, self.selected_object as usize);
            self.selected_object = -1;
        }
    }

    /// `Argent_Prendre`: selects money to give, drop or donate.
    pub fn select_money(&mut self, q: f64, all: bool) {
        let mut q = q + self.selected_money;
        if all {
            q = clng(self.money) as f64;
        } else if self.money < q {
            q = self.money;
        }
        if q >= 1.0 {
            self.selected_money = q;
        }
    }

    /// `Ressources_Prendre`: selects (or with `remove`, unselects) resource `r`.
    pub fn select_resource(&mut self, r: usize, q: f64, all: bool, remove: bool) {
        if remove {
            if all {
                self.selected_resources[r] = 0.0;
            } else {
                self.selected_resources[r] = (self.selected_resources[r] - q).max(0.0);
            }
            self.check_selection();
        } else {
            let q = if all || self.resources[r] < q { self.resources[r] } else { q };
            if q >= 1.0 {
                self.selected_resources[r] += clng(q) as f64;
                self.resources_selected = true;
            }
        }
    }

    /// `Ressources_Verifications`: the selection cannot exceed what is owned.
    pub fn check_selection(&mut self) {
        if self.selected_money > self.money {
            self.selected_money = self.money;
        }
        self.resources_selected = false;
        for r in 0..self.selected_resources.len() {
            if self.selected_resources[r] > self.resources[r] {
                self.selected_resources[r] = self.resources[r];
                if self.resources[r] < 1.0 {
                    self.selected_resources[r] = 0.0;
                }
            }
            if self.selected_resources[r] > 0.0 {
                self.resources_selected = true;
            }
        }
    }
}
