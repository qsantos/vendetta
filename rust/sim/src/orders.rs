//! Player commands (`ModCommandes`), applied at the start of the next tick (`Lire_Commandes`).
//!
//! Like the original executor, nothing is validated beyond what the called method does: the
//! UI is expected to check affordability and placement. A command that would raise a runtime
//! error in the original (e.g. an index out of range) drops the rest of the queue.

use crate::economy::Res;
use crate::rules::Rules;
use crate::world::World;

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    /// `PersoDeplacer`: walk so that the character's top-left lands on (x, y).
    Move { perso: usize, x: i64, y: i64 },
    /// `PersoChercherRessource`: harvest resource node `node`.
    Harvest { perso: usize, node: usize },
    /// `PersoTrouverRessource`: harvest the nearest node of terrain resource type `kind`.
    FindResource { perso: usize, kind: usize },
    /// `PersoRentrerChateau`
    GoOwnCastle { perso: usize },
    /// `PersoAllerChateau`: enter an allied castle.
    GoToCastle { perso: usize, castle: usize },
    /// `PersoRentrerMaison`: go home; `enter` skips repairing a damaged house.
    GoHome { perso: usize, enter: bool },
    /// `PersoAllerMaison`: enter someone's house, or help building it with `repair`.
    GoToHouse { perso: usize, house: usize, repair: bool },
    /// `PersoCreerMaisonCoordonnees` + `Type`: start a house of type `kind` at (x, y).
    BuildHouse { perso: usize, x: i32, y: i32, kind: usize },
    /// `PersoDetruireMaison`
    Demolish { perso: usize },
    /// `PersoAllerTresor`
    GoToTreasure { perso: usize, treasure: usize },
    /// `PersoTrouverTresor`: the nearest treasure.
    FindTreasure { perso: usize },
    /// `PersoArreter`
    Stop { perso: usize },
    /// `PersoRecupererCarac`: eat for bar 0 Vie, 1 Energie, 2 Magie, 3 Moral.
    EatFor { perso: usize, bar: usize },
    /// `PersoRessourceManger`: eat one unit.
    Eat { perso: usize, resource: usize },
    /// `PersoRessourcesVendreChateau`: sell to the own castle.
    SellResource { perso: usize, resource: usize, quantity: f64 },
    /// `PersoRessourcesAcheterChateau`: buy from the own castle.
    BuyResource { perso: usize, resource: usize, quantity: f64 },
    /// `PersoRessourcesAcheterMaison`: buy the product of a shop.
    BuyFromHouse { perso: usize, house: usize, quantity: f64 },
    /// `PersoRessourceMaisonPrendre` / `PersoRessourcesDeposerMaison` (`all`) /
    /// `PersoRessourceMaisonLaisser`: move products between the own house stock and the bag.
    TakeStock { perso: usize, quantity: f64 },
    DepositStock { perso: usize, quantity: f64, all: bool },
    /// `PersoArgentMaisonPrendre` / `PersoArgentMaisonLaisser`: the own house till.
    TakeMoney { perso: usize, quantity: f64 },
    DepositMoney { perso: usize, quantity: f64 },
    /// `PersoRestaurerChateau`: donate to the own castle's repairs.
    DonateToCastle { perso: usize, quantity: f64 },
    /// `PersoRessourcePrendre` / `PersoRessourceLaisser`: select resources to give or drop.
    SelectResource { perso: usize, resource: usize, quantity: f64 },
    UnselectResource { perso: usize, resource: usize, quantity: f64 },
    /// `PersoArgentPrendre`
    SelectMoney { perso: usize, quantity: f64 },
    /// `PersoObjetSelectionne`: hold object type `object` (-1 for none).
    SelectObject { perso: usize, object: i64 },
    /// Inventory click: take one object of inventory slot `slot` onto the cursor (anything
    /// already held goes back to the bag first).
    HoldObject { perso: usize, slot: usize },
    /// `PersoObjetDeSelectionne`
    Unselect { perso: usize },
    /// `PersoObjetEquiper` (`PersoObjetEquiperADroite` with `right`): equip inventory slot
    /// `slot`, or use it if it is usable (potions, spells). Whatever it displaces goes back to
    /// the bag. `right` puts a one-hand weapon or a ring in the second slot (1 or 7).
    EquipObject { perso: usize, slot: usize, right: bool },
    /// The same with the held object (`selected_object`), e.g. dropped on equipment slot 1 or 7
    /// for `right`.
    EquipHeld { perso: usize, right: bool },
    /// `PersoObjetEquipesRanger`: put equipment slot `slot` back in the bag (nothing if the bag
    /// is full). Slot 1 means slot 0 when it holds a two-handed weapon, as on screen.
    StoreEquipped { perso: usize, slot: usize },
    /// Equipment click: take the object of equipment slot `slot` onto the cursor (anything
    /// already held goes back to the bag first). Slot 1 means slot 0 under a two-handed weapon.
    HoldEquipped { perso: usize, slot: usize },
    /// Inventory click while holding an object: stack it on inventory slot `slot` (same object,
    /// stack not full), swap it with a single object there (the latter becomes held), or put it
    /// in the slot if empty. A full stack or a stack of another object is left alone.
    PlaceHeld { perso: usize, slot: usize },
    /// `PersoAllerVendre`: sell the selection at castle `target`, or at market house `target`.
    GoSell { perso: usize, target: usize, market: bool },
    /// `PersoVendreObjetsSelectionnees`: sell the selection to fief `fief` right away.
    SellSelection { perso: usize, fief: usize },
    /// `PersoRessourcesDeposerSol`
    DropSelection { perso: usize },
    /// `PersoObjetDeposerSol`: drop an inventory slot (or the held object type).
    DropObject { perso: usize, object: usize, from_inventory: bool },
    /// `PersoObjetCreer`: craft slot `slot` of the own house's object list.
    Craft { perso: usize, slot: i64 },
    /// `PersoObjetArreterFabrication`
    StopCraft { perso: usize },
    /// `PersoObjetMaisonPrendre`: take crafted objects from the own house.
    TakeObjects { perso: usize, slot: usize, count: i64 },
    /// `PersoObjetMaisonDeposer`: put an inventory slot (or object type) into the own house.
    DepositObject { perso: usize, object: usize, from_inventory: bool },
    /// `PersoObjetMaisonDetruire` + `PersoObjetSelectionne`: hold the object of the own house's
    /// inventory slot `slot` (nothing already held).
    HoldHouseObject { perso: usize, slot: usize },
    /// `PersoObjetMaisonCharger`: put the held object into the own house's inventory slot
    /// `slot`, swapping with what is there.
    PlaceInHouse { perso: usize, slot: usize },
    /// `PersoObjetInventaireAjouter` + `PersoObjetMaisonDetruire`: move the own house's
    /// inventory slot `slot` into the bag.
    HouseObjectToBag { perso: usize, slot: usize },
    /// `PersoObjetMaisonAjouter`: put the held object into the own house's inventory.
    StoreHeldInHouse { perso: usize },
    /// `PersoObjetMaisonAcheter`: buy one crafted object (the UI emits one per unit).
    BuyObject { perso: usize, house: usize, slot: usize },
    /// `PersoObjetChateauVendre`: sell an inventory slot (or object type) to the own castle.
    SellObject { perso: usize, object: usize, from_inventory: bool },
    /// `PersoUtiliserService`
    UseService { perso: usize, house: usize },
    /// `PersoDevenirEmpereur`: pay the price and rule the own fief.
    BecomeEmperor { perso: usize },
    /// `PersoDefinirIA`: automatic mode on or off (the UI only sends it for characters with an
    /// AI file, `choose_ai`).
    SetAi { perso: usize, ai: bool },
    /// `PersoDefinirIAAttitude`: a soldier's attitude (0 follow, 1 wait, 2 defend, 3 attack,
    /// 4 berserk); it decides again at once.
    SetAttitude { perso: usize, attitude: i64 },
    /// `PersoAttaquerPerso`, `PersoAttaquerMaison`, `PersoAttaquerChateau`.
    AttackPerso { perso: usize, target: usize },
    AttackHouse { perso: usize, house: usize },
    AttackCastle { perso: usize, castle: usize },
    /// `PersoTrouverPersoEnnemi` (`kind` 0), `MaisonEnnemi` (1), `ChateauEnnemi` (2): attack the
    /// nearest enemy target, or with `send` send the soldiers (Shift: 5, Ctrl+Shift: 10, Ctrl: 1).
    AttackNearest { perso: usize, kind: u8, send: bool, ctrl: bool, shift: bool },
    /// `PersoAttaquerTerrainCoordonnees`: with a projectile weapon, shoot at (x, y) while
    /// walking on, hitting targets of `kind` (0 persos, 1 houses, 2 castles).
    AttackGround { perso: usize, x: i64, y: i64, kind: i64 },
    /// `PersoAllerDonner`: walk to perso `target` and give it the selection.
    Give { perso: usize, target: usize },
    /// `PersoOrdonnerChargez`: send the soldiers at a perso (`kind` 0), a house (1) or a castle
    /// (2), or with -1 at the chef's own target (else back into formation); Shift sends 5
    /// (10 with Ctrl), Ctrl alone 1.
    Charge { perso: usize, kind: i64, target: i64, ctrl: bool, shift: bool },
    /// `PersoOrdonnerFormation`: line the soldiers up, switching to the next formation with
    /// `change`.
    Formation { perso: usize, change: bool },
    /// `PersoAttaquerAllies`: become hostile to everybody until death.
    AttackAllies { perso: usize },
}

