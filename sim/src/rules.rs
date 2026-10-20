//! Game rules, parsed once from the original `Donnees/` files.

use crate::files::Files;
use crate::ini::Ini;
use crate::ia_file::AiFile;
use crate::params::{Kind, Params, Record};

mod economy;
pub use economy::{BuildingType, DecorType, EpochSpeed, ObjectType, Resource, Service, StartResources, TreasureTier};

#[derive(Debug, Clone)]
pub struct TerrainType {
    pub name: String,
    pub speed: f64,
    pub unbuildable: bool,
    /// Object index of the effect applied to characters standing on it, or -1.
    pub effect: i64,
    pub animated: bool,
    pub sprite_offset: (i32, i32),
}

#[derive(Debug, Clone)]
pub struct WorldType {
    pub name: String,
    pub relief: f64,
    /// Growth passes per terrain type.
    pub surface: Vec<i64>,
    /// Terrain type used for roads, 0 for none.
    pub road: u8,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub race: usize,
    pub chance: f64,
    pub ai_file: String,
}

#[derive(Debug, Clone)]
pub struct People {
    pub name: String,
    /// Castle sprite, e.g. `Images\Batiments\Chateaux\Humains`.
    pub image: String,
    pub castle_name: String,
    pub castle_max_life: i64,
    pub castle_size: (i32, i32),
    pub castle_entrance: (i32, i32),
    /// Empty for peoples of citizens (Humans).
    pub monsters: Vec<Monster>,
}

#[derive(Debug, Clone)]
pub struct TerrainResource {
    pub name: String,
    pub resource: i64,
}

/// Per-sex value: index 0 male, 1 female.
pub type BySex<T> = [T; 2];

