//! Economy rules: resources, buildings, objects, services, treasures, decors and the fief
//! presets (`ClsJeuParam` Init 1–4 and `Verification_Donnees`).

use super::{BySex, ini, last_of};
use crate::files::Files;
use crate::ini::vb_val;
use crate::params::{Kind, Params, Record};
use crate::vb::clng;

#[derive(Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub skill_name: String,
    pub image: String,
    /// `VitesseExtraction`
    pub speed: f64,
    /// Raw material resource (`TypeMatierePremiere`), -1 for none.
    pub raw: i64,
    pub raw_quantity: f64,
    /// `PrixVente`; buying costs `price * Rules::buy_coef`.
    pub price: f64,
    pub edible: bool,
    /// `GainVie/Energie/Magie/Moral` when eaten.
    pub gain: [f64; 4],
    /// `NecessairePourConstruire`
    pub building_material: bool,
    pub needed_to_attack: bool,
    /// `Recoltable`: some map node yields it.
    pub harvestable: bool,
    /// `TypeTerrainRessource`: the last node type yielding it, or -1.
    pub terrain_resource: i64,
    /// First epoch (0-based).
    pub epoch: i64,
}

#[derive(Debug, Clone, Default)]
pub struct BuildingType {
    pub id: String,
    pub name: String,
    pub female: bool,
    /// `CheminImage` + `Image`, e.g. `Images\Batiments\Ferme de ble`.
    pub image: String,
    pub button_image: String,
    pub job: BySex<String>,
    pub description: String,
    /// Only the emperor may build it.
    pub emperor: bool,
    /// Maximum per fief, 0 for unlimited.
    pub limit: i64,
    /// `CreerProprietaire`: race of the unit that owns it once built, -1 for the builder.
    pub owner_race: i64,
    pub king: bool,
    /// Objects given to / taken from the owner when built / destroyed, -1 for none.
    pub gain_object: i64,
    pub loss_object: i64,
    /// `Tir`: can shoot from inside.
    pub shoot: bool,
    pub market: bool,
    /// Total value (sort key), see `Verification_Donnees`.
    pub price: i64,
    pub price_money: i64,
    pub price_resources: Vec<i64>,
    pub max_life: i64,
    /// `CoefVitesseConstruction` (a Single).
    pub build_speed: f64,
    /// `RessourceFabrique`: resource produced by a shop (0-based), -1 for none.
    pub product: i64,
    /// `ObjetsFabriques`: objects crafted by a factory, sorted by epoch.
    pub objects: Vec<usize>,
    pub shop: bool,
    pub factory: bool,
    /// Service index (0 for none).
    pub service: usize,
    pub max_stock: f64,
    /// `Niveau`: importance for the AI.
    pub level: i64,
    /// `NoEpoque`: 1-based epoch needed.
    pub epoch: i64,
    pub size: (i32, i32),
    pub entrance: (i32, i32),
    /// `NombreEtapeFabrication`: construction stages (sprite rows - 1).
    pub build_stages: i64,
    pub anim_frames: i64,
    /// `NoEpoque <= NombreEpoques`
    pub playable: bool,
}

