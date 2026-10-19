//! The scripted `.ia` AI (`ModIA2.Comportement_IA2` and its helpers), and how it chooses and
//! places its buildings (`Choisir_Batiment`, `Trouver_Emplacement_Maison`).

use super::dist;
use crate::ia_file::Step;
use crate::perso::action;
use crate::rules::Rules;
use crate::terrain::CELL;
use crate::vb::clng;
use crate::world::World;

/// `NombreRecherchesEmplacementMaison`
const SITE_TRIES: usize = 10;

impl World {
    /// `Comportement_IA2`: pursue the current goal (house to build, resource or object to get),
    /// else run the step list of the age.
    pub(super) fn ai_scripted(&mut self, rules: &Rules, i: usize, simple: bool) {
        self.pursue(rules, i);
        let a = self.persos[i].action;
        if a != action::GO_HARVEST && a != action::CRAFT {
            self.recover(rules, i);
        }
        let p = &self.persos[i];
        let busy =
            [action::HARVEST, action::GO_HARVEST, action::WORK_HOME, action::GO_GIVE, action::CRAFT, action::SERVICE];
        if p.fighting() || busy.contains(&p.action) {
            return;
        }
        self.sell_surplus(rules, i);
        let h = &self.houses[i];
        let build = self.ai.persos[i].build;
        if h.visible() && h.life < h.max_life as f64 {
            self.ai_go_home(i);
        } else if build >= 0 && (h.kind as i64 != build || !h.visible()) {
            let b = build as usize;
            if self.can_build(rules, i, b) {
                if self.may_change_house(rules, i)
                    && !simple
                    && let Some((x, y)) = self.house_site(rules, i, b)
                {
                    self.create_house(rules, i, x, y, b, i);
                }
            } else {
                let p = &self.persos[i];
                let missing =
                    (rules.buildings[b].price_resources.iter().enumerate()).find(|&(r, &q)| p.res(r as i64) < q as f64);
                if let Some((r, &q)) = missing {
                    self.gather(rules, i, r as i64, q as f64);
                }
            }
        } else if !self.harvest_raw_material(rules, i) {
            if self.ai.persos[i].resource >= 0 {
                self.make_goal_resource(rules, i);
            } else if self.ai.persos[i].object >= 0 {
                self.make_goal_object(rules, i);
            } else {
                self.run_steps(rules, i);
            }
        }
    }

    /// Fetches the raw material of the own house when it can be harvested, or goes home to
    /// transform it. Returns whether that branch was taken.
    fn harvest_raw_material(&mut self, rules: &Rules, i: usize) -> bool {
        let t = self.houses[i].kind;
        let rf = rules.buildings[t].product;
        let p = &self.persos[i];
        let room = self.houses[i].visible() && rf >= 0 && p.res(rf) < p.max_res(rf);
        // `IIf(MP = -1, False, Recoltable(IIf(MP = -1, 0, MP)))`: three draws for a factory.
        let m1 = self.raw_material(rules, t);
        let m2 = self.raw_material(rules, t);
        let m3 = self.raw_material(rules, t);
        let harvestable = m1 != -1 && self.harvestable(rules, if m2 == -1 { 0 } else { m3 });
        if !(room && harvestable) {
            return false;
        }
        let m = self.raw_material(rules, t);
        if self.persos[i].res(m) <= 1.0 {
            let m = self.raw_material(rules, t);
            self.gather_resource(rules, i, m);
        } else {
            self.ai_go_home(i);
        }
        true
    }

    /// `Ressources_Recoltable`, false for a negative index.
    pub(super) fn harvestable(&self, rules: &Rules, r: i64) -> bool {
        usize::try_from(r).ok().and_then(|r| rules.resources.get(r)).is_some_and(|r| r.harvestable)
    }

