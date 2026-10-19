//! World-level economy: houses, work, trade, treasures and decors (`ClsJeuBatiment`,
//! `ClsJeuPerso.Travailler` and its helpers, `ModTresors`, `ModDecors`, `ModIA.Obstacle`).
//!
//! Functions return `Err(())` where the original raises a runtime error; inside
//! `Lire_Commandes` that drops the rest of the command queue.

use crate::building::{LifeChange, RECYCLING, START_LIFE};
use crate::event::NumberKind;
use crate::perso::action;
use crate::rules::Rules;
use crate::stats::{BUILD_SPEED, CRAFT_SPEED, Trained, XP_CRAFT, XP_OBJECT, XP_SPECIAL};
use crate::terrain::CELL;
use crate::treasure::{Decor, TREASURE_SIZE, Treasure};
use crate::vb::clng;
use crate::world::{RESOURCE_SIZE, World, overlaps};

/// `VitesseAction`: work speeds are divided by it.
const ACTION_SPEED: f64 = 400.0;
/// `CoefExpCompConstruction`
const XP_BUILD: f64 = 0.00075;

pub type Res = Result<(), ()>;

impl World {
    // ---------------------------------------------------------------- orders

    /// The end of most orders: the soldiers follow (`Ordonner_Chargez` with the chef's own
    /// target, or back into formation), only while alive for the orders that check it.
    fn soldiers_follow(&mut self, rules: &Rules, i: usize, check_alive: bool) {
        let p = &self.persos[i];
        if p.soldiers > 0 && (p.alive() || !check_alive) {
            self.order_charge(rules, i, -1, -1, false, false, false);
        }
    }

    /// `Aller_A`: walk so that the top-left corner reaches (x, y).
    pub fn walk_to(&mut self, rules: &Rules, i: usize, x: i64, y: i64) {
        self.persos[i].go_to(x, y);
        self.soldiers_follow(rules, i, true);
    }

    /// `Rentrer_Maison`: walk to house `h` to live or work in it (or to build it).
    pub fn go_home(&mut self, rules: &Rules, i: usize, h: usize, enter: bool) {
        let (p, m) = (&mut self.persos[i], &self.houses[h]);
        if m.visible() {
            let (x, y) = m.entrance_for(p.w, p.h);
            p.head_to(x, y);
            p.action = if m.complete() || !p.alive() || enter { action::GO_HOME } else { action::GO_BUILD };
            p.house_target = m.owner;
            p.set_visible(true);
        }
        self.soldiers_follow(rules, i, true);
    }

    /// `Aller_Maison`: walk to someone's house `h` to enter it (or to help build it).
    pub fn go_to_house(&mut self, rules: &Rules, i: usize, h: usize, repair: bool) {
        let (p, m) = (&mut self.persos[i], &self.houses[h]);
        if m.visible() {
            let (x, y) = m.entrance_for(p.w, p.h);
            p.head_to(x, y);
            p.action = if m.complete() || !p.alive() || !repair { action::ENTER_HOUSE } else { action::GO_BUILD };
            p.house_target = m.owner;
            p.set_visible(true);
        }
        self.soldiers_follow(rules, i, true);
    }

    /// `Rentrer_Chateau`: back to the own castle; refused when hostile.
    pub fn go_own_castle(&mut self, rules: &Rules, i: usize) {
        let p = &mut self.persos[i];
        if p.hostile() {
            return;
        }
        let c = &self.castles[p.fief];
        if c.visible() {
            let (x, y) = c.entrance_for(p.w, p.h);
            p.head_to(x, y);
            p.action = action::GO_OWN_CASTLE;
            p.set_visible(true);
        }
        self.soldiers_follow(rules, i, true);
    }

    /// `Aller_Chateau`: to an allied castle.
    pub fn go_to_castle(&mut self, rules: &Rules, i: usize, c: usize) {
        let (p, castle) = (&mut self.persos[i], &self.castles[c]);
        if castle.visible() {
            let (x, y) = castle.entrance_for(p.w, p.h);
            p.head_to(x, y);
            p.action = action::GO_CASTLE;
            p.castle_target = c;
            p.set_visible(true);
        }
        self.soldiers_follow(rules, i, false);
    }

    /// `Aller_Ressource`: walk to resource node `k` to harvest it.
    pub fn go_harvest(&mut self, rules: &Rules, i: usize, k: usize) {
        let (p, n) = (&mut self.persos[i], &self.resources[k]);
        // The original uses the node width for both axes.
        let half = RESOURCE_SIZE as f64 / 2.0;
        p.head_to(clng(n.x as f64 + half - p.w as f64 / 2.0), clng(n.y as f64 + half - p.h as f64 / 2.0));
        p.resource_target = k;
        p.action = action::GO_HARVEST;
        p.set_visible(true);
        self.soldiers_follow(rules, i, false);
    }

    /// `Aller_Tresor`
    pub fn go_to_treasure(&mut self, rules: &Rules, i: usize, k: usize) {
        let (p, t) = (&mut self.persos[i], &self.treasures[k]);
        let half = TREASURE_SIZE as f64 / 2.0;
        p.head_to(clng(t.x as f64 + half - p.w as f64 / 2.0), clng(t.y as f64 + half - p.h as f64 / 2.0));
        p.treasure_target = k;
        p.action = action::GO_TREASURE;
        p.set_visible(true);
        self.soldiers_follow(rules, i, false);
    }

    /// `Aller_Vendre`: sell the selection at castle `k` (or at market house `k`), walking there
    /// first unless already inside.
    pub fn go_sell(&mut self, rules: &Rules, i: usize, k: usize, market: bool) -> Res {
        let p = &self.persos[i];
        if market {
            let f = self.persos.get(k).ok_or(())?.fief;
            if p.in_house && p.house_target == k {
                self.sell_selection(rules, f, i);
            } else {
                self.houses.get(k).ok_or(())?;
                self.go_to_house(rules, i, k, false);
                self.persos[i].action = action::GO_SELL_MARKET;
                self.persos[i].set_visible(true);
            }
        } else {
            self.castles.get(k).ok_or(())?;
            let inside = if p.fief == k { p.in_own_castle } else { p.in_castle && p.castle_target == k };
            if inside {
                self.sell_selection(rules, k, i);
            } else {
                if p.fief == k {
                    self.go_own_castle(rules, i);
                } else {
                    self.go_to_castle(rules, i, k);
                }
                self.persos[i].action = action::GO_SELL_CASTLE;
                self.persos[i].set_visible(true);
            }
        }
        Ok(())
    }

    /// `IA_Trouver_Ressource`: the nearest node of terrain resource type `kind` (node 0 if none).
    pub fn find_resource(&mut self, rules: &Rules, i: usize, kind: usize) {
        let p = &self.persos[i];
        let mut dist = (self.terrain.width as i64 + self.terrain.height as i64) * CELL as i64;
        let mut best = 0;
        for (j, n) in self.resources.iter().enumerate() {
            let d = ((p.x - n.x as i64) as f64).hypot((p.y - n.y as i64) as f64);
            if n.kind == kind && dist as f64 > d {
                dist = clng(d);
                best = j;
            }
        }
        if !self.resources.is_empty() {
            self.go_harvest(rules, i, best);
        }
    }

    /// `IA_Trouver_Tresor`: the nearest treasure.
    pub fn find_treasure(&mut self, rules: &Rules, i: usize) -> Res {
        if self.treasures.is_empty() {
            return Ok(());
        }
        let p = &self.persos[i];
        let mut dist = (self.terrain.width as i64 + self.terrain.height as i64) * CELL as i64;
        let mut best = None;
        for (j, t) in self.treasures.iter().enumerate() {
            let d = ((p.x - t.x as i64) as f64).hypot((p.y - t.y as i64) as f64);
            if dist as f64 > d {
                dist = clng(d);
                best = Some(j);
            }
        }
        self.go_to_treasure(rules, i, best.ok_or(())?);
        Ok(())
    }

    /// `Creer_Objet`: start (or resume) crafting slot `slot` of the house's object list.
    pub fn start_craft(&mut self, i: usize, slot: i64) {
        let p = &mut self.persos[i];
        if slot != p.craft_slot {
            p.craft_slot = slot;
            p.craft_progress = 0.0;
        }
        p.action = action::CRAFT;
    }

