//! The AI (`ModIA`, `ModIA2`, `ModDialogue`): the round robin, the built-in behaviours of
//! monsters, soldiers and ghosts, the berserk modes and the scripted `.ia` AI.
//!
//! Like the original, every `Rnd` of a condition is drawn even when the outcome is already
//! known (`And`/`Or`/`IIf` do not short-circuit), and searches are spread over many ticks.

mod dialogue;
mod scripted;
mod work;

use serde::{Deserialize, Serialize};
use crate::event::NumberKind;
use crate::perso::action;
use crate::rules::Rules;
use crate::terrain::CELL;
use crate::treasure::TREASURE_SIZE;
use crate::vb::clng;
use crate::world::World;

/// `VitesseIA`, `VitesseRechercheIA` (Singles).
const AI_SPEED: f32 = 0.01;
const SEARCH_SPEED: f32 = 0.6;
/// `DistancePalierRecherchePerso`, `DistancePalierRechercheMaison`.
const PERSO_SEARCH_STEP: i64 = 48;
const HOUSE_SEARCH_STEP: i64 = 96;
/// `DistanceAdversaireMin`, `DistanceAdversaireMax`, `DistanceTresorMax`.
const ENEMY_MIN: i64 = 64;
const ENEMY_MAX: i64 = 640;
const TREASURE_MAX: i64 = 724;
/// `ChanceReparerMaison`
const REPAIR_CHANCE: f64 = 0.1;
/// `ArgentSeuilPauvre`
const POOR: f64 = 300.0;
/// `IA_Patrouiller_Cercle` radius.
const PATROL_RADIUS: f64 = 128.0;
/// Deepest `Arreter` re-entry; the original would overflow its stack instead.
const MAX_DEPTH: u32 = 16;

/// Per-character AI state (the arrays of `ModIA`, indexed by perso).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersoAi {
    /// `DistanceTresor`, `DistanceMaison`, `DistanceAdversaire`, `DistanceMaisonEnnemi`,
    /// `DistanceChateauEnnemi`: radii of the incremental searches.
    pub treasure_distance: i64,
    pub house_distance: i64,
    pub enemy_distance: i64,
    pub enemy_house_distance: i64,
    pub enemy_castle_distance: i64,
    /// `IndiceMaison`, `IndiceAdversaire`, `IndiceMaisonEnnemi`, `IndiceChateauEnnemi`: cursors.
    pub house_cursor: usize,
    pub enemy_cursor: usize,
    pub enemy_house_cursor: usize,
    pub enemy_castle_cursor: usize,
    /// `AnglePatrouille` (a Single).
    pub patrol_angle: f32,
    /// Goals of the scripted AI: `BatimentAConstruire`, `RessourceAFabriquer`, `ObjetAFabriquer`,
    /// `QuantiteAFabriquer` (-1 for none).
    pub build: i64,
    pub resource: i64,
    pub object: i64,
    pub quantity: i64,
    /// `QuitterBatiment`: done with the building it is in.
    pub leave: bool,
}

/// `ModIA` module state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ai {
    /// `IndicePerso`: the round-robin cursor.
    pub cursor: usize,
    /// `LPerso`, `NombreIAParTour`, `NombreRechercheParTour`.
    pub last: i64,
    pub per_tick: i64,
    pub searches: i64,
    /// `IndiceObjet`: the inventory slot `IA_Equiper` examines next, shared by everybody.
    pub object_cursor: usize,
    /// `BatimentNiveauMax`
    pub level_max: i64,
    /// `DistanceObstacle`: radius of the house placement search, per fief.
    pub obstacle_distance: Vec<i64>,
    pub persos: Vec<PersoAi>,
    depth: u32,
}

fn dist(a: (i64, i64), b: (i64, i64)) -> f64 {
    (((a.0 - b.0) as f64).powi(2) + ((a.1 - b.1) as f64).powi(2)).sqrt()
}

impl World {
    // ---------------------------------------------------------------- scheduling

    /// `IA_Init`: called once the world is ready.
    pub fn ai_init(&mut self, rules: &Rules) {
        self.ai.obstacle_distance = vec![0; self.fiefs.len()];
        self.ai_resize(self.persos.len());
        let n = rules.inventory_size as i64;
        let c = clng(self.rng.next() * n as f64);
        self.ai.object_cursor = if c == n { 0 } else { c as usize };
        // Quirk: the last character is skipped.
        for i in 0..self.persos.len().saturating_sub(1) {
            self.ai_init_perso(i);
            self.ai.persos[i].patrol_angle = (self.rng.next() * 6.28) as f32;
        }
        let max = rules.buildings.iter().map(|b| b.level).max().unwrap_or(0);
        self.ai.level_max = self.ai.level_max.max(max);
    }

    /// `Nombre_IAs = n`: resizes the per-character state for `n` characters (call it whenever a
    /// character slot is appended, like `Perso_Ajouter_Nouveau`).
    pub fn ai_resize(&mut self, n: usize) {
        let ai = &mut self.ai;
        ai.last = n as i64 - 1;
        ai.persos.resize(n, PersoAi::default());
        if let Some(p) = ai.persos.last_mut() {
            p.patrol_angle = (self.rng.next() * 6.28) as f32;
        }
        let k = |speed: f32| match clng(ai.last as f64 * speed as f64) {
            0 => 1,
            k => k,
        };
        ai.per_tick = k(AI_SPEED);
        ai.searches = k(SEARCH_SPEED);
    }

