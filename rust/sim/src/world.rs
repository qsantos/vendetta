//! World state, world creation (random generation or map files) and the tick.

use serde::{Deserialize, Serialize};
use crate::ai::Ai;
use crate::building::Building;
use crate::event::Event;
use crate::fief::Fief;
use crate::files::Files;
use crate::ini::Ini;
use crate::perso::{Perso, action};
use crate::rules::Rules;
use crate::terrain::{CELL, Terrain};
use crate::treasure::{Decor, Treasure};
use crate::vb::{Rnd, clng};

pub use crate::orders::Order;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub x: i32,
    pub y: i32,
    /// Index into `Rules::terrain_resources`, also the sprite column.
    pub kind: usize,
}

pub const RESOURCE_SIZE: i32 = 32;

#[derive(Serialize, Deserialize)]
pub struct World {
    pub rng: Rnd,
    pub terrain: Terrain,
    pub fiefs: Vec<Fief>,
    /// Same index as `fiefs`.
    pub castles: Vec<Building>,
    pub resources: Vec<ResourceNode>,
    pub persos: Vec<Perso>,
    /// `Maisons(i)` belongs to `persos[i]`.
    pub houses: Vec<Building>,
    pub treasures: Vec<Treasure>,
    pub decors: Vec<Decor>,
    pub tick: u64,
    /// `TempVitesse`: phase of the slow tick (every 10th tick).
    pub slow_phase: u32,
    /// `Jeu.Vitesse`: ticks per second, for decor lifetimes.
    pub speed: i64,
    /// `VitesseEpoque_Coef`: multiplies the age durations.
    pub epoch_speed: f64,
    /// `Jeu.Confrontation`: random treasures and respawns.
    pub confrontation: bool,
    /// `NoPerso`: the local player's character, which always says its comments.
    pub player: Option<usize>,
    /// `ModIA` state: the round robin and the per-character search state.
    pub ai: Ai,
    /// `Victoire(0)`: the local player's team has won (standard victory).
    pub victory: bool,
    /// What this tick produced for the client to show, cleared when the next one starts.
    #[serde(skip)]
    pub events: Vec<Event>,
}

/// One fief of the setup screen.
#[derive(Debug, Clone)]
pub struct FiefSetup {
    pub people: usize,
    pub team: i64,
    /// 0-based age.
    pub epoch: usize,
    /// Number of characters created for this fief at the start.
    pub citizens: usize,
}

/// What the setup screen chooses for a custom world ("Monde personnalisé").
#[derive(Debug, Clone)]
pub struct WorldSetup {
    pub seed: u32,
    /// Map size in cells, ignored when loading a map.
    pub width: usize,
    pub height: usize,
    pub world_type: usize,
    pub fiefs: Vec<FiefSetup>,
    /// Number of resource nodes is `resources + 1`, like `ReDim Ressources(n)`.
    pub resources: usize,
    /// Map folder under `Cartes/`, or None for a random world.
    pub map: Option<String>,
    /// `RessourcesDepartsChoisie`: index into `Rules::start_resources`.
    pub start_resources: usize,
    /// `VitesseEpoqueChoisie`: index into `Rules::epoch_speeds`.
    pub epoch_speed: usize,
    /// `DifficulteNCoefVieChateaux`
    pub castle_life_coef: f64,
    /// `Jeu.ini Vitesse`
    pub speed: i64,
    pub confrontation: bool,
}

impl Default for WorldSetup {
    /// The original's defaults (`Partie.ini`, `Jeu.ini`) where they exist.
    fn default() -> WorldSetup {
        WorldSetup {
            seed: 0,
            width: 128,
            height: 128,
            world_type: 1,
            fiefs: (0..4).map(|i| FiefSetup { people: i, team: i as i64 + 1, epoch: 0, citizens: 10 }).collect(),
            resources: 50,
            map: None,
            start_resources: 0,
            epoch_speed: 2,
            castle_life_coef: 1.0,
            speed: 10,
            confrontation: false,
        }
    }
}

