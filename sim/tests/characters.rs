//! The characters human players make to play as (`WorldSetup::characters`).
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::perso::Look;
use sim::rules::Rules;
use sim::world::{CharacterSetup, World, WorldSetup};
use std::path::Path;

fn load() -> Option<(Files, Rules)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    if !root.exists() {
        eprintln!("no assets, skipping");
        return None;
    }
    let files = Files::from_dir(&root).unwrap();
    let rules = Rules::load(&files, "Francais").unwrap();
    Some((files, rules))
}

/// A character of `race` and sex, with the last file of each of its lists.
fn character(rules: &Rules, race: usize, female: bool) -> CharacterSetup {
    let look = rules.races[race].look_files[female as usize].iter().filter(|l| !l.is_empty()).map(|l| l.last().unwrap().clone()).collect();
    CharacterSetup { race, female, look }
}

fn player(rules: &Rules, files: &Files, characters: Vec<Option<CharacterSetup>>) -> sim::perso::Perso {
    let setup = WorldSetup { seed: 3, names: vec!["Alix".into()], characters, ..Default::default() };
    let w = World::create(rules, files, &setup).unwrap();
    w.persos[w.player.unwrap()].clone()
}

/// The citizen taken over becomes the character: race, sex, look; with layers or a sheet.
#[test]
fn takes_the_character() {
    let Some((files, rules)) = load() else { return };
    let plain = player(&rules, &files, vec![]);
    for whole in [false, true] {
        let race = (0..rules.races.len())
            .find(|&r| rules.races[r].playable && rules.races[r].whole_sheet == whole && r != plain.race && !rules.races[r].no_female)
            .unwrap();
        let c = character(&rules, race, true);
        let p = player(&rules, &files, vec![Some(c.clone())]);
        assert_eq!((p.race, p.female, p.name.as_str()), (race, true, "Alix"));
        let want = if whole { Look::Sheet(c.look[0].clone()) } else { Look::Layers(c.look.clone()) };
        assert_eq!(p.look, want);
        assert_eq!((p.w, p.h), rules.races[race].size);
    }
}

/// A character the rules do not allow leaves the citizen as it is.
#[test]
fn keeps_the_citizen_otherwise() {
    let Some((files, rules)) = load() else { return };
    let plain = player(&rules, &files, vec![]);
    let human = (0..rules.races.len()).find(|&r| rules.races[r].playable && !rules.races[r].whole_sheet).unwrap();
    let not_playable = (0..rules.races.len()).find(|&r| !rules.races[r].playable).unwrap();
    let only_female = (0..rules.races.len()).find(|&r| rules.races[r].playable && rules.races[r].no_male).unwrap();
    let mut wrong_look = character(&rules, human, false);
    wrong_look.look[0] = "images/persos/nowhere.png".into();
    let mut short = character(&rules, human, false);
    short.look.pop();
    for c in [character(&rules, not_playable, false), character(&rules, only_female, false), wrong_look, short] {
        let p = player(&rules, &files, vec![Some(c.clone())]);
        assert_eq!((p.race, p.female, &p.look), (plain.race, plain.female, &plain.look), "{c:?}");
    }
}