    /// `IA_InitPerso`
    pub fn ai_init_perso(&mut self, i: usize) {
        let s = &mut self.ai.persos[i];
        s.house_distance = HOUSE_SEARCH_STEP;
        s.build = -1;
        s.resource = -1;
        s.object = -1;
        s.quantity = 0;
    }

    /// `Comportement_IA`: `NombreIAParTour + 1` characters decide per tick, in turn.
    pub(crate) fn ai_tick(&mut self, rules: &Rules) {
        if self.ai.persos.len() < self.persos.len() {
            self.ai.persos.resize(self.persos.len(), PersoAi::default());
        }
        if self.ai.last < 0 {
            return;
        }
        for _ in 0..=self.ai.per_tick {
            self.ai.cursor += 1;
            if self.ai.cursor as i64 > self.ai.last {
                self.ai.cursor = 0;
            }
            self.ai_perso(rules, self.ai.cursor, false);
        }
    }

    /// `Arreter`: stops, and an AI that stops while visible decides again at once (`Simple`).
    pub fn stop(&mut self, rules: &Rules, i: usize) {
        self.persos[i].stop();
        let p = &self.persos[i];
        if p.ai && p.visible() && self.ai.depth < MAX_DEPTH && i < self.ai.persos.len() {
            self.ai.depth += 1;
            self.ai_perso(rules, i, true);
            self.ai.depth -= 1;
        }
    }

    // ---------------------------------------------------------------- decision tree