    // ---------------------------------------------------------------- houses

    /// `PeutConstruire_Maison`: enough money and resources, and the emperor for his buildings.
    pub fn can_build(&self, rules: &Rules, i: usize, kind: usize) -> bool {
        let (p, t) = (&self.persos[i], &rules.buildings[kind]);
        p.money() >= t.price_money as f64
            && t.price_resources.iter().enumerate().all(|(r, &q)| p.resource(r) >= q as f64)
            && (!t.emperor || p.emperor)
    }

    /// [`World::can_build`] counting what demolishing perso `i`'s finished house gives back,
    /// since building a new one demolishes it first. For the player's buttons: the original
    /// only checks what the perso holds, so a building could need a Del first.
    pub fn can_build_replacing(&self, rules: &Rules, i: usize, kind: usize) -> bool {
        let (p, t, m) = (&self.persos[i], &rules.buildings[kind], &self.houses[i]);
        let (mut money, mut resources): (f64, Vec<f64>) = (p.money(), (0..t.price_resources.len()).map(|r| p.resource(r)).collect());
        if m.built && m.max_life > 0 {
            let old = &rules.buildings[m.kind];
            let share = |q: f64| q * RECYCLING * m.life / m.max_life as f64;
            money += share(old.price_money as f64);
            for (r, have) in resources.iter_mut().enumerate() {
                let q = old.price_resources.get(r).copied().unwrap_or(0);
                if q > 0 {
                    *have = (*have + share(q as f64)).min(p.max_resource(r));
                }
            }
        }
        money >= t.price_money as f64
            && t.price_resources.iter().zip(&resources).all(|(&q, &have)| have >= q as f64)
            && (!t.emperor || p.emperor)
    }

    /// `ClsJeuBatiment.Creer`: perso `owner` starts building a house of type `kind` at (x, y)
    /// as house `h`, demolishing the previous one, and pays for it. Affordability is not checked.
    /// For a type with `CreerProprietaire` (guard towers, caravans), a new summon of that race
    /// builds and owns it instead.
    pub fn create_house(&mut self, rules: &Rules, h: usize, x: i32, y: i32, kind: usize, owner: usize) {
        let t = &rules.buildings[kind];
        if let Ok(race) = usize::try_from(t.owner_race)
            && !self.persos[owner].invocation()
        {
            let k = self.create_invocation(rules, race, 1.0, h, -1, false, 0, "");
            self.create_house(rules, k, x, y, kind, k);
        } else {
            if self.houses[h].built {
                self.demolish(rules, h, owner, true);
            }
            let p = &self.persos[owner];
            let m = &mut self.houses[h];
            m.x = x;
            m.y = y;
            m.set_kind(rules, kind);
            self.spatial.houses.moved_to(h, x as i64, y as i64, 0);
            m.shoot = t.shoot;
            m.built = false;
            let base = rules.start_special as f64;
            m.max_life = clng(((p.special[0].carac as f64 - base) / base * 0.5 + 1.0) * t.max_life as f64);
            m.life = m.max_life as f64 * START_LIFE;
            m.money = 0.0;
            m.set_stock_raw(0.0);
            m.max_stock = (t.max_stock * p.special[0].bonus as f64 / base).floor();
            m.inventory.iter_mut().for_each(|o| *o = -1);
            let p = &mut self.persos[owner];
            p.craft_progress = 0.0;
            p.set_visible(true);
            self.go_home(rules, owner, h, false);
        }
        let p = &mut self.persos[owner];
        if !p.invocation() {
            p.set_money(p.money() - t.price_money as f64);
            for (r, &q) in t.price_resources.iter().enumerate() {
                p.set_resource(r, p.resource(r) - q as f64);
            }
        }
    }

    /// `Definir_Vie` setter of a house, with its side effects on markets and objects.
    pub fn set_house_life(&mut self, rules: &Rules, h: usize, v: f64) {
        let change = self.houses[h].set_life(v);
        let (m, t) = (&self.houses[h], &rules.buildings[self.houses[h].kind]);
        let owner = m.owner;
        match change {
            LifeChange::None => {}
            LifeChange::Built => {
                if m.market {
                    self.markets_add(self.persos[owner].fief, h);
                }
                if t.gain_object >= 0 {
                    self.persos[owner].add_new_object(rules, t.gain_object as usize);
                }
            }
            LifeChange::Destroyed { was_built } => {
                if was_built && m.market {
                    self.markets_remove(self.persos[owner].fief, h);
                }
                if t.loss_object >= 0 {
                    self.persos[owner].remove_object(rules, t.loss_object as usize);
                }
            }
        }
    }

    /// `Marches_Ajouter`: adds market `h` to fief `f`, once per fief of the same team.
    fn markets_add(&mut self, f: usize, h: usize) {
        let team = self.fiefs[f].team;
        let n = self.fiefs.iter().filter(|g| g.team == team).count();
        self.fiefs[f].markets.extend(std::iter::repeat_n(h, n));
    }

    /// `Marches_Supprimer`: removes one occurrence of `h` from every fief of the same team.
    fn markets_remove(&mut self, f: usize, h: usize) {
        let team = self.fiefs[f].team;
        for g in self.fiefs.iter_mut().filter(|g| g.team == team) {
            if let Some(k) = g.markets.iter().position(|&m| m == h) {
                g.markets.remove(k);
            }
        }
    }

    /// `Marches_Actualiser`
    pub fn markets_refresh(&mut self, f: usize) {
        for h in 0..self.houses.len() {
            if self.houses[h].market && self.houses[h].built && self.persos[h].team == self.fiefs[f].team {
                self.markets_add(f, h);
            }
        }
    }

    /// `Demolir`: house `h` is demolished by perso `i`, refunding half its cost (scaled by its
    /// HP) and handing over its contents.
    pub fn demolish(&mut self, rules: &Rules, h: usize, i: usize, main: bool) {
        let p = &self.persos[i];
        let ht = p.house_target;
        if p.in_house && self.persos.get(ht).is_some_and(|o| o.chef == i as i64) && !main {
            // The house of a summon: it goes, and the summon dies.
            self.demolish(rules, ht, ht, false);
            if self.persos[ht].alive() {
                self.kill_loot(rules, i, ht, false);
                self.kill(rules, ht);
            }
        } else if self.houses[h].max_life > 0 {
            let (m, t) = (&self.houses[h], &rules.buildings[self.houses[h].kind]);
            let share = |q: f64| q * RECYCLING * m.life / m.max_life as f64;
            let money = share(t.price_money as f64);
            let refunds: Vec<(usize, f64)> = (t.price_resources.iter().enumerate())
                .filter(|(_, q)| **q > 0)
                .map(|(r, &q)| (r, share(q as f64)))
                .collect();
            let p = &mut self.persos[i];
            p.set_money(p.money() + money);
            for (r, q) in refunds {
                p.set_resource(r, p.resource(r) + q);
            }
            if p.in_own_house {
                p.in_own_house = false;
                p.action = action::IDLE;
            }
            self.empty_house(rules, i, h);
            self.set_house_life(rules, h, 0.0);
        }
        self.houses[h].clear_object_stock();
    }