impl World {
    /// `Commencer` for "NouveauMonde", then the start of `MoteurJeu`.
    pub fn create(rules: &Rules, files: &Files, setup: &WorldSetup) -> Result<World, String> {
        let mut world = World {
            rng: Rnd::new(setup.seed),
            terrain: Terrain::new(setup.width, setup.height),
            fiefs: setup.fiefs.iter().map(|f| Fief::new(rules, f.people, f.team, f.epoch, f.citizens)).collect(),
            castles: Vec::new(),
            resources: Vec::new(),
            persos: Vec::new(),
            houses: Vec::new(),
            treasures: Vec::new(),
            decors: Vec::new(),
            tick: 0,
            slow_phase: 0,
            speed: setup.speed,
            epoch_speed: rules.epoch_speeds.get(setup.epoch_speed).map(|e| e.coef).unwrap_or(1.0),
            confrontation: setup.confrontation,
            player: None,
            ai: Ai::default(),
            victory: false,
            events: Vec::new(),
        };
        world.create_citizens(rules);
        world.castles = world
            .fiefs
            .iter()
            .map(|f| {
                let p = &rules.peoples[f.people];
                Building {
                    w: p.castle_size.0,
                    h: p.castle_size.1,
                    entrance: p.castle_entrance,
                    max_life: p.castle_max_life,
                    ..Building::new(rules)
                }
            })
            .collect();
        match &setup.map {
            Some(map) => {
                let dir = format!("Cartes/{map}");
                world.load_terrain(rules, files, &dir)?;
                world.load_castles(rules, files, &dir)?;
                if !world.load_resources(rules, files, &dir)? {
                    let n = (world.buildable_cells(rules) as f64 / 300.0).round_ties_even() as usize;
                    world.resources = vec![ResourceNode { x: 0, y: 0, kind: 0 }; n + 1];
                    world.place_resources(rules);
                }
                world.init_castles(rules);
            }
            None => {
                world.resources = vec![ResourceNode { x: 0, y: 0, kind: 0 }; setup.resources + 1];
                world.generate_terrain(rules, setup.world_type);
                world.place_castles_randomly();
                world.init_castles(rules);
                world.build_roads(rules, setup.world_type);
                world.place_resources(rules);
            }
        }
        world.houses = (0..world.persos.len()).map(|i| Building { owner: i, ..Building::new(rules) }).collect();
        world.start_resources(rules, setup.start_resources);
        world.place_persos();
        // The local player takes over the first citizen of fief 0.
        if let Some(i) = world.persos.iter().position(|p| p.fief == 0) {
            world.persos[i].player = true;
            world.player = Some(i);
        }
        // Every character (the future player too: `Joueur` is still 0) gets an AI, and those of
        // a race with `IA_Selectionner` draw an AI file of the root folder.
        let choices: Vec<&str> = rules.ais.iter().filter(|a| a.player).map(|a| a.name.as_str()).collect();
        for p in &mut world.persos {
            p.ai = !p.projectile;
            if !p.choose_ai && rules.races[p.race].ai_select {
                // `NiveauIA = 1 > Rnd` always holds, but draws.
                if 1.0 > world.rng.next() {
                    p.choose_ai = true;
                    let k = world.rng.below(choices.len() as i64);
                    p.ai_file = choices.get(k as usize).map(|n| n.to_string()).unwrap_or_default();
                }
            }
            p.update_bonus(rules);
        }
        if let Some(i) = world.player {
            world.persos[i].ai = false;
        }
        for c in &mut world.castles {
            c.max_life = clng(c.max_life as f64 * setup.castle_life_coef);
            c.set_life(c.max_life as f64);
        }
        // TODO: difficulty XP bonus for NPCs (`BonusXP`, `Gestion_Tous_Niveaux`)
        for i in 0..world.persos.len() {
            let mut p = std::mem::replace(&mut world.persos[i], Perso::new(rules, &mut Rnd::default(), 0, 0, false));
            p.pick_look(rules, &mut world.rng);
            // Perso_Finalier: an unknown AI file disables the scripted AI.
            if p.choose_ai {
                p.ai_index = crate::ia_file::find(&rules.ais, &p.ai_file);
                if p.ai_index == -1 {
                    p.choose_ai = false;
                }
            }
            world.persos[i] = p;
        }
        // The race's free talents (`Changer_Race_Personnage_Equipement` for the NPCs; the player
        // gets the same ones as its preselected talents, `Charger_Sauvegarde_Tresor`).
        for i in 0..world.persos.len() {
            if world.player == Some(i) {
                world.persos[i].load_talents(rules);
            } else {
                world.persos[i].give_talents(rules);
            }
            world.special_uses();
        }
        for p in &mut world.persos {
            p.update_bonus(rules);
        }
        for f in 0..world.fiefs.len() {
            world.markets_refresh(f);
        }
        world.ai_init(rules);
        Ok(world)
    }