#[derive(Debug, Clone)]
pub struct Race {
    pub name: BySex<String>,
    /// `NomMPluriel`, `NomFPluriel` (`NomPluriel` sets both).
    pub plural: BySex<String>,
    pub random_name: bool,
    pub playable: bool,
    /// `RestrictionFeminin`: never female.
    pub no_female: bool,
    /// `RestrictionMaculin` (sic): never male. Misspelled variants in the data are ignored.
    pub no_male: bool,
    pub size: (i32, i32),
    /// Whole-sheet sprite folder under `Images\Persos\` (used when `whole_sheet`).
    pub sheet_folder: BySex<String>,
    pub whole_sheet: bool,
    /// Layered appearance: one folder under `Images\Apparences\` per category.
    pub layer_folders: BySex<Vec<String>>,
    /// Base maxima: Vie, Energie, Magie, Moral, Attaque, Defense.
    pub base: [i64; 6],
    pub coef: [f64; 6],
    pub speed: i64,
    pub acceleration: i64,
    pub flying: bool,
    pub ai_file: String,
    /// `IA_Selectionner`: NPCs of this race draw a random AI file at game start.
    pub ai_select: bool,
    /// `Equiper` (`IA_Equiper`): 0 never, 1 by value, 2 by skill.
    pub ai_equip: i64,
    /// `IA_Manger`, `IA_Pacifique`, `IA_Temeraire`, `IA_Protecteur`, `IA_Egoiste`, `IA_Incommandable`.
    pub ai_eat: bool,
    pub ai_pacific: bool,
    pub ai_bold: bool,
    pub ai_protector: bool,
    pub ai_selfish: bool,
    pub incommandable: bool,
    /// `IA_Berserk`: 0 for none, see `World::berserk`.
    pub berserk: i64,
    pub kamikaze: bool,
    /// `Inattaquable`: cannot be targeted.
    pub untargetable: bool,
    pub auto_resurrection: bool,
    /// `Parole`: may speak.
    pub speaks: bool,
    /// `CommentairesSpecifique`: comment type used for every line, -1 for none.
    pub special_comments: i64,
    /// `PorteeAttaque`: natural attack range.
    pub range: f64,
    /// `CoefCompSpeciale/Ressource/Service/Objet` (Singles, 0 ⇒ 1).
    pub coef_special: Vec<f64>,
    pub coef_resource: Vec<f64>,
    pub coef_service: Vec<f64>,
    pub coef_object: Vec<f64>,
    /// `VitesseRegenerationVie/Energie/Magie/Moral` (Singles), share of the max per slow tick.
    pub regen: [f64; 4],
    /// Gains experience.
    pub experience: bool,
    /// Carrying capacity bonus (`BonusMaxRessources`).
    pub bonus_max_resources: i64,
    pub armour: i64,
    /// Starting resources.
    pub resources: Vec<f64>,
    /// `DepartSansRessources`: ignores the game's starting resources.
    pub no_start_resources: bool,
    /// `RestaurationComplete`
    pub full_restore: bool,
    /// `Equipement(j)`: starting equipment.
    pub equipment: Vec<usize>,
    /// `Objet(j)` then the common objects of `[Races]`: talent objects.
    pub objects: Vec<usize>,
    /// `UtilisationMonture`: may ride mounts.
    pub mount: bool,
    /// `Resistances(k)` after the statuses and the clamp (Singles).
    pub resistances: Vec<f64>,
    /// `EffetAttaque`, `AttaqueResistance` (both default to 0), `AttaqueCauseEffet` (-1).
    pub attack_effect: i64,
    pub attack_resistance: i64,
    pub attack_cause_effect: i64,
    /// `ApparenceVerouillee`: equipment overlays other than mounts are not drawn.
    pub look_locked: bool,
    /// `Pietinement`: tramples the enemies with less of it while walking, 0 for none.
    pub trample: i64,
    /// `Tombe`: decor left where it dies, -1 for none.
    pub tomb: i64,
    /// `EffetMort`: visual effect when it dies, -1 for none.
    pub death_effect: i64,
    /// The image files `Tirer_Apparence_Au_Hasard` draws from, per sex: one list per draw (the
    /// sheet folder for a whole-sheet race, else one per appearance category).
    pub look_files: BySex<Vec<Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct Epoch {
    pub name: String,
    pub castle_life_bonus: i64,
    pub duration: i64,
    pub emperor_title: BySex<String>,
    pub emperor_price: i64,
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub epochs: Vec<Epoch>,
    pub epoch_speeds: Vec<EpochSpeed>,
    pub start_resources: Vec<StartResources>,
    pub resources: Vec<Resource>,
    pub buildings: Vec<BuildingType>,
    /// `BatimentsTri`: building indices in display and AI order.
    pub building_order: Vec<usize>,
    pub objects: Vec<ObjectType>,
    /// Item skill names (`ParamCompetencesObjets`), index 0 is "Sans".
    pub item_skills: Vec<String>,
    /// Special skill names: 0 build, 1 demolish, 2 critical hit, 3 bargaining.
    pub special_skills: Vec<String>,
    /// Index 0 is the "none" service.
    pub services: Vec<Service>,
    pub treasures: Vec<TreasureTier>,
    pub decors: Vec<DecorType>,
    /// `CoefficientAchat`: buying price = selling price × this.
    pub buy_coef: f64,
    /// `PersosMaxRessources`: starting carrying capacity, also the harvest skill offset.
    pub start_max_resources: i64,
    /// `PersosNiveauSpecialesDepart`, `PersosNiveauServicesDepart`, `PersosNiveauObjetsDepart`.
    pub start_special: i64,
    pub start_service: i64,
    pub start_object: i64,
    pub inventory_size: usize,
    pub equipped_size: usize,
    pub house_inventory_size: usize,
    pub lang_suffix: String,
    pub cell: i32,
    pub terrains: Vec<TerrainType>,
    pub world_types: Vec<WorldType>,
    pub peoples: Vec<People>,
    pub terrain_resources: Vec<TerrainResource>,
    pub races: Vec<Race>,
    /// `[Races] ChancesRaceAleatoire`: chance for a citizen to be of a random playable race.
    pub random_race_chance: f64,
    /// Random first names (`NomsM.ini`, `NomsF.ini`).
    pub names: BySex<Vec<String>>,
    /// Map-file colours: terrain type per `Cartes.ini [Terrain_n]`.
    pub map_terrain_colours: Vec<[u8; 3]>,
    pub map_resource_colours: Vec<[u8; 3]>,
    /// `Commentaires.ini`: the speech lines of each comment type.
    pub comments: Vec<Vec<String>>,
    /// `Commentaires.ini Duree`: display time of a speech bubble per character, in seconds.
    pub comment_duration: f64,
    /// `IA2s`: every `.ia` file, the root folder first, then each sub-folder.
    pub ais: Vec<AiFile>,
    /// `[Resistance_k] Nom` (Physique, Projectiles, Feu, …).
    pub resistances: Vec<String>,
    /// `ResistanceMinimum`, `ResistanceMaximum` (Singles).
    pub resistance_min: f64,
    pub resistance_max: f64,
    pub combat: Combat,
    /// `[Effets] EffetMaisonDetruite`, `EffetChateauDetruit`: 0-based effect indices.
    pub effect_house_destroyed: i64,
    pub effect_castle_destroyed: i64,
    /// `EtiquetteNiveau` and `EtiquetteTombeMasculin/Feminin`, for the labels of tombs.
    pub label_level: String,
    pub label_tomb: BySex<String>,
}

/// `Combat.ini` (`ModCombat.Init_Combat`), all Singles.
#[derive(Debug, Clone)]
pub struct Combat {
    /// `CoefDegatPerso`: damage per point of attack above the defence.
    pub damage_coef: f64,
    /// `CoefDegatBatiment`
    pub building_damage_coef: f64,
    /// `DureeImmobilisation`: ticks a hit character stays stunned (an Integer).
    pub stun: i64,
    /// `ChanceCoupCritique`, `CoefDegatCoupCritique`.
    pub critical_chance: f64,
    pub critical_coef: f64,
    /// `ChanceEchecCritique`, `PerteEchecCritique`: a miss costs that many attack points.
    pub failure_chance: f64,
    pub failure_loss: f64,
    /// `MaxDegatBatiment`: attack points spent at most per hit on a building.
    pub max_building_damage: f64,
    /// `CoefPillageArgent/Ressources/ObjetsPersoMort`, `CoefDestructionObjets`: looting a kill.
    pub loot_money: f64,
    pub loot_resources: f64,
    pub loot_objects: f64,
    pub destroy_objects: f64,
    /// `CoefPillageArgent/Ressources/ObjetsMaison`: looting a destroyed house.
    pub house_loot_money: f64,
    pub house_loot_resources: f64,
    pub house_loot_objects: f64,
}

fn ini(files: &Files, path: &str) -> Result<Ini, String> {
    files
        .text(path)
        .map(Ini::parse)
        .ok_or_else(|| format!("missing {path}"))
}

/// The last value written by any of `keys` (with their localised variants) at index `j`.
/// Keys like `Nom` set both sexes while `NomM` sets one; whichever comes last wins.
fn last_of<'a>(r: &'a Record, keys: &[&str], j: i64, suffix: &str) -> Option<&'a str> {
    r.fields
        .iter()
        .rev()
        .find(|(k, kj, _)| {
            *kj == j
                && keys
                    .iter()
                    .any(|key| k == key || k.strip_prefix(key) == Some(suffix))
        })
        .map(|(_, _, v)| v.as_str())
}