    /// `Perso_Demolir_Batiment`: the till, stock and objects of house `h` go to perso `i` when
    /// it stands on it, else into a new treasure. The house keeps its own copy.
    fn empty_house(&mut self, rules: &Rules, i: usize, h: usize) {
        let (p, m) = (&self.persos[i], &self.houses[h]);
        let on_it = m.x + m.w >= p.x as i32
            && m.y + m.h >= p.y as i32
            && m.x <= p.x as i32 + p.w
            && m.y <= p.y as i32 + p.h;
        let t = if on_it {
            None
        } else {
            // The original uses the width for Y too.
            let half = TREASURE_SIZE as f64 / 2.0;
            let x = clng(m.x as f64 + m.w as f64 / 2.0 - half) as i32;
            let y = clng(m.y as f64 + m.w as f64 / 2.0 - half) as i32;
            Some(self.treasure_create(rules, x, y))
        };
        let (p, m) = (&mut self.persos[i], &self.houses[h]);
        match t {
            None => p.set_money(p.money() + m.money),
            Some(t) => {
                let tr = &mut self.treasures[t];
                tr.set_money(rules, tr.money() + m.money);
            }
        }
        let product = rules.buildings[m.kind].product;
        if m.shop && product > 0 && m.stock() > 0.0 {
            let r = product as usize;
            match t {
                None => p.set_resource(r, p.resource(r) + m.stock()),
                Some(t) => {
                    let tr = &mut self.treasures[t];
                    tr.set_resource(rules, r, tr.resource(r) + m.stock());
                }
            }
        }
        if m.factory && m.stock() > 0.0 {
            for slot in 0..m.object_slots() {
                let n = clng(self.houses[h].object_stock(slot) - 1.0);
                for _ in 0..=n {
                    match t {
                        None => self.take_from_house(rules, i, slot, h, 1),
                        Some(t) => {
                            let o = rules.buildings[self.houses[h].kind].objects[slot];
                            self.treasures[t].add_object(rules, o);
                        }
                    }
                }
            }
        }
        for k in 0..self.houses[h].inventory.len() {
            let o = self.houses[h].inventory[k];
            if o > -1 {
                match t {
                    None => self.persos[i].add_new_object(rules, o as usize),
                    Some(t) => self.treasures[t].add_object(rules, o as usize),
                }
            }
        }
        if let Some(t) = t {
            if self.treasures[t].empty() {
                self.treasures.remove(t);
            }
        }
    }

    // ---------------------------------------------------------------- work

    /// `Travailler`: what a character that is not walking does where it stands. Returns the
    /// comment type it may say (`NoCommentaire`).
    pub(crate) fn work(&mut self, rules: &Rules, i: usize) -> Option<i32> {
        let mut msg = None;
        let p = &self.persos[i];
        if p.moving {
            return None;
        }
        let (ht, ct, fief) = (p.house_target, p.castle_target, p.fief);
        let house_visible = self.houses[ht].visible();
        let house_built = self.houses[ht].built;
        if p.alive() {
            match p.action {
                action::IDLE => {
                    let p = &mut self.persos[i];
                    p.current_resource = -1;
                    if !self.castles[fief].visible() {
                        p.in_own_castle = false;
                    }
                    if !self.castles[ct].visible() {
                        p.in_castle = false;
                    }
                    if !(house_visible && house_built) {
                        p.in_house = false;
                    }
                }
                action::HARVEST => {
                    let node = self.resources.get(p.resource_target);
                    let r = node.map(|n| rules.terrain_resources[n.kind].resource).unwrap_or(-1);
                    self.persos[i].current_resource = r;
                    if r >= 0 {
                        self.harvest(rules, i, r as usize, i, &mut msg);
                    }
                    self.persos[i].in_house = false;
                }
                action::BUILD => {
                    if house_visible {
                        self.persos[i].current_resource = -1;
                        self.houses[ht].under_construction = true;
                        self.build(rules, i, ht);
                    } else {
                        self.stop(rules, i);
                    }
                }
                action::AT_HOME => {
                    let p = &mut self.persos[i];
                    p.in_own_house = house_visible;
                    if house_visible {
                        p.current_resource = -1;
                    }
                }
                action::GO_HOME | action::WORK_HOME | action::SERVICE => {
                    if house_visible {
                        let product = rules.buildings[self.houses[ht].kind].product;
                        let p = &mut self.persos[i];
                        p.in_own_house = true;
                        p.current_resource = product;
                        if product >= 0 {
                            p.action = action::WORK_HOME;
                            self.harvest(rules, i, product as usize, ht, &mut msg);
                        } else if self.houses[ht].service != 0 {
                            self.prepare_service(rules, i, ht, &mut msg);
                        } else {
                            self.stop(rules, i);
                        }
                    } else {
                        self.persos[i].in_own_house = false;
                        self.stop(rules, i);
                    }
                }
                action::GO_TREASURE => {
                    let k = p.treasure_target;
                    if k < self.treasures.len() {
                        self.treasure_pickup(rules, i, k);
                    }
                    // The original crashes on a vanished treasure index; just stop.
                    self.persos[i].in_own_house = false;
                    self.stop(rules, i);
                }
                action::CRAFT => {
                    if house_visible {
                        self.craft(rules, i, &mut msg);
                    } else {
                        self.persos[i].in_own_house = false;
                        self.stop(rules, i);
                    }
                }
                action::ENTER_HOUSE => {
                    self.persos[i].in_house = house_visible && house_built;
                    self.stop(rules, i);
                }
                action::GO_OWN_CASTLE => {
                    let p = &mut self.persos[i];
                    p.in_own_castle = self.castles[fief].visible();
                    p.action = action::IDLE;
                }
                action::GO_CASTLE => {
                    let p = &mut self.persos[i];
                    p.in_castle = self.castles[ct].visible();
                    p.action = action::IDLE;
                }
                action::GO_SELL_CASTLE => {
                    let p = &mut self.persos[i];
                    // Castles' `Numero` is always 0, so only fief 0 enters its own castle here.
                    if self.castles[ct].owner == fief {
                        p.in_own_castle = self.castles[fief].visible();
                    } else {
                        p.in_castle = self.castles[ct].visible();
                    }
                    self.sell_selection(rules, ct, i);
                    self.persos[i].action = action::IDLE;
                }
                action::GO_SELL_MARKET => {
                    let p = &mut self.persos[i];
                    if self.houses[ht].owner == i {
                        // The original reads `Chateaux(Numero)`, out of range for most persos.
                        p.in_own_house = house_visible;
                    } else {
                        p.in_house = house_visible;
                    }
                    let f = self.persos[ht].fief;
                    self.sell_selection(rules, f, i);
                    self.persos[i].action = action::IDLE;
                }
                _ => self.persos[i].current_resource = -1,
            }
        } else {
            let p = &mut self.persos[i];
            match p.action {
                action::IDLE => {
                    if !self.castles[fief].visible() {
                        p.in_own_castle = false;
                    }
                    if !self.castles[ct].visible() {
                        p.in_castle = false;
                    }
                    if !(house_visible && house_built) {
                        p.in_house = false;
                    }
                }
                action::AT_HOME | action::GO_HOME | action::WORK_HOME => {
                    p.in_own_house = house_visible;
                    self.stop(rules, i);
                }
                action::ENTER_HOUSE => {
                    p.in_house = house_visible && house_built;
                    self.stop(rules, i);
                }
                action::GO_OWN_CASTLE => {
                    p.in_own_castle = self.castles[fief].visible();
                    p.action = action::IDLE;
                }
                _ => {}
            }
        }
        msg
    }

    /// `Recolter_Ressources`: one tick of harvesting or producing resource `r`, overflowing
    /// into the stock of house `h` when at home.
    fn harvest(&mut self, rules: &Rules, i: usize, r: usize, h: usize, msg: &mut Option<i32>) {
        let res = &rules.resources[r];
        let (p, m) = (&mut self.persos[i], &mut self.houses[h]);
        let base = rules.start_max_resources as f64;
        let mut speed = 1.0 / ACTION_SPEED * p.vitality * (p.harvest_level(r) as f64 + base);
        let gain = res.speed * speed;
        let cost = res.speed * speed * res.raw_quantity;
        let raw = usize::try_from(res.raw).ok();
        if p.resource(r) < p.max_resource(r) {
            match raw {
                Some(mp) if p.resource(mp) >= cost => {
                    p.set_resource(mp, p.resource(mp) - cost);
                    p.set_resource(r, p.resource(r) + gain);
                }
                Some(_) => {
                    p.action = action::AT_HOME;
                    *msg = Some(1);
                }
                None => p.set_resource(r, p.resource(r) + gain),
            }
            if p.resource(r) == p.max_resource(r) && p.ai {
                self.stop(rules, i);
            }
        } else if p.in_own_house && m.stock() < m.max_stock {
            match raw {
                Some(mp) if p.resource(mp) >= cost => {
                    p.set_resource(mp, p.resource(mp) - cost);
                    m.set_stock(m.stock() + gain);
                }
                Some(_) => {
                    p.action = action::AT_HOME;
                    *msg = Some(1);
                }
                None => m.set_stock(m.stock() + gain),
            }
        } else {
            self.stop(rules, i);
            *msg = Some(0);
        }
        let p = &mut self.persos[i];
        if p.action != action::IDLE && p.action != action::AT_HOME {
            if p.resource_boosted(rules, r) {
                let s = &p.resource_skills[r];
                speed = speed * (s.xp.level as f64 + base) / (s.bonus as f64 + base);
                p.train_objects(rules, |o| o.bonus_resource[r] > 0, speed);
            }
            p.train(rules, Trained::Resource(r), speed);
        }
    }

