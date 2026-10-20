//! What AI villagers do for a living: eating, working, trading at the castle, equipping
//! (`IA_Recuperation_Carac`, `IA_Travail`, `IA_Transaction_Chateau`, `IA_Equiper`, …).

use crate::perso::action;
use crate::rules::Rules;
use crate::world::World;

/// `QuantiteMaxTransfert`, `MatierePremiereMin`.
const MAX_TRANSFER: f64 = 10.0;
const RAW_MIN: f64 = 1.0;
/// `VitesseEquipement`, `VitesseEquipementCombat`: inventory slots examined per turn, minus one.
const EQUIP_SPEED: usize = 1;
const EQUIP_SPEED_FIGHTING: usize = 5;
/// `Service` of mercenary barracks.
const SERVICE_BOUNTY: usize = 2;

impl World {
    // ---------------------------------------------------------------- bars

    /// `IA_Recuperation_Carac`: eat; buy food in a shop; go eat or heal in a shop or a service
    /// house of the fief (or harvest grass and fruit when poor); fetch ammunition; pick up
    /// treasures.
    pub(super) fn recover(&mut self, rules: &Rules, i: usize) {
        if self.persos[i].ai_eat && self.recover_bars(rules, i) {
            return;
        }
        if self.persos[i].idle()
            && self.persos[i].ai_equip > 0
            && let Some(k) = self.find_treasure_near(i)
        {
            self.ai_go_treasure(rules, i, k);
        }
    }

    /// The `IA_Manger` part of [`World::recover`]; returns true when it acted (`Exit Sub`).
    fn recover_bars(&mut self, rules: &Rules, i: usize) -> bool {
        let p = &mut self.persos[i];
        for bar in 0..4 {
            p.eat_auto(rules, bar);
        }
        let ht = p.house_target;
        if p.in_house {
            let m = &self.houses[ht];
            let product = rules.buildings[m.kind].product;
            if m.shop && product >= 0 && rules.resources[product as usize].edible {
                let _ = self.buy_from_house(rules, ht, i, 5.0, false);
                return true;
            }
            return false;
        }
        if p.idle() {
            if p.bar_total(0) < p.bonus_max[0] as f64 {
                if self.poor(i) {
                    self.gather_resource(rules, i, 0);
                    return true;
                } else if let Some(x) = self.find_house(rules, i, 0, 1, 0) {
                    self.ai_go_house(rules, i, x);
                    return true;
                }
            }
            let p = &self.persos[i];
            if p.bar_total(1) < p.bonus_max[1] as f64 * 0.5 {
                if self.poor(i) {
                    self.gather_resource(rules, i, 1);
                    return true;
                } else if let Some(x) = self.find_house(rules, i, 0, 2, 0) {
                    self.ai_go_house(rules, i, x);
                    return true;
                } else {
                    self.gather_resource(rules, i, 1);
                    return true;
                }
            }
            for bar in [2, 3] {
                let p = &self.persos[i];
                if p.bar_total(bar) < p.bonus_max[bar] as f64 * 0.5
                    && !self.poor(i)
                    && let Some(x) = self.find_house(rules, i, 0, bar as i64 + 1, 0)
                {
                    self.ai_go_house(rules, i, x);
                    return true;
                }
            }
        }
        let p = &self.persos[i];
        let ammo = p.ammo;
        if ammo >= 0 && p.res(ammo) < p.ammo_quantity {
            if self.poor(i) {
                self.gather_resource(rules, i, ammo);
            } else if let Some(x) = self.find_house(rules, i, ammo, 0, 0) {
                self.ai_go_house(rules, i, x);
            } else {
                self.gather_resource(rules, i, ammo);
            }
            return true;
        }
        false
    }

    // ---------------------------------------------------------------- work