    /// Characters of every fief, in fief order (`Commencer`, "NouveauMonde").
    fn create_citizens(&mut self, rules: &Rules) {
        for f in 0..self.fiefs.len() {
            let (people, team) = (self.fiefs[f].people, self.fiefs[f].team);
            for _ in 0..self.fiefs[f].citizens {
                let mut p = Perso::new(rules, &mut self.rng, f, team, false);
                p.numero = self.persos.len();
                p.perso_target = self.persos.len();
                let (race, ai_file) = self.draw_race(rules, people);
                p.change_race(rules, &mut self.rng, race, false);
                if !ai_file.is_empty() {
                    p.ai_file = ai_file;
                    p.choose_ai = true;
                }
                self.persos.push(p);
            }
        }
    }

    /// Fief stocks and `Perso_Charger_Ressources_Depart`: the preset's starting resources for
    /// everything available in the fief's age (building materials only for characters).
    fn start_resources(&mut self, rules: &Rules, preset: usize) {
        let Some(s) = rules.start_resources.get(preset) else { return };
        for f in &mut self.fiefs {
            for (r, res) in rules.resources.iter().enumerate() {
                if res.epoch <= f.epoch as i64 {
                    f.set_resource(r, s.castle);
                }
            }
        }
        for p in &mut self.persos {
            if !rules.races[p.race].no_start_resources {
                let epoch = self.fiefs[p.fief].epoch as i64;
                for (r, res) in rules.resources.iter().enumerate() {
                    if res.epoch <= epoch && res.building_material {
                        p.set_resource(r, s.perso);
                    }
                    p.set_money(s.money);
                }
            }
            p.team = self.fiefs[p.fief].team;
        }
    }

    /// `Placer_Chateaux`, second half: castle HP from the people plus every age's bonus.
    fn init_castles(&mut self, rules: &Rules) {
        for (c, f) in self.castles.iter_mut().zip(&self.fiefs) {
            let p = &rules.peoples[f.people];
            c.max_life = p.castle_max_life;
            (c.w, c.h) = p.castle_size;
            c.entrance = p.castle_entrance;
            c.max_life += rules.epochs[..=f.epoch].iter().map(|e| e.castle_life_bonus).sum::<i64>();
            c.set_life(c.max_life as f64);
        }
    }

    /// `Tirer_Monstre_Aleatoire`: the race of a new citizen of this people, and the people's AI
    /// file for it.
    fn draw_race(&mut self, rules: &Rules, people: usize) -> (usize, String) {
        let monsters = &rules.peoples[people].monsters;
        if !monsters.is_empty() {
            for m in monsters {
                if self.rng.next() < m.chance && m.race != 0 {
                    return (m.race, m.ai_file.clone());
                }
            }
            let m = monsters.last().unwrap();
            return (m.race, m.ai_file.clone());
        }
        if self.rng.next() < rules.random_race_chance {
            let mut t = self.rng.below(rules.playable_races() as i64);
            for (r, race) in rules.races.iter().enumerate().skip(1) {
                if race.playable {
                    t -= 1;
                    if t <= 0 {
                        return (r, String::new());
                    }
                }
            }
        }
        (0, String::new())
    }

    /// `Placer_Joueurs`: everyone starts inside their castle.
    fn place_persos(&mut self) {
        for p in &mut self.persos {
            let c = &self.castles[p.fief];
            (p.x, p.y) = c.entrance_for(p.w, p.h);
            p.in_own_castle = true;
            p.dest = (p.x, p.y);
        }
    }

