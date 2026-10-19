//! Characters ("persos"): state, creation and movement (`ClsJeuPerso`).

use crate::files::Files;
use crate::rules::Rules;
use crate::stats::{XP_COEFS, XP_OBJECT, XP_RESOURCE, XP_SERVICE, XP_SPECIAL};
use crate::terrain::{CELL, Terrain};
use crate::vb::{Rnd, clng};

/// Facing, as in the sprite sheets' rows (minus one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    None = 0,
    Up = 1,
    Right = 2,
    Down = 3,
    Left = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Look {
    /// One whole 3×4 sheet (`Images\Persos\...`).
    Sheet(String),
    /// Stacked 3×4 sheets (`Images\Apparences\...`), drawn in order.
    Layers(Vec<String>),
}

/// `Action` codes of the original state machine (only the ones implemented so far).
pub mod action {
    pub const IDLE: i32 = 0;
    pub const WALK: i32 = 1;
    /// Harvesting a resource node, and walking to one.
    pub const HARVEST: i32 = 2;
    pub const GO_HARVEST: i32 = 3;
    /// Building or repairing a house, and walking to one.
    pub const BUILD: i32 = 6;
    pub const GO_BUILD: i32 = 7;
    pub const AT_HOME: i32 = 100;
    pub const GO_HOME: i32 = 101;
    /// Producing a resource at home.
    pub const WORK_HOME: i32 = 102;
    pub const ENTER_HOUSE: i32 = 201;
    pub const GO_OWN_CASTLE: i32 = 202;
    pub const GO_CASTLE: i32 = 203;
    pub const GO_TREASURE: i32 = 204;
    pub const GO_GIVE: i32 = 205;
    pub const GO_SELL_CASTLE: i32 = 206;
    pub const GO_SELL_MARKET: i32 = 207;
    pub const RESURRECTING: i32 = 400;
    pub const CRAFT: i32 = 500;
    pub const SERVICE: i32 = 600;
}

/// `VitesseResurrectionDepart`
pub const RESURRECTION_SPEED: f64 = 2.5;

/// Experience towards the next level (`Exp`, `Niv`, `ExpNiv`, `ExpNivPlus`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Xp {
    pub exp: f64,
    pub level: i64,
    pub exp_level: f64,
    pub exp_next: f64,
}

/// A skill: the base value that levels up (`Carac`), its effective value (`BonusNiv`) and XP.
/// For resource skills `carac` is the carrying capacity (`MaxRessources`) and `bonus` is
/// derived from the level (`BonusNivRessources`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Skill {
    pub carac: i64,
    pub bonus: i64,
    pub xp: Xp,
}

