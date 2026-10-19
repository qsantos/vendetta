//! The tooltip of an object icon (`ClsIntFenetre.Definir_InfoBulle_Objet`): inventory and
//! equipment slots, the workshop list and the temporary-effect icons.
//!
//! The text uses the original's markup: `#` starts a new line, a line starting with `&` is
//! red and one starting with `$` is blue.

use sim::files::Files;
use sim::ini::{Ini, vb_val};
use sim::items::{armour_sup, bonus_sup, gain_sup, invocation_sup, range_sup};
use sim::params::{Kind, Params};
use sim::perso::Perso;
use sim::rules::Rules;
use sim::vb::clng;
use sim::world::World;

macro_rules! labels {
    ($($field:ident = $key:literal),* $(,)?) => {
        /// The `Etiquette*` labels of `Parametres_Globales.ini` used by the tooltip, plus the
        /// few names of `Parametres_*.ini` that `Rules` does not keep.
        pub struct TooltipLabels {
            $(
                #[doc = concat!("`", $key, "`")]
                pub $field: String,
            )*
            /// `Race_Nom(i, False, True)`: the (masculine) plural of each race.
            pub race_plural: Vec<String>,
            /// `Resistance_NomComplet` and `Resistance_Attaque` of each resistance.
            pub resistance_full: Vec<String>,
            pub resistance_attack: Vec<String>,
            /// `CompetenceObjet_NoListe`: the display order of the item skills.
            pub item_skill_order: Vec<usize>,
            /// The decimal separator of VB's number-to-string conversions, which follow the
            /// system locale.
            pub decimal: char,
        }

        impl TooltipLabels {
            #[allow(dead_code)] // Only `load` calls it.
            fn from_ini(globals: &Ini) -> TooltipLabels {
                TooltipLabels {
                    $($field: globals.find($key).unwrap_or("").to_owned(),)*
                    race_plural: Vec::new(),
                    resistance_full: Vec::new(),
                    resistance_attack: Vec::new(),
                    item_skill_order: Vec::new(),
                    decimal: '.',
                }
            }
        }
    };
}

labels! {
    life = "EtiquetteVie",
    energy = "EtiquetteEnergie",
    magic = "EtiquetteMagie",
    morale = "EtiquetteMoral",
    attack = "EtiquetteAttaque",
    defence = "EtiquetteDefense",
    armour = "EtiquetteArmure",
    speed = "EtiquetteVitesse",
    movement = "EtiquetteDeplacement",
    vitality = "EtiquetteVitalite",
    two_hands = "EtiquetteArme2mains",
    range = "EtiquettePortee",
    area = "EtiquetteSurfaceAttaque",
    distance_unit = "EtiquetteUniteDistance",
    berserk = "EtiquetteBerserk",
    resurrects = "EtiquetteRessuscite",
    self_resurrects = "EtiquetteRessusciteSoi",
    mount_impossible = "EtiquetteUtilisationMontureImpossible2",
    money = "EtiquetteArgent",
    coins = "EtiquettePieces",
    price = "EtiquettePrix",
    indestructible = "EtiquetteIndestructible",
    summons = "EtiquetteInvoque",
    summon_chance1 = "EtiquetteInvoqueChance1",
    summon_chance2 = "EtiquetteInvoqueChance2",
    summon_max1 = "EtiquetteInvoqueMax1",
    summon_max1_plural = "EtiquetteInvoqueMax1Pluriel",
    summon_max2 = "EtiquetteInvoqueMax2",
    summon_max2_plural = "EtiquetteInvoqueMax2Pluriel",
    person = "EtiquettePersonne",
    persons = "EtiquettePersonnes",
    per_use = "EtiquetteParUtilisation",
    generates = "EtiquetteGenere",
    regenerates = "EtiquetteRegenere",
    restores = "EtiquetteRedonne",
    affects = "EtiquetteAffecte",
    carries = "EtiquetteTranport",
    extra_resources = "EtiquetteRessourcesSupplementaires",
    percent = "EtiquettePourcent",
    usage = "EtiquetteUtilisation",
    use_possible = "EtiquetteUtilisationPossible",
    uses_possible = "EtiquetteUtilisationsPossibles",
    ammo_left = "EtiquetteUtilisationMunition",
    ammos_left = "EtiquetteUtilisationMunitions",
    resources_missing = "EtiquetteUtilisationsRessourcesInsuffisantes",
    ammo_missing = "EtiquetteUtilisationsMunitionsInsuffisantes",
    extra_projectiles = "EtiquetteProjectilesSupplementaires",
    poison = "EtiquettePoison",
    unsellable = "EtiquetteObjetInvendable",
    undroppable = "EtiquetteObjetIndeposable",
    drop_to_sell = "EtiquetteObjetVendre",
    time_left = "EtiquetteDureeRestant",
    uncancellable = "EtiquetteEffetAnnulationImpossible",
    single_use = "EtiquetteUtilisationUnique",
    use_when_dead = "EtiquetteUtilisationMort",
    duration = "EtiquetteDuree",
    recharge = "EtiquetteDureeRecharge",
    attack_recharge = "EtiquetteDureeRechargeAttaque",
    charge = "EtiquetteRechargeMultiple",
    charges = "EtiquetteRechargesMultiples",
    time_unit = "EtiquetteUniteDuree",
}