/// Race skill coefficients `key(j)` → `[j-1]` as Singles; the first `defaulted` ones go from 0 to 1.
fn coefs(r: &Record, key: &str, n: usize, defaulted: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];
    for (k, j, value) in &r.fields {
        if k == key && *j >= 1 && (*j as usize) <= n {
            v[*j as usize - 1] = crate::ini::vb_val(value) as f32 as f64;
        }
    }
    for c in v.iter_mut().take(defaulted) {
        if *c == 0.0 {
            *c = 1.0;
        }
    }
    v
}

fn val(s: Option<&str>) -> f64 {
    s.map(crate::ini::vb_val).unwrap_or(0.0)
}

/// A language of `Langues.ini`.
#[derive(Debug, Clone, PartialEq)]
pub struct Language {
    /// `Nom`, in that language (`Français`).
    pub name: String,
    /// `Dossier`: its files' folder in `Donnees/` (`Francais`).
    pub folder: String,
    /// `Suffixe`, of the keys translated in place and of some files (`FR`).
    pub suffix: String,
}

/// The languages of `Langues.ini`, the original's French first.
pub fn languages(files: &Files) -> Vec<Language> {
    let Ok(f) = ini(files, "Donnees/Langues.ini") else { return Vec::new() };
    (1..=f.int_or("Langues", "NombreLangues", 0))
        .map(|k| {
            let s = format!("Langue_{k}");
            Language {
                name: f.str_or(&s, "Nom", "").to_owned(),
                folder: f.str_or(&s, "Dossier", "").trim_end_matches(['\\', '/']).to_owned(),
                suffix: f.str_or(&s, "Suffixe", "").to_uppercase(),
            }
        })
        .collect()
}