    /// `IA_ChercherMatieresPremiere`: fetch the raw material of the own house when out of it.
    pub(super) fn seek_raw(&mut self, rules: &Rules, i: usize) {
        let t = self.houses[i].kind;
        let m1 = self.raw_material(rules, t);
        let m2 = self.raw_material(rules, t);
        let p = &self.persos[i];
        let free = p.idle() || p.action == action::GO_OWN_CASTLE;
        if !(free && !p.in_house && self.houses[i].built && p.res(m1) < RAW_MIN && m2 >= 0) {
            return;
        }
        let m = self.raw_material(rules, t);
        if self.harvestable(rules, m) {
            let m = self.raw_material(rules, t);
            self.gather_resource(rules, i, m);
        } else if self.persos[i].money() > 1.0 {
            let m = self.raw_material(rules, t);
            if let Some(x) = self.find_house(rules, i, m, 0, 0) {
                self.ai_go_house(rules, i, x);
            } else {
                let m = self.raw_material(rules, t);
                let p = &self.persos[i];
                let nf = p.fief;
                let leave = self.ai.persos[i].leave;
                if leave
                    && p.action != action::ENTER_HOUSE
                    && self.fief_res(nf, m) > RAW_MIN
                    && self.castles[nf].visible()
                    || !self.fiefs[nf].markets.is_empty()
                {
                    self.ai_go_castle(rules, i, true);
                }
            }
        }
    }

    /// `IA_Travail`: shop in someone's building, work at home, trade at the castle, fetch the
    /// till, hunt bounties, or go and sell.
    pub(super) fn ai_work(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if !p.idle() {
            return;
        }
        let leave = self.ai.persos[i].leave;
        let ht = p.house_target;
        let own = self.houses[i].kind;
        if p.in_house && !leave {
            let m = &self.houses[ht];
            let kind = m.kind;
            if m.shop {
                let raw = self.raw_material(rules, own);
                if rules.buildings[kind].product == raw {
                    let _ = self.buy_from_house(rules, ht, i, 0.0, true);
                } else {
                    let q = (MAX_TRANSFER * self.rng.next()).floor();
                    let _ = self.buy_from_house(rules, ht, i, q, false);
                }
            } else if m.factory {
                let objects = &rules.buildings[kind].objects;
                let z = self.rng.below(objects.len() as i64) as usize;
                // The original fails on a factory without objects.
                if let Some(&o) = objects.get(z)
                    && self.houses[ht].object_stock(z) > 0.0
                    && self.persos[i].object_count(o) == 0
                {
                    let _ = self.buy_object(rules, i, z, ht);
                }
            } else if self.can_use_service(rules, ht, i) {
                let _ = self.use_service(rules, ht, i);
            }
            self.ai.persos[i].leave = true;
        } else if p.in_own_house && !leave {
            self.work_at_home(rules, i);
        } else if (p.in_castle || p.in_own_castle) && !leave || p.in_house && self.houses[ht].market {
            self.castle_trade(rules, i);
            self.ai.persos[i].leave = true;
        } else if self.poor(i) && self.houses[i].money >= 1.0 {
            self.ai_go_home(rules, i);
        } else if self.houses[i].service == SERVICE_BOUNTY {
            if p.bars[0] == p.bar_total(0) {
                for _ in 0..3 {
                    self.aggressive(rules, i);
                }
                if self.persos[i].idle() && self.houses[i].money > 0.0 {
                    self.ai_go_home(rules, i);
                }
            }
        } else {
            let h = &self.houses[i];
            let rf = rules.buildings[own].product;
            if p.res(rf) > RAW_MIN && h.stock() < h.max_stock && (rf >= 0 || h.factory || h.service > 0) {
                self.ai_go_home(rules, i);
            } else {
                // `IA_Stock_Exedentaire(i) Or (…)`: the first term is always true, but the second
                // still draws three raw materials; so does nothing else, and the final
                // `IA_Agressif` is unreachable.
                for _ in 0..3 {
                    self.raw_material(rules, own);
                }
                if self.persos[i].in_own_castle {
                    self.ai_go_home(rules, i);
                } else {
                    self.ai_go_castle(rules, i, true);
                }
            }
        }
    }