    /// Goal "make resource" (`RessourceAFabriquer >= 0`).
    fn make_goal_resource(&mut self, rules: &Rules, i: usize) {
        let s = &self.ai.persos[i];
        let (r, q) = (s.resource, s.quantity);
        let p = &self.persos[i];
        if p.res(r) >= q as f64 {
            self.ai.persos[i].resource = -1;
        } else if self.harvestable(rules, r) {
            self.gather_resource(rules, i, r);
        } else if self.houses[i].visible() && rules.buildings[self.houses[i].kind].product == r {
            let t = self.houses[i].kind;
            if !p.in_own_house {
                self.ai_go_home(i);
            } else if self.raw_material(rules, t) >= 0 {
                let m = self.raw_material(rules, t);
                if self.persos[i].res(m) < 1.0 {
                    let m = self.raw_material(rules, t);
                    self.gather(rules, i, m, rules.start_max_resources as f64);
                }
            }
        } else if p.action != action::GO_OWN_CASTLE && p.action != action::GO_CASTLE {
            self.gather(rules, i, r, q as f64);
        }
    }

    /// Goal "get object" (`ObjetAFabriquer >= 0`): craft it at home, or buy it.
    fn make_goal_object(&mut self, rules: &Rules, i: usize) {
        let s = &self.ai.persos[i];
        let (o, q) = (s.object, s.quantity);
        let p = &self.persos[i];
        if p.action == action::CRAFT || self.object_count(i, o) >= q {
            return;
        }
        let h = &self.houses[i];
        if !h.visible() {
            self.seek_object(rules, i, o, q);
        } else if !p.in_own_house {
            self.seek_object(rules, i, o, q);
            if self.persos[i].idle() {
                self.ai_go_home(i);
            }
        } else if h.factory {
            let objects = &rules.buildings[h.kind].objects;
            if let Some(j) = objects.iter().position(|&x| x as i64 == o) {
                if h.object_stock(j) == 0.0 {
                    if self.can_craft(rules, i, j as i64, i) {
                        self.start_craft(i, j as i64);
                    } else {
                        self.seek_object(rules, i, o, q);
                    }
                } else {
                    self.take_from_house(rules, i, j, i, 1);
                    self.ai.persos[i].object = -1;
                }
            }
        } else if self.may_change_house(rules, i) {
            self.demolish(rules, i, i, false);
            self.ai_scripted(rules, i, false);
            self.ai.persos[i].build = -1;
        }
    }

    /// `Objet_NombreType`, 0 for an invalid type.
    fn object_count(&self, i: usize, o: i64) -> i64 {
        usize::try_from(o).map_or(0, |o| self.persos[i].object_count(o))
    }