/// An inventory slot (`ClsJeuObjet`, only what the economy needs).
// TODO: cached object parameters for equipment (items)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub object: usize,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct Perso {
    pub name: String,
    pub female: bool,
    pub race: usize,
    pub fief: usize,
    pub team: i64,
    /// Controlled by a local player (`Joueur > 0`).
    pub player: bool,
    /// `IA`: the AI drives this character.
    pub ai: bool,
    pub look: Look,
    /// `NumeroChef`: the chef of an invocation, -1 otherwise.
    pub chef: i64,
    pub emperor: bool,

    /// Vie, Energie, Magie, Moral.
    pub bars: [f64; 4],
    /// `RecuperationX`: pending healing from food, drained on slow ticks.
    pub recovery: [f64; 4],
    /// Combat bars `Attaque`, `Defense`.
    pub attack: f64,
    pub defense: f64,
    /// Base maxima (race base + levels): Vie, Energie, Magie, Moral, Attaque, Defense.
    pub max: [i64; 6],
    /// Effective maxima (`BonusMax*`): (base + items) × race coefficient.
    pub bonus_max: [i64; 6],
    pub bar_xp: [Xp; 6],
    pub vitality: f64,
    pub bonus_vitality: f64,
    /// `BonusVitesseRegeneration*`: share of the max regenerated per slow tick.
    pub bonus_regen: [f64; 4],
    pub gain_experience: bool,
    pub armour: i64,
    pub bonus_armour: i64,

    /// Special skills: 0 build, 1 demolish, 2 critical hit, 3 bargaining.
    pub special: Vec<Skill>,
    /// One per resource; `carac` is the carrying capacity.
    pub resource_skills: Vec<Skill>,
    /// Service skills (service s uses skill s-1).
    pub services: Vec<Skill>,
    /// Item skills.
    pub objects: Vec<Skill>,
    /// `BonusMaxRessourcesObjet`: race and item carrying bonus.
    pub bonus_max_resources: i64,
    pub(crate) money: f64,
    pub(crate) resources: Vec<f64>,
    /// `GenerationRessources`, `GainArgent`, `GainRessource`: item income per slow tick.
    pub generating: bool,
    pub gain_money: f64,
    pub gain_resources: Vec<f64>,

    /// What is held for giving, dropping or selling.
    pub selected_money: f64,
    pub selected_resources: Vec<f64>,
    pub resources_selected: bool,
    /// Object type held, -1 for none.
    pub selected_object: i64,
    pub inventory: Vec<Option<Item>>,
    // TODO: equipment (items)
    pub equipped: Vec<Option<Item>>,
    /// `IndiceObjet`: slot of the house's object list being crafted.
    pub craft_slot: i64,
    /// `EtatFabricationObjet`
    pub craft_progress: f64,

    /// Top-left corner, in pixels.
    pub x: i64,
    pub y: i64,
    pub w: i32,
    pub h: i32,
    /// Destination (`DirectionX/Y`).
    pub dest: (i64, i64),
    pub direction: Dir,
    pub moving: bool,
    /// Walk-cycle frame 0..3 (`NumeroPas`).
    pub frame: u8,
    /// Race speed (`LongueurPas`).
    pub speed: i64,
    pub bonus_move: i64,
    pub bonus_speed: i64,
    pub acceleration: i64,
    /// Current step length (`Longueur_Pas`) and its accelerated value (`Longueur_PasTemp`).
    pub step: i64,
    pub step_temp: i64,
    pub flying: bool,
    pub stun: i64,
    path: Vec<(i64, i64)>,
    path_dest: (i64, i64),
    path_complete: bool,

    pub action: i32,
    pub in_own_castle: bool,
    pub in_castle: bool,
    pub in_own_house: bool,
    pub in_house: bool,
    /// Targets: `IndiceMaison`, `IndiceChateau`, `IndiceRessource`, `IndiceTresor`, `IndicePerso`.
    pub house_target: usize,
    pub castle_target: usize,
    pub resource_target: usize,
    pub treasure_target: usize,
    pub perso_target: usize,
    /// `RessourceCourante`: resource being worked, -1 for none.
    pub current_resource: i64,
    /// `ServiceCourant`
    pub current_service: usize,

    /// `DureeRessurection`, `ResurrectionNombre` (-1 unlimited), `VitesseResurrection`.
    pub resurrection_time: f64,
    pub resurrections: i64,
    pub resurrection_speed: f64,
    /// `Morts`
    pub deaths: i64,
    /// `Commentaires`: the speech bubble ("" for none) and the tick it was said.
    pub comment: String,
    pub comment_tick: u64,
    /// `Parole`: may speak; `CommentairesSpecifiqueIndice`: comment type of every line, or -1.
    pub speaks: bool,
    pub special_comments: i64,

    /// `ChoisirIA`, `FichierIA`, `NumeroIA`: the scripted `.ia` AI (index into `Rules::ais`).
    pub choose_ai: bool,
    pub ai_file: String,
    pub ai_index: i64,
    /// `ChoisirIAScript`, `FichierIAScript`: a VBScript AI, which replaces all the others.
    pub choose_script: bool,
    pub script_file: String,
    /// `IA_Equiper`: 0 never, 1 by value, 2 by skill.
    pub ai_equip: i64,
    /// `IA_Manger`, `IA_Pacifique`, `IA_Temeraire`, `IA_Protecteur`, `IA_Egoiste`.
    pub ai_eat: bool,
    pub ai_pacific: bool,
    pub ai_bold: bool,
    pub ai_protector: bool,
    pub ai_selfish: bool,
    /// `Berserk` and its effective value `BonusBerserk` (`IA_Berserk`), 0 for none.
    pub berserk: i64,
    pub bonus_berserk: i64,
    /// `IA_Ordre` (0 none, 1 attack a perso, 2 a house, 3 a castle, 4 guard a perso), `IA_Cible`.
    pub ai_order: i64,
    pub ai_target: usize,
    /// Soldiers: `Attitude`, `Formation`, `ChefDecalageX/Y`, `ChefPositionX/Y`, `Incommandable`.
    pub attitude: i64,
    pub formation: i64,
    pub chef_offset: (i64, i64),
    pub chef_post: (i64, i64),
    pub incommandable: bool,
    /// `NombreSoldats`, `ProjectileEnCours`
    pub soldiers: i64,
    pub projectiles: i64,
    /// `Projectile`: a projectile has no AI.
    pub projectile: bool,
    pub kamikaze: bool,
    /// `Inattaquable`
    pub untargetable: bool,
    /// `PorteeAttaque` and `BonusPorteeAttaque`.
    pub range: f64,
    pub bonus_range: f64,
    /// `LanceProjectile`: the projectile race of the weapon, -1 for none.
    // TODO: set by equipped objects (items)
    pub launcher: i64,
    /// `CibleX/Y`, `ProjectileCible`: the ground target of action 303.
    pub ground_target: (i64, i64),
    pub ground_target_kind: i64,
    /// `DureeRechargeAttaque`, `DureeRechargeAttaqueMax`, `DureeRechargeAttaqueObjet`, `ForcerAttaque`.
    // TODO: attack cooldowns (combat)
    pub reload: i64,
    pub reload_max: i64,
    pub reload_object: i64,
    pub force_attack: bool,
    /// `PerteRessourceIndice`, `PerteRessourceQuantite`: ammunition of the equipped weapons.
    // TODO: set by equipped objects (items)
    pub ammo: i64,
    pub ammo_quantity: f64,
    /// `NumeroObjetUtilise`: an object waiting for `Gestion_Utilisation_Speciale`, -1 for none.
    pub used_object: i64,
}