impl Rules {
    /// The rules, with their text in the language of folder `lang` (`Donnees/<lang>/`).
    pub fn load(files: &Files, lang: &str) -> Result<Rules, String> {
        let suffix = languages(files).into_iter().find(|l| l.folder == lang).map_or_else(|| "FR".to_owned(), |l| l.suffix);
        let params = Params::load(files, lang)?;

        let terrain = ini(files, &format!("Donnees/{lang}/Terrain.ini"))?;
        let n = terrain.int_or("Terrains", "NombreTypesTerrains", 0);
        let terrains = (0..=n)
            .map(|i| {
                let s = format!("Terrain_{i}");
                TerrainType {
                    name: terrain.str_or(&s, "Nom", "").to_owned(),
                    speed: terrain.num_or(&s, "CoefVitesse", 1.0),
                    unbuildable: terrain.int_or(&s, "Inconstructible", 0) != 0,
                    effect: terrain.int_or(&s, "Effet", 0) - 1,
                    animated: terrain.int_or(&s, "Animation", 0) != 0,
                    sprite_offset: (
                        terrain.int_or(&s, "ImgPersoDecalageX", 0) as i32,
                        terrain.int_or(&s, "ImgPersoDecalageY", 0) as i32,
                    ),
                }
            })
            .collect();
        let nw = terrain.int_or("Mondes", "NombreTypesMondes", 0);
        let world_types = (1..=nw)
            .map(|i| {
                let s = format!("Monde_{i}");
                WorldType {
                    name: terrain.str_or(&s, "Nom", "").to_owned(),
                    relief: terrain.num_or(&s, "Relief", 0.0),
                    surface: (0..=n)
                        .map(|t| terrain.int_or(&s, &format!("SurfaceTerrain({t})"), 0))
                        .collect(),
                    road: terrain.int_or(&s, "Route", 0) as u8,
                }
            })
            .collect();

        let default_castle_path = params
            .list(Kind::Peuples)
            .iter()
            .chain(std::iter::once(params.global(Kind::Peuples)))
            .filter_map(|r| r.get("CheminImageDefaut"))
            .last()
            .unwrap_or("Images\\Batiments\\Chateaux\\")
            .to_owned();
        let peoples = params
            .list(Kind::Peuples)
            .iter()
            .map(|r| {
                let path = r.get("CheminImage").filter(|p| !p.is_empty()).unwrap_or(&default_castle_path);
                let count = r.int("NombreMonstres", 0);
                People {
                    name: r.get_l("Nom", &suffix).unwrap_or("").to_owned(),
                    image: format!("{path}{}", r.get("Image").unwrap_or("")),
                    castle_name: r.get_l("NomChateau", &suffix).unwrap_or("").to_owned(),
                    castle_max_life: r.int("ChateauMaxVie", 0),
                    castle_size: (r.int("ChateauLargeur", 0) as i32, r.int("ChateauHauteur", 0) as i32),
                    castle_entrance: (
                        r.int("ChateauEntreePositionX", 0) as i32,
                        r.int("ChateauEntreePositionY", 0) as i32,
                    ),
                    monsters: (1..=count)
                        .map(|j| Monster {
                            race: (r.num_j("MonstreIndice", j, 0.0) as i64 - 1).max(0) as usize,
                            chance: r.num_j("MonstreChance", j, 0.0),
                            ai_file: r.get_j("MonstreFichierIA", j).unwrap_or("").to_owned(),
                        })
                        .collect(),
                }
            })
            .collect();

        let terrain_resources = params
            .list(Kind::TerrainRessources)
            .iter()
            .map(|r| TerrainResource {
                name: r.get("Nom").unwrap_or("").to_owned(),
                resource: r.int("TypeRessource", 0) - 1,
            })
            .collect::<Vec<_>>();

        let fiefs = ini(files, &format!("Donnees/{lang}/Parametres_Fiefs.ini"))?;
        let epochs: Vec<Epoch> = (1..=fiefs.int_or("Epoques", "NombreEpoques", 0))
            .map(|i| {
                let s = format!("Epoque_{i}");
                Epoch {
                    name: fiefs.str_or(&s, "Nom", "").to_owned(),
                    castle_life_bonus: fiefs.int_or(&s, "BonusVieChateau", 0),
                    duration: fiefs.int_or(&s, "Duree", 0),
                    emperor_title: [
                        fiefs.str_or(&s, "EmpereurNomM", "").to_owned(),
                        fiefs.str_or(&s, "EmpereurNomF", "").to_owned(),
                    ],
                    emperor_price: fiefs.int_or(&s, "EmpereurPrix", 0),
                }
            })
            .collect();

        let eco = economy::load(files, lang, &suffix, &params, epochs.len())?;
        let (nspec, nres, nserv, nobj) =
            (eco.special_skills.len(), eco.resources.len(), eco.services.len(), eco.item_skills.len());
        let persos = ini(files, "Donnees/Parametres_Persos.ini")?;
        let pint = |k: &str, d: i64| persos.int_or("Personnages", k, d);

        const STATS: [&str; 6] = ["Vie", "Energie", "Magie", "Moral", "Attaque", "Defense"];
        let res_globals = params.global(Kind::Resistances);
        let res_default = val(res_globals.get("Defaut")) as f32 as f64;
        let resistance_min = val(res_globals.get("Minimum")) as f32 as f64;
        let resistance_max = val(res_globals.get("Maximum")) as f32 as f64;
        let nresist = params.list(Kind::Resistances).len();
        let statuses: Vec<Vec<f64>> = params
            .list(Kind::Statuts)
            .iter()
            .map(|r| {
                (1..=nresist as i64)
                    .map(|k| {
                        let v = last_of(r, &["Resistance", "Resistances"], k, "\0");
                        v.map_or(res_default, |v| crate::ini::vb_val(v) as f32 as f64)
                    })
                    .collect()
            })
            .collect();
        // Object references `v` → `v-1`; slots never assigned stay 0.
        let objects_of = |r: &Record, count: &str, key: &str| -> Vec<usize> {
            (1..=r.int(count, 0)).map(|j| crate::vb::clng(r.num_j(key, j, 1.0) - 1.0).max(0) as usize).collect()
        };
        let common = objects_of(params.global(Kind::Races), "NombreObjets", "Objet");
        let races: Vec<Race> = params
            .list(Kind::Races)
            .iter()
            .map(|r| {
                let sexed = |base: &str, j: i64| -> BySex<String> {
                    let m = format!("{base}M");
                    let f = format!("{base}F");
                    [
                        last_of(r, &[base, &m], j, &suffix).unwrap_or("").to_owned(),
                        last_of(r, &[base, &f], j, &suffix).unwrap_or("").to_owned(),
                    ]
                };
                let categories = r.int("NombreCategoriesApparences", 0);
                let folders = |sex: usize| -> Vec<String> {
                    (1..=categories).map(|j| sexed("DossiersCategories", j)[sex].clone()).collect()
                };
                let speed = r.int("Vitesse", -1);
                let acceleration = r.int("Acceleration", 0);
                Race {
                    name: sexed("Nom", 0),
                    plural: ["NomMPluriel", "NomFPluriel"].map(|k| last_of(r, &["NomPluriel", k], 0, &suffix).unwrap_or("").to_owned()),
                    random_name: r.flag("NomAleatoire"),
                    playable: r.flag("Jouable"),
                    no_female: r.flag("RestrictionFeminin"),
                    no_male: r.flag("RestrictionMaculin"),
                    size: (r.int("Largeur", 0) as i32, r.int("Hauteur", 0) as i32),
                    sheet_folder: sexed("CheminApparence", 0),
                    whole_sheet: r.flag("DefinirApparence"),
                    layer_folders: [folders(0), folders(1)],
                    base: STATS.map(|s| r.int(s, 0)),
                    coef: STATS.map(|s| r.num(&format!("Coef{s}"), 0.0)),
                    speed: if speed == -1 { 4 } else { speed },
                    acceleration: if acceleration <= 0 { 1 } else { acceleration },
                    flying: r.flag("Volant"),
                    ai_file: r.get("IA_Fichier").unwrap_or("").to_owned(),
                    ai_select: r.flag("IA_Selectionner"),
                    ai_equip: r.int("Equiper", 0),
                    ai_eat: r.flag("IA_Manger"),
                    ai_pacific: r.flag("IA_Pacifique"),
                    ai_bold: r.flag("IA_Temeraire"),
                    ai_protector: r.flag("IA_Protecteur"),
                    ai_selfish: r.flag("IA_Egoiste"),
                    incommandable: r.flag("IA_Incommandable"),
                    berserk: r.int("IA_Berserk", 0),
                    kamikaze: r.flag("Kamikaze"),
                    untargetable: r.flag("Inattaquable"),
                    auto_resurrection: r.flag("AutoResurrection"),
                    speaks: r.flag("Parole"),
                    special_comments: r.int("CommentairesSpecifique", 0) - 1,
                    range: crate::vb::clng(r.num("PorteeAttaque", 0.0)) as f64,
                    coef_special: coefs(r, "CoefCompSpeciale", nspec, nspec),
                    coef_resource: coefs(r, "CoefCompRessource", nres, nres),
                    coef_service: coefs(r, "CoefCompService", nserv, nserv.saturating_sub(1)),
                    coef_object: coefs(r, "CoefCompObjet", nobj, nobj),
                    regen: if r.flag("Infatigable") {
                        [r.num("VitesseRegenerationVie", 0.0), 0.01, r.num("VitesseRegenerationMagie", 0.0), 0.01]
                    } else {
                        ["Vie", "Energie", "Magie", "Moral"].map(|s| r.num(&format!("VitesseRegeneration{s}"), 0.0))
                    }
                    .map(|v| v as f32 as f64),
                    experience: r.flag("Experience"),
                    bonus_max_resources: crate::vb::clng(r.num("BonusMaxRessources", 0.0)),
                    armour: crate::vb::clng(r.num("Armure", 0.0)),
                    // `Ressource(j)` and `Ressources(j)` write the same slot: the last one wins.
                    resources: (1..=nres as i64)
                        .map(|j| val(last_of(r, &["Ressource", "Ressources"], j, "\0")))
                        .collect(),
                    no_start_resources: last_of(r, &["DepartSansRessource", "DepartSansRessources"], 0, "")
                        .map(|v| crate::ini::vb_val(v) != 0.0)
                        .unwrap_or(false),
                    full_restore: r.flag("RestaurationComplete"),
                    equipment: objects_of(r, "NombreEquipement", "Equipement"),
                    objects: objects_of(r, "NombreObjets", "Objet").into_iter().chain(common.iter().copied()).collect(),
                    mount: r.flag("UtilisationMonture"),
                    resistances: {
                        let mut v: Vec<f64> = (1..=nresist as i64)
                            .map(|k| {
                                let x = last_of(r, &["Resistance", "Resistances"], k, "\0");
                                x.map_or(res_default, |x| crate::ini::vb_val(x) as f32 as f64)
                            })
                            .collect();
                        let mut list: Vec<usize> = Vec::new();
                        for st in r.all("Statut") {
                            match crate::ini::vb_val(st) as i64 {
                                0 => list.clear(),
                                n => list.push((n - 1).max(0) as usize),
                            }
                        }
                        for st in list.iter().filter_map(|&k| statuses.get(k)) {
                            for (x, &y) in v.iter_mut().zip(st) {
                                if y != res_default {
                                    *x = y;
                                }
                            }
                        }
                        v.iter().map(|&x| x.clamp(resistance_min, resistance_max)).collect()
                    },
                    attack_effect: r.get("EffetAttaque").map_or(0, |v| crate::vb::clng(crate::ini::vb_val(v) - 1.0)),
                    attack_resistance: r.get("AttaqueResistance").map_or(0, |v| crate::vb::clng(crate::ini::vb_val(v) - 1.0)),
                    attack_cause_effect: r.get("AttaqueCauseEffet").map_or(-1, |v| crate::vb::clng(crate::ini::vb_val(v) - 1.0)),
                    look_locked: r.flag("ApparenceVerouillee"),
                    trample: r.int("Pietinement", 0),
                    tomb: r.get("Tombe").map_or(-1, |v| crate::vb::clng(crate::ini::vb_val(v) - 1.0)),
                    death_effect: r.get("EffetMort").map_or(-1, |v| crate::vb::clng(crate::ini::vb_val(v) - 1.0)),
                    look_files: Default::default(),
                }
            })
            .map(|mut race| {
                // Verification_Donnees defaults
                if race.size.0 <= 0 {
                    race.size.0 = 24;
                }
                if race.size.1 <= 0 {
                    race.size.1 = 32;
                }
                for (b, c) in race.base.iter_mut().zip(race.coef.iter_mut()) {
                    if *b <= 0 {
                        *b = 20;
                    }
                    if *c == 0.0 {
                        *c = 1.0;
                    }
                }
                race
            })
            .collect();
        let random_race_chance = val(params.global(Kind::Races).get("ChancesRaceAleatoire"));

        let names_of = |file: &str| -> Vec<String> {
            files
                .text(&format!("Donnees/{file}"))
                .unwrap_or("")
                .lines()
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty())
                .collect()
        };