    /// `Preparer_Service`: one tick of preparing the service of house `h` into its stock.
    fn prepare_service(&mut self, rules: &Rules, i: usize, h: usize, msg: &mut Option<i32>) {
        let (p, m) = (&mut self.persos[i], &mut self.houses[h]);
        let s = m.service;
        let srv = &rules.services[s];
        p.current_service = s;
        let speed = 1.0 / ACTION_SPEED * p.vitality * p.services[s - 1].bonus as f64;
        if m.stock() < m.max_stock {
            match usize::try_from(srv.raw).ok() {
                Some(mp) if p.resource(mp) >= srv.speed * speed * srv.raw_quantity => {
                    p.set_resource(mp, p.resource(mp) - srv.speed * speed * srv.raw_quantity);
                    m.set_stock(m.stock() + srv.speed * speed);
                }
                Some(_) => {
                    p.action = action::AT_HOME;
                    *msg = Some(1);
                }
                None => m.set_stock(m.stock() + srv.speed * speed),
            }
            // Set even when the raw material is missing.
            p.action = action::SERVICE;
        } else {
            self.stop(rules, i);
            *msg = Some(0);
        }
        let p = &mut self.persos[i];
        if p.action != action::IDLE && p.action != action::AT_HOME {
            let gain = srv.speed * speed * srv.xp_coef;
            p.train(rules, Trained::Service(s - 1), gain);
            if p.service_boosted(rules, s - 1) {
                // Quirk: the objects are checked on the next service's skill (`[s]`, not `[s-1]`);
                // the original fails on the last service.
                p.train_objects(rules, |o| o.bonus_service.get(s).is_some_and(|&b| b > 0), gain);
            }
        }
    }

    /// `Batir_Maison`: one tick of building or repairing house `h`.
    fn build(&mut self, rules: &Rules, i: usize, h: usize) {
        let kind = self.houses[h].kind;
        let t = &rules.buildings[kind];
        let p = &self.persos[i];
        let base = rules.start_special as f64;
        let v = BUILD_SPEED * p.vitality * (p.special[0].bonus as f64 + base) / base * t.build_speed;
        self.set_house_life(rules, h, self.houses[h].life_with_repair() + v);
        let p = &mut self.persos[i];
        p.train(rules, Trained::Special(0), v * XP_BUILD * XP_SPECIAL);
        if p.special_boosted(rules, 0) {
            p.train_objects(rules, |o| o.bonus_special[0] > 0, v * XP_BUILD * XP_SPECIAL);
        }
        if self.houses[h].complete() {
            if self.houses[h].owner == i {
                self.persos[i].action = action::GO_HOME;
            } else {
                self.persos[i].in_house = true;
                self.stop(rules, i);
            }
            let f = self.persos[i].fief;
            if t.epoch > self.fiefs[f].epoch as i64 {
                self.fiefs[f].change_epoch(rules, &mut self.castles[f], self.epoch_speed);
            }
        } else {
            self.persos[i].action = action::BUILD;
        }
    }

    /// `PeutConstruire_Objet`: room in house `h` and all the resources for slot `slot`.
    pub fn can_craft(&self, rules: &Rules, i: usize, slot: i64, h: usize) -> bool {
        let m = &self.houses[h];
        let Some(&o) = usize::try_from(slot).ok().and_then(|s| rules.buildings[m.kind].objects.get(s)) else {
            return false;
        };
        let p = &self.persos[i];
        m.stock() <= m.max_stock - 1.0
            && rules.objects[o].price_resources.iter().enumerate().all(|(r, &q)| p.resource(r) >= q as f64)
    }

    /// `Fabriquer_Objet`: one tick of crafting in the own house; resources are paid at the end.
    fn craft(&mut self, rules: &Rules, i: usize, msg: &mut Option<i32>) {
        let slot = self.persos[i].craft_slot;
        if !self.can_craft(rules, i, slot, i) {
            self.stop(rules, i);
            return;
        }
        let (p, m) = (&mut self.persos[i], &mut self.houses[i]);
        let o = rules.buildings[m.kind].objects[slot as usize];
        let obj = &rules.objects[o];
        let c = obj.skill;
        let mut v = CRAFT_SPEED * p.vitality * p.objects[c].bonus as f64;
        p.craft_progress += v;
        v = v * XP_OBJECT * XP_CRAFT * obj.craft_xp * p.objects[c].carac as f64 / p.objects[c].bonus as f64;
        if p.object_boosted(rules, c) {
            p.train_objects(rules, |o| o.bonus_object[c] > 0, v);
        }
        p.train(rules, Trained::Object(c), v);
        if p.craft_progress >= obj.craft_time as f64 {
            p.craft_progress = 0.0;
            let s = slot as usize;
            m.set_object_stock(s, m.object_stock(s) + 1.0);
            *msg = Some(0);
            p.action = action::AT_HOME;
            for (r, &q) in obj.price_resources.iter().enumerate() {
                p.set_resource(r, p.resource(r) - q as f64);
            }
        } else {
            p.action = action::CRAFT;
        }
    }

    // ---------------------------------------------------------------- trade

    /// `ClsJeuBatiment.Acheter`: perso `i` buys the product of shop `h` at the bargained price.
    pub fn buy_from_house(&mut self, rules: &Rules, h: usize, i: usize, q: f64, all: bool) -> Res {
        let m = self.houses.get_mut(h).ok_or(())?;
        let r = usize::try_from(rules.buildings[m.kind].product).map_err(|_| ())?;
        let p = &mut self.persos[i];
        let unit = p.bargained(rules, rules.resources[r].price * rules.buy_coef) as f64;
        let mut q = q;
        if all || p.money() < q * unit {
            q = p.money() / unit;
        }
        if q > m.stock() {
            q = m.stock();
        }
        let room = p.max_resource(r) - p.resource(r).floor();
        if q > room {
            q = room;
        }
        let q = q.floor();
        p.set_resource(r, p.resource(r) + q);
        m.set_stock_raw(m.stock() - q);
        let price = clng(q * unit) as f64;
        p.set_money(p.money() - price);
        m.money += price;
        p.train_special(rules, 3, price);
        let (x, y) = (m.x as f64 + m.w as f64 / 2.0, m.y as f64);
        self.number(x, y, price, NumberKind::Money, false);
        Ok(())
    }

    /// `Transfert_Stock_Prendre`: the owner takes products from the house stock.
    pub fn take_stock(&mut self, rules: &Rules, h: usize, i: usize, q: f64, all: bool) -> Res {
        let (m, p) = (&mut self.houses[h], &mut self.persos[i]);
        let r = usize::try_from(rules.buildings[m.kind].product).map_err(|_| ())?;
        let mut q = if all || q > m.stock() { m.stock() } else { q };
        let room = p.max_resource(r) - p.resource(r).floor();
        if q > room {
            q = room;
        }
        let q = q.floor();
        p.set_resource(r, p.resource(r) + q);
        m.set_stock_raw(m.stock() - q);
        Ok(())
    }

    /// `Transfert_Stock_Deposer`: the owner stores products in the house stock.
    pub fn deposit_stock(&mut self, rules: &Rules, h: usize, i: usize, q: f64, all: bool) -> Res {
        let (m, p) = (&mut self.houses[h], &mut self.persos[i]);
        let r = usize::try_from(rules.buildings[m.kind].product).map_err(|_| ())?;
        let mut q = if all || q > p.resource(r) { p.resource(r) } else { q };
        if q > m.max_stock - m.stock() {
            q = m.max_stock - m.stock();
        }
        let q = q.floor();
        p.set_resource(r, p.resource(r) - q);
        m.set_stock_raw(m.stock() + q);
        Ok(())
    }

    /// `Transfert_Argent_Prendre`: the owner takes money from the till.
    pub fn take_money(&mut self, h: usize, i: usize, q: f64, all: bool) {
        let (m, p) = (&mut self.houses[h], &mut self.persos[i]);
        let q = if all || q > m.money { m.money } else { q };
        p.set_money(p.money() + q);
        m.money -= q;
    }