impl TooltipLabels {
    pub fn load(files: &Files, lang: &str) -> TooltipLabels {
        let globals = Ini::parse(files.text(&format!("Donnees/{lang}/Parametres_Globales.ini")).unwrap_or(""));
        let mut labels = TooltipLabels::from_ini(&globals);
        // Approximation: the original follows the Windows locale; assume a French system for
        // the French game.
        labels.decimal = if lang == "Francais" { ',' } else { '.' };
        let suffix = if lang == "English" { "EN" } else { "FR" };
        if let Ok(params) = Params::load(files, lang) {
            // `NomPluriel` sets both sexes, `NomMPluriel` the masculine one: the last wins.
            let keys = ["NomPluriel".to_owned(), format!("NomPluriel{suffix}"), "NomMPluriel".to_owned(), format!("NomMPluriel{suffix}")];
            labels.race_plural = params
                .list(Kind::Races)
                .iter()
                .map(|r| {
                    r.fields
                        .iter()
                        .rev()
                        .find(|(k, j, _)| *j == 0 && keys.contains(k))
                        .map_or("", |(_, _, v)| v.as_str())
                        .to_owned()
                })
                .collect();
            let resistances = params.list(Kind::Resistances);
            labels.resistance_full = resistances.iter().map(|r| r.get("NomComplet").unwrap_or("").to_owned()).collect();
            labels.resistance_attack = resistances.iter().map(|r| r.get("Attaque").unwrap_or("").to_owned()).collect();
            labels.item_skill_order = params
                .list(Kind::CompetencesObjets)
                .iter()
                .enumerate()
                .map(|(k, r)| r.get("NoListe").map_or(k, |v| (vb_val(v) as i64 - 1).max(0) as usize))
                .collect();
        }
        labels
    }

    /// VB's implicit number-to-string conversion of a Single (7 significant digits).
    fn single(&self, x: f32) -> String {
        vb_str(x as f64, 7, self.decimal)
    }

    /// VB's implicit number-to-string conversion of a Double (15 significant digits).
    fn double(&self, x: f64) -> String {
        vb_str(x, 15, self.decimal)
    }
}

/// `CStr` of a number: `digits` significant digits, no trailing zeros, the locale's decimal
/// separator. (The exponent form VB uses below 1e-4 or above 1e15 is not reproduced.)
fn vb_str(x: f64, digits: usize, decimal: char) -> String {
    if x == 0.0 || !x.is_finite() {
        return "0".to_owned();
    }
    let rounded: f64 = format!("{:.*e}", digits - 1, x).parse().unwrap_or(x);
    let s = rounded.to_string();
    if decimal == '.' { s } else { s.replace('.', &decimal.to_string()) }
}

