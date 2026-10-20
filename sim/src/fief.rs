//! Fiefs: the castle's stock, treasury and age (`ClsJeuFief`).

use serde::{Deserialize, Serialize};
use crate::building::Building;
use crate::perso::Perso;
use crate::rules::Rules;
use crate::vb::clng;

/// `QuantiteVieParArgent`: castle HP bought per gold.
pub const LIFE_PER_GOLD: f64 = 0.2;
/// `VitesseTransaction`: units converted per citizen per tick by the castle market.
pub const TRANSACTION_SPEED: f64 = 0.02 / 4.0 / 4.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fief {
    pub people: usize,
    pub team: i64,
    /// 0-based age.
    pub epoch: usize,
    /// `NombreCitoyens`: characters created for this fief at the start.
    pub citizens: usize,
    /// `NombreMorts`
    pub dead: i64,
    /// `ChangerEpoque`: an age advance is in progress.
    pub changing_epoch: bool,
    /// `TempsChangementEpoque`: ticks left before the next age (keeps counting down).
    pub epoch_timer: i64,
    /// Perso index of the ruler, -1 for none.
    pub emperor: i64,
    /// Castle repair fund.
    pub money: f64,
    /// Castle stock (use [`Fief::set_resource`] to clamp).
    pub resources: Vec<f64>,
    /// `Marches`: house indices of the markets usable by this fief (with duplicates).
    pub markets: Vec<usize>,
}

impl Fief {
    /// `Init`
    pub fn new(rules: &Rules, people: usize, team: i64, epoch: usize, citizens: usize) -> Fief {
        Fief {
            people,
            team,
            epoch,
            citizens,
            dead: 0,
            changing_epoch: false,
            epoch_timer: 0,
            emperor: -1,
            money: 0.0,
            resources: vec![0.0; rules.resources.len()],
            markets: Vec::new(),
        }
    }

    /// `Definir_ressources` setter: clamped at 0.
    pub fn set_resource(&mut self, r: usize, v: f64) {
        self.resources[r] = v.max(0.0);
    }

    /// `Changer_Epoque`: starts the advance to the next age.
    pub fn change_epoch(&mut self, rules: &Rules, castle: &mut Building, speed_coef: f64) {
        if !self.changing_epoch && self.epoch + 1 < rules.epochs.len() {
            let next = &rules.epochs[self.epoch + 1];
            self.changing_epoch = true;
            castle.max_life += next.castle_life_bonus;
            self.epoch_timer = clng(next.duration as f64 * speed_coef);
            castle.repair += (next.castle_life_bonus + 1) as f64;
        }
    }

    /// `Transaction_Ressources`: the castle market converts its most abundant good into the
    /// scarcest one of the current age.
    pub fn transaction(&mut self, rules: &Rules) {
        let r = &mut self.resources;
        let mut min = 0;
        for i in 0..r.len() {
            if r[i].floor() < r[min].floor() && rules.resources[i].epoch <= self.epoch as i64 {
                min = i;
            }
        }
        let mut max = 0;
        for i in 0..r.len() {
            if r[i].floor() > r[max].floor() {
                max = i;
            }
        }
        if (r[max] - r[min]).abs() > 1.0 {
            r[min] += TRANSACTION_SPEED * self.citizens as f64;
            r[max] -= TRANSACTION_SPEED * self.citizens as f64;
        }
    }

    /// `Restaurer_Chateau`: a donation to the castle repair fund.
    pub fn donate(&mut self, q: f64, p: &mut Perso, all: bool) {
        let q = if all || q > p.money() { p.money() } else { q };
        self.money += q;
        p.set_money(p.money() - q);
    }

    /// `Restauration`: spends the fund on castle repairs, and runs the age timer.
    pub fn restoration(&mut self, rules: &Rules, castle: &mut Building) {
        if castle.life_with_repair() < castle.max_life as f64 {
            let q = (castle.max_life as f64 - castle.life_with_repair() + 1.0).min(self.money * LIFE_PER_GOLD);
            castle.repair += q;
            self.money -= q / LIFE_PER_GOLD;
        }
        if self.epoch + 1 < rules.epochs.len() {
            self.epoch_timer -= 1;
            if self.epoch_timer <= 0 && self.changing_epoch {
                self.epoch += 1;
                self.changing_epoch = false;
            }
        }
    }

    /// `Empereur_Prix`
    pub fn emperor_price(&self, rules: &Rules) -> i64 {
        rules.epochs[self.epoch].emperor_price
    }

    /// `Vendre`: a character sells resource `r` to the castle.
    pub fn sell(&mut self, rules: &Rules, r: usize, q: f64, p: &mut Perso, all: bool) {
        let q = if all || q > p.resource(r) { p.resource(r) } else { q }.floor();
        p.set_resource(r, p.resource(r) - q);
        self.resources[r] += q;
        p.set_money(p.money() + q * rules.resources[r].price);
    }

    /// `Acheter`: a character buys resource `r` from the castle at twice its price.
    pub fn buy(&mut self, rules: &Rules, r: usize, q: f64, p: &mut Perso, all: bool) {
        let price = rules.resources[r].price * rules.buy_coef;
        let mut q = q;
        if all || p.money() < q * price {
            q = p.money() / price;
        }
        if q > self.resources[r] {
            q = self.resources[r];
        }
        let room = p.max_resource(r) - p.resource(r).floor();
        if q > room {
            q = room;
        }
        let q = q.floor();
        p.set_resource(r, p.resource(r) + q);
        self.set_resource(r, self.resources[r] - q);
        p.set_money(p.money() - q * price);
    }
}