    /// Living characters of a fief.
    pub fn alive_citizens(&self, fief: usize) -> usize {
        self.persos.iter().filter(|p| p.fief == fief && p.alive()).count()
    }

    /// One simulation tick (`FrmMoteur2D.MoteurJeu`, solo branch).
    pub fn step(&mut self, rules: &Rules, orders: &[Order]) {
        self.events.clear();
        self.tick += 1;
        // Lire_Commandes: a failing command drops the rest of the queue.
        for order in orders {
            if self.apply_order(rules, order).is_err() {
                break;
            }
        }
        self.ai_tick(rules);
        self.fiefs_tick(rules);
        self.move_persos(rules);
        self.slow_phase += 1;
        if self.slow_phase >= 10 {
            self.slow_phase = 0;
            // Actions_Persos
            for i in 0..self.persos.len() {
                self.slow_tick(rules, i);
            }
            // Actions_Tresors
            if self.confrontation {
                self.treasure_generate(rules);
            }
        }
        self.special_uses();
        // TODO: scenario events (`Evenements_Lancer "Jouer"`)
        // Actualiser_Affichage_Batiments
        for m in &mut self.houses {
            if m.visible() {
                m.next_animation(rules);
            }
        }
        self.update_decors();
        // Victory (standard rules): every enemy fief has lost its castle and all its citizens.
        let team = self.persos.get(self.player.unwrap_or(0)).map_or(0, |p| p.team);
        self.victory = !self.confrontation
            && (self.fiefs.iter().zip(&self.castles))
                .all(|(f, c)| f.team == team || !(c.visible() || f.dead < f.citizens as i64));
        // TODO: defeat, scenario victories; the player's automatic actions (`Actions_Automatiques`)
        // Afficher_Commentaires: a line stays `Duree` seconds per character.
        let duration = rules.comment_duration * self.speed as f64;
        for p in &mut self.persos {
            if !p.comment.is_empty() && (self.tick - p.comment_tick) as f64 > p.comment.chars().count() as f64 * duration {
                p.comment.clear();
            }
        }
    }

    /// `Comportement_IA_Fiefs`: castle market, repairs and the age timer.
    fn fiefs_tick(&mut self, rules: &Rules) {
        for (f, c) in self.fiefs.iter_mut().zip(&mut self.castles) {
            f.transaction(rules);
            f.restoration(rules, c);
            c.restoration();
        }
    }

    /// `Deplacer_Persos`
    fn move_persos(&mut self, rules: &Rules) {
        for i in 0..self.persos.len() {
            if self.persos[i].ia_berserk() > 0 && self.persos[i].alive() {
                self.berserk(rules, i);
            }
            let p = &mut self.persos[i];
            p.quick_recovery(rules);
            // Deplacement1
            p.update_vitality();
            // TODO: tracking combat and giving targets (`EnCombat`, action 205)
            // Deplacement2: TODO acting on targets in range; otherwise Deplacement3
            self.move_step(rules, i);
            self.persos[i].advance_frame();
            let msg = self.work(rules, i);
            if let Some(m) = msg {
                if self.rng.next() > 0.99 || self.player == Some(i) {
                    self.say(rules, i, m as i64);
                }
            }
        }
    }

    /// `Deplacement3`, with its effects on the target house.
    fn move_step(&mut self, rules: &Rules, i: usize) {
        let ht = self.persos[i].house_target;
        let before = self.persos[i].action;
        match self.persos[i].step_move(rules, &self.terrain) {
            None => {}
            Some(true) => {
                if before == action::GO_BUILD && !self.houses[ht].visible() {
                    self.stop(rules, i);
                }
                self.houses[ht].under_construction = false;
            }
            Some(false) => {
                let p = &self.persos[i];
                if p.action == action::AT_HOME && p.craft_progress > 0.0 && self.can_craft(rules, i, p.craft_slot, ht) {
                    self.start_craft(i, p.craft_slot);
                }
            }
        }
    }

