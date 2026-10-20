//! The translations change only text: every language's files (`Donnees/<language>/`) have the
//! same files, sections and keys as the French ones, with the same numbers, and the official
//! mods' language overlays (`Donnees/ModsOfficiels/<language>/`) change no number of the files
//! they apply to. So players in different languages play the same game (network play depends
//! on it).
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::ini::Ini;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Keys that may differ between languages, by (file, key prefix): each kind of speech has as
/// many lines as its translation (`Message(n)`, counted by `NombreMessages`).
const PER_TRANSLATION: [(&str, &str); 2] = [("Commentaires.ini", "nombremessages"), ("Commentaires.ini", "message(")];

fn per_translation(f: &str, k: &str) -> bool {
    PER_TRANSLATION.iter().any(|&(tf, tk)| tf == f && k.starts_with(tk))
}

fn data() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/Donnees");
    root.exists().then_some(root).or_else(|| {
        eprintln!("no assets, skipping");
        None
    })
}

/// A value that is a number, or a list of them.
fn numeric(v: &str) -> bool {
    v.chars().any(|c| c.is_ascii_digit()) && v.chars().all(|c| c.is_ascii_digit() || "+-., ;".contains(c))
}

fn files(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect()
}

fn ini(path: &Path) -> Ini {
    Ini::parse(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

fn languages(root: &Path) -> Vec<String> {
    let mut langs: Vec<String> = files(root).into_iter().filter(|f| root.join(f).is_dir() && f != "ModsOfficiels").collect();
    langs.sort();
    langs
}

#[test]
fn languages_differ_only_in_text() {
    let Some(root) = data() else { return };
    let langs = languages(&root);
    assert!(langs.len() >= 2, "languages: {langs:?}");
    let reference = root.join("Francais");
    let mut errors = Vec::new();
    for lang in langs.iter().filter(|l| *l != "Francais") {
        let dir = root.join(lang);
        let (ours, theirs) = (files(&reference), files(&dir));
        for f in ours.symmetric_difference(&theirs) {
            errors.push(format!("{f}: only in {}", if ours.contains(f) { "Francais" } else { lang }));
        }
        for f in ours.intersection(&theirs).filter(|f| f.ends_with(".ini")) {
            let (a, b) = (ini(&reference.join(f)), ini(&dir.join(f)));
            let names = |i: &Ini| i.sections().map(|(n, _)| n.clone()).collect::<BTreeSet<_>>();
            let (sa, sb) = (names(&a), names(&b));
            for s in sa.symmetric_difference(&sb) {
                errors.push(format!("{lang}/{f}: section [{s}] only in {}", if sa.contains(s) { "Francais" } else { lang }));
            }
            for s in sa.intersection(&sb) {
                let (ka, kb) = (a.section(s).unwrap(), b.section(s).unwrap());
                for (k, va) in ka.iter().filter(|(k, _)| !per_translation(f, k)) {
                    let Some(vb) = kb.get(k) else {
                        errors.push(format!("{lang}/{f}: [{s}] {k} missing"));
                        continue;
                    };
                    if (numeric(va) || numeric(vb)) && va != vb {
                        errors.push(format!("{lang}/{f}: [{s}] {k} = {vb}, in French {va}"));
                    }
                }
                for k in kb.keys().filter(|k| !ka.contains_key(*k) && !per_translation(f, k)) {
                    errors.push(format!("{lang}/{f}: [{s}] {k} not in French"));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} differences:\n{}", errors.len(), errors.join("\n"));
}

#[test]
fn language_mods_change_only_text() {
    let Some(root) = data() else { return };
    let mods = root.join("ModsOfficiels");
    let mut errors = Vec::new();
    for lang in languages(&root) {
        let dir = mods.join(&lang);
        if !dir.exists() {
            continue;
        }
        for f in files(&dir).into_iter().filter(|f| f.ends_with(".ini")) {
            // what the overlay applies to: the base file, then the mods for every language
            let base: Vec<Ini> = [root.join(&f), mods.join(&f)].iter().filter(|p| p.exists()).map(|p| ini(p)).collect();
            for (s, keys) in ini(&dir.join(&f)).sections() {
                for (k, v) in keys {
                    let before = base.iter().rev().find_map(|i| i.get(s, k));
                    if numeric(v) || before.is_some_and(numeric) {
                        errors.push(format!("ModsOfficiels/{lang}/{f}: [{s}] {k} = {v} (base {before:?})"));
                    }
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} numbers in language mods:\n{}", errors.len(), errors.join("\n"));
}

/// Each language's names and descriptions land on the records of the French ones: in every
/// language, a record has a text wherever the French has one, and nowhere else (the official
/// mods' language overlays carry them; a wrong header there names other records).
#[test]
fn language_texts_fill_the_same_records() {
    use sim::params::{Kind, Params};
    let Some(root) = data() else { return };
    let files = sim::files::Files::from_dir(root.parent().unwrap()).unwrap();
    let french = Params::load(&files, "Francais").unwrap();
    const KINDS: [Kind; 11] = [
        Kind::Ressources,
        Kind::TerrainRessources,
        Kind::Batiments,
        Kind::CompetencesObjets,
        Kind::Objets,
        Kind::Peuples,
        Kind::Effets,
        Kind::Races,
        Kind::Decors,
        Kind::Resistances,
        Kind::Statuts,
    ];
    const TEXTS: [&str; 9] = ["Nom", "NomPluriel", "NomM", "NomF", "Description", "MetierM", "MetierF", "Metier", "NomEffet"];
    let mut errors = Vec::new();
    // (the original's English has a few gaps, where its French shows)
    for language in sim::rules::languages(&files).iter().filter(|l| !["Francais", "English"].contains(&l.folder.as_str())) {
        let other = Params::load(&files, &language.folder).unwrap();
        for kind in KINDS {
            let (a, b) = (french.list(kind), other.list(kind));
            if a.len() != b.len() {
                errors.push(format!("{}: {kind:?}: {} records, in French {}", language.folder, b.len(), a.len()));
                continue;
            }
            for (i, (ra, rb)) in a.iter().zip(b).enumerate() {
                for key in TEXTS {
                    let has = |r: &sim::params::Record, suffix: &str| r.get_l(key, suffix).is_some_and(|t| !t.trim().is_empty());
                    if has(ra, "FR") != has(rb, &language.suffix) {
                        errors.push(format!("{}: {kind:?} {i} {key}: {:?}, in French {:?}", language.folder, rb.get_l(key, &language.suffix), ra.get_l(key, "FR")));
                    }
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} differences:\n{}", errors.len(), errors.join("\n"));
}