/// `ParamObjets`, only the fields the economy needs so far.
// TODO: equipment bonuses, usage and effects (items)
#[derive(Debug, Clone)]
pub struct ObjectType {
    pub name: String,
    pub plural: String,
    pub secret: bool,
    pub unique: bool,
    pub indestructible: bool,
    pub inseparable: bool,
    /// `Superposable`: stack size.
    pub stack: i64,
    pub emperor: bool,
    pub category: i64,
    /// Item skill index (`Competence`).
    pub skill: usize,
    pub price_resources: Vec<i64>,
    /// 1-based minimum epoch, -1 for secret objects.
    pub epoch: i64,
    /// `DureeFabrication`
    pub craft_time: i64,
    /// `CoefXPFabrication` (a Single).
    pub craft_xp: f64,
    /// `PrixVente`
    pub price: i64,
    pub usable: bool,
    pub single_use: bool,
    /// `PerteRessourceIndice`, `PerteRessourceQuantite`: ammunition.
    pub ammo: i64,
    pub ammo_quantity: f64,
    pub talent_price: i64,
    /// `BonusPorteeAttaque`: range bonus in pixels.
    pub range_bonus: f64,
    /// `UtilisationMort`: usable while dead.
    pub use_when_dead: bool,
    /// `Invocation`: race summoned, -1 for none (the relative `+N` forms of mods are not handled).
    pub invocation: i64,
    /// `FabricationBatiment`: building IDs this object is appended to (mods).
    made_in: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub description: String,
    /// `VitesseCreation`
    pub speed: f64,
    /// `TypeMatierePremiere`, kept raw (-1 for none).
    pub raw: i64,
    pub raw_quantity: f64,
    pub price: f64,
    /// `Coefficient` (a Single): share of HP restored, share of bounties.
    pub coef: f64,
    pub skill_name: String,
    /// 0-based effect index, -1 for none.
    pub effect: i64,
    pub xp_coef: f64,
}

#[derive(Debug, Clone)]
pub struct TreasureTier {
    pub name: String,
    pub min_value: i64,
}

#[derive(Debug, Clone)]
pub struct DecorType {
    pub name: String,
    pub image: String,
    pub size: (i32, i32),
    pub anim_frames: i64,
    /// Lifetime in seconds, 0 for permanent.
    pub duration: i64,
}

/// `[RessourcesDepart_k]`
#[derive(Debug, Clone)]
pub struct StartResources {
    pub name: String,
    /// Fief stock of each resource of an available epoch.
    pub castle: f64,
    /// Character stock of each building material of an available epoch.
    pub perso: f64,
    pub money: f64,
}

#[derive(Debug, Clone)]
pub struct EpochSpeed {
    pub name: String,
    pub coef: f64,
}

pub struct Economy {
    pub resources: Vec<Resource>,
    pub buildings: Vec<BuildingType>,
    pub building_order: Vec<usize>,
    pub objects: Vec<ObjectType>,
    pub item_skills: Vec<String>,
    pub special_skills: Vec<String>,
    pub services: Vec<Service>,
    pub treasures: Vec<TreasureTier>,
    pub decors: Vec<DecorType>,
    pub start_resources: Vec<StartResources>,
    pub epoch_speeds: Vec<EpochSpeed>,
}

fn int(r: &Record, key: &str) -> i64 {
    clng(r.num(key, 0.0))
}

fn indexed(r: &Record, key: &str, n: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];
    for (k, j, value) in &r.fields {
        if k == key && *j >= 1 && (*j as usize) <= n {
            v[*j as usize - 1] = vb_val(value);
        }
    }
    v
}

fn single(x: f64) -> f64 {
    x as f32 as f64
}

