//! Scripted AI files (`.ia`, `ClsJeuIA2.Charger_IA`, `ModIA2.Charger_Dossiers_IAs`).

use crate::files::Files;
use crate::rules::BuildingType;

/// One step of an age, in file order (`Etape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `Ressource_R=Q`: own `quantity` units of resource `resource` (0-based).
    Resource { resource: i64, quantity: i64 },
    /// `Batiment_ID`: own a house of this type.
    Building(usize),
    /// `Objet_O=Q`: own `quantity` objects of type `object` (0-based).
    Object { object: i64, quantity: i64 },
    /// `Travailler`
    Work,
    /// `Patrouiller`
    Patrol,
    /// `Attaquer Ennemi`
    AttackEnemy,
    /// `Attaquer Donjon`
    AttackKeep,
    /// `ChangerMode_M`, stored as `M - 1`.
    Mode(i64),
    /// `GagnerArgent_ID=Q`: earn `quantity` gold working in a building of this type.
    EarnMoney { building: usize, quantity: i64 },
    /// `GagnerArgentRessource_R=Q`: earn `quantity` gold selling resource `resource`.
    EarnMoneyResource { resource: i64, quantity: i64 },
}

#[derive(Debug, Clone, Default)]
pub struct AiEpoch {
    pub comment: String,
    pub steps: Vec<Step>,
    /// `BatimentDefini`: the age has a `Batiment_` step.
    pub building_defined: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AiFile {
    /// Bare file name (lower case, like the asset keys).
    pub name: String,
    /// `Joueur`: in the root folder, selectable by the player and drawn for villagers.
    pub player: bool,
    pub comment: String,
    /// `Equiper_N`: parsed but never used by the game.
    pub equip: i64,
    pub epochs: Vec<AiEpoch>,
}

/// VB's implicit string-to-number conversion, then `CInt`.
fn int(s: &str) -> Result<i64, String> {
    let v: f64 = s.trim().parse().map_err(|_| format!("not a number: {s:?}"))?;
    Ok(crate::vb::clng(v))
}

/// `prefix V=Q` → (V, Q)
fn pair(rest: &str) -> Result<(&str, i64), String> {
    let (v, q) = rest.split_once('=').ok_or_else(|| format!("missing '=' in {rest:?}"))?;
    Ok((v, int(q)?))
}

/// `ClsJeuIA2.Charger_IA`. Like the original, the last line of the file is never read.
pub fn parse(name: &str, text: &str, buildings: &[BuildingType], epochs: usize) -> Result<AiFile, String> {
    let mut lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let building =
        |id: &str| buildings.iter().position(|b| b.id == id).ok_or_else(|| format!("{name}: unknown building {id:?}"));
    let mut ai = AiFile { name: name.to_owned(), epochs: vec![AiEpoch::default(); epochs], ..Default::default() };
    let mut k = 0;
    let line = |k: usize| lines.get(k).copied().ok_or_else(|| format!("{name}: unexpected end of file"));
    let mut cur = line(k)?;
    if let Some(c) = cur.strip_prefix('#') {
        ai.comment = c.to_owned();
        k += 1;
        cur = line(k)?;
    }
    if let Some(e) = cur.strip_prefix("Equiper_") {
        ai.equip = int(e)?;
        k += 1;
        cur = line(k)?;
    }
    let mut a = 0;
    while k + 1 < lines.len() {
        let err = |e: String| format!("{name}: {e}");
        if let Some(n) = cur.strip_prefix("Age_") {
            a = usize::try_from(int(n).map_err(err)? - 1).map_err(|_| format!("{name}: bad age {n}"))?;
            if a >= epochs {
                return Err(format!("{name}: bad age {n}"));
            }
        } else if let Some(c) = cur.strip_prefix('#') {
            ai.epochs[a].comment = c.to_owned();
        } else {
            let step = if let Some(r) = cur.strip_prefix("Ressource_") {
                let (v, q) = pair(r).map_err(err)?;
                Some(Step::Resource { resource: int(v).map_err(err)? - 1, quantity: q })
            } else if let Some(id) = cur.strip_prefix("Batiment_") {
                ai.epochs[a].building_defined = true;
                Some(Step::Building(building(id)?))
            } else if let Some(r) = cur.strip_prefix("Objet_") {
                let (v, q) = pair(r).map_err(err)?;
                Some(Step::Object { object: int(v).map_err(err)? - 1, quantity: q })
            } else if let Some(m) = cur.strip_prefix("ChangerMode_") {
                Some(Step::Mode(int(m).map_err(err)? - 1))
            } else if let Some(r) = cur.strip_prefix("GagnerArgent_") {
                let (id, q) = pair(r).map_err(err)?;
                Some(Step::EarnMoney { building: building(id)?, quantity: q })
            } else if let Some(r) = cur.strip_prefix("GagnerArgentRessource_") {
                let (v, q) = pair(r).map_err(err)?;
                Some(Step::EarnMoneyResource { resource: int(v).map_err(err)? - 1, quantity: q })
            } else {
                match cur {
                    "Travailler" => Some(Step::Work),
                    "Patrouiller" => Some(Step::Patrol),
                    "Attaquer Ennemi" => Some(Step::AttackEnemy),
                    "Attaquer Donjon" => Some(Step::AttackKeep),
                    _ => None,
                }
            };
            ai.epochs[a].steps.extend(step);
        }
        k += 1;
        cur = lines[k];
    }
    Ok(ai)
}

/// `Charger_Dossiers_IAs`: the files of `IAs/` (selectable by the player), then those of each
/// sub-folder, in `Dir` order.
pub fn load_all(files: &Files, buildings: &[BuildingType], epochs: usize) -> Result<Vec<AiFile>, String> {
    let root = crate::files::key("IAs") + "/";
    let mut folders: Vec<String> = files
        .list("IAs")
        .iter()
        .filter_map(|p| p[root.len()..].split_once('/').map(|(d, _)| format!("IAs/{d}")))
        .collect();
    folders.dedup();
    let mut ais = Vec::new();
    for (dir, player) in std::iter::once(("IAs".to_owned(), true)).chain(folders.into_iter().map(|d| (d, false))) {
        for path in files.list_files(&dir) {
            let name = path.rsplit('/').next().unwrap_or("");
            let text = files.text(&path).ok_or_else(|| format!("{path}: not UTF-8"))?;
            ais.push(AiFile { player, ..parse(name, text, buildings, epochs)? });
        }
    }
    Ok(ais)
}

/// `Charger_IA`: index of the AI file with this name (case-insensitive), -1 if none.
pub fn find(ais: &[AiFile], name: &str) -> i64 {
    if name.is_empty() {
        return -1;
    }
    let name = name.to_lowercase();
    ais.iter().position(|a| a.name == name).map_or(-1, |i| i as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buildings() -> Vec<BuildingType> {
        let b = |id: &str| BuildingType { id: id.to_owned(), ..Default::default() };
        vec![b("1"), b("3"), b("CASERNE")]
    }

    #[test]
    fn grammar_and_last_line() {
        let text = "#Desc\r\nEquiper_0\r\n\r\nAge_1\r\n#One\r\nChangerMode_1\r\nGagnerArgent_3=120\r\n\
                    GagnerArgentRessource_2=5\r\nRessource_13=6\r\nBatiment_CASERNE\r\nObjet_4=1\r\nTravailler\r\n\
                    Age_2\r\nPatrouiller\r\nAttaquer Ennemi\r\nAttaquer Donjon\r\nTravailler\r\n";
        let ai = parse("x.ia", text, &buildings(), 5).unwrap();
        assert_eq!(ai.comment, "Desc");
        assert_eq!(ai.epochs[0].comment, "One");
        assert!(ai.epochs[0].building_defined && !ai.epochs[1].building_defined);
        assert_eq!(
            ai.epochs[0].steps,
            [
                Step::Mode(0),
                Step::EarnMoney { building: 1, quantity: 120 },
                Step::EarnMoneyResource { resource: 1, quantity: 5 },
                Step::Resource { resource: 12, quantity: 6 },
                Step::Building(2),
                Step::Object { object: 3, quantity: 1 },
                Step::Work,
            ]
        );
        // The last line ("Travailler") is never read.
        assert_eq!(ai.epochs[1].steps, [Step::Patrol, Step::AttackEnemy, Step::AttackKeep]);
    }

    #[test]
    fn header_is_optional_and_unknown_buildings_fail() {
        let ai = parse("y.ia", "Age_1\nTravailler\n#Fin\n", &buildings(), 5).unwrap();
        assert_eq!(ai.epochs[0].steps, [Step::Work]);
        assert!(parse("z.ia", "Age_1\nBatiment_NOPE\n#Fin\n", &buildings(), 5).is_err());
        assert_eq!(find(&[ai], "Y.IA"), 0);
    }
}