    /// `ClsJeuMonde.Generer_Monde`: random seeds, then cellular growth per terrain type.
    fn generate_terrain(&mut self, rules: &Rules, world_type: usize) {
        let wt = &rules.world_types[world_type];
        let n = (rules.terrains.len() - 1) as i64;
        let t = &mut self.terrain;
        let r = 1.0 - wt.relief;
        let mut k = 0;
        for _ in 0..=t.width {
            for _ in 0..=t.height {
                if self.rng.next() > r {
                    let mut c = clng((n - 1) as f64 * self.rng.next() + 1.0);
                    if c > n || wt.surface[c as usize] == 0 {
                        c = 0;
                    }
                    t.category[k] = c as u8;
                }
                k += 1;
            }
        }
        for ty in 0..=n as usize {
            for _ in 0..=wt.surface[ty] {
                self.grow(ty as u8);
            }
        }
        self.terrain.reshape();
    }

    /// `ClsJeuMonde.Faire_Pousser`: one in-place growth pass of terrain type `ty`.
    fn grow(&mut self, ty: u8) {
        let t = &mut self.terrain;
        let w = t.width as isize;
        let n = t.category.len() as isize;
        for j in 0..n {
            if t.shape[j as usize] != 0 {
                continue;
            }
            let is = |cat: &[u8], i: isize| i >= 0 && i < n && cat[i as usize] == ty;
            let mut ch = 0;
            for d in [-w, w, -1, 1] {
                if is(&t.category, j + d) {
                    ch += 15;
                }
            }
            if ch > 0 {
                for d in [-w - 1, -w + 1, w - 1, w + 1] {
                    if is(&t.category, j + d) {
                        ch += 10;
                    }
                }
            }
            if self.rng.next() * 100.0 < ch as f64 {
                t.category[j as usize] = ty;
            }
        }
    }

    /// `ModGenererMonde.Placer_Chateaux(True)` with `ClsPlaceObject`.
    fn place_castles_randomly(&mut self) {
        let nf = self.castles.len();
        if nf < 2 {
            return;
        }
        let lines = self.terrain.height as i64 - 9;
        let columns = self.terrain.width as i64 - 9;
        let objects = place_objects(&mut self.rng, lines, columns, nf);
        for (castle, (line, col)) in self.castles.iter_mut().zip(objects) {
            let jx = (8.0 - self.rng.next() * 16.0).floor() as i64;
            castle.x = (jx + col * 16) as i32;
            let jy = (8.0 - self.rng.next() * 16.0).floor() as i64;
            castle.y = (jy + line * 16) as i32;
        }
    }

    /// `ClsJeuMonde.Contruire_Routes2`: a ring of jittery roads between castles.
    fn build_roads(&mut self, rules: &Rules, world_type: usize) {
        let road = rules.world_types[world_type].road;
        if road == 0 {
            return;
        }
        // The original walks a throwaway character; its `Init` draws these.
        self.rng.next(); // NumeroPas
        self.rng.next(); // sex
        self.rng.next(); // name
        // That character's step is computed once, right after `Init`: acceleration from 0.
        let step = 1.0;
        let max_x = (self.terrain.width as i32 * CELL) as i64;
        let max_y = (self.terrain.height as i32 * CELL) as i64;
        let nf = self.castles.len();
        for i in 0..nf {
            let j = (i + 1) % nf;
            let (a, b) = (&self.castles[i], &self.castles[j]);
            let mut x = clng(a.x as f64 + a.w as f64 / 2.0);
            let mut y = clng(a.y as f64 + a.h as f64 / 2.0);
            let tx = clng(b.x as f64 + b.w as f64 / 2.0);
            let ty = clng(b.y as f64 + b.h as f64 / 2.0);
            // Orienter_Direction: not moving if already within half a step.
            let half = step / 2.0;
            let mut moving = (tx - x).abs() as f64 > half || (ty - y).abs() as f64 > half;
            loop {
                for (dx, dy) in [(0, 0), (8, 8), (16, 16)] {
                    self.set_cell_at(x + dx, y + dy, road);
                }
                if self.rng.next() > 0.8 {
                    if self.rng.next() > 0.5 {
                        x = if self.rng.next() > 0.5 { (x + 8).min(max_x) } else { (x - 8).max(0) };
                    } else {
                        y = if self.rng.next() > 0.5 { (y + 8).min(max_y) } else { (y - 8).max(0) };
                    }
                }
                // Marcher2
                if moving {
                    let (dx, dy) = ((tx - x) as f64, (ty - y) as f64);
                    let l = (dx * dx + dy * dy).sqrt();
                    if l >= step && l > 0.0 {
                        x = clng(step * dx / l + x as f64);
                        y = clng(step * dy / l + y as f64);
                    } else {
                        x = tx;
                        y = ty;
                        moving = false;
                    }
                }
                if !moving {
                    break;
                }
            }
            let a = &self.castles[i];
            let (ax, ay, aw, ah) = (a.x, a.y, a.w, a.h);
            let mut kx = ax - 16;
            while kx <= ax + aw + 16 {
                let mut ly = ay;
                while ly <= ay + ah {
                    self.set_cell_at(kx as i64, ly as i64, road);
                    ly += 16;
                }
                kx += 16;
            }
        }
        self.terrain.reshape();
    }