    /// `IA_Travailler_Maison`: in the own house, take the till, then craft (preferring an
    /// out-of-stock object) or put half the products on sale, and go to the castle.
    fn work_at_home(&mut self, rules: &Rules, i: usize) {
        self.take_money(i, i, 0.0, true);
        let h = &self.houses[i];
        if h.factory {
            let objects = &rules.buildings[h.kind].objects;
            let e = self.fiefs[self.persos[i].fief].epoch as i64;
            let allowed = |z: usize| rules.objects[objects[z]].epoch - 1 <= e;
            if objects.is_empty() || !objects.iter().enumerate().any(|(z, _)| allowed(z)) {
                // The original fails or loops forever here.
                self.ai_go_castle(rules, i, true);
                return;
            }
            let mut z = loop {
                let z = self.rng.below(objects.len() as i64) as usize;
                if allowed(z) {
                    break z;
                }
            };
            if let Some(x) = (0..objects.len())
                .find(|&x| self.houses[i].object_stock(x) == 0.0 && self.can_craft(rules, i, x as i64, i) && allowed(x))
            {
                z = x;
            }
            let h = &self.houses[i];
            if self.can_craft(rules, i, z as i64, i) && h.stock() < h.max_stock {
                self.start_craft(i, z as i64);
            } else {
                if (self.poor(i) || h.stock() == h.max_stock)
                    && let Some(x) = (0..objects.len()).find(|&x| self.houses[i].object_stock(x) >= 1.0)
                {
                    self.take_from_house(rules, i, x, i, 1);
                }
                self.ai_go_castle(rules, i, true);
            }
        } else if h.shop && (h.stock() <= 1.0 || !self.poor(i)) {
            if h.stock() != h.max_stock {
                let rf = rules.buildings[h.kind].product;
                let q = 0.5 * self.persos[i].res(rf);
                let _ = self.deposit_stock(rules, i, i, q, false);
            }
            self.ai_go_castle(rules, i, true);
        } else {
            self.ai_go_castle(rules, i, true);
        }
    }

    // ---------------------------------------------------------------- castle

    /// `IA_Transaction_Chateau`: in a castle or a market, sell the products (a bankrupt
    /// factory is demolished), maybe become emperor, buy food and materials, repair the castle.
    fn castle_trade(&mut self, rules: &Rules, i: usize) {
        let h = &self.houses[i];
        let nf = self.persos[i].fief;
        if !h.shop || self.poor(i) || h.stock() >= h.max_stock - 1.0 {
            let kind = h.kind;
            if h.factory {
                let objects = &rules.buildings[kind].objects;
                let castle = self.persos[i].castle_target;
                let mut sold = 0;
                for x in 0..self.persos[i].inventory.len() {
                    if self.persos[i].inventory[x].is_none() {
                        continue;
                    }
                    for &o in objects {
                        if self.persos[i].inventory[x].is_some_and(|it| it.object == o) {
                            let _ = self.sell_object(rules, i, x, castle, true);
                            sold += 1;
                        }
                    }
                }
                let h = &self.houses[i];
                if sold == 0 && h.money == 0.0 && h.stock() == 0.0 {
                    self.demolish(rules, i, i, false);
                }
            } else if let Ok(rf) = usize::try_from(rules.buildings[kind].product) {
                self.fiefs[nf].sell(rules, rf, 0.0, &mut self.persos[i], true);
            }
        }
        self.become_emperor(rules, i);
        self.buy_food(rules, i);
        self.buy_for_work(rules, i);
        if !self.poor(i) {
            self.buy_for_building(rules, i);
        }
        self.restore_castle(i);
        self.ai.persos[i].leave = true;
    }

    /// `IA_Vente_Surplus_Objets`: in a castle or a market, sell the duplicates (more than two
    /// one-hand weapons, more than one of anything else).
    pub(super) fn sell_surplus(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if !(p.in_own_castle || p.in_castle || p.in_house && self.houses[p.house_target].market) {
            return;
        }
        let nf = p.fief;
        for j in (0..p.inventory.len()).rev() {
            let p = &self.persos[i];
            if let Some(it) = p.inventory[j] {
                let o = &rules.objects[it.object];
                let n = p.object_count(it.object);
                if !o.single_use && (n > 1 && o.category != 0 || n > 2 && o.category == 0) {
                    let _ = self.sell_object(rules, i, j, nf, true);
                }
            }
        }
    }