    /// `Transfert_Argent_Deposer`
    pub fn deposit_money(&mut self, h: usize, i: usize, q: f64, all: bool) {
        let (m, p) = (&mut self.houses[h], &mut self.persos[i]);
        let q = if all || q > p.money() { p.money() } else { q };
        p.set_money(p.money() - q);
        m.money += q;
    }

    /// `Service_Utilisable`: the temple does not resurrect the living; the client pays unless
    /// it owns the house; there must be a prepared service.
    pub fn can_use_service(&self, rules: &Rules, h: usize, i: usize) -> bool {
        let (m, p) = (&self.houses[h], &self.persos[i]);
        !(m.service == 1 && p.alive())
            && (p.money() >= p.bargained(rules, rules.services[m.service].price) as f64 || i == m.owner)
            && m.stock() >= 1.0
    }

    /// `Service_Utiliser`: perso `i` uses the service of house `h` (not checked).
    pub fn use_service(&mut self, rules: &Rules, h: usize, i: usize) -> Res {
        self.use_service_score(rules, h, i, 0.0, false)
    }

    /// `Service_Utiliser` with the bounty `score` of a kill (and its XP with `experience`).
    pub(crate) fn use_service_score(&mut self, rules: &Rules, h: usize, i: usize, score: f64, experience: bool) -> Res {
        let s = self.houses.get(h).ok_or(())?.service;
        let m = &self.houses[h];
        let (x, y) = (m.x as f64 + m.w as f64 / 2.0, m.y as f64);
        match s {
            1 => {
                self.revive(rules, i);
                let p = &mut self.persos[i];
                p.bars[0] = rules.services[s].coef * p.bonus_max[0] as f64;
                let m = &mut self.houses[h];
                m.set_stock_raw(m.stock() - 1.0);
            }
            2 => {
                self.houses[h].money += score;
                if experience {
                    self.persos[i].train_bounty(rules, score);
                }
                self.number(x, y, score, NumberKind::Money, false);
            }
            _ => {}
        }
        if i != self.houses[h].owner {
            let p = &mut self.persos[i];
            let price = p.bargained(rules, rules.services[s].price) as f64;
            p.set_money(p.money() - price);
            self.houses[h].money += price;
            self.number(x, y, price, NumberKind::Money, false);
            self.persos[i].train_special(rules, 3, price);
        }
        let m = &self.houses[h];
        let (w, hh) = (m.w as f64, m.h as f64);
        self.effect(rules.services[s].effect, m.x as f64, m.y as f64, w, hh);
        Ok(())
    }

    /// `Objet_Acheter_Maison`: buys one crafted object from factory `h` (paid even if the
    /// object cannot be taken).
    pub fn buy_object(&mut self, rules: &Rules, i: usize, slot: usize, h: usize) -> Res {
        let m = self.houses.get(h).ok_or(())?;
        let &o = rules.buildings[m.kind].objects.get(slot).ok_or(())?;
        let p = &mut self.persos[i];
        let price = p.bargained(rules, rules.objects[o].price as f64 * rules.buy_coef) as f64;
        if p.money >= price {
            p.money -= price;
            self.houses[h].money += price;
            self.take_from_house(rules, i, slot, h, 1);
            let m = &self.houses[h];
            let (x, y) = (m.x as f64 + m.w as f64 / 2.0, m.y as f64);
            self.number(x, y, price, NumberKind::Money, false);
            self.persos[i].train_special(rules, 3, price);
        }
        Ok(())
    }

    /// `Objet_Ramasser_Maison`: takes up to `n` crafted objects of `slot` from house `h`.
    pub fn take_from_house(&mut self, rules: &Rules, i: usize, slot: usize, h: usize, n: i64) {
        for _ in 0..n {
            let (p, m) = (&mut self.persos[i], &mut self.houses[h]);
            if m.object_stock(slot) >= 1.0 && !p.inventory_full() {
                let Some(&o) = rules.buildings[m.kind].objects.get(slot) else { return };
                p.add_new_object(rules, o);
                m.set_object_stock(slot, m.object_stock(slot) - 1.0);
            } else {
                return;
            }
        }
    }

    /// `Objet_Deposer_Maison`: puts an object (inventory slot `n`, or object type `n`) into
    /// house `h`: as stock when the house crafts it, else in its inventory.
    pub fn deposit_object(&mut self, rules: &Rules, i: usize, n: usize, h: usize, from_inventory: bool) -> Res {
        let o = if from_inventory { self.persos[i].inventory.get(n).ok_or(())?.ok_or(())?.object } else { n };
        rules.objects.get(o).ok_or(())?;
        let m = &self.houses[h];
        if m.stock() <= m.max_stock - 1.0 {
            if let Some(s) = rules.buildings[m.kind].objects.iter().position(|&x| x == o) {
                if from_inventory {
                    self.persos[i].unload_slot(n);
                }
                let m = &mut self.houses[h];
                m.set_object_stock(s, m.object_stock(s) + 1.0);
                return Ok(());
            }
        }
        self.deposit_object_inventory(rules, i, o, h)
    }

    /// `Objet_DeposerInventaire_Maison`: object type `o` into the house inventory, or dropped.
    pub fn deposit_object_inventory(&mut self, rules: &Rules, i: usize, o: usize, h: usize) -> Res {
        if let Some(slot) = self.houses[h].inventory.iter_mut().find(|s| **s == -1) {
            *slot = o as i64;
            Ok(())
        } else {
            self.drop_object(rules, i, o, false)
        }
    }

    /// The own house's inventory slot `slot` when perso `i` is alive in its own house.
    fn own_house_slot(&self, i: usize, slot: usize) -> Result<i64, ()> {
        let p = &self.persos[i];
        if !(p.in_own_house && p.alive()) {
            return Err(());
        }
        self.houses[i].inventory.get(slot).copied().ok_or(())
    }

    /// Inventory window, house view: picks up the object of house inventory slot `slot`.
    pub fn hold_house_object(&mut self, i: usize, slot: usize) -> Res {
        let o = self.own_house_slot(i, slot)?;
        if o < 0 || self.persos[i].selected_object >= 0 {
            return Err(());
        }
        self.persos[i].selected_object = o;
        self.houses[i].inventory[slot] = -1;
        Ok(())
    }

    /// Inventory window, house view: puts the held object in house inventory slot `slot`,
    /// holding what was there.
    pub fn place_in_house(&mut self, rules: &Rules, i: usize, slot: usize) -> Res {
        let o = self.own_house_slot(i, slot)?;
        let held = self.persos[i].selected_object;
        if held < 0 || rules.objects[held as usize].inseparable {
            return Err(());
        }
        self.houses[i].inventory[slot] = held;
        self.persos[i].selected_object = o;
        Ok(())
    }

    /// Inventory window, house view: right click moves house inventory slot `slot` to the bag
    /// (the original lost the object when the bag was full).
    pub fn house_object_to_bag(&mut self, rules: &Rules, i: usize, slot: usize) -> Res {
        let o = self.own_house_slot(i, slot)?;
        if o < 0 || self.persos[i].inventory_full() {
            return Err(());
        }
        self.persos[i].add_new_object(rules, o as usize);
        self.houses[i].inventory[slot] = -1;
        Ok(())
    }

    /// Inventory window, bag view: the house toggle with an object held stores it in the house.
    pub fn store_held_in_house(&mut self, rules: &Rules, i: usize) -> Res {
        self.own_house_slot(i, 0)?;
        let held = self.persos[i].selected_object;
        if held < 0 || rules.objects[held as usize].inseparable {
            return Err(());
        }
        self.persos[i].selected_object = -1;
        self.deposit_object_inventory(rules, i, held as usize, i)
    }

    /// `Objet_Deposer_Sol`: drops an object (inventory slot `n`, or object type `n`) and the
    /// selection into a new treasure. Inseparable objects make the original fail.
    pub fn drop_object(&mut self, rules: &Rules, i: usize, n: usize, from_inventory: bool) -> Res {
        let o = if from_inventory { self.persos[i].inventory.get(n).ok_or(())?.ok_or(())?.object } else { n };
        if rules.objects.get(o).ok_or(())?.inseparable {
            let p = &mut self.persos[i];
            p.check_selection();
            if p.selected_money > 0.0 || p.resources_selected {
                return Err(());
            }
            p.clear_selection(rules);
            return Err(());
        }
        let (x, y) = (self.persos[i].x as i32, self.persos[i].y as i32);
        let t = self.treasure_create(rules, x, y);
        self.treasures[t].add_object(rules, o);
        let p = &mut self.persos[i];
        if from_inventory {
            p.unload_slot(n);
        } else {
            p.selected_object = -1;
        }
        self.drop_selection(rules, i, Some(t));
        Ok(())
    }