    /// Writes a cell by pixel position, ignoring out-of-range indices like the original's
    /// `On Error Resume Next`.
    fn set_cell_at(&mut self, x: i64, y: i64, cat: u8) {
        let k = self.terrain.index_at(x as f64, y as f64);
        if k >= 0 && (k as usize) < self.terrain.category.len() {
            self.terrain.category[k as usize] = cat;
        }
    }

    /// `ModGenererMonde.Placer_Ressources`
    fn place_resources(&mut self, rules: &Rules) {
        let w = (self.terrain.width as i32 * CELL - RESOURCE_SIZE) as f64;
        let h = (self.terrain.height as i32 * CELL - RESOURCE_SIZE) as f64;
        let n = rules.terrain_resources.len() as i64;
        for i in 0..self.resources.len() {
            let (x, y) = loop {
                let x = clng(self.rng.next() * w) as i32;
                let y = clng(self.rng.next() * h) as i32;
                if self.obstacle(rules, 0, x, y, RESOURCE_SIZE, RESOURCE_SIZE) == 0 {
                    break (x, y);
                }
            };
            let mut kind = clng(self.rng.next() * n as f64);
            if kind >= n {
                kind = 0;
            }
            self.resources[i] = ResourceNode { x, y, kind: kind as usize };
        }
    }

    /// `ClsJeuMonde.Constructible`
    pub fn buildable(&self, rules: &Rules, x: i32, y: i32, w: i32, h: i32) -> bool {
        let point = |x: i32, y: i32| {
            let t = self.terrain.index_at(x as f64, y as f64);
            t > 0
                && (t as usize) < self.terrain.category.len() - 1
                && !rules.terrains[self.terrain.category[t as usize] as usize].unbuildable
        };
        if w == 0 && h == 0 {
            return point(x, y);
        }
        let mut i = 0;
        while i <= w {
            let mut j = 0;
            while j <= h {
                if !point(x + i, y + j) {
                    return false;
                }
                j += CELL;
            }
            i += CELL;
        }
        true
    }

    fn buildable_cells(&self, rules: &Rules) -> usize {
        self.terrain
            .category
            .iter()
            .filter(|&&c| !rules.terrains[c as usize].unbuildable)
            .count()
    }

    /// `ClsJeuCartes.Charger_Monde`: one pixel per cell, colour → terrain type.
    fn load_terrain(&mut self, rules: &Rules, files: &Files, dir: &str) -> Result<(), String> {
        let img = decode_png(files, &format!("{dir}/Terrain.png"))?
            .ok_or_else(|| format!("missing {dir}/Terrain.png"))?;
        self.terrain = Terrain::new(img.width, img.height);
        for r in 0..img.height {
            for x in 0..img.width {
                let rgb = img.rgb(x, r);
                let ty = rules.map_terrain_colours.iter().position(|&c| c == rgb).unwrap_or(0);
                self.terrain.category[r * img.width + x] = ty as u8;
            }
        }
        self.terrain.reshape();
        Ok(())
    }