    /// The step list of the age (`Empereur.ia` for an emperor); stops at the first step that acts.
    fn run_steps(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let n = if p.emperor { crate::ia_file::find(&rules.ais, "Empereur.ia") } else { p.ai_index };
        let nf = p.fief;
        let e = self.fiefs[nf].epoch;
        let Some(epoch) = usize::try_from(n).ok().and_then(|n| rules.ais.get(n)).and_then(|a| a.epochs.get(e)) else {
            return;
        };
        for &step in &epoch.steps {
            let p = &self.persos[i];
            let h = &self.houses[i];
            let trade = self.castles[nf].visible() || !self.fiefs[nf].markets.is_empty();
            match step {
                Step::Resource { resource, quantity } => {
                    if p.res(resource) < quantity as f64 {
                        self.gather(rules, i, resource, quantity as f64);
                        break;
                    }
                }
                Step::Building(v) => {
                    if h.kind != v && self.ai.persos[i].build != v as i64 {
                        self.choose_house(rules, i, v);
                        break;
                    }
                }
                Step::Object { object, quantity } => {
                    if self.object_count(i, object) < quantity {
                        self.seek_object(rules, i, object, quantity);
                        break;
                    }
                }
                Step::Work => {
                    if !h.visible() {
                        self.ai.persos[i].build = self.choose_building(rules, i, nf, -1);
                    } else if !trade {
                        self.aggressive(rules, i);
                    } else if epoch.building_defined {
                        self.seek_raw(rules, i);
                        self.ai_work(rules, i);
                    } else {
                        // A free career: try to move on to a better building.
                        let kind = h.kind;
                        let b = self.choose_building(rules, i, nf, rules.buildings[kind].level + 1);
                        self.ai.persos[i].build = b;
                        if b < 0 {
                            self.seek_raw(rules, i);
                            self.ai_work(rules, i);
                        } else if rules.buildings[b as usize].epoch - 1 > e as i64 {
                            self.ai.persos[i].build = kind as i64;
                            self.seek_raw(rules, i);
                            self.ai_work(rules, i);
                        }
                    }
                    break;
                }
                Step::Patrol => {
                    for _ in 0..3 {
                        self.aggressive(rules, i);
                    }
                    break;
                }
                Step::AttackEnemy => {
                    if self.victory {
                        self.aggressive(rules, i);
                    } else if !p.fighting() && p.attack == p.bonus_max[4] as f64 {
                        self.attack_nearest_enemy(rules, i, false, false, false);
                        if self.persos[i].soldiers > 0 {
                            self.order_charge(rules, i, -1, -1, false, false, false);
                        }
                        if !self.persos[i].fighting() {
                            self.aggressive(rules, i);
                        }
                    }
                    break;
                }
                Step::AttackKeep => {
                    if self.victory {
                        self.aggressive(rules, i);
                    } else if !p.fighting() && p.attack == p.bonus_max[4] as f64 {
                        self.attack_nearest_castle(rules, i, false, false, false);
                        if self.persos[i].soldiers > 0 {
                            self.order_charge(rules, i, -1, -1, false, false, false);
                        }
                        let p = &self.persos[i];
                        if !p.fighting() && p.attack == p.bonus_max[4] as f64 {
                            self.attack_nearest_enemy(rules, i, false, false, false);
                            if self.persos[i].soldiers > 0 {
                                self.order_charge(rules, i, -1, -1, false, false, false);
                            }
                            if !self.persos[i].fighting() {
                                self.aggressive(rules, i);
                            }
                        }
                    }
                    break;
                }
                Step::Mode(m) => {
                    let p = &mut self.persos[i];
                    match m {
                        0 => p.ai_pacific = true,
                        1 => p.ai_pacific = false,
                        2..=5 => {
                            let (money, resources, objects) = match m {
                                2 => (true, true, true),
                                3 => (true, false, false),
                                4 => (false, true, false),
                                _ => (false, false, true),
                            };
                            if self.give(i, money, resources, objects) {
                                return;
                            }
                        }
                        6 => {
                            if self.invoke(rules, i) {
                                return;
                            }
                        }
                        7 => p.ai_protector = true,
                        8 => p.ai_protector = false,
                        9 => p.ai_selfish = true,
                        10 => p.ai_selfish = false,
                        11..=13 => p.ai_equip = m - 11,
                        _ => {}
                    }
                }
                Step::EarnMoney { building, quantity } => {
                    if p.money() < quantity as f64 {
                        if h.visible() && h.kind == building {
                            if trade {
                                self.ai_work(rules, i);
                            } else {
                                self.aggressive(rules, i);
                            }
                        } else {
                            self.choose_house(rules, i, building);
                        }
                        break;
                    }
                }
                Step::EarnMoneyResource { resource: v, quantity } => {
                    if p.money() < quantity as f64 {
                        let at_market =
                            p.in_castle || p.in_own_castle || p.in_house && self.houses[p.house_target].market;
                        if !trade {
                            self.aggressive(rules, i);
                        } else if p.res(v) < p.max_res(v) {
                            self.gather_resource(rules, i, v);
                        } else if at_market {
                            if let Ok(v) = usize::try_from(v) {
                                self.fiefs[nf].sell(rules, v, 0.0, &mut self.persos[i], true);
                            }
                        } else {
                            self.ai_go_castle(i, true);
                        }
                        break;
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- goals

    /// `IA2_ChercherMatieresPremiere`: obtain `q` units of resource `r` by harvesting, buying
    /// in a shop or at the castle, or producing it.
    fn gather(&mut self, rules: &Rules, i: usize, r: i64, q: f64) {
        let Some(res) = usize::try_from(r).ok().and_then(|r| rules.resources.get(r)) else { return };
        if res.harvestable {
            self.gather_resource(rules, i, r);
            let s = &mut self.ai.persos[i];
            s.resource = r;
            s.quantity = clng(q);
            return;
        }
        let p = &self.persos[i];
        if p.money() <= q * res.price {
            self.make_resource(rules, i, r, q);
            return;
        }
        let nf = p.fief;
        if p.action != action::ENTER_HOUSE
            && p.action != action::GO_OWN_CASTLE
            && !(p.in_house || p.in_own_castle || p.in_castle)
        {
            if let Some(x) = self.find_house(rules, i, r, 0, 0) {
                self.ai_go_house(i, x);
                let s = &mut self.ai.persos[i];
                s.resource = r;
                s.quantity = clng(q);
                return;
            }
            // Quirk: the fief's stock of the house's raw material is tested, not that of `r`.
            let m = self.raw_material(rules, self.houses[i].kind);
            let trade = self.castles[nf].visible() || !self.fiefs[nf].markets.is_empty();
            if self.fief_res(nf, m) > q && trade {
                self.ai_go_castle(i, true);
                let s = &mut self.ai.persos[i];
                s.resource = r;
                s.quantity = clng(q);
            } else {
                self.make_resource(rules, i, r, q);
            }
            return;
        }
        let ht = p.house_target;
        let missing = q - p.res(r);
        if p.in_house && self.houses[ht].shop {
            if self.houses[ht].stock() >= 1.0 {
                let _ = self.buy_from_house(rules, ht, i, missing, false);
                self.ai.persos[i].resource = -1;
            } else {
                self.persos[i].in_house = false;
                self.gather(rules, i, r, q);
            }
        } else if (p.in_castle || p.in_own_castle || p.in_house && self.houses[ht].market) && self.fief_res(nf, r) >= q
        {
            self.fiefs[nf].buy(rules, r as usize, missing, &mut self.persos[i], false);
            self.ai.persos[i].resource = -1;
        } else {
            self.make_resource(rules, i, r, q);
        }
    }

    /// `Definir_ressources` of a fief, 0 for a negative index.
    pub(super) fn fief_res(&self, f: usize, r: i64) -> f64 {
        usize::try_from(r).map_or(0.0, |r| self.fiefs[f].resources[r])
    }

    /// `IA2_Fabriquer_Ressource`: walk down the transformation chain to the first missing
    /// material; harvest it, or plan the building that produces it.
    fn make_resource(&mut self, rules: &Rules, i: usize, r: i64, q: f64) {
        let mut r = r as usize;
        // The guard stops a cyclic chain, which would hang the original.
        for _ in 0..rules.resources.len() {
            let raw = rules.resources[r].raw;
            if raw >= 0 && self.persos[i].res(raw) < q {
                r = raw as usize;
            } else {
                break;
            }
        }
        if rules.resources[r].harvestable {
            self.gather_resource(rules, i, r as i64);
        } else if let Some(t) = rules.buildings.iter().position(|b| b.product == r as i64) {
            self.ai.persos[i].build = t as i64;
        }
        let s = &mut self.ai.persos[i];
        s.resource = r as i64;
        s.quantity = clng(q);
    }

    /// `IA2_ChercherObjet`: get `q` objects of type `o`: take or buy them in a factory of the
    /// team, else gather their ingredients, else plan a factory making them.
    fn seek_object(&mut self, rules: &Rules, i: usize, o: i64, q: i64) {
        let Some(obj) = usize::try_from(o).ok().and_then(|o| rules.objects.get(o)) else { return };
        let p = &self.persos[i];
        if p.action == action::ENTER_HOUSE {
            return;
        }
        let mut decided = false;
        if p.money() >= obj.price as f64 {
            let found = (0..self.houses.len()).find_map(|j| {
                let m = &self.houses[j];
                if !(m.factory && self.persos[j].team == p.team) {
                    return None;
                }
                let objects = &rules.buildings[m.kind].objects;
                let k = (0..objects.len()).find(|&k| objects[k] as i64 == o && m.object_stock(k) >= q as f64)?;
                Some((j, k))
            });
            if let Some((j, k)) = found {
                if i == j && p.in_own_house {
                    self.take_from_house(rules, i, k, j, 1);
                    self.ai.persos[i].object = -1;
                } else if p.in_house && p.house_target == j {
                    let _ = self.buy_object(rules, i, k, j);
                    self.ai.persos[i].object = -1;
                } else {
                    self.ai_go_house(i, j);
                }
                decided = true;
            }
        }
        let h = &self.houses[i];
        if !decided && h.factory && h.stock() > 0.0 {
            // Quirk: the last object slot is not examined, and there is no break.
            let objects = &rules.buildings[h.kind].objects;
            for j in 0..h.object_slots().saturating_sub(1) {
                let h = &self.houses[i];
                if h.object_stock(j) > 0.0 && objects.get(j).is_some_and(|&x| x as i64 == o) {
                    self.ai_go_home(i);
                    decided = true;
                }
            }
        }
        if !decided {
            let s = &mut self.ai.persos[i];
            s.object = o;
            s.quantity = q;
            let p = &self.persos[i];
            let missing = (obj.price_resources.iter().enumerate()).find(|&(r, &x)| p.res(r as i64) < x as f64);
            if let Some((r, &x)) = missing {
                self.gather(rules, i, r as i64, x as f64);
                decided = true;
            }
        }
        if !decided
            && let Some(t) =
                (rules.buildings.iter()).position(|b| b.factory && b.objects.iter().any(|&x| x as i64 == o))
        {
            let s = &mut self.ai.persos[i];
            s.build = t as i64;
            s.object = o;
            s.quantity = q;
        }
    }

    /// `IA2_Choisir_Maison`: gather what is needed to work in building type `t`, then plan it.
    fn choose_house(&mut self, rules: &Rules, i: usize, t: usize) {
        self.ai.persos[i].resource = -1;
        let b = &rules.buildings[t];
        let p = &self.persos[i];
        let e = self.fiefs[p.fief].epoch as i64;
        if b.factory {
            let max = rules.start_max_resources as f64;
            let needed = (0..rules.resources.len()).find(|&r| {
                rules.resources[r].epoch <= e
                    && b.objects.iter().any(|&o| rules.objects[o].price_resources[r] > 0 && p.res(r as i64) < max)
            });
            if let Some(r) = needed {
                self.make_resource(rules, i, r as i64, max);
            }
        } else {
            let raw = if b.service != 0 {
                Some(rules.services[b.service].raw)
            } else if b.product >= 0 {
                Some(rules.resources[b.product as usize].raw)
            } else {
                None
            };
            if let Some(j) = raw.filter(|&j| j >= 0) {
                let max = p.max_res(j);
                if p.res(j) < max {
                    self.make_resource(rules, i, j, max);
                }
            }
        }
        let s = &mut self.ai.persos[i];
        if s.resource == -1 {
            s.build = t as i64;
        }
    }

    /// `IA_Changer_Maison`: may the perso leave its house? It first empties the till and the
    /// stock (going home for it if needed).
    pub(super) fn may_change_house(&mut self, rules: &Rules, i: usize) -> bool {
        let h = &self.houses[i];
        // `Not Service` is a bitwise Not on an Integer: always true.
        if !h.visible() || !(h.money > 0.0 || h.stock() > 0.0) {
            return true;
        }
        if !self.persos[i].in_own_house {
            self.ai_go_home(i);
            return false;
        }
        self.take_money(i, i, 0.0, true);
        let h = &self.houses[i];
        if h.shop {
            let _ = self.take_stock(rules, i, i, 0.0, true);
        } else if h.factory {
            for j in 0..rules.buildings[h.kind].objects.len() {
                while self.houses[i].object_stock(j) >= 1.0 && !self.persos[i].inventory_full() {
                    self.take_from_house(rules, i, j, i, 1);
                }
            }
        }
        true
    }

    /// `IA_Donner`. Quirk: the target is held in a Boolean, so only perso 0 ever receives: the
    /// chef of an invocation if it is perso 0, or for the others when the last allied index drawn
    /// (`LPerso + 1` draws) is 0.
    fn give(&mut self, i: usize, money: bool, resources: bool, objects: bool) -> bool {
        let p = &self.persos[i];
        let mut target = true;
        if p.invocation() {
            let c = p.chef as usize;
            let chef = &self.persos[c];
            let (pp, cp) = ((p.x, p.y), (chef.x, chef.y));
            if chef.alive() && dist(cp, pp) <= 1024.0 && (chef.visible() || chef.in_own_house) {
                target = c != 0;
            }
        } else {
            let last = self.ai.last;
            for _ in 0..=last {
                let k = self.rng.below(last) as usize;
                if self.persos[i].team == self.persos[k].team {
                    target = k != 0;
                }
            }
        }
        if target {
            return false;
        }
        let mut gave = false;
        let p = &self.persos[i];
        let room: Vec<bool> =
            (0..p.resources.len()).map(|r| self.persos[0].max_resource(r) - self.persos[0].resource(r) > 1.0).collect();
        let full = self.persos[0].inventory_full();
        let p = &mut self.persos[i];
        if money && p.money() >= 1.0 {
            p.selected_money = p.money();
            gave = true;
        }
        if resources && (0..p.resources.len()).any(|r| p.resource(r) >= 1.0 && room[r]) {
            for r in 0..p.resources.len() {
                p.select_resource(r, 0.0, true, false);
            }
            gave = true;
        }
        if objects && p.inventory_count() >= 1 && !full {
            p.take_first_object();
            gave = true;
        }
        if gave {
            self.go_give(i, 0);
        }
        gave
    }

    /// `IA_Invoquer`: use the first summoning object that works.
    fn invoke(&mut self, rules: &Rules, i: usize) -> bool {
        for j in 0..self.persos[i].inventory.len() {
            if let Some(it) = self.persos[i].inventory[j]
                && rules.objects[it.object].invocation >= 0
            {
                self.equip_object(rules, i, j);
                if self.persos[i].used_object >= 0 {
                    return true;
                }
            }
        }
        false
    }

    // ---------------------------------------------------------------- buildings

    /// `Nombre_Batiments`: houses of type `t` in fief `f`, counting destroyed and never built
    /// ones (whose type stays 0).
    fn count_buildings(&self, t: i64, f: usize) -> i64 {
        if t < 0 {
            return 0;
        }
        (0..self.houses.len()).filter(|&j| self.houses[j].kind as i64 == t && self.persos[j].fief == f).count() as i64
    }

    /// `PeutConstruire_Maison`, false for -1.
    fn can_build_kind(&self, rules: &Rules, i: usize, t: i64) -> bool {
        t >= 0 && self.can_build(rules, i, t as usize)
    }

    /// `Choisir_Batiment`: a building type of level `level`; with -1, of a random higher level
    /// than the current one that perso `n` can afford, else of level 0. A fief with several
    /// houses of the current type takes a missing lower-level one instead ("monopoly").
    pub fn choose_building(&mut self, rules: &Rules, n: usize, f: usize, level: i64) -> i64 {
        self.choose_building_in(rules, n, f, level, false)
    }

    /// [`World::choose_building`]; `fallback` is the level-0 retry for a building of a later
    /// age, which the monopoly rule can make recurse forever in the original (then: none).
    fn choose_building_in(&mut self, rules: &Rules, n: usize, f: usize, level: i64, fallback: bool) -> i64 {
        let own = self.houses[n].kind;
        let mut res = -1;
        let mut level = level;
        if level == -1 {
            level = 0;
            let mut l = self.rng.below(self.ai.level_max) + 1;
            let end = rules.buildings[own].level + 1;
            while l >= end {
                if res == -1 {
                    res = self.choose_building_in(rules, n, f, l, false);
                    if self.can_build_kind(rules, n, res) {
                        l = 0;
                    } else {
                        res = -1;
                    }
                }
                l -= 1;
            }
        }
        let own_level = rules.buildings[own].level;
        if self.count_buildings(own as i64, f) > 1
            && let Some(t) = (0..rules.buildings.len())
                .find(|&t| self.count_buildings(t as i64, f) == 0 && rules.buildings[t].level < own_level)
        {
            res = t as i64;
        }
        if res == -1 {
            let of_level: Vec<usize> =
                (0..rules.buildings.len()).filter(|&t| rules.buildings[t].level == level).collect();
            let k = self.rng.below(of_level.len() as i64);
            res = of_level.get(k as usize).map_or(-1, |&t| t as i64);
        } else if rules.buildings[res as usize].epoch - 1 > self.fiefs[f].epoch as i64
            && self.can_build_kind(rules, n, res)
            && !self.houses[n].visible()
        {
            res = if fallback { -1 } else { self.choose_building_in(rules, n, f, 0, true) };
        }
        res
    }

    /// `Trouver_Emplacement_Maison`: the old site, else up to 11 random tries in a ring around
    /// the castle or a market (or the chef for an invocation) that widens with every failure.
    fn house_site(&mut self, rules: &Rules, n: usize, t: usize) -> Option<(i32, i32)> {
        let nf = self.persos[n].fief;
        let (w, h) = rules.buildings[t].size;
        let (mut x, mut y) = (self.houses[n].x as i64, self.houses[n].y as i64);
        let castle_w = self.castles[nf].w as i64;
        if self.obstacle(rules, n, x as i32, y as i32, w, h) == 0 {
            self.ai.obstacle_distance[nf] = castle_w;
            return Some((x as i32, y as i32));
        }
        let (mw, mh) = (self.terrain.width as i64 * CELL as i64, self.terrain.height as i64 * CELL as i64);
        for _ in 0..=SITE_TRIES {
            self.ai.obstacle_distance[nf] += CELL as i64;
            let d = self.ai.obstacle_distance[nf] as f64;
            let p = &self.persos[n];
            let anchor = if p.invocation() {
                let c = p.chef as usize;
                let (m, q) = (&self.houses[c], &self.persos[c]);
                if m.visible() {
                    (m.x as i64, m.y as i64, m.w as i64, m.h as i64)
                } else {
                    (q.x, q.y, q.w as i64, q.h as i64)
                }
            } else {
                let markets = self.fiefs[nf].markets.len();
                let visible = self.castles[nf].visible();
                let r = self.rng.next();
                if visible && r <= 1.0 / (markets + 1) as f64 || !visible && markets == 0 {
                    let c = &self.castles[nf];
                    (c.x as i64, c.y as i64, c.w as i64, c.h as i64)
                } else {
                    let j = self.fiefs[nf].markets[self.rng.below(markets as i64) as usize];
                    let m = &self.houses[j];
                    (m.x as i64, m.y as i64, m.w as i64, m.h as i64)
                }
            };
            let (ax, ay, aw, ah) = anchor;
            x = clng(ax as f64 + self.rng.next() * (d * 2.0 + aw as f64) - d);
            y = clng(ay as f64 + self.rng.next() * (d * 2.0 + ah as f64) - d);
            if x < 0 {
                x = clng(self.rng.next() * CELL as f64);
            } else if x > mw - w as i64 {
                x = clng((mw - w as i64) as f64 - self.rng.next() * CELL as f64);
            }
            if y < 0 {
                // The original uses the cell width here.
                y = clng(self.rng.next() * CELL as f64);
            } else if y > mh - h as i64 {
                y = clng((mh - h as i64) as f64 - self.rng.next() * CELL as f64);
            }
            if self.obstacle(rules, n, x as i32, y as i32, w, h) == 0 {
                self.ai.obstacle_distance[nf] = castle_w;
                return Some((x as i32, y as i32));
            }
        }
        None
    }
}