    /// `Ressources_Deposer_Sol`: drops the selected money and resources into treasure `t` (a
    /// new one if None), then merges it with an overlapping one.
    pub fn drop_selection(&mut self, rules: &Rules, i: usize, t: Option<usize>) {
        self.persos[i].check_selection();
        let t = match t {
            Some(t) => t,
            None => {
                let (x, y) = (self.persos[i].x as i32, self.persos[i].y as i32);
                self.treasure_create(rules, x, y)
            }
        };
        let (p, tr) = (&mut self.persos[i], &mut self.treasures[t]);
        if p.selected_money > 0.0 {
            tr.set_money(rules, tr.money() + p.selected_money);
            p.set_money(p.money() - p.selected_money);
        }
        for r in 0..p.selected_resources.len() {
            let q = p.selected_resources[r];
            if q > 0.0 {
                tr.set_resource(rules, r, tr.resource(r) + q);
                p.set_resource(r, p.resource(r) - q);
            }
        }
        p.clear_selection(rules);
        self.treasure_check(rules, t);
    }

    /// `Objet_Vendre_Chateau`: sells an object (inventory slot `n`, or object type `n`) to the
    /// castle of fief `f`, whose stock recycles it into its resource cost.
    pub fn sell_object(&mut self, rules: &Rules, i: usize, n: usize, f: usize, from_inventory: bool) -> Res {
        let o = if from_inventory { self.persos[i].inventory.get(n).ok_or(())?.ok_or(())?.object } else { n };
        let obj = rules.objects.get(o).ok_or(())?;
        if !obj.inseparable {
            let p = &mut self.persos[i];
            p.set_money(p.money() + obj.price as f64);
            let fief = self.fiefs.get_mut(f).ok_or(())?;
            for (r, &q) in obj.price_resources.iter().enumerate() {
                fief.set_resource(r, fief.resources[r] + q as f64);
            }
            if from_inventory {
                p.unload_slot(n);
            }
        }
        Ok(())
    }

    /// `Vendre_ObjetsSelectionnees`: at castle (or market of) fief `f`, the selected money goes
    /// to the castle repairs and the selected resources and object are sold.
    pub fn sell_selection(&mut self, rules: &Rules, f: usize, i: usize) {
        let p = &mut self.persos[i];
        if p.selected_money > 0.0 {
            let q = p.selected_money;
            self.fiefs[f].donate(q, p, false);
        }
        for r in 0..rules.resources.len() {
            let q = self.persos[i].selected_resources[r];
            if q > 0.0 {
                self.fiefs[f].sell(rules, r, q, &mut self.persos[i], false);
            }
        }
        let o = self.persos[i].selected_object;
        if o >= 0 && !rules.objects[o as usize].inseparable {
            // The object always goes to the seller's own fief.
            let own = self.persos[i].fief;
            let _ = self.sell_object(rules, i, o as usize, own, false);
            self.persos[i].selected_object = -1;
        }
        self.persos[i].clear_selection(rules);
    }

    // ---------------------------------------------------------------- emperor

    /// `PeutDevenir_Empereur`
    pub fn can_become_emperor(&self, rules: &Rules, i: usize) -> bool {
        let p = &self.persos[i];
        let f = &self.fiefs[p.fief];
        f.emperor < 0 && p.money() >= f.emperor_price(rules) as f64 && p.alive()
    }

    /// `Empereur_Changer`: perso `i` (or nobody, -1) becomes the ruler of fief `f`.
    pub fn change_emperor(&mut self, rules: &Rules, f: usize, i: i64, pay: bool) {
        if i >= 0 {
            for j in 0..self.persos.len() {
                if self.persos[j].fief == f {
                    if j as i64 == i {
                        self.set_emperor(rules, j, true);
                    } else if self.persos[j].emperor {
                        self.set_emperor(rules, j, false);
                    }
                }
            }
            if pay {
                let price = self.fiefs[f].emperor_price(rules) as f64;
                let p = &mut self.persos[i as usize];
                p.set_money(p.money() - price);
            }
        }
        self.fiefs[f].emperor = i;
    }

    /// `Definir_Empereur` setter: the title brings the emperor's objects; losing it takes them
    /// back and demolishes an emperor-only house.
    pub fn set_emperor(&mut self, rules: &Rules, i: usize, v: bool) {
        if v {
            for (o, obj) in rules.objects.iter().enumerate() {
                if obj.emperor {
                    self.persos[i].add_object(rules, o);
                }
            }
        } else {
            let f = self.persos[i].fief;
            self.change_emperor(rules, f, -1, false);
            if rules.buildings[self.houses[i].kind].emperor && self.houses[i].visible() {
                self.demolish(rules, i, i, true);
                let m = &self.houses[i];
                let (x, y, w, h) = (m.x as f64, m.y as f64, m.w as f64, m.h as f64);
                self.effect(rules.effect_house_destroyed, x, y, w, h);
            }
            let p = &mut self.persos[i];
            for s in 0..p.equipped.len() {
                if p.equipped[s].is_some_and(|it| rules.objects[it.object].emperor) {
                    p.destroy_equipped(rules, s);
                }
            }
            for s in 0..p.inventory.len() {
                if p.inventory[s].is_some_and(|it| rules.objects[it.object].emperor) {
                    p.unload_slot(s);
                }
            }
        }
        self.persos[i].emperor = v;
    }

    // ---------------------------------------------------------------- life and death

    /// `Vivant = False`. Like the original it does not check that the perso was alive: dying
    /// again counts again.
    pub fn kill(&mut self, rules: &Rules, i: usize) {
        if self.god == Some(i) {
            let p = &mut self.persos[i];
            p.bars[0] = p.bonus_max[0] as f64;
            return;
        }
        let p = &mut self.persos[i];
        p.deaths += 1;
        p.comment.clear();
        // Still alive while it stops: an AI decides once more.
        self.stop(rules, i);
        let p = &mut self.persos[i];
        p.bars[0] = 0.0;
        let effect = rules.races[p.race].death_effect;
        self.effect_on(effect, i);
        let p = &mut self.persos[i];
        p.recovery = [0.0; 4];
        p.stun = 0;
        p.resurrection_time = 100.0;
        p.effects_on_death(rules);
        // Back in its fief's team.
        self.attack_allies(rules, i, false);
        let p = &self.persos[i];
        if let Ok(chef) = usize::try_from(p.chef) {
            if p.projectile {
                self.persos[chef].projectiles -= 1;
            } else {
                self.persos[chef].soldiers -= 1;
            }
            // The unfinished building of a summon goes with it.
            if self.houses[i].visible() && !self.houses[i].built {
                self.demolish(rules, i, i, false);
            }
        } else {
            self.fiefs[p.fief].dead += 1;
        }
        if self.persos[i].emperor {
            self.set_emperor(rules, i, false);
        }
    }

    /// `Vivant = True`
    pub fn revive(&mut self, rules: &Rules, i: usize) {
        self.stop(rules, i);
        let p = &mut self.persos[i];
        p.bars[0] = p.bonus_max[0] as f64;
        p.set_attack(rules, p.bonus_max[4] as f64);
        p.set_defense(rules, p.bonus_max[5] as f64);
        match usize::try_from(p.chef) {
            // A summon counts again among its chef's soldiers.
            Ok(chef) => {
                if !p.projectile {
                    self.persos[chef].soldiers += 1;
                }
            }
            Err(_) => self.fiefs[p.fief].dead -= 1,
        }
        let p = &mut self.persos[i];
        let race = &rules.races[p.race];
        if race.full_restore {
            for (r, &q) in race.resources.iter().enumerate() {
                if p.resource(r) < q {
                    p.set_resource(r, q);
                }
            }
            for &o in &race.equipment {
                if p.object_count(o) == 0 {
                    p.add_race_object(rules, o);
                }
            }
            for i in 1..4 {
                p.bars[i] = p.bonus_max[i] as f64;
            }
        } else if self.confrontation {
            for i in 1..4 {
                p.bars[i] = p.bonus_max[i] as f64;
            }
        } else {
            for i in [1, 3] {
                if p.bars[i] == 0.0 {
                    p.bars[i] = p.bonus_max[i] as f64;
                }
            }
        }
    }

