//! The player's language (`Langues.ini`): the data's text comes from its folder in `Donnees/`;
//! the few texts the port adds, not in the original's files, are here in each language.

use crate::{options, storage};
use sim::rules::Language;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The languages of the port's own texts (`Langues.ini` suffixes), in the order `tr` takes them.
const LANGUAGES: [&str; 5] = ["FR", "EN", "ES", "JA", "ZH"];

/// The current one, in `LANGUAGES`.
static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// The language: the menu's, else `lang=`, else the original's French (the first listed).
pub fn choose(list: &[Language]) -> Language {
    let wanted = storage::language().or_else(|| options::Options::load().lang);
    let language = wanted.and_then(|w| list.iter().find(|l| l.suffix == w)).or(list.first()).cloned();
    let language = language.unwrap_or_else(|| Language { name: "Français".into(), folder: "Francais".into(), suffix: "FR".into() });
    // a language without the port's texts gets them in English
    CURRENT.store(LANGUAGES.iter().position(|&l| l == language.suffix).unwrap_or(1), Ordering::Relaxed);
    language
}

/// A text the port adds, in each language of `LANGUAGES`.
pub fn tr(texts: [&'static str; 5]) -> &'static str {
    texts[CURRENT.load(Ordering::Relaxed)]
}