fn skills(n: usize, carac: i64, k: f64) -> Vec<Skill> {
    vec![Skill { carac, bonus: 0, xp: Xp { exp_next: k, ..Xp::default() } }; n]
}

impl Perso {
    /// `Init` (with `Init_Carac`) followed by `Changer_Race_Personnage(0)`.
    pub fn new(rules: &Rules, rng: &mut Rnd, fief: usize, team: i64, player: bool) -> Perso {
        let frame = rng.below(3) as u8;
        let nr = rules.resources.len();
        let mut p = Perso {
            name: String::new(),
            female: false,
            race: 0,
            fief,
            team,
            player,
            ai: false,
            look: Look::Layers(Vec::new()),
            chef: -1,
            emperor: false,
            bars: [20.0; 4],
            recovery: [0.0; 4],
            attack: 20.0,
            defense: 20.0,
            max: [20; 6],
            bonus_max: [0; 6],
            bar_xp: XP_COEFS.map(|k| Xp { exp_next: k, ..Xp::default() }),
            vitality: 1.0,
            bonus_vitality: 1.0,
            bonus_regen: [0.0; 4],
            gain_experience: true,
            armour: 0,
            bonus_armour: 0,
            special: skills(rules.special_skills.len(), rules.start_special, XP_SPECIAL),
            resource_skills: skills(nr, rules.start_max_resources, XP_RESOURCE),
            services: skills(rules.services.len().saturating_sub(1), rules.start_service, XP_SERVICE),
            objects: skills(rules.item_skills.len(), rules.start_object, XP_OBJECT),
            bonus_max_resources: 0,
            money: 0.0,
            resources: vec![0.0; nr],
            generating: false,
            gain_money: 0.0,
            gain_resources: vec![0.0; nr],
            selected_money: 0.0,
            selected_resources: vec![0.0; nr],
            resources_selected: false,
            selected_object: -1,
            inventory: vec![None; rules.inventory_size],
            equipped: vec![None; rules.equipped_size],
            craft_slot: 0,
            craft_progress: 0.0,
            x: 0,
            y: 0,
            w: 24,
            h: 32,
            dest: (0, 0),
            direction: Dir::None,
            moving: false,
            frame,
            speed: 4,
            bonus_move: 4,
            bonus_speed: 0,
            acceleration: 1,
            step: 0,
            step_temp: 0,
            flying: false,
            stun: 0,
            path: Vec::new(),
            path_dest: (0, 0),
            path_complete: false,
            action: action::IDLE,
            in_own_castle: false,
            in_castle: false,
            in_own_house: false,
            in_house: false,
            house_target: 0,
            castle_target: fief,
            resource_target: 0,
            treasure_target: 0,
            perso_target: 0,
            current_resource: -1,
            current_service: 0,
            resurrection_time: 0.0,
            resurrections: -1,
            resurrection_speed: RESURRECTION_SPEED as f32 as f64,
            deaths: 0,
            comment: String::new(),
            comment_tick: 0,
            speaks: true,
            special_comments: -1,
            choose_ai: false,
            ai_file: String::new(),
            ai_index: -1,
            choose_script: false,
            script_file: String::new(),
            ai_equip: 0,
            ai_eat: false,
            ai_pacific: false,
            ai_bold: false,
            ai_protector: false,
            ai_selfish: false,
            berserk: 0,
            bonus_berserk: 0,
            ai_order: 0,
            ai_target: 0,
            attitude: 0,
            formation: 0,
            chef_offset: (0, 0),
            chef_post: (0, 0),
            incommandable: false,
            soldiers: 0,
            projectiles: 0,
            projectile: false,
            kamikaze: false,
            untargetable: false,
            range: 0.0,
            bonus_range: 0.0,
            launcher: -1,
            ground_target: (0, 0),
            ground_target_kind: 0,
            reload: 0,
            reload_max: 0,
            reload_object: 0,
            force_attack: false,
            ammo: 0,
            ammo_quantity: 0.0,
            used_object: -1,
        };
        p.change_race(rules, rng, 0, false);
        p.adjust_bonus(rules);
        p.update_vitality();
        p.stop();
        p.choose_ai = false;
        p
    }