    /// `IA_Devenir_Empereur`: buy the title, selling resources, then objects, then equipment
    /// if needed.
    fn become_emperor(&mut self, rules: &Rules, i: usize) {
        let nf = self.persos[i].fief;
        if !self.castles[nf].visible() || self.fiefs[nf].emperor >= 0 {
            return;
        }
        let price = self.fiefs[nf].emperor_price(rules) as f64;
        let p = &self.persos[i];
        if p.money() >= price {
            self.change_emperor(rules, nf, i as i64, true);
            return;
        }
        if p.money() + ((p.resources_value(rules) + p.objects_value(rules)) as f64) < price {
            return;
        }
        for r in 0..rules.resources.len() {
            self.fiefs[nf].sell(rules, r, 0.0, &mut self.persos[i], true);
            if self.persos[i].money() >= price {
                break;
            }
        }
        if self.persos[i].money() >= price {
            self.change_emperor(rules, nf, i as i64, true);
            return;
        }
        for j in 0..self.persos[i].inventory.len() {
            if self.persos[i].inventory[j].is_some() {
                let _ = self.sell_object(rules, i, j, nf, true);
            }
            if self.persos[i].money() >= price {
                break;
            }
        }
        if self.persos[i].money() >= price {
            self.change_emperor(rules, nf, i as i64, true);
            return;
        }
        for j in 0..self.persos[i].equipped.len() {
            if self.persos[i].equipped[j].is_some() {
                // The original then sells inventory slot 0, whatever it holds.
                self.persos[i].store(rules, j);
                let _ = self.sell_object(rules, i, 0, nf, true);
            }
            if self.persos[i].money() >= price {
                break;
            }
        }
    }

    /// `IA_Acheter_Ressources_Pour_Manger`: unless poor, buy at the castle the last
    /// non-harvestable food restoring Moral, else Vie, else Energie, else Magie.
    fn buy_food(&mut self, rules: &Rules, i: usize) {
        if self.poor(i) {
            return;
        }
        let nf = self.persos[i].fief;
        for bar in [3, 0, 1, 2] {
            let p = &self.persos[i];
            if p.bar_total(bar) < p.bonus_max[bar] as f64 {
                let food = (0..rules.resources.len())
                    .rev()
                    .find(|&j| rules.resources[j].gain[bar] > 0.0 && !rules.resources[j].harvestable);
                if let Some(j) = food {
                    self.fiefs[nf].buy(rules, j, 0.0, &mut self.persos[i], true);
                    return;
                }
            }
        }
    }

    /// `IA_Acheter_Ressources_Pour_Travailler`: buy at the castle the non-harvestable
    /// ingredients of the house's objects, or its raw material.
    fn buy_for_work(&mut self, rules: &Rules, i: usize) {
        let h = &self.houses[i];
        if !h.visible() {
            return;
        }
        let nf = self.persos[i].fief;
        let b = &rules.buildings[h.kind];
        if b.factory {
            for j in 0..rules.resources.len() {
                if !rules.resources[j].harvestable && b.objects.iter().any(|&o| rules.objects[o].price_resources[j] > 0)
                {
                    self.fiefs[nf].buy(rules, j, 0.0, &mut self.persos[i], true);
                }
            }
        } else {
            let raw = if b.service != 0 {
                rules.services[b.service].raw
            } else if b.product >= 0 {
                rules.resources[b.product as usize].raw
            } else {
                -1
            };
            if let Some(r) = usize::try_from(raw).ok().filter(|&r| !rules.resources[r].harvestable) {
                self.fiefs[nf].buy(rules, r, 0.0, &mut self.persos[i], true);
            }
        }
    }

    /// `IA_Acheter_Ressources_Pour_Construire`: buy a few units of each missing non-harvestable
    /// material of the planned building (one `Rnd` each).
    fn buy_for_building(&mut self, rules: &Rules, i: usize) {
        let Ok(b) = usize::try_from(self.ai.persos[i].build) else { return };
        let nf = self.persos[i].fief;
        let rf = rules.buildings[self.houses[i].kind].product;
        for j in 0..rules.resources.len() {
            if rules.buildings[b].price_resources[j] as f64 > self.persos[i].resource(j)
                && j as i64 != rf
                && !rules.resources[j].harvestable
            {
                let q = MAX_TRANSFER * self.rng.next();
                self.fiefs[nf].buy(rules, j, q, &mut self.persos[i], false);
            }
        }
    }

