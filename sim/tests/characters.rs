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
    CharacterSetup { race, female, look, progress: None }
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

/// What a character kept from its earlier games: the XP its bars and skills earned, with the
/// levels it earned with them, and its bestiary; its bars as full as a new one's.
#[test]
fn keeps_its_progress() {
    use sim::stats::{Progress, Trained};
    let Some((files, rules)) = load() else { return };
    let race = (0..rules.races.len()).find(|&r| rules.races[r].playable && rules.races[r].experience && !rules.races[r].whole_sheet).unwrap();
    let mut trained = player(&rules, &files, vec![Some(character(&rules, race, false))]);
    for (t, gain) in [(Trained::Bar(0), 500.0), (Trained::Bar(4), 3000.0), (Trained::Special(2), 70.0), (Trained::Resource(3), 55.5), (Trained::Service(1), 12.25), (Trained::Object(5), 99.0)] {
        trained.train(&rules, t, gain);
    }
    trained.bestiary[2] = 7;
    let progress = trained.progress();
    assert_eq!(Progress::parse(&progress.text()).map(|p| p.text()), Some(progress.text()));
    let mut c = character(&rules, race, false);
    c.progress = Some(progress.clone());
    let p = player(&rules, &files, vec![Some(c)]);
    assert_eq!(p.progress(), progress);
    assert_eq!(p.max, trained.max);
    assert!(p.max[0] > player(&rules, &files, vec![Some(character(&rules, race, false))]).max[0]);
    let levels = |p: &sim::perso::Perso| (p.bar_xp.map(|x| x.level), [&p.special, &p.resource_skills, &p.services, &p.objects].map(|v| v.iter().map(|s| (s.carac, s.xp.level)).collect::<Vec<_>>()));
    assert_eq!(levels(&p), levels(&trained));
    assert_eq!(p.bars[0], p.bonus_max[0] as f64);
    assert_eq!(p.bestiary[2], 7);
}

/// The text of a progress: short, and only numbers.
#[test]
fn progress_text() {
    use sim::stats::Progress;
    let p = Progress { bars: vec![1.0, 0.5, 0.0, 0.0], special: vec![0.0; 4], resources: vec![], services: vec![0.126], objects: vec![], bestiary: vec![0, 3, 0] };
    assert_eq!(p.text(), "b=1,0.5;v=0.13;k=0,3");
    assert_eq!(Progress::parse("b=1,0.5;v=0.13;k=0,3").map(|p| p.text()), Some(p.text()));
    for bad in ["b=x", "q=1", "b=-1", "k=1.5", "b"] {
        assert_eq!(Progress::parse(bad), None, "{bad}");
    }
    assert_eq!(Progress::parse(""), Some(Progress::default()));
}
