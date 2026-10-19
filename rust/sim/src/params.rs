//! The original's line parser for `Parametres_*.ini` (`ClsJeuParam.Parametre_Lire_Fichier`).
//!
//! It is not an INI parser: headers are matched by case-sensitive prefix, some headers
//! select an entity without switching the current section, keys are case-sensitive, and a
//! key's `(j)` suffix is an index. Entities are kept as raw records in file order; typed
//! rules read the fields they need from them (see `rules.rs`).

use crate::files::Files;
use crate::ini::vb_val;

#[derive(Debug, Clone, Default)]
pub struct Record {
    /// String ID for buildings and item skills, empty otherwise.
    pub id: String,
    /// `(key, j, value)` in file order; `j` is 0 when the key has no `(j)`.
    pub fields: Vec<(String, i64, String)>,
}

impl Record {
    /// Last value assigned to `key(j)`.
    pub fn get_j(&self, key: &str, j: i64) -> Option<&str> {
        self.fields
            .iter()
            .rev()
            .find(|(k, kj, _)| k == key && *kj == j)
            .map(|(_, _, v)| v.as_str())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.get_j(key, 0)
    }

    /// Last value of `key` or of its localised variant `key` + `suffix` (e.g. `NomFR`),
    /// whichever comes last in the file.
    pub fn get_l(&self, key: &str, suffix: &str) -> Option<&str> {
        let localised = format!("{key}{suffix}");
        self.fields
            .iter()
            .rev()
            .find(|(k, j, _)| *j == 0 && (k == key || *k == localised))
            .map(|(_, _, v)| v.as_str())
    }

    /// `Val(...)` of the key, or `default` when absent.
    pub fn num(&self, key: &str, default: f64) -> f64 {
        self.get(key).map(vb_val).unwrap_or(default)
    }

    pub fn num_j(&self, key: &str, j: i64, default: f64) -> f64 {
        self.get_j(key, j).map(vb_val).unwrap_or(default)
    }

    pub fn int(&self, key: &str, default: i64) -> i64 {
        self.get(key).map(|v| vb_val(v) as i64).unwrap_or(default)
    }

    pub fn flag(&self, key: &str) -> bool {
        self.num(key, 0.0) != 0.0
    }

    /// Every value of a repeatable key, in file order (e.g. a race's `Statut`).
    pub fn all(&self, key: &str) -> impl Iterator<Item = &str> {
        self.fields
            .iter()
            .filter(move |(k, _, _)| k == key)
            .map(|(_, _, v)| v.as_str())
    }

    /// Indices `j` used with `key(j)`, in file order, deduplicated.
    pub fn indices(&self, key: &str) -> Vec<i64> {
        let mut v: Vec<i64> = Vec::new();
        for (k, j, _) in &self.fields {
            if k == key && !v.contains(j) {
                v.push(*j);
            }
        }
        v
    }

    fn set(&mut self, key: String, j: i64, value: String) {
        self.fields.push((key, j, value));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ressources,
    TerrainRessources,
    Batiments,
    CompetencesObjets,
    Objets,
    Peuples,
    Effets,
    Races,
    Decors,
    Resistances,
    Statuts,
}

const KINDS: usize = 11;

#[derive(Debug, Clone, Default)]
pub struct Params {
    lists: [Vec<Record>; KINDS],
    /// Keys written while the "current entity" is a section header itself: `[Races]`
    /// (common items) or a global like `[Effets] EffetClique`.
    globals: [Record; KINDS],
}

impl Params {
    pub fn list(&self, kind: Kind) -> &[Record] {
        &self.lists[kind as usize]
    }

    pub fn global(&self, kind: Kind) -> &Record {
        &self.globals[kind as usize]
    }

    /// Loads the base files, then the official mods (`Donnees/ModsOfficiels/` and its
    /// language folder, which carries the English names), in the original's order.
    pub fn load(files: &Files, lang: &str) -> Result<Params, String> {
        let mut p = Params::default();
        for name in [
            "Ressources",
            "Batiments",
            "CompetencesObjets",
            "Resistances",
            "Objets",
            "Decors",
            "Peuples",
            "Effets",
            "Races",
        ] {
            let path = format!("Donnees/Parametres_{name}.ini");
            let text = files.text(&path).ok_or_else(|| format!("missing {path}"))?;
            p.parse_file(text);
        }
        for dir in ["Donnees/ModsOfficiels".to_owned(), format!("Donnees/ModsOfficiels/{lang}")] {
            let prefix = crate::files::key(&dir) + "/";
            for path in files.list(&dir) {
                let rest = &path[prefix.len()..];
                if !rest.contains('/') && rest.ends_with(".ini") {
                    p.parse_file(files.text(&path).unwrap_or(""));
                }
            }
        }
        Ok(p)
    }