    /// `Changer_Race_Personnage` (equipment left out).
    pub fn change_race(&mut self, rules: &Rules, rng: &mut Rnd, race: usize, no_equipment: bool) {
        let r = &rules.races[race];
        // The original keeps the ratios in Singles, and skips a bar whose max is 0.
        let ratio = |v: f64, m: i64| if m > 0 { (v / m as f64) as f32 as f64 } else { 0.0 };
        let mut ratios = [0.0; 6];
        for i in 0..4 {
            ratios[i] = ratio(self.bars[i], self.bonus_max[i]);
        }
        ratios[4] = ratio(self.attack, self.bonus_max[4]);
        ratios[5] = ratio(self.defense, self.bonus_max[5]);
        self.race = race;
        self.female = rng.next() > 0.5 && !r.no_female;
        if !self.player {
            self.name = if r.random_name {
                let names = &rules.names[self.female as usize];
                let last = names.len().saturating_sub(1);
                let i = clng(rng.next() * last as f64) as usize;
                names.get(i).cloned().unwrap_or_default()
            } else {
                r.name[self.female as usize].clone()
            };
        }
        self.w = r.size.0;
        self.h = r.size.1;
        self.max = r.base;
        self.gain_experience = r.experience;
        self.flying = r.flying;
        self.armour = r.armour;
        self.speed = r.speed;
        self.acceleration = r.acceleration;
        self.speaks = r.speaks;
        self.special_comments = r.special_comments;
        self.range = r.range;
        self.kamikaze = r.kamikaze;
        self.incommandable = r.incommandable;
        if !no_equipment {
            for (have, &start) in self.resources.iter_mut().zip(&r.resources) {
                if *have < start {
                    *have = start;
                }
            }
            // TODO: race equipment (`Race_Equipement`), then `Actualiser_Bonus`
        }
        // TODO: unequip mounts the race cannot ride
        self.adjust_bonus(rules);
        self.untargetable = r.untargetable;
        self.ai_equip = r.ai_equip;
        self.ai_eat = r.ai_eat;
        self.ai_pacific = r.ai_pacific;
        self.ai_bold = r.ai_bold;
        self.ai_protector = r.ai_protector;
        self.ai_selfish = r.ai_selfish;
        self.berserk = r.berserk;
        self.choose_ai = !r.ai_file.is_empty();
        self.ai_file = r.ai_file.clone();
        for i in 0..4 {
            if ratios[i] > 0.0 {
                self.bars[i] = self.bonus_max[i] as f64 * ratios[i];
            }
        }
        if ratios[4] > 0.0 {
            self.attack = self.bonus_max[4] as f64 * ratios[4];
        }
        if ratios[5] > 0.0 {
            self.defense = self.bonus_max[5] as f64 * ratios[5];
        }
    }

