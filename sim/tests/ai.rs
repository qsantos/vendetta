//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::ia_file::{Step, find};
use sim::perso::action;
use sim::rules::Rules;
use sim::world::{World, WorldSetup};
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

fn world(files: &Files, rules: &Rules, seed: u32, map: Option<&str>) -> World {
    World::create(rules, files, &WorldSetup { seed, map: map.map(str::to_owned), ..Default::default() }).unwrap()
}

#[test]
fn every_shipped_ia_file_loads() {
    let Some((_, rules)) = load() else { return };
    assert_eq!(rules.ais.len(), 74);
    assert_eq!(rules.ais.iter().filter(|a| a.player).count(), 52, "the root folder");
    for a in &rules.ais {
        assert_eq!(a.epochs.len(), 5, "{}", a.name);
        for (e, epoch) in a.epochs.iter().enumerate() {
            assert!(!epoch.steps.is_empty(), "{} age {}", a.name, e + 1);
            let last = epoch.steps.last().unwrap();
            assert!(
                matches!(last, Step::Work | Step::Patrol | Step::AttackEnemy | Step::AttackKeep),
                "{} age {} ends with {last:?}",
                a.name,
                e + 1
            );
        }
    }
    let ble = &rules.ais[find(&rules.ais, "Fermier de ble.ia") as usize];
    assert_eq!(ble.comment, "IA officielle. C'est le villageois de base qui se contente de fabriquer du blé.");
    let farm = rules.buildings.iter().position(|b| b.id == "1").unwrap();
    assert_eq!(
        ble.epochs[0].steps,
        [
            Step::Mode(0),
            Step::EarnMoney { building: farm, quantity: 100 },
            Step::Object { object: 5, quantity: 1 },
            Step::Building(farm),
            Step::Resource { resource: 12, quantity: 5 },
            Step::Work,
        ]
    );
    let caserne = rules.buildings.iter().position(|b| b.id == "CASERNE").unwrap();
    let m = &rules.ais[find(&rules.ais, "milicien IMPERIAL.ia") as usize];
    assert!(m.epochs[1].steps.contains(&Step::Building(caserne)));
    assert_eq!(find(&rules.ais, "nope.ia"), -1);
}

#[test]
fn game_start_assigns_ais() {
    let Some((files, rules)) = load() else { return };
    let w = world(&files, &rules, 42, None);
    let player = w.player.unwrap();
    assert!(!w.persos[player].ai);
    for (i, p) in w.persos.iter().enumerate() {
        if i != player {
            assert!(p.ai, "{i}");
        }
        if rules.races[p.race].ai_select {
            assert!(p.choose_ai, "{i} draws an AI file");
        }
        if p.choose_ai {
            assert!(p.ai_index >= 0 && rules.ais[p.ai_index as usize].name == p.ai_file.to_lowercase());
        }
    }
    // IA_Init: the last character keeps zeroed goals.
    let last = w.persos.len() - 1;
    assert_eq!(w.ai.persos[last].build, 0);
    assert_eq!(w.ai.persos[0].build, -1);
    assert_eq!(w.ai.persos[0].house_distance, 96);
    assert_eq!(w.ai.persos[last].house_distance, 0);
    // 40 characters: LPerso = 39, so 2 AIs per tick and 23 search steps.
    assert_eq!((w.ai.per_tick, w.ai.searches), (1, 23));
}

#[test]
fn ai_init_draws() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 7, None);
    let mut rng = w.rng.clone();
    w.ai_init(&rules);
    // One angle for the last perso, the object cursor, then one angle per other perso.
    for _ in 0..w.persos.len() + 1 {
        rng.next();
    }
    assert_eq!(rng.next(), w.rng.next());
}

/// Every character runs `IAScripts\Test.txt`, which moves an idle character at once, so the
/// ones the round robin visited are walking.
#[test]
fn round_robin_visits_in_turn() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 3, None);
    let n = w.persos.len();
    let reset = |w: &mut World| {
        for p in &mut w.persos {
            p.ai = true;
            p.choose_script = true;
            p.script_file = "Test.txt".to_owned();
            p.berserk = 0;
            p.bonus_berserk = 0;
            p.speaks = true;
            p.comment.clear();
            p.set_visible(true);
            p.stop();
        }
    };
    for t in 0..n {
        reset(&mut w);
        w.step(&rules, &[]);
        let walking: Vec<usize> = (0..n).filter(|&i| w.persos[i].action == action::WALK).collect();
        let mut expected = [(2 * t + 1) % n, (2 * t + 2) % n];
        expected.sort();
        assert_eq!(walking, expected);
        assert!(w.persos[walking[0]].comment.starts_with("Je vais à X="));
    }
}

/// `Arreter` on a visible AI re-enters the AI at once.
#[test]
fn stopping_reenters_the_ai() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 3, None);
    let i = 5;
    let p = &mut w.persos[i];
    p.choose_script = true;
    p.script_file = "Test.txt".to_owned();
    p.set_visible(true);
    p.action = action::HARVEST;
    w.stop(&rules, i);
    assert_eq!(w.persos[i].action, action::WALK);
    // Hidden in its castle, it does not.
    let p = &mut w.persos[i];
    p.in_own_castle = true;
    p.action = action::HARVEST;
    w.stop(&rules, i);
    assert_eq!(w.persos[i].action, action::IDLE);
}

