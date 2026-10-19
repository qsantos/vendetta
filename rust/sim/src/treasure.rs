//! Treasures (`ClsJeuTresors`, `ModTresors`) and decorations (`ClsJeuDecors`, `ModDecors`).

use serde::{Deserialize, Serialize};
use crate::rules::Rules;

/// `AffTresor.Largeur/Hauteur`: the treasure box.
pub const TREASURE_SIZE: i32 = 26;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasure {
    pub x: i32,
    pub y: i32,
    /// Running estimated value.
    fortune: f64,
    /// `IndiceImage`: value tier, the sprite and name.
    pub tier: usize,
    money: f64,
    resources: Vec<f64>,
    /// Object type indices.
    pub objects: Vec<usize>,
}

impl Treasure {
    /// `Tresor_Creer` (without appending).
    pub fn new(rules: &Rules, x: i32, y: i32) -> Treasure {
        Treasure { x, y, fortune: 0.0, tier: 0, money: 0.0, resources: vec![0.0; rules.resources.len()], objects: Vec::new() }
    }

    pub fn fortune(&self) -> f64 {
        self.fortune
    }

    /// `Definir_Fortune` setter: also picks the tier.
    fn set_fortune(&mut self, rules: &Rules, v: f64) {
        self.fortune = v;
        if let Some(i) = rules.treasures.iter().rposition(|t| self.fortune >= t.min_value as f64) {
            self.tier = i;
        }
    }

    pub fn money(&self) -> f64 {
        self.money
    }

    pub fn set_money(&mut self, rules: &Rules, v: f64) {
        self.set_fortune(rules, self.fortune + v - self.money);
        self.money = v;
    }

    pub fn resource(&self, r: usize) -> f64 {
        self.resources[r]
    }

    pub fn set_resource(&mut self, rules: &Rules, r: usize, v: f64) {
        self.set_fortune(rules, self.fortune + (v - self.resources[r]) * rules.resources[r].price);
        self.resources[r] = v;
    }

    /// `Ajouter_Objet`
    pub fn add_object(&mut self, rules: &Rules, object: usize) {
        self.objects.push(object);
        self.set_fortune(rules, self.fortune + rules.objects[object].price as f64);
    }

    /// `Vide`
    pub fn empty(&self) -> bool {
        self.money <= 1.0 && self.objects.is_empty() && self.resources.iter().all(|&r| r < 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decor {
    /// Decor type (`Categorie`).
    pub kind: usize,
    /// Tooltip (tombs carry the victim's name).
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub temporary: bool,
    /// Ticks left (`Duree`), for temporary ones.
    pub duration: i64,
    pub frame: i64,
    pub frames: i64,
}

impl Decor {
    /// `Decor_Creer` (without appending); `speed` is `Jeu.Vitesse`.
    pub fn new(rules: &Rules, kind: usize, x: i32, y: i32, name: String, speed: i64) -> Decor {
        let t = &rules.decors[kind];
        Decor {
            kind,
            name,
            x,
            y,
            w: t.size.0,
            h: t.size.1,
            temporary: t.duration > 0,
            duration: t.duration * speed,
            frame: 0,
            frames: t.anim_frames,
        }
    }

    /// `ChangerEtapeAnimation`
    pub fn next_frame(&mut self) {
        self.frame += 1;
        if self.frame >= self.frames {
            self.frame = 0;
        }
    }
}