    /// `Tirer_Apparence_Au_Hasard`: a random sheet, or one random layer per category.
    pub fn pick_look(&mut self, rules: &Rules, files: &Files, rng: &mut Rnd) {
        let r = &rules.races[self.race];
        let sex = self.female as usize;
        let mut pick = |dir: String| -> Option<String> {
            let list = files.list_files(&dir);
            if list.is_empty() {
                return None;
            }
            let i = rng.below(list.len() as i64) as usize;
            Some(list[i].clone())
        };
        self.look = if r.whole_sheet {
            let dir = format!("Images/Persos/{}", r.sheet_folder[sex]);
            match pick(dir) {
                Some(f) => Look::Sheet(f),
                None => Look::Layers(Vec::new()),
            }
        } else {
            Look::Layers(
                r.layer_folders[sex]
                    .iter()
                    .filter_map(|f| pick(format!("Images/Apparences/{f}")))
                    .collect(),
            )
        };
    }

    pub fn alive(&self) -> bool {
        self.bars[0] > 0.0
    }

    pub fn visible(&self) -> bool {
        !(self.in_own_castle || self.in_castle || self.in_own_house || self.in_house)
    }

    pub fn set_visible(&mut self, v: bool) {
        self.in_own_castle = !v;
        self.in_castle = !v;
        self.in_own_house = !v;
        self.in_house = !v;
    }

    /// `Aller_A`
    pub fn go_to(&mut self, x: i64, y: i64) {
        self.dest = (x, y);
        self.orient(x, y);
        self.action = action::WALK;
        self.set_visible(true);
    }

    /// `Arreter` without its AI re-entry; see `World::stop`.
    pub fn stop(&mut self) {
        self.dest = (self.x, self.y);
        self.step_temp = 0;
        self.action = if self.in_own_house { action::AT_HOME } else { action::IDLE };
        self.current_resource = -1;
    }

    /// `Attaquable`
    pub fn attackable(&self) -> bool {
        self.alive() && self.visible() && !self.untargetable
    }

    /// `EnCombat`
    pub fn fighting(&self) -> bool {
        (300..400).contains(&self.action)
    }

    /// `IA_Berserk`
    pub fn ia_berserk(&self) -> i64 {
        self.bonus_berserk
    }

    /// `Definir_ressources` getter, which reads 0 for a negative index.
    pub fn res(&self, r: i64) -> f64 {
        usize::try_from(r).ok().and_then(|r| self.resources.get(r)).copied().unwrap_or(0.0)
    }

    /// `Definir_MaxRessources` getter, 0 for a negative index.
    pub fn max_res(&self, r: i64) -> f64 {
        usize::try_from(r).map_or(0.0, |r| self.max_resource(r))
    }

    /// `Definir_Commentaires` setter: a new line only replaces an empty one, and mute races
    /// never speak.
    pub fn set_comment(&mut self, text: String, tick: u64) {
        if (self.comment.is_empty() || text.is_empty()) && self.speaks {
            if !text.is_empty() {
                self.comment_tick = tick;
            }
            self.comment = text;
        }
    }

    /// `Hostile`
    pub fn hostile(&self) -> bool {
        self.team <= 0
    }

    /// `Invocation`
    pub fn invocation(&self) -> bool {
        self.chef >= 0
    }

    /// `Inactif`
    pub fn idle(&self) -> bool {
        self.action == action::IDLE || self.action == action::AT_HOME
    }