pub fn load(files: &Files, lang: &str, suffix: &str, params: &Params, epochs: usize) -> Result<Economy, String> {
    let resources: Vec<Resource> = params
        .list(Kind::Ressources)
        .iter()
        .map(|r| Resource {
            name: r.get("Nom").unwrap_or("").to_owned(),
            skill_name: r.get("NomCompetence").unwrap_or("").to_owned(),
            image: r.get("Image").filter(|s| !s.is_empty()).or(r.get("Nom")).unwrap_or("").to_owned(),
            speed: r.num("VitesseExtraction", 0.0),
            raw: clng(r.num("TypeMatierePremiere", 0.0) - 1.0),
            raw_quantity: r.num("QuantiteMatierePremiere", 0.0),
            price: r.num("PrixVente", 0.0),
            edible: r.flag("Mangeable"),
            gain: ["GainVie", "GainEnergie", "GainMagie", "GainMoral"].map(|k| r.num(k, 0.0)),
            building_material: r.flag("NecessairePourConstruire"),
            needed_to_attack: false,
            harvestable: false,
            terrain_resource: -1,
            epoch: clng(r.num("Epoque", 0.0) - 1.0),
        })
        .collect();
    let nr = resources.len();
    let mut resources = resources;
    for (t, tr) in params.list(Kind::TerrainRessources).iter().enumerate() {
        let k = clng(tr.num("TypeRessource", 0.0) - 1.0);
        if let Some(res) = usize::try_from(k).ok().and_then(|k| resources.get_mut(k)) {
            res.harvestable = true;
            res.terrain_resource = t as i64;
        }
    }

    let item_skills: Vec<String> = params
        .list(Kind::CompetencesObjets)
        .iter()
        .map(|r| r.get_l("Nom", suffix).unwrap_or("").to_owned())
        .collect();

    let default_path = params
        .list(Kind::Batiments)
        .iter()
        .chain(std::iter::once(params.global(Kind::Batiments)))
        .filter_map(|r| r.get("CheminImageDefaut"))
        .last()
        .unwrap_or("Images\\Batiments\\")
        .to_owned();
    let mut buildings: Vec<BuildingType> = params
        .list(Kind::Batiments)
        .iter()
        .map(|r| {
            let name = r.get_l("Nom", suffix).unwrap_or("").to_owned();
            let image = r.get("Image").filter(|s| !s.is_empty()).unwrap_or(&name).to_owned();
            let path = match r.get("CheminImage").filter(|s| !s.is_empty()) {
                Some(p) if p.starts_with('.') => p.chars().skip(2).collect(),
                Some(p) => p.to_owned(),
                None => default_path.clone(),
            };
            // NombreObjetsfabriques ReDims; ObjetsFabriques(j) fills slot j-1.
            let n = r.get("NombreObjetsfabriques").map(|v| clng(vb_val(v)).max(0) as usize).unwrap_or(0);
            let objects = indexed(r, "ObjetsFabriques", n).into_iter().map(|v| clng(v - 1.0).max(0) as usize).collect();
            BuildingType {
                id: r.id.clone(),
                female: r.flag(&format!("Feminin{suffix}")),
                button_image: r.get("ImageBouton").filter(|s| !s.is_empty()).unwrap_or(&image).to_owned(),
                image: format!("{path}{image}"),
                name,
                job: [
                    last_of(r, &["Metier", "MetierM"], 0, suffix).unwrap_or("").to_owned(),
                    last_of(r, &["Metier", "MetierF"], 0, suffix).unwrap_or("").to_owned(),
                ],
                description: r.get_l("Description", suffix).unwrap_or("").to_owned(),
                emperor: r.flag("Empereur"),
                limit: int(r, "Limite"),
                owner_race: r.get("CreerProprietaire").map(|v| clng(vb_val(v) - 1.0)).unwrap_or(-1),
                king: r.flag("Roi"),
                gain_object: clng(r.num("GainObjet", 0.0) - 1.0),
                loss_object: clng(r.num("PerteObjet", 0.0) - 1.0),
                shoot: r.flag("Tir"),
                market: r.flag("Marche"),
                price: int(r, "Prix"),
                price_money: int(r, "PrixArgent"),
                price_resources: indexed(r, "PrixRessources", nr).into_iter().map(clng).collect(),
                max_life: int(r, "MaxVie"),
                build_speed: single(r.num("CoefVitesseConstruction", 0.0)),
                product: int(r, "RessourceFabrique"),
                objects,
                shop: r.flag("Magasin"),
                factory: r.flag("Fabrique"),
                service: int(r, "Service").max(0) as usize,
                max_stock: r.num("MaxStock", 0.0),
                level: int(r, "Niveau"),
                epoch: int(r, "NoEpoque"),
                size: (int(r, "Largeur") as i32, int(r, "Hauteur") as i32),
                entrance: (int(r, "EntreePositionX") as i32, int(r, "EntreePositionY") as i32),
                build_stages: int(r, "NombreEtapeFabrication"),
                anim_frames: int(r, "NombreEtapeAnimation"),
                playable: false,
            }
        })
        .collect();

    let mut objects: Vec<ObjectType> = params
        .list(Kind::Objets)
        .iter()
        .map(|r| {
            let name = r.get_l("Nom", suffix).unwrap_or("").to_owned();
            let skill = r.get("Competence").map(|id| params.find_id(Kind::CompetencesObjets, id).unwrap_or(0));
            ObjectType {
                plural: r.get_l("NomPluriel", suffix).filter(|s| !s.is_empty()).unwrap_or(&name).to_owned(),
                name,
                secret: r.flag("Secret"),
                unique: r.flag("Unique"),
                indestructible: r.flag("Indestructible"),
                inseparable: r.flag("Inseparable"),
                stack: int(r, "Superposable"),
                emperor: r.flag("Empereur"),
                category: int(r, "Categorie"),
                skill: skill.unwrap_or(0),
                price_resources: indexed(r, "PrixRessources", nr).into_iter().map(clng).collect(),
                epoch: int(r, "Epoque"),
                craft_time: int(r, "DureeFabrication"),
                craft_xp: single(r.num("CoefXPFabrication", 1.0)),
                price: int(r, "PrixVente"),
                usable: r.flag("Utilisable"),
                single_use: r.flag("UtilisationUnique"),
                ammo: clng(r.num("PerteRessourceIndice", 0.0) - 1.0),
                ammo_quantity: single(r.num("PerteRessourceQuantite", 0.0)),
                talent_price: r.get("PrixTalents").or(r.get("PrixTalent")).map(|v| clng(vb_val(v))).unwrap_or(0),
                range_bonus: r.num("BonusPorteeAttaque", 0.0),
                use_when_dead: r.flag("UtilisationMort"),
                invocation: r.get("Invocation").map(|v| clng(vb_val(v) - 1.0)).unwrap_or(-1),
                made_in: r.all("FabricationBatiment").map(str::to_owned).collect(),
            }
        })
        .collect();
    // FabricationBatiment appends to the building's list (approximation: in object order,
    // after the building's own list).
    for (i, o) in objects.iter().enumerate() {
        for id in &o.made_in {
            if let Some(b) = buildings.iter_mut().find(|b| &b.id == id) {
                b.objects.push(i);
            }
        }
    }

    // Verification_Donnees: buildings.
    for b in &mut buildings {
        if b.price == 0 {
            for (q, r) in b.price_resources.iter().zip(&resources) {
                b.price = clng(b.price as f64 + *q as f64 * r.price);
            }
            if b.emperor {
                b.price += 100_000;
            }
        }
        b.price += b.price_money;
        if b.build_speed == 0.0 {
            b.build_speed = 1.0;
        }
        b.product -= 1;
        b.playable = b.epoch <= epochs as i64;
    }
    let building_order = sort_buildings(&buildings, epochs as i64);

    // Verification_Donnees: objects.
    for i in 0..objects.len() {
        if objects[i].ammo_quantity > 0.0 {
            if let Some(r) = usize::try_from(objects[i].ammo).ok().and_then(|k| resources.get_mut(k)) {
                r.needed_to_attack = true;
            }
        }
        if objects[i].secret {
            objects[i].epoch = -1;
            continue;
        }
        if let Some(b) = buildings.iter().find(|b| b.factory && b.objects.contains(&i)) {
            objects[i].epoch = objects[i].epoch.max(b.epoch);
        }
        for (j, r) in resources.iter().enumerate() {
            let o = &mut objects[i];
            if o.epoch >= 0 && o.price_resources[j] > 0 && r.epoch > o.epoch - 1 {
                o.epoch = r.epoch + 1;
            }
        }
    }
    for b in &mut buildings {
        // bubble sort by epoch, swapping only on strict <
        loop {
            let mut sorted = true;
            for j in 1..b.objects.len() {
                if objects[b.objects[j]].epoch < objects[b.objects[j - 1]].epoch {
                    b.objects.swap(j - 1, j);
                    sorted = false;
                }
            }
            if sorted {
                break;
            }
        }
    }

    let fiefs = ini(files, &format!("Donnees/{lang}/Parametres_Fiefs.ini"))?;
    let start_resources = (1..=fiefs.int_or("RessourcesDeparts", "NombreRessourcesDeparts", 0))
        .map(|k| {
            let s = format!("RessourcesDepart_{k}");
            StartResources {
                name: fiefs.str_or(&s, "Nom", "").to_owned(),
                castle: fiefs.num_or(&s, "ChateauxRessourcesDepart", 0.0),
                perso: fiefs.num_or(&s, "PersosRessourcesDepart", 0.0),
                money: fiefs.num_or(&s, "PersosArgentDepart", 0.0),
            }
        })
        .collect();
    let epoch_speeds = (1..=fiefs.int_or("VitesseEpoques", "NombreVitesseEpoques", 0))
        .map(|k| {
            let s = format!("VitesseEpoque_{k}");
            EpochSpeed { name: fiefs.str_or(&s, "Nom", "").to_owned(), coef: fiefs.num_or(&s, "Coef", 0.0) }
        })
        .collect();

    let tresors = ini(files, &format!("Donnees/{lang}/Tresors.ini"))?;
    let treasures = (1..=tresors.int_or("Tresors", "NombreTresors", 0))
        .map(|k| {
            let s = format!("Tresor_{k}");
            TreasureTier {
                name: tresors.str_or(&s, "Nom", "").to_owned(),
                min_value: clng(tresors.num_or(&s, "ValeurMin", 0.0)),
            }
        })
        .collect();

    let srv = ini(files, &format!("Donnees/{lang}/Services.ini"))?;
    let services = (1..=srv.int_or("Services", "NombreServices", 0))
        .map(|k| {
            let s = format!("Service_{k}");
            Service {
                name: srv.str_or(&s, "Nom", "").to_owned(),
                description: srv.str_or(&s, "Description", "").to_owned(),
                speed: srv.num_or(&s, "VitesseCreation", 0.0),
                raw: clng(srv.num_or(&s, "TypeMatierePremiere", 0.0)),
                raw_quantity: srv.num_or(&s, "QuantiteMatierePremiere", 0.0),
                price: srv.num_or(&s, "Prix", 0.0),
                coef: single(srv.num_or(&s, "Coefficient", 0.0)),
                skill_name: srv.str_or(&s, "NomCompetence", "").to_owned(),
                effect: clng(srv.num_or(&s, "Effet", 0.0)),
                xp_coef: srv.num_or(&s, "CoefExp", 0.0),
            }
        })
        .collect();

    let spec = ini(files, &format!("Donnees/{lang}/Competences_Speciales.ini"))?;
    let special_skills = (1..=spec.int_or("Competences_Speciales", "NombreCompetencesSpeciales", 0))
        .map(|k| spec.str_or("Competences_Speciales", &format!("CompetencesSpeciales{k}"), "").to_owned())
        .collect();

    let decors = params
        .list(Kind::Decors)
        .iter()
        .map(|r| DecorType {
            name: r.get("Nom").unwrap_or("").to_owned(),
            image: r.get("Image").unwrap_or("").to_owned(),
            size: (int(r, "Largeur") as i32, int(r, "Hauteur") as i32),
            anim_frames: int(r, "NombreEtapeAnimation"),
            duration: int(r, "Duree"),
        })
        .collect();

    Ok(Economy {
        resources,
        buildings,
        building_order,
        objects,
        item_skills,
        special_skills,
        services,
        treasures,
        decors,
        start_resources,
        epoch_speeds,
    })
}

/// `Verification_Donnees`: the display and AI order `BatimentsTri` (a selection sort).
fn sort_buildings(b: &[BuildingType], epochs: i64) -> Vec<usize> {
    let mut temp: Vec<usize> = (0..b.len()).collect();
    let mut sorted = Vec::with_capacity(b.len());
    while !temp.is_empty() {
        let mut k = 0;
        for j in 0..temp.len() {
            let (bj, bk) = (&b[temp[j]], &b[temp[k]]);
            if (j == 0 || (bj.price < bk.price && bj.epoch <= bk.epoch) || bj.epoch < bk.epoch)
                && (!bj.emperor || bk.epoch == epochs + 1)
            {
                k = j;
            }
        }
        sorted.push(temp.remove(k));
    }
    sorted
}