        let cartes = ini(files, "Donnees/Cartes.ini")?;
        let colours = |prefix: &str, count: i64| {
            (1..=count)
                .map(|i| {
                    let s = format!("{prefix}_{i}");
                    [
                        cartes.int_or(&s, "R", 0) as u8,
                        cartes.int_or(&s, "V", 0) as u8,
                        cartes.int_or(&s, "B", 0) as u8,
                    ]
                })
                .collect::<Vec<_>>()
        };
        let map_terrain_colours = colours("Terrain", cartes.int_or("Cartes", "NombreTypesTerrains", 0));
        let map_resource_colours = colours("Ressource", terrain_resources.len() as i64);

        let com = ini(files, &format!("Donnees/{lang}/Commentaires.ini"))?;
        let comments = (1..=com.int_or("Commentaires", "NombreTypeMessages", 0))
            .map(|t| {
                let s = format!("Commentaires_{t}");
                (1..=com.int_or(&s, "NombreMessages", 0))
                    .map(|j| com.str_or(&s, &format!("Message({j})"), "").to_owned())
                    .collect()
            })
            .collect();
        let comment_duration = com.num_or("Commentaires", "Duree", 0.0) / 100.0;
        let races: Vec<Race> = races
            .into_iter()
            .map(|mut race: Race| {
                for sex in 0..2 {
                    let dirs = if race.whole_sheet {
                        vec![format!("Images/Persos/{}", race.sheet_folder[sex])]
                    } else {
                        race.layer_folders[sex].iter().map(|f| format!("Images/Apparences/{f}")).collect()
                    };
                    race.look_files[sex] = dirs.iter().map(|d| files.list_files(d)).collect();
                }
                race
            })
            .collect();
        let combat_ini = ini(files, "Donnees/Combat.ini")?;
        let single = |k: &str| combat_ini.num_or("Combat", k, 0.0) as f32 as f64;
        let combat = Combat {
            damage_coef: single("CoefDegatPerso"),
            building_damage_coef: single("CoefDegatBatiment"),
            stun: combat_ini.int_or("Combat", "DureeImmobilisation", 0),
            critical_chance: single("ChanceCoupCritique"),
            critical_coef: single("CoefDegatCoupCritique"),
            failure_chance: single("ChanceEchecCritique"),
            failure_loss: single("PerteEchecCritique"),
            max_building_damage: single("MaxDegatBatiment"),
            loot_money: single("CoefPillageArgentPersoMort"),
            loot_resources: single("CoefPillageRessourcesPersoMort"),
            loot_objects: single("CoefPillageObjetsPersoMort"),
            destroy_objects: single("CoefDestructionObjets"),
            house_loot_money: single("CoefPillageArgentMaison"),
            house_loot_resources: single("CoefPillageRessourcesMaison"),
            house_loot_objects: single("CoefPillageObjetsMaison"),
        };
        let effects = params.global(Kind::Effets);
        let effect_global = |k: &str| crate::vb::clng(val(effects.get(k)) - 1.0);
        let globals = ini(files, &format!("Donnees/{lang}/Parametres_Globales.ini"))?;
        let label = |k: &str| globals.str_or("Parametres_Globales", k, "").to_owned();
        let ais = crate::ia_file::load_all(files, &eco.buildings, epochs.len())?;