    /// Sets the destination and faces it (`DirectionX/Y = ...: Orienter_Direction`).
    pub(crate) fn head_to(&mut self, x: i64, y: i64) {
        self.dest = (x, y);
        self.orient(x, y);
    }

    /// `Orienter_Direction`
    pub(crate) fn orient(&mut self, tx: i64, ty: i64) {
        let h = self.step as f64 / 2.0;
        let (x, y) = (self.x as f64, self.y as f64);
        let (tx, ty) = (tx as f64, ty as f64);
        let (dx, dy) = (tx - x, ty - y);
        let sideways = |s: &Self| {
            if (tx < x - h || tx > x + h) && dx.abs() >= dy.abs() {
                Some(if tx < x { Dir::Left } else { Dir::Right })
            } else {
                let _ = s;
                None
            }
        };
        let d = if ty < y - h {
            sideways(self).unwrap_or(Dir::Up)
        } else if ty > y + h {
            sideways(self).unwrap_or(Dir::Down)
        } else if tx < x - h {
            Dir::Left
        } else if tx > x + h {
            Dir::Right
        } else {
            Dir::None
        };
        self.moving = d != Dir::None;
        if self.moving {
            self.direction = d;
        }
    }

    /// `Calculer_Vitalite`: productivity, lowered by low Vie, Energie and Moral.
    pub(crate) fn update_vitality(&mut self) {
        if !self.alive() {
            self.vitality = 0.0;
            return;
        }
        let mut v = self.bonus_vitality;
        for i in [0, 1, 3] {
            let r = self.bars[i] / self.bonus_max[i] as f64;
            if r < 0.1 {
                v -= 0.45;
            } else if r < 0.25 {
                v -= 0.25;
            } else if r < 0.5 {
                v -= 0.125;
            }
        }
        self.vitality = v.max(0.1);
    }

    /// `Calculer_Longueur_Pas`
    fn update_step(&mut self, rules: &Rules, terrain: &Terrain) {
        if !self.alive() {
            self.step = 6;
            return;
        }
        let mut l = (self.bonus_move as f64 * self.vitality).floor();
        if !self.flying {
            l = (l * speed_coef(rules, terrain, self.x, self.y + self.h as i64 - 16)).floor();
        }
        let mut l = l as i64;
        if l < 1 && self.bonus_move != 0 {
            l = 1;
        } else if l > 16 {
            l = 16;
        }
        self.step_temp = (self.step_temp + self.acceleration).min(l);
        self.step = self.step_temp;
    }

    /// The character's own part of `Deplacement3`: one step and the action transitions.
    /// Returns whether it is still walking, or None while stunned.
    pub(crate) fn step_move(&mut self, rules: &Rules, terrain: &Terrain) -> Option<bool> {
        if self.stun > 0 {
            self.stun -= 1;
            // TODO: projectiles die when stunned
            return None;
        }
        self.update_step(rules, terrain);
        if self.walk(rules, terrain) {
            // TODO: projectile collisions
            if self.action > 99 && self.action % 2 == 0 && self.action < 200 {
                self.action += 1;
            }
            Some(true)
        } else {
            // TODO: projectiles die on arrival
            self.direction = if self.flying { Dir::Down } else { Dir::None };
            self.dest = (self.x, self.y);
            self.step_temp = 0;
            if self.action % 2 == 1 && self.action < 100 {
                self.action -= 1;
            }
            Some(false)
        }
    }

    /// Walk-cycle frame, after `Deplacement3`.
    pub(crate) fn advance_frame(&mut self) {
        if self.moving || self.flying {
            self.frame = (self.frame + 1) % 4;
        }
    }

