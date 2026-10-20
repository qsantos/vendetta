//! The player's language (`Langues.ini`): the data's text comes from its folder in `Donnees/`;
//! the few texts the port adds, not in the original's files, are here in French and English.

use crate::{options, storage};
use sim::rules::Language;
use std::sync::atomic::{AtomicBool, Ordering};

static FRENCH: AtomicBool = AtomicBool::new(true);

/// The language: the menu's, else `lang=`, else the original's French (the first listed).
pub fn choose(list: &[Language]) -> Language {
    let wanted = storage::language().or_else(|| options::Options::load().lang);
    let language = wanted.and_then(|w| list.iter().find(|l| l.suffix == w)).or(list.first()).cloned();
    let language = language.unwrap_or_else(|| Language { name: "Français".into(), folder: "Francais".into(), suffix: "FR".into() });
    FRENCH.store(language.suffix == "FR", Ordering::Relaxed);
    language
}

/// A text the port adds: `fr` in French, `en` in any other language.
pub fn tr(fr: &'static str, en: &'static str) -> &'static str {
    if FRENCH.load(Ordering::Relaxed) { fr } else { en }
}