        Ok(Rules {
            epochs,
            epoch_speeds: eco.epoch_speeds,
            start_resources: eco.start_resources,
            resources: eco.resources,
            buildings: eco.buildings,
            building_order: eco.building_order,
            objects: eco.objects,
            item_skills: eco.item_skills,
            special_skills: eco.special_skills,
            services: eco.services,
            treasures: eco.treasures,
            decors: eco.decors,
            buy_coef: persos.num_or("Transactions", "CoefficientAchat", 2.0),
            start_max_resources: pint("PersosMaxRessources", 20),
            start_special: pint("PersosNiveauSpecialesDepart", 20),
            start_service: pint("PersosNiveauServicesDepart", 20),
            start_object: pint("PersosNiveauObjetsDepart", 20),
            inventory_size: pint("PersosNombreObjetsInventaire", 32) as usize,
            equipped_size: pint("PersosNombreObjetsEquipes", 11) as usize,
            house_inventory_size: pint("PersosNombreObjetsInventaireMaison", 32) as usize,
            lang_suffix: suffix,
            cell: terrain.int_or("Terrains", "LargeurCase", 16) as i32,
            terrains,
            world_types,
            peoples,
            terrain_resources,
            races,
            random_race_chance,
            names: [names_of("NomsM.ini"), names_of("NomsF.ini")],
            map_terrain_colours,
            map_resource_colours,
            comments,
            comment_duration,
            ais,
            resistances: params.list(Kind::Resistances).iter().map(|r| r.get("Nom").unwrap_or("").to_owned()).collect(),
            resistance_min,
            resistance_max,
            combat,
            effect_house_destroyed: effect_global("EffetMaisonDetruite"),
            effect_castle_destroyed: effect_global("EffetChateauDetruit"),
            label_level: label("EtiquetteNiveau"),
            label_tomb: [label("EtiquetteTombeMasculin"), label("EtiquetteTombeFeminin")],
        })
    }

    pub fn playable_races(&self) -> usize {
        self.races.iter().filter(|r| r.playable).count()
    }
}
