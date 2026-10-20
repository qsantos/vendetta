//! Castles and houses (`ClsJeuBatiment`).

use serde::{Deserialize, Serialize};
use crate::rules::Rules;

/// `CoefDepartVie`: share of the max HP a new building starts with.
pub const START_LIFE: f64 = 0.09;
/// `VitesseRecuperationVie`: HP per tick drained from the repair pool.
pub const REPAIR_SPEED: f64 = 0.1;
/// `CoefRecyclage`: share of the cost refunded on demolition.
pub const RECYCLING: f64 = 0.5;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Building {
    /// Owner perso index for houses (`Numero`), 0 for castles.
    pub owner: usize,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub entrance: (i32, i32),
    /// Building type (`TypeBatiment`); 0 for castles.
    pub kind: usize,
    pub built: bool,
    pub under_construction: bool,
    /// `Vie`; the building exists while it is positive.
    pub life: f64,
    /// `RecuperationVie`: pending repair.
    pub repair: f64,
    pub max_life: i64,
    /// The till (`Argent`).
    pub money: f64,
    /// `Stock`: total goods (resource units or objects); see [`Building::set_object_stock`].
    pub(crate) stock: f64,
    pub max_stock: f64,
    /// `StockObjets`: crafted objects per slot of the type's object list.
    pub(crate) object_stock: Vec<f64>,
    /// `ObjetsInventaire`: deposited objects, -1 for an empty slot.
    pub inventory: Vec<i64>,
    pub shop: bool,
    pub factory: bool,
    pub service: usize,
    pub market: bool,
    /// `Tir`: can shoot from inside.
    pub shoot: bool,
    /// `NumeroAnimation`
    pub animation: i64,
}

/// What a [`Building::set_life`] call changed, for the side effects the caller must apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeChange {
    None,
    /// Construction just completed.
    Built,
    /// HP reached 0; `was_built` if it had been completed.
    Destroyed { was_built: bool },
}

impl Building {
    pub fn new(rules: &Rules) -> Building {
        Building {
            inventory: vec![-1; rules.house_inventory_size],
            object_stock: vec![0.0],
            ..Default::default()
        }
    }

    pub fn visible(&self) -> bool {
        self.life > 0.0
    }

    /// `Definir_Vie` getter: HP including the pending repair, capped at the max.
    pub fn life_with_repair(&self) -> f64 {
        (self.life + self.repair).min(self.max_life as f64)
    }

    /// Built and at full HP (`Construit And Definir_Vie = MaxVie`).
    pub fn complete(&self) -> bool {
        self.built && self.life_with_repair() == self.max_life as f64
    }

    /// `Definir_Vie` setter: the construction state machine.
    pub fn set_life(&mut self, v: f64) -> LifeChange {
        self.life = v;
        if self.life >= self.max_life as f64 {
            self.life = self.max_life as f64;
            self.under_construction = false;
            if !self.built {
                self.built = true;
                return LifeChange::Built;
            }
            LifeChange::None
        } else if self.life <= 0.0 {
            self.under_construction = false;
            self.repair = 0.0;
            let was_built = self.built;
            self.built = false;
            LifeChange::Destroyed { was_built }
        } else {
            self.under_construction = true;
            LifeChange::None
        }
    }

    /// `Definir_TypeBatiment`
    pub fn set_kind(&mut self, rules: &Rules, kind: usize) {
        let t = &rules.buildings[kind];
        (self.w, self.h) = t.size;
        self.entrance = t.entrance;
        self.kind = kind;
        self.shop = t.shop;
        self.factory = t.factory;
        self.service = t.service;
        self.market = t.market;
        let n = if t.factory { t.objects.len() as i64 - 1 } else { 0 };
        if self.object_stock.len() as i64 - 1 != n {
            self.object_stock = vec![0.0; (n + 1).max(0) as usize];
        }
    }

    /// `Restauration`: applies the repair pool at [`REPAIR_SPEED`] per tick.
    pub fn restoration(&mut self) -> LifeChange {
        if self.repair >= REPAIR_SPEED {
            let c = self.set_life(self.life + REPAIR_SPEED);
            self.repair -= REPAIR_SPEED;
            c
        } else {
            self.repair = 0.0;
            LifeChange::None
        }
    }

    pub fn stock(&self) -> f64 {
        self.stock
    }

    /// `Definir_Stock` setter: capped at `max_stock`, no lower bound.
    pub fn set_stock(&mut self, v: f64) {
        self.stock = v.min(self.max_stock);
    }

    /// Unclamped write, like the original's direct `Stock = ...`.
    pub(crate) fn set_stock_raw(&mut self, v: f64) {
        self.stock = v;
    }

    pub fn object_stock(&self, slot: usize) -> f64 {
        self.object_stock.get(slot).copied().unwrap_or(0.0)
    }

    /// Highest object slot (`Definir_Nombre_Stock_Objet`).
    pub fn object_slots(&self) -> usize {
        self.object_stock.len()
    }

    /// `Stock_Objets(i) = v`: also moves the total stock.
    pub fn set_object_stock(&mut self, slot: usize, v: f64) {
        self.stock += v - self.object_stock[slot];
        self.object_stock[slot] = v;
    }

    pub(crate) fn clear_object_stock(&mut self) {
        self.object_stock.iter_mut().for_each(|s| *s = 0.0);
    }

    /// `Etat`: construction stage, the sprite row.
    pub fn state(&self, rules: &Rules) -> i64 {
        let stages = rules.buildings[self.kind].build_stages;
        if self.built {
            if (self.shop || self.factory || (self.service != 0 && self.max_stock > 0.0)) && self.stock < 1.0 {
                stages - 1
            } else {
                stages
            }
        } else {
            (self.life / self.max_life as f64 * stages as f64).floor() as i64
        }
    }

    /// `Actualiser_NumeroAnimation`
    pub fn next_animation(&mut self, rules: &Rules) {
        self.animation += 1;
        if self.animation > rules.buildings[self.kind].anim_frames - 1 {
            self.animation = 0;
        }
    }

    /// Entrance point for a character of size `(w, h)`: where it walks to enter.
    pub fn entrance_for(&self, w: i32, h: i32) -> (i64, i64) {
        use crate::vb::clng;
        (
            clng(self.x as f64 + self.w as f64 / 2.0 - w as f64 / 2.0 + self.entrance.0 as f64),
            clng(self.y as f64 + self.h as f64 / 2.0 - h as f64 / 2.0 + self.entrance.1 as f64),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_state_machine() {
        let mut b = Building { max_life: 100, life: 9.0, ..Default::default() };
        assert_eq!(b.set_life(50.0), LifeChange::None);
        assert!(b.under_construction && !b.built);
        assert_eq!(b.set_life(120.0), LifeChange::Built);
        assert_eq!(b.life, 100.0);
        assert!(b.complete() && !b.under_construction);
        assert_eq!(b.set_life(100.0), LifeChange::None, "built only once");
        b.repair = 5.0;
        assert_eq!(b.set_life(0.0), LifeChange::Destroyed { was_built: true });
        assert!(!b.visible() && !b.built);
        assert_eq!(b.repair, 0.0);
    }

    #[test]
    fn restoration_heals_a_tenth_per_tick() {
        let mut b = Building { max_life: 100, life: 50.0, built: true, repair: 0.25, ..Default::default() };
        b.restoration();
        b.restoration();
        assert!((b.life - 50.2).abs() < 1e-12);
        b.restoration();
        assert_eq!(b.repair, 0.0, "a remainder below 0.1 is discarded");
        assert!((b.life - 50.2).abs() < 1e-12);
    }
}