    /// `RecuperationLente` for perso `i`, every 10th tick.
    pub(crate) fn slow_tick(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if p.alive() {
            let kind = self.houses[i].kind;
            let skill = usize::try_from(p.craft_slot)
                .ok()
                .and_then(|s| rules.buildings[kind].objects.get(s))
                .map(|&o| rules.objects[o].skill);
            self.persos[i].slow_fatigue(rules, skill);
            let p = &self.persos[i];
            if p.moving && p.trample > 0 {
                self.trample(rules, i);
            }
            self.persos[i].slow_recovery(rules);
            if !self.persos[i].alive() {
                // A negative regeneration killed it.
                self.kill(rules, i);
            }
            let p = &mut self.persos[i];
            if p.poison >= 1 {
                let poison = p.poison as f64;
                let (x, y) = (p.x as f64 + p.w as f64 / 2.0, p.y as f64);
                if p.set_life(rules, p.bars[0] - poison) {
                    self.kill(rules, i);
                }
                self.number(x, y, poison, NumberKind::Damage, false);
                // The poisoner gets the kill.
                if let Ok(k) = usize::try_from(self.persos[i].poisoner)
                    && !self.persos[i].alive()
                {
                    self.kill_loot(rules, k, i, false);
                }
            }
        } else if p.in_own_castle {
            let f = p.fief;
            if self.castles[f].visible() {
                if p.resurrections > 0 || p.resurrections == -1 {
                    let (nc, nm) = (self.fiefs[f].citizens as f64, self.fiefs[f].dead as f64);
                    let speed = p.resurrection_speed;
                    let p = &mut self.persos[i];
                    if nc - nm == 0.0 {
                        p.resurrection_time -= speed / nc;
                    } else {
                        p.resurrection_time -= speed / (nc / (nc - nm));
                    }
                    if p.resurrection_time <= 0.0 {
                        self.revive(rules, i);
                        let p = &mut self.persos[i];
                        if p.resurrections != -1 {
                            p.resurrections -= 1;
                        }
                    } else {
                        p.action = action::RESURRECTING;
                    }
                } else {
                    self.persos[i].action = action::IDLE;
                }
            } else {
                let p = &mut self.persos[i];
                p.action = action::IDLE;
                p.in_own_castle = false;
            }
        } else if p.invocation() {
            // Summons come back to life in their own house.
            if p.in_own_house && self.houses[i].visible() {
                let p = &mut self.persos[i];
                p.resurrection_time -= p.resurrection_speed;
                if p.resurrection_time <= 0.0 {
                    self.revive(rules, i);
                }
            }
        } else if self.confrontation {
            self.revive(rules, i);
            let (w, h) = (self.terrain.width as f64 * CELL as f64, self.terrain.height as f64 * CELL as f64);
            let x = clng(self.rng.next() * (w - self.persos[i].w as f64));
            let y = clng(self.rng.next() * (h - self.persos[i].h as f64));
            (self.persos[i].x, self.persos[i].y) = (x, y);
        }
    }

    // ---------------------------------------------------------------- treasures and decors

    /// `Tresor_Creer`: appends an empty treasure, returns its index.
    pub fn treasure_create(&mut self, rules: &Rules, x: i32, y: i32) -> usize {
        self.treasures.push(Treasure::new(rules, x, y));
        self.treasures.len() - 1
    }

    /// `Tresors_Verifier`: erases treasure `k` if empty, else merges it into the first other
    /// treasure it overlaps. Persos heading to a treasure are not renumbered.
    pub fn treasure_check(&mut self, rules: &Rules, k: usize) {
        if self.treasures[k].empty() {
            self.treasures.remove(k);
            return;
        }
        let s = TREASURE_SIZE;
        let (kx, ky) = (self.treasures[k].x, self.treasures[k].y);
        let Some(i) = (0..self.treasures.len())
            .find(|&i| i != k && overlaps((self.treasures[i].x, self.treasures[i].y, s, s), (kx, ky, s, s)))
        else {
            return;
        };
        let t = self.treasures[k].clone();
        let dst = &mut self.treasures[i];
        dst.set_money(rules, dst.money() + t.money());
        for r in 0..rules.resources.len() {
            dst.set_resource(rules, r, dst.resource(r) + t.resource(r));
        }
        for &o in &t.objects {
            dst.add_object(rules, o);
        }
        self.treasures.remove(k);
    }

    /// `Tresor_Ramasser`: perso `i` takes everything in treasure `k`, which disappears; the
    /// other persos heading to it stop, those heading further down the list are renumbered.
    pub fn treasure_pickup(&mut self, rules: &Rules, i: usize, k: usize) {
        let t = self.treasures.remove(k);
        self.show_treasure(rules, i, &t);
        let p = &mut self.persos[i];
        p.set_money(p.money() + t.money());
        for r in 0..rules.resources.len() {
            p.set_resource(r, p.resource(r) + t.resource(r));
        }
        for &o in &t.objects {
            p.add_new_object(rules, o);
        }
        for j in 0..self.persos.len() {
            let p = &mut self.persos[j];
            if p.action == action::GO_TREASURE {
                if p.treasure_target == k {
                    self.stop(rules, j);
                } else if p.treasure_target > k {
                    p.treasure_target -= 1;
                }
            }
        }
    }

    /// What perso `i` found in treasure `t`, floating over it (not in the original, which showed
    /// nothing): the money as a number, then a line per resource and object.
    fn show_treasure(&mut self, rules: &Rules, i: usize, t: &crate::treasure::Treasure) {
        let p = &self.persos[i];
        let (x, y) = (p.x as f64 + p.w as f64 / 2.0, p.y as f64);
        self.number(x, y, t.money(), crate::event::NumberKind::Money, false);
        let mut lines: Vec<String> = (0..rules.resources.len())
            .filter(|&r| t.resource(r) >= 1.0)
            .map(|r| format!("+{} {}", t.resource(r).floor(), rules.resources[r].name))
            .collect();
        let mut objects: Vec<usize> = t.objects.clone();
        objects.sort_unstable();
        objects.dedup();
        for o in objects {
            let n = t.objects.iter().filter(|&&x| x == o).count();
            let obj = &rules.objects[o];
            lines.push(if n > 1 { format!("{n} {}", obj.plural) } else { obj.name.clone() });
        }
        for (n, line) in lines.into_iter().enumerate() {
            self.text(x, y - 11.0 * (n + 1) as f64, line, crate::event::NumberKind::Money);
        }
    }

    /// `Tresor_Generer`: in confrontation mode, a random treasure now and then (slow tick).
    pub(crate) fn treasure_generate(&mut self, rules: &Rules) {
        let c = self.rng.next();
        if !((c < 1.0 / 60.0 || self.treasures.is_empty()) && self.treasures.len() <= 10) {
            return;
        }
        let s = TREASURE_SIZE as f64;
        let x = clng(self.rng.next() * self.terrain.width as f64 * CELL as f64 - s) as i32;
        let y = clng(self.rng.next() * self.terrain.height as f64 * CELL as f64 - s) as i32;
        let k = self.treasure_create(rules, x, y);
        if self.rng.next() < 1.0 / 3.0 {
            let v = self.rng.next() * 1000.0;
            self.treasures[k].set_money(rules, v);
        }
        let nr = rules.resources.len() as f64;
        let n = clng(self.rng.next() * 6.0 + 1.0);
        for _ in 0..=n {
            if self.rng.next() < 1.0 / 6.0 {
                // Index drawn before the value (the original's order is uncertain).
                let r = clng(self.rng.next() * (nr - 1.0)) as usize;
                let v = self.rng.next() * 20.0 + 1.0;
                self.treasures[k].set_resource(rules, r, v);
            }
        }
        let no = rules.objects.len() as f64;
        let n = clng(self.rng.next() * 6.0 + 1.0);
        for _ in 0..=n {
            let o = clng(self.rng.next() * (no - 1.0)) as usize;
            if !rules.objects[o].secret {
                self.treasures[k].add_object(rules, o);
            }
        }
        if self.treasures[k].empty() {
            let v = self.rng.next() * 10000.0;
            self.treasures[k].set_money(rules, v);
        }
    }