    fn parse_file(&mut self, text: &str) {
        let mut section: Option<Kind> = None;
        // Current entity index; -1 means the section's global record.
        let mut i: i64 = -1;
        for raw in text.lines() {
            let line = filter(raw);
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                self.header(line, &mut section, &mut i);
                continue;
            };
            let Some(kind) = section else { continue };
            let key = key.trim_matches(' ');
            let value = value.trim_matches(' ').to_owned();
            let (name, j) = match (key.find('('), key.rfind(')')) {
                (Some(a), Some(b)) if a < b => (key[..a].trim_matches(' '), vb_val(&key[a + 1..b]) as i64),
                _ => (key, 0),
            };
            if name == "ID" && matches!(kind, Kind::Batiments | Kind::CompetencesObjets) {
                i = self.by_id(kind, &value) as i64;
                continue;
            }
            let record = if i < 0 {
                &mut self.globals[kind as usize]
            } else {
                let list = &mut self.lists[kind as usize];
                if i as usize >= list.len() {
                    list.resize_with(i as usize + 1, Record::default);
                }
                &mut list[i as usize]
            };
            record.set(name.to_owned(), j, value);
        }
    }

    fn header(&mut self, line: &str, section: &mut Option<Kind>, i: &mut i64) {
        // (prefix, kind, switches the section)
        const ENTITY: [(&str, Kind, bool); 11] = [
            ("[Ressource_", Kind::Ressources, false),
            ("[TerrainRessource_", Kind::TerrainRessources, false),
            ("[Batiment_", Kind::Batiments, true),
            ("[CompetenceObjet_", Kind::CompetencesObjets, true),
            ("[Objet_", Kind::Objets, true),
            ("[Peuple_", Kind::Peuples, true),
            ("[Effet_", Kind::Effets, true),
            ("[Race_", Kind::Races, true),
            ("[Decor_", Kind::Decors, false),
            ("[Resistance_", Kind::Resistances, true),
            ("[Statut_", Kind::Statuts, true),
        ];
        const SECTION: [(&str, Kind); 11] = [
            ("[Ressources]", Kind::Ressources),
            ("[Terrain_Ressources]", Kind::TerrainRessources),
            ("[Batiments]", Kind::Batiments),
            ("[CompetencesObjets]", Kind::CompetencesObjets),
            ("[Objets]", Kind::Objets),
            ("[Peuples]", Kind::Peuples),
            ("[Effets]", Kind::Effets),
            ("[Races]", Kind::Races),
            ("[Decors]", Kind::Decors),
            ("[Resistances]", Kind::Resistances),
            ("[Statuts]", Kind::Statuts),
        ];
        for (prefix, kind) in SECTION {
            if line.starts_with(prefix) {
                *section = Some(kind);
                *i = -1;
                return;
            }
        }
        for (prefix, kind, switches) in ENTITY {
            let Some(rest) = line.strip_prefix(prefix) else { continue };
            // everything but the last character, assumed to be ']'
            let index = &rest[..rest.len().saturating_sub(1)];
            if switches {
                *section = Some(kind);
            }
            *i = match kind {
                Kind::Batiments | Kind::CompetencesObjets => self.by_id(kind, index) as i64,
                _ if index == "+" => self.lists[kind as usize].len() as i64,
                _ => vb_val(index) as i64 - 1,
            };
            let list = &mut self.lists[kind as usize];
            if *i >= 0 && *i as usize >= list.len() {
                list.resize_with(*i as usize + 1, Record::default);
            }
            return;
        }
        // anything else is ignored: section and entity unchanged
    }

    /// `Gestion_Cle_Batiments` / `Gestion_Cle_CompetencesObjets`: index of the entity with
    /// this ID, appended if unknown. An empty ID is index 0.
    fn by_id(&mut self, kind: Kind, id: &str) -> usize {
        if id.is_empty() {
            return 0;
        }
        let list = &mut self.lists[kind as usize];
        if let Some(i) = list.iter().position(|r| r.id == id) {
            return i;
        }
        list.push(Record { id: id.to_owned(), fields: Vec::new() });
        list.len() - 1
    }

    /// Index of a string-keyed entity, without creating it.
    pub fn find_id(&self, kind: Kind, id: &str) -> Option<usize> {
        self.lists[kind as usize].iter().position(|r| r.id == id)
    }
}

/// `Parametre_Filtre`: trim spaces, strip leading tabs, trim spaces again.
fn filter(line: &str) -> &str {
    line.trim_matches(' ').trim_start_matches('\t').trim_matches(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_and_keys() {
        let mut p = Params::default();
        p.parse_file(
            "[Races]\nNombreObjets=2\nObjet(1) = 113\n[Race_1]\n\tNom = Humain\n\tNomFR = Humain\n\
             quantite = 3\n[Race_+]\nNom=Nain\nStatut=1\nStatut=5\n[Batiment_PALAIS]\nNom=Palais\n",
        );
        let races = p.list(Kind::Races);
        assert_eq!(races.len(), 2);
        assert_eq!(races[0].get_l("Nom", "FR"), Some("Humain"));
        assert_eq!(races[0].get("Quantite"), None);
        assert_eq!(races[1].all("Statut").collect::<Vec<_>>(), ["1", "5"]);
        assert_eq!(p.global(Kind::Races).num_j("Objet", 1, 0.0), 113.0);
        assert_eq!(p.find_id(Kind::Batiments, "PALAIS"), Some(0));
    }

    #[test]
    fn entity_header_without_section_switch() {
        let mut p = Params::default();
        p.parse_file("[Decors]\n[Decor_1]\nNom=Pont\n[Decor_2]\nNom=Drapeau\n");
        assert_eq!(p.list(Kind::Decors)[1].get("Nom"), Some("Drapeau"));
    }
}