/// A temporary effect shown by the tooltip (`Duree`, `NumeroEffet` and its owner).
#[derive(Debug, Clone, Copy)]
pub struct EffectInfo<'a> {
    /// `Duree`: the remaining slow ticks, -1 when it never expires.
    pub duration: i64,
    /// The name of the character who cast it, None when it is the character itself.
    pub owner: Option<&'a str>,
    /// The caster's level in the object's item skill.
    pub owner_skill: i64,
}

impl<'a> EffectInfo<'a> {
    /// Effect `effect` of character `perso`. The caster's skill level is read live, like the
    /// original does.
    #[allow(dead_code)] // The effect icons will call it.
    pub fn new(rules: &Rules, world: &'a World, perso: usize, effect: usize) -> EffectInfo<'a> {
        let e = &world.persos[perso].effects[effect];
        let owner = &world.persos[e.owner];
        EffectInfo {
            duration: e.duration,
            owner: (e.owner != perso).then_some(owner.name.as_str()),
            owner_skill: owner.object_skill_level(rules, e.object),
        }
    }
}

/// The original's `Inventaire` flag: the object is in the character's bag or equipment.
#[derive(Debug, Clone, Copy)]
pub struct InInventory {
    /// In a castle or in a market: objects can be sold here.
    pub at_market: bool,
}

impl InInventory {
    pub fn of(world: &World, perso: usize) -> InInventory {
        let p = &world.persos[perso];
        let market = p.in_house && world.houses.get(p.house_target).is_some_and(|h| h.market);
        InInventory { at_market: p.in_own_castle || p.in_castle || market }
    }
}

/// `Definir_InfoBulle_Objet`: the tooltip of `object` for `perso`. `count` is the size of the
/// inventory stack (`Nombre`, 0 elsewhere), `effect` the temporary effect it shows, and
/// `inventory` is set for the bag and equipment slots.
pub fn object_tooltip(
    rules: &Rules,
    labels: &TooltipLabels,
    perso: &Perso,
    object: usize,
    count: i64,
    effect: Option<EffectInfo>,
    inventory: Option<InInventory>,
) -> String {
    let l = labels;
    let o = &rules.objects[object];
    let race = &rules.races[perso.race];
    let own_level = |skill: usize| perso.objects.get(skill).map_or(0, |s| s.bonus);
    // `Effet = Duree <> 0`
    let effect = effect.filter(|e| e.duration != 0);
    let is_effect = effect.is_some();
    let sign = |v: bool| if v { "#+" } else { "#&" };

    let mut s;
    let level;
    if let Some(e) = effect {
        s = o.effect_name[perso.female as usize].clone();
        if let Some(owner) = e.owner {
            s += &format!(" ({owner})");
        }
        level = e.owner_skill;
    } else {
        s = if count >= 2 { format!("{count} {}", o.plural) } else { o.name.clone() };
        level = own_level(o.skill);
    }
    let sup = |v: f64| bonus_sup(rules, level, v);

    if o.skill > 0 && !is_effect {
        s += &format!(" ({} : {})", rules.item_skills[o.skill], own_level(o.skill));
    }

    if !is_effect && o.recharge > 0 {
        // `EffetRecharge_TempRestant`
        let left = perso.recharges.iter().find(|r| r.object == object).map_or(0, |r| r.duration);
        if left > 0 {
            s += &format!("#&{} : {left} {}", l.recharge, l.time_unit);
        } else {
            s += &format!("#{} : {} {}", l.recharge, o.recharge, l.time_unit);
        }
        let charges = sup(o.charges);
        if charges > 1 {
            let ready = (bonus_sup(rules, own_level(o.skill), o.charges) as f64 - left as f64 / o.recharge as f64).floor();
            s += &format!("#{ready}/{charges} {}", if charges >= 2 { &l.charges } else { &l.charge });
        }
    }

    if o.range_bonus > 0.0 {
        s += &format!("#{} : {} {}", l.range, range_sup(rules, level, o.range_bonus), l.distance_unit);
    } else if o.use_range > 0 && !is_effect {
        s += &format!("#{} : {} {}", l.range, range_sup(rules, level, o.use_range as f64), l.distance_unit);
    }
    // The four bars and attack/defence: `Int(BonusSup × CoefRace)`, an Integer × Single.
    let bar = |k: usize| (sup(o.bonus[k] as f64) as f32 * race.coef[k] as f32).floor();
    if o.bonus[4] != 0 {
        s += &format!("{}{} {}", sign(o.bonus[4] > 0), bar(4), l.attack);
        if o.bonus[4] > 0 {
            if (o.category == 0 || o.category == 1) && o.attack_resistance > -1 {
                let name = usize::try_from(o.attack_resistance).ok().and_then(|k| l.resistance_attack.get(k));
                s += &format!(" ({})", name.map_or("", String::as_str));
            }
            if o.attack_cause_effect > 0 {
                let ce = &rules.objects[o.attack_cause_effect as usize];
                s += &format!("#${}", ce.name);
                if ce.duration != -1 {
                    let d = clng(gain_sup(rules, own_level(ce.skill), ce.duration as f64));
                    s += &format!(" : {d} {}", l.time_unit);
                }
            }
        }
    }
    if o.area > 0 {
        // `Objet_Equipe_SurfaceSup` scales like the range.
        s += &format!("#{} : {}", l.area, range_sup(rules, level, o.area as f64));
    }
    if o.attack_recharge > 0 {
        s += &format!("#{} : {}", l.attack_recharge, o.attack_recharge);
    }
    if o.bonus[5] != 0 {
        s += &format!("{}{} {}", sign(o.bonus[5] > 0), bar(5), l.defence);
    }
    if o.category == 1 && !o.usable {
        s += &format!("#{}", l.two_hands);
    }
    for (k, name) in [&l.life, &l.energy, &l.magic, &l.morale].into_iter().enumerate() {
        if o.bonus[k] != 0 {
            s += &format!("{}{} {name}", sign(o.bonus[k] > 0), bar(k));
        }
    }
    if o.bonus_move > 0.0 {
        s += &format!("#+{} {}", sup(o.bonus_move), l.movement);
    }
    if o.bonus_move < 0.0 {
        s += &format!("#&{} {}", l.single(o.bonus_move as f32), l.movement);
    }
    if o.bonus_speed != 0.0 {
        s += &format!("{}{} {}", sign(o.bonus_speed > 0.0), sup(o.bonus_speed), l.speed);
    }
    if o.bonus_vitality != 0.0 {
        let v = l.single(o.bonus_vitality as f32 * 100.0);
        s += &format!("{}{v}{} {}", sign(o.bonus_vitality > 0.0), l.percent, l.vitality);
    }
    if o.bonus_armour > 0 {
        s += &format!("#+{} {}", armour_sup(rules, level, o.bonus_armour as f64), l.armour);
    }
    // Skill bonuses: `Int(BonusSup × CoefRaceComp…)`.
    let skill_bonus = |b: i64, coef: Option<&f64>| (sup(b as f64) as f32 * coef.copied().unwrap_or(0.0) as f32).floor();
    for (j, name) in rules.special_skills.iter().enumerate() {
        if let Some(&b) = o.bonus_special.get(j).filter(|&&b| b > 0) {
            s += &format!("#+{} {name}", skill_bonus(b, race.coef_special.get(j)));
        }
    }
    for (j, res) in rules.resources.iter().enumerate() {
        if let Some(&b) = o.bonus_resource.get(j).filter(|&&b| b > 0) {
            s += &format!("#+{} {}", skill_bonus(b, race.coef_resource.get(j)), res.skill_name);
        }
    }
    for j in 0..perso.services.len() {
        if let Some(&b) = o.bonus_service.get(j).filter(|&&b| b > 0) {
            let name = rules.services.get(j + 1).map_or("", |sv| sv.skill_name.as_str());
            s += &format!("#+{} {name}", skill_bonus(b, race.coef_service.get(j)));
        }
    }
    for k in 1..rules.item_skills.len() {
        let j = l.item_skill_order.get(k).copied().unwrap_or(k);
        if let Some(&b) = o.bonus_object.get(j).filter(|&&b| b > 0) {
            s += &format!("#+{} {}", skill_bonus(b, race.coef_object.get(j)), rules.item_skills[j]);
        }
    }
    if o.bonus_max_resources > 0 {
        s += &format!("#{} {} {}", l.carries, sup(o.bonus_max_resources as f64), l.extra_resources);
    }

    // Regeneration: `Format(v, "0.000000000") * 1000`, a Double.
    for (k, name) in [&l.life, &l.energy, &l.magic, &l.morale].into_iter().enumerate() {
        let v = o.bonus_regen[k];
        if v > 0.0 {
            let v = (v * 1e9).round() / 1e9 * 1000.0;
            s += &format!("#${} {}{} {name}", l.regenerates, l.double(v), l.percent);
        }
    }

    if o.invocation >= 0 {
        let inv = o.invocation as usize;
        let singular = rules.races.get(inv).map_or("", |r| r.name[0].as_str());
        let plural = l.race_plural.get(inv).map_or("", String::as_str);
        let n = invocation_sup(rules, level, o.quantity) as f32;
        let whole = n.floor();
        s += &format!("#${} : {whole} {}", l.summons, if n >= 2.0 { plural } else { singular });
        if whole != n {
            let chance = ((n - whole) * 100.0).floor();
            s += &format!("#${chance}{}{singular}{}", l.summon_chance1, l.summon_chance2);
        }
        if o.quantity_max > 0.0 {
            let max = sup(o.quantity_max);
            let many = max >= 2;
            s += &format!(
                "#&{} {} {}{}",
                if many { &l.summon_max1_plural } else { &l.summon_max1 },
                if max == 0 { "1".to_owned() } else { max.to_string() },
                if many { plural } else { singular }.to_lowercase(),
                if many { &l.summon_max2_plural } else { &l.summon_max2 },
            );
        }
    }

    let persons = |n: i64| if n >= 2 { &l.persons } else { &l.person };
    if o.resurrection > 0 {
        let n = sup(o.quantity);
        s += &format!("#${} {n} {}", l.resurrects, persons(n));
    }
    if o.self_resurrection {
        s += &format!("#${}", l.self_resurrects);
    }
    if o.berserk > 0 {
        s += &format!("#${}", l.berserk);
    }
    for (k, name) in [&l.life, &l.energy, &l.magic, &l.morale].into_iter().enumerate() {
        if o.gain[k] > 0.0 {
            s += &format!("#${} : {} {name}", l.restores, sup(o.gain[k]));
        }
    }
    if o.poison > 0.0 {
        s += &format!("#&{} : {}", l.poison, sup(o.poison).max(1));
    }
    if !is_effect {
        // VB precedence: `(gains > 0 And Resurrection = 0 And Quantite > 0) Or allies Or enemies`.
        let gains: f64 = o.gain.iter().sum();
        if gains > 0.0 && o.resurrection == 0 && o.quantity > 0.0 || o.create_object[1] > 0 || o.create_object[2] > 0 {
            let n = sup(o.quantity);
            s += &format!("#{} {n} {}", l.affects, persons(n));
        }
    }
    if o.gain_money > 0.0 {
        s += &format!("#${} {} {}", l.generates, l.single(gain_sup(rules, level, o.gain_money) as f32), l.money);
    }
    for (j, res) in rules.resources.iter().enumerate() {
        if let Some(&g) = o.gain_resources.get(j).filter(|&&g| g > 0.0) {
            s += &format!("#${} {} {}", l.generates, l.single(gain_sup(rules, level, g) as f32), res.name.to_lowercase());
        }
    }
    for j in 0..rules.resistances.len() {
        let v = o.bonus_resistances.get(j).copied().unwrap_or(0.0);
        if v != 0.0 {
            s += if v > 0.0 { "#$" } else { "#&" };
            let name = l.resistance_full.get(j).map_or("", String::as_str);
            s += &format!("{name} : {}{}{}", if v > 0.0 { "+" } else { "" }, l.single(v as f32 * 100.0), l.percent);
        }
    }

    if !is_effect {
        if o.duration > 0 {
            let d = gain_sup(rules, level, o.duration as f64).floor();
            s += &format!("#{} : {d} {}", l.duration, l.time_unit);
        }
        for (k, name) in [&l.life, &l.energy, &l.magic, &l.morale].into_iter().enumerate() {
            if o.cost[k] > 0.0 {
                s += &format!("#(- {} {name} {})", l.double(o.cost[k]), l.per_use);
            }
        }
        if o.cost_money > 0 {
            let red = if o.cost_money as f64 > perso.money() { "&" } else { "" };
            s += &format!("#{red}(- {} {} {})", o.cost_money, l.coins, l.per_use);
        }
        if o.ammo_quantity > 0.0 {
            let res_name = usize::try_from(o.ammo).ok().and_then(|r| rules.resources.get(r)).map_or("", |r| r.name.as_str());
            s += &format!("#({} : {} {res_name})", l.usage, l.single(o.ammo_quantity as f32));
            let have = perso.res(o.ammo);
            let (missing, one, many) = if o.usable {
                (&l.resources_missing, &l.use_possible, &l.uses_possible)
            } else {
                (&l.ammo_missing, &l.ammo_left, &l.ammos_left)
            };
            if have < o.ammo_quantity {
                s += &format!("#&{missing}");
            } else {
                let uses = have / o.ammo_quantity;
                s += &format!("#({} {})", uses.floor(), if uses >= 2.0 { many } else { one });
            }
        }
        let projectiles = gain_sup(rules, level, o.extra_projectiles);
        if projectiles >= 1.0 {
            s += &format!("#${} {}", projectiles.floor() + 1.0, l.extra_projectiles);
        }
        if o.indestructible {
            s += &format!("#${}", l.indestructible);
        }
        if o.single_use {
            s += &format!("#{}", l.single_use);
        }
        if o.use_when_dead {
            s += &format!("#${}", l.use_when_dead);
        }
        if !o.inseparable && o.price > 0 {
            // Objects in the bag sell for less than they cost in the workshop.
            let price = if inventory.is_some() || perso.in_own_house {
                o.price
            } else {
                perso.bargained(rules, o.price as f64 * rules.buy_coef)
            };
            s += &format!("#{} : {price}", l.price);
        }
        if !o.description.is_empty() {
            s += &format!("#{}", o.description);
        }
    }
    if let Some(inv) = inventory {
        if inv.at_market {
            s += &if o.inseparable { format!("#&{}", l.unsellable) } else { format!("#{}", l.drop_to_sell) };
        } else if o.inseparable {
            s += &format!("#&{}", l.undroppable);
        }
        if o.mount && !race.mount {
            s += &format!("#&{}", l.mount_impossible);
        }
    }

    if let Some(e) = effect {
        if e.duration != -1 {
            s += &format!("#{} : {} {}", l.time_left, e.duration, l.time_unit);
        }
        if o.effect_uncancellable && !o.auto_use {
            s += &format!("#{}", l.uncancellable);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::vb::Rnd;
    use std::path::Path;

    fn load() -> Option<(Files, Rules, TooltipLabels)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        if !root.exists() {
            eprintln!("no assets, skipping");
            return None;
        }
        let files = Files::from_dir(&root).unwrap();
        let rules = Rules::load(&files, "Francais").unwrap();
        let labels = TooltipLabels::load(&files, "Francais");
        Some((files, rules, labels))
    }

    fn human(rules: &Rules) -> Perso {
        let mut p = Perso::new(rules, &mut Rnd::new(1), 0, 1, false);
        p.update_bonus(rules);
        p
    }

    const SWORD: usize = 13; // Epée
    const ICE_SWORD: usize = 204; // Epées de glace: causes Glace
    const BATTLE_ARMOUR: usize = 223; // Armure de combat
    const STRENGTH: usize = 149; // Elixir de force
    const LIFE_ELIXIR: usize = 109; // Elixir de vie: recharges
    const RAISE_DEAD: usize = 34; // Anim de morts: summons 4 skeletons
    const FOX: usize = 164; // Renard klepto: summons at most 1 fox
    const WATER: usize = 158; // Eau: an effect
    const WEAKNESS: usize = 161; // Poison affaiblissant: an effect
    const REGEN_RING: usize = 141; // Anneau de régénération

    fn set_level(rules: &Rules, p: &mut Perso, o: usize, level: i64) {
        p.objects[rules.objects[o].skill].bonus = level;
    }

    #[test]
    fn number_formatting() {
        assert_eq!(vb_str(0.1f32 as f64, 7, ','), "0,1");
        assert_eq!(vb_str((0.3f32 * 100.0) as f64, 7, ','), "30");
        assert_eq!(vb_str(-1.0, 7, ','), "-1");
        assert_eq!(vb_str(0.0025f32 as f64, 15, '.'), "0.00249999994412065");
        assert_eq!(vb_str(2.5, 15, '.'), "2.5");
    }

    #[test]
    fn weapons() {
        let Some((_, rules, l)) = load() else { return };
        let mut p = human(&rules);
        // Workshop: Prix_Marchandee(600 × 2) at bargaining 20 = 1200; the attack causes Glace
        // (20 s at the Glace skill level 20); AttaqueResistance 4 is the cold attack.
        assert_eq!(
            object_tooltip(&rules, &l, &p, ICE_SWORD, 0, None, None),
            "Epées de glace (Armes tranchantes : 20)#+12 Attaque (Attaque de froid)#$Glace : 20 s#Prix : 1200\
             #Arme de contact standard."
        );
        // In the bag at a market, at skill 30: Int(10 × 30 / 20) = 15, the selling price.
        set_level(&rules, &mut p, SWORD, 30);
        assert_eq!(
            object_tooltip(&rules, &l, &p, SWORD, 1, None, Some(InInventory { at_market: true })),
            "Epée (Armes tranchantes : 30)#+15 Attaque (Attaque physique)#Prix : 150#Arme de contact standard.\
             #(Déposez l'objet pour le vendre)"
        );
    }

    #[test]
    fn armour() {
        let Some((_, rules, l)) = load() else { return };
        let mut p = human(&rules);
        // A dwarf: CoefVie 1.25 so Int(60 × 1.25) = 75; the negative movement is shown raw.
        p.race = 1;
        assert_eq!(
            object_tooltip(&rules, &l, &p, BATTLE_ARMOUR, 1, None, Some(InInventory { at_market: false })),
            "Armure de combat (Armures lourdes : 20)#+75 Vie#&-1 Déplacement#+8 Armure#$Résistance physique : +30%\
             #&Résistance à l'électricité : -30%#Prix : 2200#Armure lourde et lente."
        );
        // It is also a mount: a race that cannot ride is told so.
        p.race = rules.races.iter().position(|r| !r.mount).unwrap();
        let tip = object_tooltip(&rules, &l, &p, BATTLE_ARMOUR, 1, None, Some(InInventory { at_market: false }));
        assert!(tip.ends_with("#&Impossible d'utiliser cette monture"), "{tip}");
    }

    #[test]
    fn potions() {
        let Some((_, rules, l)) = load() else { return };
        let mut p = human(&rules);
        // A stack of 3 uses the plural; usable, so not two-handed despite category 1.
        assert_eq!(
            object_tooltip(&rules, &l, &p, STRENGTH, 3, None, Some(InInventory { at_market: false })),
            "3 Elixirs de force (Elixirs magiques : 20)#+20 Attaque#+20 Détruire bâtiment#Durée : 30 s\
             #Utilisable qu'une fois#Prix : 250#Accroit votre force d'attaque temporairement."
        );
        // Recharging: the time left is red.
        p.add_recharge(LIFE_ELIXIR, 10);
        assert_eq!(
            object_tooltip(&rules, &l, &p, LIFE_ELIXIR, 1, None, Some(InInventory { at_market: false })),
            "Elixir de vie (Elixirs magiques : 20)#&Durée de recharge : 10 s#$Ressuscite soi-même\
             #Utilisable qu'une fois#$Utilisable même mort#Prix : 720#Permet de se ressusciter instantanément."
        );
        // Regeneration: Format(0.0025, "0.000000000") * 1000 = 2.5.
        let tip = object_tooltip(&rules, &l, &p, REGEN_RING, 0, None, None);
        assert!(tip.contains("#$Régénère 2,5% Vie#"), "{tip}");
    }

    #[test]
    fn summons() {
        let Some((_, rules, l)) = load() else { return };
        let mut p = human(&rules);
        // Skill 25: InvocationSup = (4 × 25 / 20 - 4) × 0.5 + 4 = 4.5; one charge.
        set_level(&rules, &mut p, RAISE_DEAD, 25);
        assert_eq!(
            object_tooltip(&rules, &l, &p, RAISE_DEAD, 0, None, None),
            "Anim de morts (Magie nécromantique : 25)#Durée de recharge : 12 s#$Invoque : 4 Squelettes\
             #$50% de chance d'invoquer 1 Squelette de plus#(- 12 Magie par utilisation)#Utilisable qu'une fois\
             #Prix : 900"
        );
        // Skill 40: 2 charges, Int(2 - 6 / 12) = 1 ready; (8 - 4) × 0.5 + 4 = 6 skeletons.
        set_level(&rules, &mut p, RAISE_DEAD, 40);
        p.add_recharge(RAISE_DEAD, 6);
        assert_eq!(
            object_tooltip(&rules, &l, &p, RAISE_DEAD, 0, None, None),
            "Anim de morts (Magie nécromantique : 40)#&Durée de recharge : 6 s#1/2 charges consécutives\
             #$Invoque : 6 Squelettes#(- 12 Magie par utilisation)#Utilisable qu'une fois#Prix : 900"
        );
        // A limited, inseparable familiar (no skill, no price), in the bag outside a market.
        assert_eq!(
            object_tooltip(&rules, &l, &p, FOX, 1, None, Some(InInventory { at_market: false })),
            "Renard klepto#Durée de recharge : 240 s#$Invoque : 1 Renard#&Limité à  1 renard à la fois.\
             #$Indestructible#Appelle un familier qui vous ramasse des trésors.#&(Ne peut être déposé ou donné)"
        );
    }

    #[test]
    fn effects() {
        let Some((_, rules, l)) = load() else { return };
        let mut p = human(&rules);
        // Cast by someone else: the owner's name, the time left, uncancellable.
        let e = EffectInfo { duration: 42, owner: Some("Bob"), owner_skill: 20 };
        assert_eq!(
            object_tooltip(&rules, &l, &p, WEAKNESS, 0, Some(e), None),
            "Affaibli (Bob)#&-5 Attaque#&-5 Défense#&-5 Vie#&-5 Energie#&-5 Moral#Temps restant : 42 s\
             #&Ne peut être annulé"
        );
        // A woman's own endless effect: the feminine name and no time left.
        p.female = true;
        let e = EffectInfo { duration: -1, owner: None, owner_skill: 20 };
        assert_eq!(
            object_tooltip(&rules, &l, &p, WATER, 0, Some(e), None),
            "Mouillée#$Résistance au feu : +35%#&Résistance au froid : -10%#&Résistance à l'électricité : -25%\
             #&Ne peut être annulé"
        );
    }
}