    /// `Decor_Creer`
    pub fn decor_create(&mut self, rules: &Rules, kind: usize, x: i32, y: i32, name: String) {
        self.decors.push(Decor::new(rules, kind, x, y, name, self.speed));
    }

    /// `Actualiser_Affichage_Decors`: animation, then at most one temporary decor expires.
    pub(crate) fn update_decors(&mut self) {
        for d in &mut self.decors {
            d.next_frame();
        }
        for k in 0..self.decors.len() {
            let d = &mut self.decors[k];
            if d.temporary {
                d.duration -= 1;
                if d.duration <= 0 {
                    self.decors.remove(k);
                    break;
                }
            }
        }
    }

    // ---------------------------------------------------------------- placement

    /// `ModIA.Obstacle`: what is at (x, y, w, h) from perso `i`'s point of view, 0 for nothing.
    /// 9 enemy, 13 treasure, 6/8/11 own/allied/enemy castle, 5/4 own house (complete or not),
    /// 7/10 allied/enemy house, 1 resource node, 16 decor, 999 unbuildable terrain.
    pub fn obstacle(&self, rules: &Rules, i: usize, x: i32, y: i32, w: i32, h: i32) -> i32 {
        self.obstacle_at(rules, i, x, y, w, h).0
    }

    /// Indexes persos, houses and resource nodes by position for [`World::obstacle_at`], at the
    /// start of each tick.
    pub(crate) fn index_positions(&mut self, rules: &Rules) {
        let (w, h) = (self.terrain.width as i32 * CELL, self.terrain.height as i32 * CELL);
        let s = &mut self.spatial;
        s.persos.build(self.persos.iter().map(|p| (p.x as i32, p.y as i32)), w, h);
        s.houses.build(self.houses.iter().map(|m| (m.x, m.y)), w, h);
        s.resources.build(self.resources.iter().map(|n| (n.x, n.y)), w, h);
        let side = |(w, h): (i32, i32)| w.max(h);
        s.perso_reach = (rules.races.iter().map(|r| side(r.size)))
            .chain(self.persos.iter().map(|p| side((p.w, p.h))))
            .max()
            .unwrap_or(0);
        s.house_reach = (rules.buildings.iter().map(|b| side(b.size)))
            .chain(self.houses.iter().map(|m| side((m.w, m.h))))
            .max()
            .unwrap_or(0);
    }

    /// [`World::obstacle`] with the index of what was found (perso, treasure, castle, house,
    /// resource node or decor; 0 for terrain). The persos, houses and resource nodes come from
    /// grids (`spatial.rs`), giving the same answers as scanning them in order, which debug
    /// builds check.
    pub fn obstacle_at(&self, rules: &Rules, i: usize, x: i32, y: i32, w: i32, h: i32) -> (i32, usize) {
        let found = self.obstacle_indexed(rules, i, x, y, w, h);
        debug_assert_eq!(found, self.obstacle_scan(rules, i, x, y, w, h), "obstacle_at({i}, {x}, {y}, {w}, {h})");
        found
    }

    /// Where the corner of a box of side up to `reach` overlapping (x, y, w, h) can be, give or
    /// take `slack` (the overlap test sometimes compares widths with heights: both are covered).
    fn reach((x, y, w, h): (i32, i32, i32, i32), reach: i32, slack: i32) -> (i32, i32, i32, i32) {
        let side = w.max(h);
        (x - reach - slack, y - reach - slack, x + side + slack, y + side + slack)
    }

    fn obstacle_indexed(&self, rules: &Rules, i: usize, x: i32, y: i32, w: i32, h: i32) -> (i32, usize) {
        let r = (x, y, w, h);
        let team = self.persos.get(i).map(|p| p.team).unwrap_or(0);
        let fief = self.persos.get(i).map(|p| p.fief).unwrap_or(0);
        let s = &self.spatial;
        let near = Self::reach(r, s.perso_reach, crate::spatial::DRIFT as i32 + 1);
        let enemy = s.persos.first(self.persos.len(), near, |j| {
            let p = &self.persos[j];
            overlaps(r, (p.x as i32, p.y as i32, p.w, p.h)) && p.team != team && p.attackable() && j != i
        });
        if let Some(j) = enemy {
            return (9, j);
        }
        let s = TREASURE_SIZE;
        if let Some(k) = self.treasures.iter().position(|t| overlaps(r, (t.x, t.y, s, s))) {
            return (13, k);
        }
        for (j, c) in self.castles.iter().enumerate() {
            if overlaps(r, (c.x, c.y, c.w, c.h)) && c.visible() {
                let code = if self.fiefs[j].team != team {
                    11
                } else if j == fief {
                    6
                } else {
                    8
                };
                return (code, j);
            }
        }
        let sp = &self.spatial;
        let house = sp.houses.first(self.houses.len(), Self::reach(r, sp.house_reach, 1), |j| {
            let m = &self.houses[j];
            m.visible() && overlaps(r, (m.x, m.y, m.w, m.h))
        });
        if let Some(j) = house {
            let m = &self.houses[j];
            let code = if j == i {
                if m.complete() { 5 } else { 4 }
            } else if self.persos[j].team == team {
                7
            } else {
                10
            };
            return (code, j);
        }
        let resource = sp.resources.first(self.resources.len(), Self::reach(r, RESOURCE_SIZE, 1), |k| {
            let n = &self.resources[k];
            overlaps(r, (n.x, n.y, RESOURCE_SIZE, RESOURCE_SIZE))
        });
        if let Some(k) = resource {
            return (1, k);
        }
        if let Some(k) = self.decors.iter().position(|d| overlaps(r, (d.x, d.y, d.w, d.h))) {
            return (16, k);
        }
        if !self.buildable(rules, x, y, w, h) {
            return (999, 0);
        }
        (0, 0)
    }

    /// `ModIA.Obstacle` as the original does it, scanning everything in order: what the grids
    /// must agree with.
    fn obstacle_scan(&self, rules: &Rules, i: usize, x: i32, y: i32, w: i32, h: i32) -> (i32, usize) {
        let r = (x, y, w, h);
        let team = self.persos.get(i).map(|p| p.team).unwrap_or(0);
        let fief = self.persos.get(i).map(|p| p.fief).unwrap_or(0);
        for (j, p) in self.persos.iter().enumerate() {
            if overlaps(r, (p.x as i32, p.y as i32, p.w, p.h)) && p.team != team && p.attackable() && j != i {
                return (9, j);
            }
        }
        let s = TREASURE_SIZE;
        if let Some(k) = self.treasures.iter().position(|t| overlaps(r, (t.x, t.y, s, s))) {
            return (13, k);
        }
        for (j, c) in self.castles.iter().enumerate() {
            if overlaps(r, (c.x, c.y, c.w, c.h)) && c.visible() {
                let code = if self.fiefs[j].team != team {
                    11
                } else if j == fief {
                    6
                } else {
                    8
                };
                return (code, j);
            }
        }
        for (j, m) in self.houses.iter().enumerate() {
            if m.visible() && overlaps(r, (m.x, m.y, m.w, m.h)) {
                let code = if j == i {
                    if m.complete() { 5 } else { 4 }
                } else if self.persos[j].team == team {
                    7
                } else {
                    10
                };
                return (code, j);
            }
        }
        if let Some(k) = self.resources.iter().position(|n| overlaps(r, (n.x, n.y, RESOURCE_SIZE, RESOURCE_SIZE))) {
            return (1, k);
        }
        if let Some(k) = self.decors.iter().position(|d| overlaps(r, (d.x, d.y, d.w, d.h))) {
            return (16, k);
        }
        if !self.buildable(rules, x, y, w, h) {
            return (999, 0);
        }
        (0, 0)
    }

    /// Houses of type `kind` built or being built in fief `f` (for `Limite`).
    pub fn houses_of_kind(&self, f: usize, kind: usize) -> usize {
        self.houses.iter().filter(|m| m.visible() && m.kind == kind && self.persos[m.owner].fief == f).count()
    }
}