impl World {
    /// Executes one command (`Lire_Commandes`).
    pub(crate) fn apply_order(&mut self, rules: &Rules, order: &Order) -> Res {
        use Order::*;
        let perso = match *order {
            Move { perso, .. }
            | Harvest { perso, .. }
            | FindResource { perso, .. }
            | GoOwnCastle { perso }
            | GoToCastle { perso, .. }
            | GoHome { perso, .. }
            | GoToHouse { perso, .. }
            | BuildHouse { perso, .. }
            | Demolish { perso }
            | GoToTreasure { perso, .. }
            | FindTreasure { perso }
            | Stop { perso }
            | EatFor { perso, .. }
            | Eat { perso, .. }
            | SellResource { perso, .. }
            | BuyResource { perso, .. }
            | BuyFromHouse { perso, .. }
            | TakeStock { perso, .. }
            | DepositStock { perso, .. }
            | TakeMoney { perso, .. }
            | DepositMoney { perso, .. }
            | DonateToCastle { perso, .. }
            | SelectResource { perso, .. }
            | UnselectResource { perso, .. }
            | SelectMoney { perso, .. }
            | SelectObject { perso, .. }
            | HoldObject { perso, .. }
            | Unselect { perso }
            | EquipObject { perso, .. }
            | EquipHeld { perso, .. }
            | StoreEquipped { perso, .. }
            | HoldEquipped { perso, .. }
            | PlaceHeld { perso, .. }
            | GoSell { perso, .. }
            | SellSelection { perso, .. }
            | DropSelection { perso }
            | DropObject { perso, .. }
            | Craft { perso, .. }
            | StopCraft { perso }
            | TakeObjects { perso, .. }
            | DepositObject { perso, .. }
            | HoldHouseObject { perso, .. }
            | PlaceInHouse { perso, .. }
            | HouseObjectToBag { perso, .. }
            | StoreHeldInHouse { perso }
            | BuyObject { perso, .. }
            | SellObject { perso, .. }
            | UseService { perso, .. }
            | BecomeEmperor { perso }
            | SetAi { perso, .. }
            | SetAttitude { perso, .. }
            | AttackPerso { perso, .. }
            | AttackHouse { perso, .. }
            | AttackCastle { perso, .. }
            | AttackNearest { perso, .. }
            | AttackGround { perso, .. }
            | Give { perso, .. }
            | Charge { perso, .. }
            | Formation { perso, .. }
            | AttackAllies { perso } => perso,
        };
        let p = self.persos.get_mut(perso).ok_or(())?;
        let f = p.fief;
        let nr = rules.resources.len();
        let check = |i: usize, n: usize| if i < n { Ok(()) } else { Err(()) };
        match *order {
            Move { x, y, .. } => self.walk_to(rules, perso, x, y),
            Harvest { node, .. } => {
                check(node, self.resources.len())?;
                self.go_harvest(rules, perso, node);
            }
            FindResource { kind, .. } => self.find_resource(rules, perso, kind),
            GoOwnCastle { .. } => self.go_own_castle(rules, perso),
            GoToCastle { castle, .. } => {
                check(castle, self.castles.len())?;
                self.go_to_castle(rules, perso, castle);
            }
            GoHome { enter, .. } => self.go_home(rules, perso, perso, enter),
            GoToHouse { house, repair, .. } => {
                check(house, self.houses.len())?;
                self.go_to_house(rules, perso, house, repair);
            }
            BuildHouse { x, y, kind, .. } => {
                check(kind, rules.buildings.len())?;
                self.create_house(rules, perso, x, y, kind, perso);
            }
            Demolish { .. } => self.demolish(rules, perso, perso, false),
            GoToTreasure { treasure, .. } => {
                check(treasure, self.treasures.len())?;
                self.go_to_treasure(rules, perso, treasure);
            }
            FindTreasure { .. } => self.find_treasure(rules, perso)?,
            Stop { .. } => self.stop(rules, perso),
            EatFor { bar, .. } => {
                if bar < 4 {
                    p.eat_for(rules, bar);
                }
            }
            Eat { resource, .. } => {
                check(resource, nr)?;
                p.eat(rules, resource);
            }
            SellResource { resource, quantity, .. } => {
                check(resource, nr)?;
                self.fiefs[f].sell(rules, resource, quantity, &mut self.persos[perso], false);
            }
            BuyResource { resource, quantity, .. } => {
                check(resource, nr)?;
                self.fiefs[f].buy(rules, resource, quantity, &mut self.persos[perso], false);
            }
            BuyFromHouse { house, quantity, .. } => self.buy_from_house(rules, house, perso, quantity, false)?,
            TakeStock { quantity, .. } => self.take_stock(rules, perso, perso, quantity, false)?,
            DepositStock { quantity, all, .. } => self.deposit_stock(rules, perso, perso, quantity, all)?,
            TakeMoney { quantity, .. } => self.take_money(perso, perso, quantity, false),
            DepositMoney { quantity, .. } => self.deposit_money(perso, perso, quantity, false),
            DonateToCastle { quantity, .. } => self.fiefs[f].donate(quantity, &mut self.persos[perso], false),
            SelectResource { resource, quantity, .. } => {
                check(resource, nr)?;
                p.select_resource(resource, quantity, false, false);
            }
            UnselectResource { resource, quantity, .. } => {
                check(resource, nr)?;
                p.select_resource(resource, quantity, false, true);
            }
            SelectMoney { quantity, .. } => p.select_money(quantity, false),
            SelectObject { object, .. } => p.selected_object = object,
            HoldObject { slot, .. } => {
                p.clear_selection(rules);
                let item = p.inventory.get(slot).copied().flatten().ok_or(())?;
                p.unload_slot(slot);
                p.selected_object = item.object as i64;
            }
            Unselect { .. } => p.clear_selection(rules),
            EquipObject { slot, right, .. } => {
                check(slot, p.inventory.len())?;
                self.equip_object_at(rules, perso, Some(slot), right);
            }
            EquipHeld { right, .. } => {
                self.equip_object_at(rules, perso, None, right);
            }
            StoreEquipped { slot, .. } => {
                check(slot, p.equipped.len())?;
                let slot = if slot == 1 && p.two_handed(rules) { 0 } else { slot };
                p.store(rules, slot);
            }
            HoldEquipped { slot, .. } => {
                check(slot, p.equipped.len())?;
                let slot = if slot == 1 && p.two_handed(rules) { 0 } else { slot };
                p.clear_selection(rules);
                p.selected_object = p.equipped[slot].map_or(-1, |it| it.object as i64);
                p.destroy_equipped(rules, slot);
            }
            PlaceHeld { slot, .. } => {
                check(slot, p.inventory.len())?;
                p.place_held(rules, slot);
            }
            GoSell { target, market, .. } => self.go_sell(rules, perso, target, market)?,
            SellSelection { fief, .. } => {
                check(fief, self.fiefs.len())?;
                self.sell_selection(rules, fief, perso);
            }
            DropSelection { .. } => self.drop_selection(rules, perso, None),
            DropObject { object, from_inventory, .. } => self.drop_object(rules, perso, object, from_inventory)?,
            Craft { slot, .. } => self.start_craft(perso, slot),
            StopCraft { .. } => {
                self.stop(rules, perso);
                self.persos[perso].craft_progress = 0.0;
            }
            TakeObjects { slot, count, .. } => self.take_from_house(rules, perso, slot, perso, count),
            DepositObject { object, from_inventory, .. } => {
                self.deposit_object(rules, perso, object, perso, from_inventory)?
            }
            HoldHouseObject { slot, .. } => self.hold_house_object(perso, slot)?,
            PlaceInHouse { slot, .. } => self.place_in_house(rules, perso, slot)?,
            HouseObjectToBag { slot, .. } => self.house_object_to_bag(rules, perso, slot)?,
            StoreHeldInHouse { .. } => self.store_held_in_house(rules, perso)?,
            BuyObject { house, slot, .. } => self.buy_object(rules, perso, slot, house)?,
            SellObject { object, from_inventory, .. } => self.sell_object(rules, perso, object, f, from_inventory)?,
            UseService { house, .. } => self.use_service(rules, house, perso)?,
            BecomeEmperor { .. } => self.change_emperor(rules, f, perso as i64, true),
            SetAi { ai, .. } => p.ai = ai,
            SetAttitude { attitude, .. } => {
                p.attitude = attitude;
                p.ai_order = 0;
                self.ai_perso(rules, perso, false);
            }
            AttackPerso { target, .. } => {
                check(target, self.persos.len())?;
                self.attack_perso(rules, perso, target);
            }
            AttackHouse { house, .. } => {
                check(house, self.houses.len())?;
                self.attack_house(rules, perso, house);
            }
            AttackCastle { castle, .. } => {
                check(castle, self.castles.len())?;
                self.attack_castle(rules, perso, castle);
            }
            AttackNearest { kind, send, ctrl, shift, .. } => match kind {
                0 => self.attack_nearest_enemy(rules, perso, send, ctrl, shift),
                1 => self.attack_nearest_house(rules, perso, send, ctrl, shift),
                _ => self.attack_nearest_castle(rules, perso, send, ctrl, shift),
            },
            AttackGround { x, y, kind, .. } => self.attack_ground(perso, x, y, kind),
            Give { target, .. } => {
                check(target, self.persos.len())?;
                self.go_give(perso, target);
            }
            Charge { kind, target, ctrl, shift, .. } => {
                let n = [self.persos.len(), self.houses.len(), self.castles.len()];
                if let (Some(&n), Ok(t)) = (usize::try_from(kind).ok().and_then(|k| n.get(k)), usize::try_from(target)) {
                    check(t, n)?;
                }
                self.order_charge(rules, perso, kind, target, ctrl, shift, false);
            }
            Formation { change, .. } => self.order_formation(rules, perso, change, false),
            AttackAllies { .. } => self.attack_allies(rules, perso, true),
        }
        Ok(())
    }
}