fn villager_with(files: &Files, rules: &Rules, ai: &str) -> (World, usize) {
    let mut w = world(files, rules, 42, None);
    let i = (0..w.persos.len()).find(|&i| w.persos[i].fief == 0 && Some(i) != w.player).unwrap();
    for (j, p) in w.persos.iter_mut().enumerate() {
        p.ai = j == i;
    }
    let p = &mut w.persos[i];
    p.choose_ai = true;
    p.ai_file = ai.to_owned();
    p.ai_index = find(&rules.ais, ai);
    assert!(p.ai_index >= 0);
    (w, i)
}

/// A villager builds the house of its `.ia` file, then produces in it.
fn builds_and_produces(ai: &str, building: &str) {
    let Some((files, rules)) = load() else { return };
    let (mut w, i) = villager_with(&files, &rules, ai);
    let kind = rules.buildings.iter().position(|b| b.id == building).unwrap();
    let product = rules.buildings[kind].product as usize;
    let mut built = None;
    for t in 0..30_000 {
        w.step(&rules, &[]);
        let m = &w.houses[i];
        if built.is_none() && m.built && m.kind == kind {
            built = Some(t);
        }
        if built.is_some() && (w.persos[i].action == action::WORK_HOME || m.stock() > 0.0) {
            eprintln!("{ai}: built after {} ticks, producing after {t}", built.unwrap());
            assert!(w.persos[i].resource(product) > 0.0 || m.stock() > 0.0);
            return;
        }
    }
    let p = &w.persos[i];
    panic!("{ai}: built {built:?}, house {} {}, action {}", w.houses[i].kind, w.houses[i].built, p.action);
}

#[test]
fn wheat_farmer_builds_and_produces() {
    builds_and_produces("Fermier de ble.ia", "1");
}

#[test]
fn woodcutter_builds_and_produces() {
    builds_and_produces("Bucheron.ia", "3");
}

#[test]
fn monsters_wander_and_pick_targets() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 42, None);
    let monsters: Vec<usize> = (0..w.persos.len()).filter(|&i| !w.persos[i].choose_ai && w.persos[i].ai).collect();
    assert!(monsters.len() > 10);
    let start: Vec<(i64, i64)> = monsters.iter().map(|&i| (w.persos[i].x, w.persos[i].y)).collect();
    let mut targeted = 0;
    for _ in 0..3000 {
        w.step(&rules, &[]);
        for &i in &monsters {
            let p = &w.persos[i];
            if p.action == 300 {
                let t = &w.persos[p.perso_target];
                assert_ne!(t.team, p.team, "attacks an enemy");
                targeted += 1;
            }
        }
    }
    let moved = monsters.iter().zip(&start).filter(|&(&i, &s)| (w.persos[i].x, w.persos[i].y) != s).count();
    eprintln!("{moved} of {} monsters moved, {targeted} attack-ticks", monsters.len());
    assert!(moved > monsters.len() / 2);
    assert!(targeted > 0);
}

fn long_run(map: Option<&str>) {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 2024, map);
    for _ in 0..20_000 {
        w.step(&rules, &[]);
    }
    let built = w.houses.iter().filter(|m| m.built).count();
    let fiefs: Vec<usize> = (0..w.fiefs.len())
        .map(|f| (0..w.houses.len()).filter(|&j| w.houses[j].built && w.persos[j].fief == f).count())
        .collect();
    eprintln!(
        "{map:?}: {built} built, per fief {fiefs:?}, epochs {:?}",
        w.fiefs.iter().map(|f| f.epoch).collect::<Vec<_>>()
    );
    assert!(built >= 5);
}

#[test]
fn long_run_random_world() {
    long_run(None);
}

#[test]
fn long_run_shipped_map() {
    long_run(Some("(04) Delta"));
}

#[test]
fn ai_is_deterministic() {
    let Some((files, rules)) = load() else { return };
    let state = |w: &World| {
        let p: Vec<_> = w.persos.iter().map(|p| (p.x, p.y, p.action, p.money().to_bits(), p.comment.clone())).collect();
        let a: Vec<_> =
            w.ai.persos
                .iter()
                .map(|s| (s.build, s.resource, s.object, s.house_cursor, s.patrol_angle.to_bits()))
                .collect();
        (p, a, w.ai.cursor, w.ai.object_cursor)
    };
    let mut a = world(&files, &rules, 99, None);
    let mut b = world(&files, &rules, 99, None);
    for _ in 0..5000 {
        a.step(&rules, &[]);
        b.step(&rules, &[]);
    }
    assert_eq!(state(&a), state(&b));
    assert_eq!(a.rng.next(), b.rng.next());
}

#[test]
fn comments_draw_and_expire() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules, 1, None);
    let i = (0..w.persos.len()).find(|&i| w.persos[i].speaks && w.persos[i].special_comments <= 0).unwrap();
    let mut rng = w.rng.clone();
    w.say(&rules, i, 16);
    rng.next();
    assert_eq!(rng.next(), w.rng.clone().next(), "one draw per line");
    let line = w.persos[i].comment.clone();
    assert!(rules.comments[16].contains(&line));
    // A new line does not replace the current one.
    w.say(&rules, i, 0);
    assert_eq!(w.persos[i].comment, line);
    for p in &mut w.persos {
        p.ai = false;
    }
    let ticks = (line.chars().count() as f64 * rules.comment_duration * w.speed as f64) as usize + 2;
    for _ in 0..ticks {
        w.step(&rules, &[]);
    }
    assert!(w.persos[i].comment.is_empty());
}