    /// `Marcher3`: one step along the path; returns whether still moving.
    fn walk(&mut self, rules: &Rules, terrain: &Terrain) -> bool {
        if !self.moving {
            return false;
        }
        if self.dest != self.path_dest || !self.path_complete {
            self.compute_path(rules, terrain);
            self.path_dest = self.dest;
        }
        let step = self.step as f64;
        let toward = |p: &mut Perso, (wx, wy): (i64, i64)| {
            let (dx, dy) = ((wx - p.x) as f64, (wy - p.y) as f64);
            let l = (dx * dx + dy * dy).sqrt();
            p.x = clng(step * dx / l + p.x as f64);
            p.y = clng(step * dy / l + p.y as f64);
        };
        let (wx, wy) = self.path[0];
        let (dx, dy) = ((wx - self.x) as f64, (wy - self.y) as f64);
        let l = (dx * dx + dy * dy).sqrt();
        if l >= step && l > 0.0 {
            toward(self, (wx, wy));
            self.set_visible(true);
        } else if self.path.len() > 1 {
            self.path.remove(0);
            let next = self.path[0];
            self.orient(next.0, next.1);
            if next == (self.x, self.y) {
                (self.x, self.y) = self.dest;
                self.moving = false;
            } else {
                toward(self, next);
            }
        } else {
            (self.x, self.y) = self.dest;
            self.moving = false;
        }
        self.moving
    }

    /// `Calculer_Chemin`: greedy 8-neighbour search toward the destination.
    fn compute_path(&mut self, rules: &Rules, terrain: &Terrain) {
        self.path.clear();
        let i = CELL as i64;
        let d = (16.0 * 0.7f64).round_ties_even() as i64;
        const PATHFINDING: i64 = 1; // Jeu.ini Regle3
        if !self.flying && self.alive() && PATHFINDING > 0 {
            // TODO: chasing a moving target limits the search to 2 steps per tick
            let max_steps = 1024;
            self.path_complete = true;
            let (dir_x, dir_y) = self.dest;
            let (mut tx, mut ty) = (0i64, 0i64);
            let mut prev_choice: i64 = -1;
            let mut quit = false;
            let offsets = [(-d, -d), (0, -i), (d, -d), (-i, 0), (i, 0), (-d, d), (0, i), (d, d)];
            while ((tx - dir_x).abs() > 16 || (ty - dir_y).abs() > 16)
                && self.path.len() < max_steps
                && !quit
            {
                let p = *self.path.last().unwrap_or(&(self.x, self.y));
                let dist = |a: (i64, i64)| (((a.0 - dir_x) as f64).powi(2) + ((a.1 - dir_y) as f64).powi(2)).sqrt();
                let l1 = dist(p);
                let mut best = 0.0;
                let mut choice = 0;
                for (k, (ox, oy)) in offsets.iter().enumerate() {
                    let n = (p.0 + ox, p.1 + oy);
                    let cost = 16.0 / speed_coef(rules, terrain, n.0, n.1 + self.h as i64 - 16) + (dist(n) - l1);
                    if k == 0 || cost < best {
                        best = cost;
                        choice = k as i64;
                        (tx, ty) = n;
                    }
                }
                let count = self.path.len();
                if count == 0 {
                    self.path.push((tx, ty));
                } else if prev_choice == choice {
                    *self.path.last_mut().unwrap() = (tx, ty);
                } else if count >= 2 {
                    if (tx, ty) == self.path[count - 2] {
                        quit = true;
                    } else {
                        self.path.push((tx, ty));
                        prev_choice = choice;
                    }
                } else if (tx, ty) == self.path[count - 1] {
                    quit = true;
                } else {
                    self.path.push((tx, ty));
                    prev_choice = choice;
                }
            }
        } else {
            self.path_complete = true;
        }
        match self.path.last_mut() {
            None => self.path.push(self.dest),
            Some(last) => *last = self.dest,
        }
    }

    /// Sprite sheet cell for the current pose: (column, row).
    pub fn sprite_cell(&self) -> (i32, i32) {
        match self.direction {
            Dir::None => (1, 2),
            d => (if self.frame == 3 { 1 } else { self.frame as i32 }, d as i32 - 1),
        }
    }
}

/// `ClsJeuMonde.CoefVitesseSimple`
pub fn speed_coef(rules: &Rules, terrain: &Terrain, x: i64, y: i64) -> f64 {
    let t = terrain.index_at(x as f64, y as f64);
    if t > 0 && (t as usize) < terrain.category.len() - 1 {
        rules.terrains[terrain.category[t as usize] as usize].speed
    } else {
        1.0
    }
}