    /// `Comportement_IA_Persos`. `simple` (after `Arreter`) only forbids placing a new house.
    pub fn ai_perso(&mut self, rules: &Rules, i: usize, simple: bool) {
        let Some(p) = self.persos.get(i) else { return };
        if !p.ai {
            return;
        }
        if p.choose_script {
            self.ai_script(rules, i);
            return;
        }
        if p.alive() {
            self.ai_equip(rules, i);
            let p = &self.persos[i];
            if p.ia_berserk() > 0 {
                // Berserk characters are driven every tick by `Deplacer_Persos`.
                return;
            }
            if p.choose_ai {
                self.ai_scripted(rules, i, simple);
            } else if p.invocation() {
                self.recover_simple(rules, i);
                let p = &self.persos[i];
                if p.idle() {
                    if p.ai_order > 0 {
                        self.obey(rules, i);
                    } else if p.ia_berserk() == 0 {
                        self.soldier(rules, i);
                    }
                }
            } else {
                self.monster_equipment(i);
                self.monster(rules, i);
            }
        } else {
            self.ai_dead_equip(rules, i);
            let p = &self.persos[i];
            if p.invocation() {
                if self.houses[i].visible() && self.persos[p.chef as usize].alive() && !p.in_own_house {
                    self.ai_go_home(rules, i);
                }
            } else {
                let h = p.house_target;
                if (p.in_house || p.in_own_house) && self.houses[h].built && self.can_use_service(rules, h, i) {
                    let _ = self.use_service(rules, h, i);
                    self.ai.persos[i].leave = true;
                } else if p.action != action::WALK && p.action != action::ENTER_HOUSE {
                    match self.find_house(rules, i, 0, 0, 1) {
                        Some(k) if !self.poor(i) => self.ai_go_house(rules, i, k),
                        _ => {
                            let p = &self.persos[i];
                            if self.castles[p.fief].visible() && !p.in_own_castle {
                                self.ai_go_castle(rules, i, false);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The `IA_Ordre` part of `Comportement_IA_Persos`: soldiers obeying their chef.
    fn obey(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if p.fighting() {
            return;
        }
        let t = p.ai_target;
        match p.ai_order {
            1 => {
                if self.persos.get(t).is_some_and(|q| q.attackable() && q.team != p.team) {
                    self.attack_perso(rules, i, t);
                } else {
                    self.persos[i].ai_order = 0;
                }
            }
            2 => {
                if self.houses.get(t).is_some_and(|m| m.visible()) && self.persos[t].team != p.team {
                    self.attack_house(rules, i, t);
                } else {
                    self.persos[i].ai_order = 0;
                }
            }
            3 => {
                if self.castles.get(t).is_some_and(|c| c.visible()) {
                    self.attack_castle(rules, i, t);
                } else {
                    self.persos[i].ai_order = 0;
                }
            }
            4 => self.guard(rules, i),
            _ => {}
        }
    }

    /// `IA_Argent_Besoin`: poor for its age.
    fn poor(&self, i: usize) -> bool {
        let p = &self.persos[i];
        p.money() < POOR * (self.fiefs[p.fief].epoch + 1) as f64
    }

    /// `Monde.Diagonale`
    fn diagonal(&self) -> i64 {
        let (w, h) = ((self.terrain.width as i32 * CELL) as f64, (self.terrain.height as i32 * CELL) as f64);
        clng((w * w + h * h).sqrt())
    }

    /// `Monde.Largeur * AffTerrain.Largeur + Monde.Hauteur * AffTerrain.Hauteur`
    fn half_perimeter(&self) -> i64 {
        (self.terrain.width + self.terrain.height) as i64 * CELL as i64
    }

    fn pos(&self, i: usize) -> (i64, i64) {
        (self.persos[i].x, self.persos[i].y)
    }

    /// `Batiment_TypeMatierePremiere`: the raw material of building type `t`. For a factory it
    /// is a random ingredient of its objects, with one `Rnd > 0.5` per candidate until one passes.
    fn raw_material(&mut self, rules: &Rules, t: usize) -> i64 {
        let b = &rules.buildings[t];
        if b.factory {
            let mut temp = -1;
            for r in 0..rules.resources.len() {
                for &o in &b.objects {
                    if rules.objects[o].price_resources[r] > 0 {
                        temp = r as i64;
                        if self.rng.next() > 0.5 {
                            return r as i64;
                        }
                    }
                }
            }
            temp
        } else if b.service != 0 {
            rules.services[b.service].raw
        } else if b.product >= 0 {
            rules.resources[b.product as usize].raw
        } else {
            -1
        }
    }

    // ---------------------------------------------------------------- movement helpers

    /// `IA_Rentrer_Maison`
    fn ai_go_home(&mut self, rules: &Rules, i: usize) {
        self.go_home(rules, i, i, false);
        self.ai.persos[i].leave = false;
    }

    /// `IA_Aller_Maison`: someone's house, sometimes to help repair it.
    fn ai_go_house(&mut self, rules: &Rules, i: usize, k: usize) {
        if k == i {
            self.ai_go_home(rules, i);
        } else {
            let repair = self.rng.next() < REPAIR_CHANCE;
            self.go_to_house(rules, i, k, repair);
            self.ai.persos[i].leave = false;
        }
    }

    /// `IA_Aller_Tresor`
    fn ai_go_treasure(&mut self, rules: &Rules, i: usize, k: usize) {
        self.go_to_treasure(rules, i, k);
        self.ai.persos[i].leave = true;
    }

    /// `IA_Rentrer_Chateau`: with `market`, the nearest market when it is closer than the castle
    /// (house 0 when there is no market and no castle).
    fn ai_go_castle(&mut self, rules: &Rules, i: usize, market: bool) {
        let nf = self.persos[i].fief;
        if market {
            let p = self.pos(i);
            let mut d = self.diagonal();
            let mut l = 0;
            for &k in &self.fiefs[nf].markets {
                let dk = dist(p, (self.houses[k].x as i64, self.houses[k].y as i64));
                if dk < d as f64 {
                    d = clng(dk);
                    l = k;
                }
            }
            let c = &self.castles[nf];
            if (d as f64) < dist(p, (c.x as i64, c.y as i64)) || !c.visible() {
                let repair = self.rng.next() < REPAIR_CHANCE;
                self.go_to_house(rules, i, l, repair);
            } else {
                self.go_own_castle(rules, i);
            }
        } else {
            self.go_own_castle(rules, i);
        }
        self.ai.persos[i].leave = false;
    }

    /// `IA_Deplacement_Aleatoire`: up to ±1024 × ±768 px, retried up to 41 times until the
    /// terrain is not slow.
    pub fn random_walk(&mut self, rules: &Rules, i: usize) {
        let tl = CELL as f64;
        let p = &self.persos[i];
        let nx = self.terrain.width as i64 * CELL as i64 - p.w as i64;
        let ny = self.terrain.height as i64 * CELL as i64 - p.h as i64;
        let (px, py, flying, h) = (p.x as f64, p.y as f64, p.flying, (p.h - CELL) as i64);
        let clamp = |v: i64, max: i64| {
            if v < 0 {
                0
            } else if v > max {
                max
            } else {
                v
            }
        };
        let mut tries = 0;
        let (x, y) = loop {
            let x = clamp(clng(px + (self.rng.next() * 128.0 - 64.0) * tl), nx);
            let y = clamp(clng(py + (self.rng.next() * 96.0 - 48.0) * tl), ny);
            tries += 1;
            if flying || crate::perso::speed_coef(rules, &self.terrain, x, y + h) >= 1.0 || tries > 40 {
                break (x, y);
            }
        };
        self.walk_to(rules, i, x, y);
    }

    /// `IA_Patrouiller_Cercle`: the next point of a circle around perso `t`.
    fn patrol_circle(&mut self, rules: &Rules, i: usize, t: usize) {
        let a = self.ai.persos[i].patrol_angle;
        let (p, q) = (&self.persos[i], &self.persos[t]);
        let x = clng((a as f64).cos() * PATROL_RADIUS - p.w as f64 / 2.0 + q.x as f64 + q.w as f64 / 2.0);
        let y = clng((a as f64).sin() * PATROL_RADIUS - p.h as f64 / 2.0 + q.y as f64 + q.h as f64 / 2.0);
        let bm = p.bonus_move;
        self.walk_to(rules, i, x, y);
        // The original divides by zero for immobile races.
        if bm != 0 {
            let mut a = (a as f64 + 6.28 * PATROL_RADIUS / bm as f64 * 2.0) as f32;
            if a > 6.28 {
                a = (a as f64 - 6.28) as f32;
            }
            self.ai.persos[i].patrol_angle = a;
        }
    }

    // ---------------------------------------------------------------- searches

    /// `IA_Attaquer_Perso`: the next slice of the search for an attackable enemy; the radius
    /// grows by 48 each time the cursor wraps (from 0, then 64..640).
    fn search_enemy(&mut self, i: usize) -> Option<usize> {
        let lb = self.persos.len() - 1;
        for _ in 0..=self.ai.searches {
            let s = &mut self.ai.persos[i];
            if s.enemy_cursor > lb {
                s.enemy_cursor = 0;
                s.enemy_distance += PERSO_SEARCH_STEP;
                if s.enemy_distance > ENEMY_MAX {
                    s.enemy_distance = ENEMY_MIN;
                }
            }
            let c = s.enemy_cursor;
            let (p, q) = (&self.persos[i], &self.persos[c]);
            if s.enemy_distance as f64 > dist((p.x, p.y), (q.x, q.y)) && q.team != p.team && q.attackable() {
                return Some(c);
            }
            s.enemy_cursor += 1;
        }
        None
    }

    /// `IA_Attaquer_Assaillant_Perso`: someone attacking perso `t`, its house or its castle
    /// (shares the cursor of `search_enemy`).
    fn search_assailant(&mut self, i: usize, t: usize) -> Option<usize> {
        let lb = self.persos.len() - 1;
        for _ in 0..=self.ai.searches {
            let s = &mut self.ai.persos[i];
            if s.enemy_cursor > lb {
                s.enemy_cursor = 0;
            }
            let c = s.enemy_cursor;
            let (q, v) = (&self.persos[c], &self.persos[t]);
            if q.action == 300 && q.perso_target == t
                || q.action == 301 && q.house_target == t
                || q.action == 302 && v.in_own_castle && q.castle_target == v.fief
            {
                return Some(c);
            }
            s.enemy_cursor += 1;
        }
        None
    }

    /// `IA_Attaquer_Maison`: the next slice of the search for a visible enemy house.
    fn search_enemy_house(&mut self, i: usize) -> Option<usize> {
        let lb = self.houses.len() - 1;
        let p = self.pos(i);
        for _ in 0..=self.ai.searches {
            let s = &mut self.ai.persos[i];
            if s.enemy_house_cursor > lb {
                s.enemy_house_cursor = 0;
                s.enemy_house_distance += HOUSE_SEARCH_STEP;
                if s.enemy_house_distance > ENEMY_MAX {
                    s.enemy_house_distance = ENEMY_MIN;
                }
            }
            let c = s.enemy_house_cursor;
            let m = &self.houses[c];
            if s.enemy_house_distance as f64 > dist(p, (m.x as i64, m.y as i64))
                && self.persos[c].team != self.persos[i].team
                && m.visible()
            {
                return Some(c);
            }
            s.enemy_house_cursor += 1;
        }
        None
    }

    /// `IA_Attaquer_Chateau`: the next slice of the search for a visible enemy castle; unlike
    /// the others, the cursor moves on after a hit.
    fn search_enemy_castle(&mut self, rules: &Rules, i: usize) -> Option<usize> {
        let lb = self.castles.len() - 1;
        let p = self.pos(i);
        let step = rules.peoples[0].castle_size.0 as i64 * 2;
        let mut found = None;
        for _ in 0..=self.ai.searches {
            let s = &mut self.ai.persos[i];
            if s.enemy_castle_cursor > lb {
                s.enemy_castle_cursor = 0;
                s.enemy_castle_distance += step;
                if s.enemy_castle_distance > ENEMY_MAX {
                    s.enemy_castle_distance = ENEMY_MIN;
                }
            }
            let c = s.enemy_castle_cursor;
            let castle = &self.castles[c];
            let hit = s.enemy_castle_distance as f64 > dist(p, (castle.x as i64, castle.y as i64))
                && self.fiefs[c].team != self.persos[i].team
                && castle.visible();
            s.enemy_castle_cursor += 1;
            if hit {
                found = Some(c);
                break;
            }
        }
        found
    }

    /// `Trouver_Ennemi`: the nearest attackable enemy of perso `i`, from (x, y) or (with -1, -1)
    /// from its position.
    pub fn nearest_enemy(&self, i: usize, x: i64, y: i64) -> Option<usize> {
        let (x, y) = if x == -1 && y == -1 { self.pos(i) } else { (x, y) };
        let team = self.persos[i].team;
        let mut d = self.diagonal();
        let mut best = None;
        for (j, q) in self.persos.iter().enumerate() {
            if q.team != team && q.attackable() {
                let t = clng(dist((x, y), (q.x, q.y)));
                if d > t {
                    d = t;
                    best = Some(j);
                }
            }
        }
        best
    }

    /// `Trouver_MaisonEnnemi`: the nearest visible enemy house.
    pub fn nearest_enemy_house(&self, i: usize) -> Option<usize> {
        let p = self.pos(i);
        let mut d = self.half_perimeter();
        let mut best = None;
        for (j, m) in self.houses.iter().enumerate() {
            let dj = dist(p, (m.x as i64, m.y as i64));
            if self.persos[j].team != self.persos[i].team && m.visible() && d as f64 > dj {
                d = clng(dj);
                best = Some(j);
            }
        }
        best
    }

    /// `Trouver_ChateauEnnemi`: the nearest visible enemy castle.
    pub fn nearest_enemy_castle(&self, i: usize) -> Option<usize> {
        let p = self.pos(i);
        let mut d = self.half_perimeter();
        let mut best = None;
        for (j, c) in self.castles.iter().enumerate() {
            let dj = dist(p, (c.x as i64, c.y as i64));
            if self.persos[i].team != self.fiefs[j].team && c.visible() && d as f64 > dj {
                d = clng(dj);
                best = Some(j);
            }
        }
        best
    }

    /// `Trouver_Allie`: the nearest ally that is not an invocation (`i` included), alive only
    /// with `alive`, dead only with `dead`.
    fn nearest_ally(&self, i: usize, alive: bool, dead: bool) -> Option<usize> {
        let p = &self.persos[i];
        let mut d = self.diagonal();
        let mut best = None;
        for (j, q) in self.persos.iter().enumerate() {
            if q.team == p.team && !q.invocation() && (!alive || q.alive()) && !(dead && q.alive()) {
                let t = clng(dist((p.x, p.y), (q.x, q.y)));
                if d > t {
                    d = t;
                    best = Some(j);
                }
            }
        }
        best
    }

    /// `IA_Trouver_Ennemi`: attack the nearest enemy, or send the soldiers.
    pub fn attack_nearest_enemy(&mut self, rules: &Rules, i: usize, send: bool, ctrl: bool, shift: bool) {
        if let Some(k) = self.nearest_enemy(i, -1, -1) {
            if send {
                self.order_charge(rules, i, 0, k as i64, ctrl, shift, false);
            } else {
                self.attack_perso(rules, i, k);
            }
        }
    }

    /// `IA_Trouver_MaisonEnnemi`
    pub fn attack_nearest_house(&mut self, rules: &Rules, i: usize, send: bool, ctrl: bool, shift: bool) {
        if let Some(k) = self.nearest_enemy_house(i) {
            if send {
                self.order_charge(rules, i, 1, k as i64, ctrl, shift, false);
            } else {
                self.attack_house(rules, i, k);
            }
        }
    }

    /// `IA_Trouver_ChateauEnnemi`
    pub fn attack_nearest_castle(&mut self, rules: &Rules, i: usize, send: bool, ctrl: bool, shift: bool) {
        if let Some(k) = self.nearest_enemy_castle(i) {
            if send {
                self.order_charge(rules, i, 2, k as i64, ctrl, shift, false);
            } else {
                self.attack_castle(rules, i, k);
            }
        }
    }

    /// `IA_Trouver_Allie`: walk to the nearest such ally (`EnDeplacement` tells whether it is
    /// already there) and target it.
    fn go_to_ally(&mut self, rules: &Rules, i: usize, alive: bool, dead: bool) -> bool {
        let Some(k) = self.nearest_ally(i, alive, dead) else { return false };
        let (p, q) = (&self.persos[i], &self.persos[k]);
        let (qx, qy) = (q.x, q.y);
        if (p.x - qx).abs() < p.w as i64 && (p.y - qy).abs() < p.h as i64 {
            self.persos[i].moving = false;
        } else {
            self.walk_to(rules, i, qx, qy);
            self.persos[i].moving = true;
        }
        self.persos[i].perso_target = k;
        true
    }

    /// `IA_Chercher_Ressource`: harvest the nearest node yielding resource `r` (nothing for a
    /// negative index, where the original fails).
    fn gather_resource(&mut self, rules: &Rules, i: usize, r: i64) {
        if let Some(res) = usize::try_from(r).ok().and_then(|r| rules.resources.get(r)) {
            self.harvest_nearest(rules, i, res.terrain_resource);
        }
    }

    /// `IA_Trouver_Ressource`: the nearest node of terrain resource type `t`; node 0 when none
    /// matches.
    pub fn harvest_nearest(&mut self, rules: &Rules, i: usize, t: i64) {
        let p = self.pos(i);
        let mut d = self.half_perimeter();
        let mut best = 0;
        for (j, n) in self.resources.iter().enumerate() {
            let dj = dist(p, (n.x as i64, n.y as i64));
            if n.kind as i64 == t && d as f64 > dj {
                d = clng(dj);
                best = j;
            }
        }
        if !self.resources.is_empty() {
            self.go_harvest(rules, i, best);
        }
    }

    /// `Trouver_Maison_Proche`: the next slice of the search for a house of the same fief in a
    /// growing box: one offering service `service`, a factory (`need` 5), a shop or factory
    /// selling food for bar `need` - 1 (`resource` 0, "any"), or one selling `resource`.
    fn find_house(&mut self, rules: &Rules, n: usize, resource: i64, need: i64, service: i64) -> Option<usize> {
        let ub = self.houses.len() - 1;
        let mut found = false;
        let mut k = None;
        let mut it = 0;
        while it <= self.ai.searches {
            let s = &mut self.ai.persos[n];
            if s.house_cursor > ub {
                s.house_distance += HOUSE_SEARCH_STEP;
                s.house_cursor = 0;
            }
            let c = s.house_cursor;
            let d = s.house_distance;
            let (m, p) = (&self.houses[c], &self.persos[n]);
            let same = self.persos[c].fief == p.fief;
            let near = (p.x - m.x as i64).abs() < d && (p.y - m.y as i64).abs() < d;
            let ready = m.built && m.stock() >= 1.0 && same && near;
            let product = rules.buildings[m.kind].product;
            if service > 0 {
                found = m.service as i64 == service && ready;
            } else if need == 5 {
                found = m.factory && ready;
            } else if m.shop || m.factory {
                if resource == 0 {
                    if product >= 0 && need < 5 && ready && (1..=4).contains(&need) {
                        found |= rules.resources[product as usize].gain[need as usize - 1] > 0.0;
                    }
                } else if product == resource && ready && d as f64 >= dist((p.x, p.y), (m.x as i64, m.y as i64)) {
                    found = true;
                }
            }
            if found {
                k = Some(c);
                s.house_distance = HOUSE_SEARCH_STEP;
                it = ub as i64;
            }
            s.house_cursor += 1;
            it += 1;
        }
        k.filter(|_| found)
    }

    /// `Trouver_Tresor_Proche`: a treasure within a random radius that grows each call.
    fn find_treasure_near(&mut self, i: usize) -> Option<usize> {
        let p = self.pos(i);
        let s = &mut self.ai.persos[i];
        s.treasure_distance = clng(s.treasure_distance as f64 + TREASURE_SIZE as f64 * self.rng.next() * 10.0);
        if s.treasure_distance > TREASURE_MAX {
            s.treasure_distance = 0;
            return None;
        }
        let k = self.treasures.iter().position(|t| s.treasure_distance as f64 >= dist(p, (t.x as i64, t.y as i64)))?;
        s.treasure_distance = 0;
        Some(k)
    }

    // ---------------------------------------------------------------- monsters and soldiers

    /// `IA_Recuperer_Equipement_Monstre`: a monster back in its castle repairs it once.
    fn monster_equipment(&mut self, i: usize) {
        if self.persos[i].in_own_castle && !self.ai.persos[i].leave {
            self.restore_castle(i);
            self.ai.persos[i].leave = true;
        }
    }

    /// `IA_Comportement_Monstre`
    fn monster(&mut self, rules: &Rules, i: usize) {
        self.pursue(rules, i);
        if self.persos[i].idle() {
            self.recover_simple(rules, i);
            if self.persos[i].idle() {
                self.aggressive(rules, i);
            }
        } else if self.rng.next() > 0.99 {
            self.stop(rules, i);
        }
    }

    /// `IA_Poursuivre_Fuyard`: a perso attacking a building turns on its last target perso when
    /// that one is an attackable enemy.
    fn pursue(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let t = p.perso_target;
        let q = &self.persos[t];
        if (p.action == 301 || p.action == 302) && q.attackable() && p.team != q.team {
            self.attack_perso(rules, i, t);
            self.say(rules, i, 16);
        }
    }

    /// `IA_Agressif`: one random move: go home or follow the chef, wander, or (if not pacifist)
    /// attack a castle, a house or a perso found by the incremental searches.
    fn aggressive(&mut self, rules: &Rules, i: usize) {
        let r = self.rng.next();
        let pacific = self.persos[i].ai_pacific;
        if pacific {
            if r < 0.1 {
                self.go_home_or_chef(rules, i);
            } else if r < 0.2 {
                self.random_walk(rules, i);
            }
        } else if r < 0.05 {
            if let Some(k) = self.search_enemy_castle(rules, i) {
                self.attack_castle(rules, i, k);
            }
        } else if r < 0.1 {
            self.go_home_or_chef(rules, i);
        } else if r < 0.2 {
            self.random_walk(rules, i);
        } else if r < 0.4 {
            if let Some(k) = self.search_enemy_house(i) {
                self.attack_house(rules, i, k);
            }
        } else if let Some(k) = self.search_enemy(i) {
            self.attack_perso(rules, i, k);
        }
    }

    fn go_home_or_chef(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if p.invocation() {
            let c = &self.persos[p.chef as usize];
            let (x, y) = (c.dest.0 + p.chef_offset.0, c.dest.1 + p.chef_offset.1);
            self.walk_to(rules, i, x, y);
        } else {
            self.ai_go_castle(rules, i, false);
        }
    }

    /// `IA_Comportement_Soldats`: an idle soldier without an `.ia` file, by attitude.
    fn soldier(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        let chef = p.chef as usize;
        let fighting = p.fighting();
        let post = p.chef_post;
        match p.attitude {
            0 => {
                if !fighting {
                    let c = &self.persos[chef];
                    let (x, y) = (c.dest.0 + p.chef_offset.0, c.dest.1 + p.chef_offset.1);
                    self.walk_to(rules, i, x, y);
                }
            }
            1 => {
                if !fighting {
                    self.walk_to(rules, i, post.0, post.1);
                }
            }
            2 => match self.nearest_enemy(i, post.0, post.1) {
                Some(j) if dist(post, self.pos(j)) <= 384.0 => self.attack_perso(rules, i, j),
                _ => self.walk_to(rules, i, post.0, post.1),
            },
            3 => {
                if !fighting
                    && let Some(j) = self.nearest_enemy(i, -1, -1)
                    && dist(self.pos(i), self.pos(j)) <= ENEMY_MAX as f64
                {
                    self.attack_perso(rules, i, j);
                }
            }
            4 => self.berserk(rules, i),
            _ => {}
        }
    }

    /// `IA_Garder_Perso`: guard perso `IA_Cible`, attacking its assailants.
    fn guard(&mut self, rules: &Rules, i: usize) {
        if self.persos[i].fighting() {
            return;
        }
        let t = self.persos[i].ai_target;
        if self.persos.get(t).is_some_and(|q| q.alive()) {
            match self.search_assailant(i, t) {
                Some(k) => self.attack_perso(rules, i, k),
                None => self.patrol_circle(rules, i, t),
            }
        } else {
            self.persos[i].ai_order = 0;
        }
    }

    /// `IA_Recuperation_CaracSimple`: eat, then fetch grass or fruit, or ammunition.
    fn recover_simple(&mut self, rules: &Rules, i: usize) {
        let p = &mut self.persos[i];
        if !p.ai_eat {
            return;
        }
        for bar in 0..4 {
            p.eat_auto(rules, bar);
        }
        if p.idle() {
            if p.bar_total(0) < p.bonus_max[0] as f64 {
                self.gather_resource(rules, i, 0);
                return;
            }
            if p.bar_total(1) < p.bonus_max[1] as f64 * 0.5 {
                self.gather_resource(rules, i, 1);
                return;
            }
        }
        let p = &self.persos[i];
        if p.ammo >= 0 && p.res(p.ammo) < p.ammo_quantity {
            self.gather_resource(rules, i, p.ammo);
        }
    }

    // ---------------------------------------------------------------- berserk

    /// `IA_ComportementBerserk`: run every tick by `Deplacer_Persos` for living persos with
    /// `IA_Berserk > 0`, and by soldiers in attitude 4.
    pub(crate) fn berserk(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if p.fighting() && p.step != 0 {
            return;
        }
        let (chef, fighting) = (p.chef, p.fighting());
        match p.ia_berserk() {
            0 | 1 => self.seek_and_destroy(rules, i, false),
            2 => {
                if !fighting && chef >= 0 {
                    let chef = chef as usize;
                    match self.nearest_enemy(i, -1, -1) {
                        Some(j) if dist(self.pos(chef), self.pos(j)) < 384.0 => self.attack_perso(rules, i, j),
                        _ => self.patrol_circle(rules, i, chef),
                    }
                }
            }
            3 => {
                if self.go_to_ally(rules, i, false, true) {
                    let p = &self.persos[i];
                    if !p.moving {
                        if rules.races[p.race].auto_resurrection {
                            let effect = p.attack_effect(rules);
                            self.resurrect(rules, p.perso_target, effect);
                        }
                        if self.persos[i].kamikaze {
                            self.kill(rules, i);
                        }
                    }
                } else if chef >= 0 {
                    self.patrol_circle(rules, i, chef as usize);
                }
            }
            4 => {
                if chef > -1 && self.persos[chef as usize].alive() && self.persos[i].money() > 0.0 {
                    let p = &mut self.persos[i];
                    p.selected_money = p.money();
                    for r in 0..p.resources.len() {
                        p.select_resource(r, 0.0, true, false);
                    }
                    p.take_first_object();
                    self.go_give(i, chef as usize);
                }
                if !self.persos[i].moving {
                    let _ = self.find_treasure(rules, i);
                }
                if !self.persos[i].moving {
                    self.random_walk(rules, i);
                }
            }
            5 => self.berserk_guard_house(rules, i),
            6 => self.berserk_caravan(rules, i),
            _ => {}
        }
    }

    /// Berserk 0 and 1: attack the nearest enemy, else castle, else house; wander otherwise.
    /// Mode 5 (`guarded`) only wanders when not fighting.
    fn seek_and_destroy(&mut self, rules: &Rules, i: usize, guarded: bool) {
        self.attack_nearest_enemy(rules, i, false, false, false);
        if !self.persos[i].fighting() {
            self.attack_nearest_castle(rules, i, false, false, false);
            if !self.persos[i].fighting() {
                self.attack_nearest_house(rules, i, false, false, false);
                let p = &self.persos[i];
                if !p.fighting() && p.kamikaze {
                    self.kill(rules, i);
                }
                let p = &self.persos[i];
                if if guarded { !p.fighting() } else { !p.moving } {
                    self.random_walk(rules, i);
                }
            }
        }
    }

    /// Berserk 5: defend its building from inside, shooting at enemies in range.
    fn berserk_guard_house(&mut self, rules: &Rules, i: usize) {
        let p = &mut self.persos[i];
        if p.idle() && p.bar_total(0) < p.bonus_max[0] as f64 && p.ai_eat {
            p.eat_auto(rules, 0);
            if !p.fighting() && p.idle() {
                self.gather_resource(rules, i, 0);
            }
        }
        let p = &self.persos[i];
        if p.idle() && p.res(p.ammo) < p.ammo_quantity {
            let ammo = p.ammo;
            if self.poor(i) {
                self.gather_resource(rules, i, ammo);
            } else if let Some(x) = self.find_house(rules, i, ammo, 0, 0) {
                self.ai_go_house(rules, i, x);
            } else {
                self.gather_resource(rules, i, ammo);
            }
        }
        if !self.persos[i].idle() {
            return;
        }
        if self.houses[i].visible() {
            if !self.persos[i].in_own_house {
                self.go_home(rules, i, i, false);
            } else {
                let p = &self.persos[i];
                match p.equipped.first().copied().flatten() {
                    Some(it) if rules.objects[it.object].range_bonus != 0.0 => {}
                    _ => self.ai_equip(rules, i),
                }
                let p = &self.persos[i];
                if p.attack == p.bonus_max[4] as f64 && p.projectiles == 0 {
                    self.shoot_nearest_enemy(i);
                    if self.persos[i].idle() && self.houses[i].life_with_repair() < self.houses[i].max_life as f64 {
                        self.go_home(rules, i, i, false);
                    }
                }
            }
        } else {
            self.seek_and_destroy(rules, i, true);
        }
    }

    /// `IA_Lancer_projectile`: shoot at the nearest enemy within twice the range.
    fn shoot_nearest_enemy(&mut self, n: usize) {
        if let Some(k) = self.nearest_enemy(n, -1, -1)
            && dist(self.pos(k), self.pos(n)) <= self.persos[n].bonus_range * 2.0
        {
            let q = &self.persos[k];
            let (x, y) = (clng(q.x as f64 + q.w as f64 / 2.0), clng(q.y as f64 + q.h as f64 / 2.0));
            self.attack_ground(n, x, y, 0);
        }
    }

    /// Berserk 6: a caravan going between its house and the markets or the castle, earning
    /// money by the distance travelled.
    fn berserk_caravan(&mut self, rules: &Rules, i: usize) {
        let p = &self.persos[i];
        if p.idle() && p.res(p.ammo) < p.ammo_quantity {
            let ammo = p.ammo;
            self.gather_resource(rules, i, ammo);
            self.gather_resource(rules, i, ammo);
        }
        let p = &mut self.persos[i];
        if !p.idle() {
            return;
        }
        let nf = p.fief;
        if p.bar_total(0) < p.bonus_max[0] as f64 {
            if p.ai_eat {
                p.eat_auto(rules, 0);
                if !p.fighting() && p.idle() {
                    self.gather_resource(rules, i, 0);
                }
            }
        } else if !self.houses[i].visible() {
            self.kill(rules, i);
        } else if self.houses[i].life_with_repair() < self.houses[i].max_life as f64 {
            if self.rng.next() < 0.01 {
                self.go_home(rules, i, i, false);
            }
        } else if p.in_own_castle || p.in_house {
            let m = &self.houses[i];
            let j = (dist((m.x as i64, m.y as i64), (p.x, p.y)) / 12.0).floor();
            p.set_money(p.money() + j);
            let (x, y, chef) = (p.x as f64 + p.w as f64 / 2.0, p.y as f64, p.chef);
            self.number(x, y, j, NumberKind::Money, false);
            if let Some(c) = usize::try_from(chef).ok().filter(|&c| self.persos[c].alive()) {
                let c = &mut self.persos[c];
                c.set_money(c.money() + j);
                let (x, y) = (c.x as f64 + c.w as f64 / 2.0, c.y as f64);
                self.number(x, y, j, NumberKind::Money, false);
            }
            self.go_home(rules, i, i, false);
        } else if p.in_own_house {
            if self.rng.next() < 0.005 {
                let markets = self.fiefs[nf].markets.len();
                let r = self.rng.next();
                if self.castles[nf].visible() && r <= 1.0 / (markets + 1) as f64 {
                    self.go_own_castle(rules, i);
                } else {
                    let j = self.rng.below(markets as i64) as usize;
                    // The original raises an error when there is no market.
                    if let Some(&k) = self.fiefs[nf].markets.get(j)
                        && k != i
                    {
                        let repair = self.rng.next() < REPAIR_CHANCE;
                        self.go_to_house(rules, i, k, repair);
                    }
                }
            }
        } else {
            self.go_home(rules, i, i, false);
        }
    }

    /// `Ressusciter_Perso`: back to life with 5 % of its Vie, the rest healing over time.
    pub fn resurrect(&mut self, rules: &Rules, i: usize, effect: i64) {
        self.revive(rules, i);
        let p = &mut self.persos[i];
        p.bars[0] = p.bonus_max[0] as f64 * 0.05;
        p.recovery[0] = p.bonus_max[0] as f64 * (1.0 - 0.05);
        self.effect_on(effect, i);
    }

    // ---------------------------------------------------------------- VBScript AIs

    /// `Run "Jouer"` of a VBScript AI. Only `IAScripts\Test.txt` is shipped: it is implemented
    /// natively (wander, saying where to).
    // TODO: other scripts need the `ClSurPerso` object model (spec 04 §5)
    fn ai_script(&mut self, rules: &Rules, i: usize) {
        if self.persos[i].script_file.eq_ignore_ascii_case("Test.txt") && self.persos[i].idle() {
            self.random_walk(rules, i);
            let p = &mut self.persos[i];
            let text = format!("Je vais à X={}, Y= {}", p.dest.0, p.dest.1);
            p.set_comment(text, self.tick);
        }
    }
}