    /// `IA_Restaurer_Chateau`: give money to repair the own castle, more when it is badly hurt.
    pub(super) fn restore_castle(&mut self, i: usize) {
        let nf = self.persos[i].fief;
        let c = &self.castles[nf];
        if c.visible() && c.life_with_repair() < c.max_life as f64 {
            let m = self.persos[i].money();
            let q = (m - m * c.life_with_repair() / c.max_life as f64).floor() + 1.0;
            self.fiefs[nf].donate(q, &mut self.persos[i], false);
        }
    }

    // ---------------------------------------------------------------- equipment

    /// `IA_Equiper`: examine the next 2 inventory slots (6 in combat, `IndiceObjet` is shared
    /// by everybody): use consumables and equip anything better than what is worn (by value,
    /// or with `IA_Equiper = 2` by skill then value).
    pub(super) fn ai_equip(&mut self, rules: &Rules, i: usize) {
        let mode = self.persos[i].ai_equip;
        if mode != 1 && mode != 2 {
            return;
        }
        let n = if self.persos[i].fighting() { EQUIP_SPEED_FIGHTING } else { EQUIP_SPEED };
        for _ in 0..=n {
            let s = self.ai.object_cursor;
            if let Some(Some(it)) = self.persos[i].inventory.get(s).copied()
                && self.should_equip(rules, i, it.object, mode)
            {
                self.equip_object(rules, i, s);
            }
            self.ai.object_cursor += 1;
            if self.ai.object_cursor > self.persos[i].inventory.len() - 1 {
                self.ai.object_cursor = 0;
            }
        }
    }

    fn should_equip(&self, rules: &Rules, i: usize, t: usize, mode: i64) -> bool {
        let p = &self.persos[i];
        let o = &rules.objects[t];
        if o.usable {
            return !o.single_use || p.fighting();
        }
        let worn = |slot: usize| p.equipped.get(slot).copied().flatten().map(|it| it.object);
        let skill = |x: usize| p.objects[rules.objects[x].skill].bonus;
        let better = |slot: usize| match worn(slot) {
            None => false,
            Some(e) if mode == 1 => o.price > rules.objects[e].price,
            Some(e) => skill(t) > skill(e) || skill(t) == skill(e) && o.price > rules.objects[e].price,
        };
        match o.category {
            0 => worn(0).is_none() || worn(1).is_none() || better(0),
            1 => {
                let shoot = self.houses[p.house_target].shoot;
                let range = |e: Option<usize>| e.map_or(0.0, |e| rules.objects[e].range_bonus);
                worn(0).is_none()
                    || p.in_house
                    || p.in_own_house && shoot && o.range_bonus > range(worn(0))
                    || better(0)
            }
            c => usize::try_from(c).is_ok_and(|c| c < p.equipped.len() && (worn(c).is_none() || better(c))),
        }
    }

    /// `IA_Mort_Equiper`: a ghost uses the objects usable while dead.
    pub(super) fn ai_dead_equip(&mut self, rules: &Rules, i: usize) {
        if self.persos[i].ai_equip <= 0 {
            return;
        }
        for _ in 0..=EQUIP_SPEED {
            let s = self.ai.object_cursor;
            if let Some(Some(it)) = self.persos[i].inventory.get(s).copied()
                && rules.objects[it.object].use_when_dead
            {
                self.equip_object(rules, i, s);
            }
            self.ai.object_cursor += 1;
            if self.ai.object_cursor > self.persos[i].inventory.len() - 1 {
                self.ai.object_cursor = 0;
            }
        }
    }

    /// `Objet_Equiper`: equip, or use, the object of inventory slot `slot`.
    pub fn equip_object(&mut self, rules: &Rules, i: usize, slot: usize) {
        self.equip_object_at(rules, i, Some(slot), false);
    }
}