    /// `ClsJeuCartes.Charger_Chateaux`, then `Echanger_Emplacement_Chateaux`.
    fn load_castles(&mut self, rules: &Rules, files: &Files, dir: &str) -> Result<(), String> {
        let mut positions = Vec::new();
        let cached = files.text(&format!("{dir}/Informations.ini")).map(Ini::parse);
        let from_cache = cached.and_then(|ini| {
            let n = ini.get("Chateaux", "NombreChateaux")?;
            let n = crate::ini::vb_val(n) as usize;
            (1..=n)
                .map(|i| {
                    let x = ini.get("Chateaux", &format!("Chateau({i})_PositionX"))?;
                    let y = ini.get("Chateaux", &format!("Chateau({i})_PositionY"))?;
                    Some((crate::ini::vb_val(x) as i32, crate::ini::vb_val(y) as i32))
                })
                .collect::<Option<Vec<_>>>()
        });
        if let Some(cached) = from_cache {
            positions = cached;
        } else if let Some(img) = decode_png(files, &format!("{dir}/Chateaux.png"))? {
            let (w0, h0) = rules.peoples[0].castle_size;
            for r in 0..img.height {
                for x in 0..img.width {
                    if img.rgb(x, r) == [255, 255, 255] {
                        // The original's row counter is off by one compared with the terrain.
                        let px = clng(x as f64 * 16.0 - w0 as f64 / 2.0) as i32;
                        let py = clng((r + 1) as f64 * 16.0 - h0 as f64 / 2.0) as i32;
                        positions.push((px, py));
                    }
                }
            }
        }
        // The map decides the number of castles; extra fiefs from the setup are dropped and
        // missing ones copy the last fief.
        let template = self.castles.last().cloned();
        let fief_template = self.fiefs.last().cloned();
        self.castles.resize_with(positions.len(), || template.clone().unwrap());
        self.fiefs.resize_with(positions.len(), || fief_template.clone().unwrap());
        // The setup screen normally matches the fief count to the map; drop the rest.
        let n = self.fiefs.len();
        self.persos.retain(|p| p.fief < n);
        let mut remaining = positions;
        for c in self.castles.iter_mut() {
            let j = (self.rng.next() * remaining.len() as f64).floor() as usize;
            let (x, y) = remaining.remove(j);
            c.x = x;
            c.y = y;
        }
        Ok(())
    }

    /// `ClsJeuCartes.Charger_Ressources`; returns false when the map has no resource image.
    fn load_resources(&mut self, rules: &Rules, files: &Files, dir: &str) -> Result<bool, String> {
        let Some(img) = decode_png(files, &format!("{dir}/Ressources.png"))? else {
            return Ok(false);
        };
        let n = rules.terrain_resources.len() as i64;
        self.resources.clear();
        for r in 0..img.height {
            for x in 0..img.width {
                let rgb = img.rgb(x, r);
                if rgb == [0, 0, 0] {
                    continue;
                }
                let kind = if rgb == [255, 255, 255] {
                    let k = clng(self.rng.next() * n as f64);
                    if k >= n { 0 } else { k as usize }
                } else {
                    rules.map_resource_colours.iter().position(|&c| c == rgb).unwrap_or(0)
                };
                self.resources.push(ResourceNode {
                    x: x as i32 * 16,
                    y: (r as i32 + 1) * 16 - 32,
                    kind,
                });
            }
        }
        Ok(true)
    }
}

/// `ClsPlaceObject`: spreads `count` objects over a board, returns (line, column) pairs
/// (1-based cells).
fn place_objects(rng: &mut Rnd, lines: i64, columns: i64, count: usize) -> Vec<(i64, i64)> {
    let nf = count as f64;
    let min_space =
        ((lines + columns) as f64 / ((nf - 2.0).sqrt() + 1.0)).floor() as i64 - ((nf / nf.sqrt()).floor() as i64 + 2);

    // GetBoxSize
    let (mut box_lines, mut box_columns) = (lines, columns);
    let mut n = count as f64;
    while n > 1.0 {
        n = (n / 2.0 + 0.5).floor();
        if box_lines > box_columns {
            box_lines /= 2;
        } else {
            box_columns /= 2;
        }
    }

    // InitOjectPlace: one object per free box, sweeping row by row.
    let mut objects: Vec<(i64, i64)> = Vec::new();
    let (mut top, mut left) = (1, 1);
    let mut guard = 0;
    while objects.len() < count && guard < 100_000 {
        guard += 1;
        let occupied = objects.iter().any(|&(l, c)| {
            top <= l && l <= top + box_lines && left <= c && c <= left + box_columns
        });
        if !occupied {
            let l = (rng.next() * box_lines as f64 + top as f64).floor() as i64;
            let c = (rng.next() * box_columns as f64 + left as f64).floor() as i64;
            objects.push((l, c));
        }
        if left + 2 * box_columns - 1 <= columns {
            left += box_columns;
        } else {
            left = 1;
            top += box_lines;
        }
    }

    // MoveObjects: repulsion relaxation.
    let mut moved = true;
    let mut it = 0;
    while it < 10_000 && moved {
        moved = false;
        for j in 0..objects.len() {
            let (jl, jc) = objects[j];
            let mut pos = [[columns * lines; 3]; 3];
            for k in -1..=1 {
                for l in -1..=1 {
                    for &(ol, oc) in &objects {
                        if (oc, ol) != (jc, jl) {
                            let d = (jl + k - ol).abs() + (jc + l - oc).abs();
                            let p = &mut pos[(k + 1) as usize][(l + 1) as usize];
                            if d < min_space && d < *p {
                                *p = d;
                            }
                        }
                    }
                }
            }
            if pos[1][1] < min_space {
                moved = true;
                let mut v = pos[1][1];
                let mut target = (jl, jc);
                for k in -1..=1 {
                    for l in -1..=1 {
                        let p = pos[(k + 1) as usize][(l + 1) as usize];
                        if p >= v
                            && p > 0
                            && 1 <= jl + k
                            && jl + k <= lines
                            && 1 <= jc + l
                            && jc + l <= columns
                        {
                            let r = rng.next();
                            if (p == v && r < 0.5) || p > v {
                                v = p;
                                target = (jl + k, jc + l);
                            }
                        }
                    }
                }
                objects[j] = target;
            }
        }
        it += 1;
    }
    objects
}

/// `ModIA.Est_Superpose`, typos included (`Y + w` where `Y + h` was meant).
pub fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (x1, y1, w1, h1) = a;
    let (x2, y2, w2, h2) = b;
    if w1 == 0 && h1 == 0 {
        return x1 >= x2 && x1 <= x2 + w2 && y1 >= y2 && y1 <= y2 + h2;
    }
    ((x1 <= x2 && x2 <= x1 + w1 || x1 <= x2 + w2 && x2 + w2 <= x1 + w1)
        && (y1 <= y2 && y2 <= y1 + h1 || y1 <= y2 + h2 && y2 + h2 <= y1 + h1)
        || (x2 <= x1 && x1 <= x2 + w2 || x2 <= x1 + w1 && x1 + w1 <= x2 + w2)
            && (y2 <= y1 && y1 <= y2 + h2 || y2 <= y1 + h1 && y1 + h1 <= y2 + h2)
        || (x1 >= x2 && x1 + w1 <= x2 + w2 && y1 >= y2 && y1 + w1 <= y2 + w2)
        || (x2 >= x1 && x2 + w2 <= x1 + w1 && y2 >= y1 && y2 + w2 <= y1 + w1))
        || (x1 < x2 && x1 + w1 > x2 + w2 && y2 < y1 && y1 + h1 < y2 + h2
            || x2 < x1 && x2 + w2 > x1 + w1 && y1 < y2 && y2 + h2 < y1 + h1)
}

struct Rgb {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl Rgb {
    fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.width + x) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }
}

fn decode_png(files: &Files, path: &str) -> Result<Option<Rgb>, String> {
    let Some(bytes) = files.get(path) else {
        return Ok(None);
    };
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| format!("{path}: {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("{path}: {e}"))?;
    let (width, height) = (info.width as usize, info.height as usize);
    // normalise to RGBA8
    let data = match info.color_type {
        png::ColorType::Rgba => buf[..width * height * 4].to_vec(),
        png::ColorType::GrayscaleAlpha => buf[..width * height * 2]
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        other => return Err(format!("{path}: unexpected colour type {other:?}")),
    };
    Ok(Some(Rgb { width, height, data }))
}
