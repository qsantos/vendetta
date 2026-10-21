//! Golden tests: fixed worlds played for a while must give exactly the same state, hashed at
//! checkpoints. They lock behaviour, so that optimisations change only speed. When a change
//! is meant to alter behaviour, run with `GOLDEN_PRINT=1` and update the hashes below in the
//! same commit.
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::orders::Order;
use sim::path::{Chase, Paths};
use sim::rules::Rules;
use sim::world::{FiefSetup, Roads, World, WorldSetup};
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

/// 64-bit FNV-1a of the saved world: everything that persists, the random generator included.
fn hash(world: &World) -> u64 {
    world.digest()
}

/// Plays `world` up to each checkpoint tick, giving the player `orders` at their ticks, and
/// compares the hashes there with `expected` (all printed with `GOLDEN_PRINT=1`).
fn check(name: &str, rules: &Rules, mut world: World, orders: &[(u64, Order)], expected: &[(u64, u64)]) {
    let mut got = vec![(0, hash(&world))];
    for &(tick, _) in &expected[1..] {
        while world.tick < tick {
            let now: Vec<Order> = orders.iter().filter(|(t, _)| *t == world.tick + 1).map(|(_, o)| o.clone()).collect();
            world.step(rules, &now);
        }
        got.push((tick, hash(&world)));
    }
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        let lines: Vec<String> = got.iter().map(|(t, h)| format!("({t}, 0x{h:016x})")).collect();
        eprintln!("{name}: [{}]", lines.join(", "));
    }
    for ((tick, want), (_, have)) in expected.iter().zip(&got) {
        assert_eq!(*have, *want, "{name}: state at tick {tick} changed (0x{have:016x})");
    }
}

fn player(world: &World) -> usize {
    world.persos.iter().position(|p| p.player).unwrap()
}

/// The default random world on a torus, left to its villagers.
#[test]
fn torus_world() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &WorldSetup { seed: 1, torus: true, ..Default::default() }).unwrap();
    check("torus_world", &rules, world, &[], &[(0, 0x2e3dd98dc9e10933), (500, 0x89b1a06cc7ca50cf), (1500, 0xf471896b132e841d), (3000, 0xe738720e0d260c5c)]);
}

/// On a torus, a character sent to a point just past the left edge walks there across the edge,
/// not the long way through the map.
#[test]
fn torus_crossing() {
    let Some((files, rules)) = load() else { return };
    let mut world = World::create(&rules, &files, &WorldSetup { seed: 1, torus: true, ..Default::default() }).unwrap();
    let perso = player(&world);
    let (w, _) = world.terrain.size_px();
    let (x, y) = (world.persos[perso].x, world.persos[perso].y);
    // past the edge on the side the character is nearest to
    let (tx, left) = if x < w / 2 { (w - 100, true) } else { (100, false) };
    world.step(&rules, &[Order::Move { perso, x: tx, y }]);
    let mut crossed = false;
    for _ in 0..3000 {
        let before = world.persos[perso].x;
        world.step(&rules, &[]);
        let now = world.persos[perso].x;
        assert!((0..w).contains(&now), "off the map at {now}");
        crossed |= (now - before).abs() > w / 2;
        if left {
            assert!(now <= before || crossed, "walking the long way: {before} → {now}");
        }
        if !world.persos[perso].moving {
            break;
        }
    }
    assert!(crossed, "never crossed the edge");
    assert_eq!(world.persos[perso].x, tx);
}

/// The default random world, left to its villagers.
#[test]
fn default_world() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &WorldSetup { seed: 1, ..Default::default() }).unwrap();
    check("default_world", &rules, world, &[], &[(0, 0x371660d50611670c), (500, 0x718101c76a833636), (1500, 0x828e78b4e4d8c8b9), (3000, 0x5059f6ab37a66aa9)]);
}

/// A shipped map.
#[test]
fn shipped_map() {
    let Some((files, rules)) = load() else { return };
    let setup = WorldSetup { seed: 2, map: Some("(04) Delta".into()), ..Default::default() };
    let world = World::create(&rules, &files, &setup).unwrap();
    check("shipped_map", &rules, world, &[], &[(0, 0x590086b7eb589067), (1000, 0x5473b1d23006ba72), (2500, 0x03688fdb6b4ea943)]);
}

/// Many fiefs and the nearest-castle roads.
fn crowded(rules: &Rules) -> WorldSetup {
    let d = WorldSetup::default();
    WorldSetup {
        seed: 3,
        width: 256,
        height: 256,
        resources: d.resources * 4,
        fiefs: (0..8).map(|i| FiefSetup { people: i % rules.peoples.len(), team: i as i64 + 1, epoch: 0, citizens: 60 }).collect(),
        roads: Roads::Nearest,
        ..d
    }
}

/// A crowded world.
#[test]
fn crowded_world() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &crowded(&rules)).unwrap();
    check("crowded_world", &rules, world, &[], &[(0, 0xa5cb15ebc88627fa), (500, 0x5c4492091b8cef99), (1500, 0xd80fa9bb77aea061)]);
}

/// A crowded world on a torus, walking the fastest paths and heading off the targets (the
/// client's defaults).
#[test]
fn fastest_paths() {
    let Some((files, rules)) = load() else { return };
    let setup = WorldSetup { torus: true, paths: Paths::Fastest, chase: Chase::Intercept, ..crowded(&rules) };
    let world = World::create(&rules, &files, &setup).unwrap();
    check("fastest_paths", &rules, world, &[], &[(0, 0xf7fba3d80eeb0880), (500, 0x44adb9f24a218408), (1500, 0x15bc4509b1858ec1)]);
}

/// The player at work and at war: harvesting, going home, attacking the nearest enemy.
#[test]
fn player_orders() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &WorldSetup { seed: 4, ..Default::default() }).unwrap();
    let perso = player(&world);
    let orders = [
        (5, Order::FindResource { perso, kind: 2 }),
        (400, Order::GoOwnCastle { perso }),
        (600, Order::AttackNearest { perso, kind: 0, send: false, ctrl: false, shift: false }),
        (1200, Order::FindTreasure { perso }),
    ];
    check("player_orders", &rules, world, &orders, &[(0, 0x42f607aff63d8fb4), (300, 0x99a213c378ba85ce), (800, 0x17f23eb3426d1667), (2000, 0x64349e544b69b561)]);
}

/// A large world, the scale optimisations target: 2400 characters on a 512×512 map.
#[test]
fn large_world() {
    let Some((files, rules)) = load() else { return };
    let d = WorldSetup::default();
    let setup = WorldSetup {
        seed: 5,
        width: 512,
        height: 512,
        resources: d.resources * 16,
        fiefs: (0..16).map(|i| FiefSetup { people: i % rules.peoples.len(), team: i as i64 + 1, epoch: 0, citizens: 150 }).collect(),
        roads: Roads::Nearest,
        ..d
    };
    let world = World::create(&rules, &files, &setup).unwrap();
    check("large_world", &rules, world, &[], &[(0, 0xd1c8efebe021d444), (1000, 0x3acd9604660a6857), (2000, 0xca3fd5f790cc7ab6)]);
}

/// Three human players, two of them in the same fief, giving orders.
#[test]
fn several_players() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &WorldSetup { seed: 6, players: vec![0, 0, 2], ..Default::default() }).unwrap();
    let humans: Vec<usize> = (0..world.persos.len()).filter(|&i| world.persos[i].player).collect();
    assert_eq!(humans.len(), 3);
    assert!(humans.iter().all(|&i| !world.persos[i].ai));
    assert_eq!(humans.iter().map(|&i| world.persos[i].fief).collect::<Vec<_>>(), [0, 0, 2]);
    assert_eq!(world.humans(&[0, 0, 2]), humans.iter().map(|&i| Some(i)).collect::<Vec<_>>());
    assert_eq!(world.humans(&[2, 0, 0]), [Some(humans[2]), Some(humans[0]), Some(humans[1])]);
    // named after their players, the one without a name keeping its citizen's
    let named = World::create(&rules, &files, &WorldSetup { seed: 6, players: vec![0, 0, 2], names: vec!["Alice".into(), "".into(), "Carol".into()], ..Default::default() }).unwrap();
    let names: Vec<&str> = humans.iter().map(|&i| named.persos[i].name.as_str()).collect();
    assert_eq!((names[0], names[2]), ("Alice", "Carol"));
    assert_eq!(names[1], world.persos[humans[1]].name);
    let orders = [
        (5, Order::FindResource { perso: humans[0], kind: 2 }),
        (5, Order::FindResource { perso: humans[1], kind: 3 }),
        (300, Order::AttackNearest { perso: humans[2], kind: 0, send: false, ctrl: false, shift: false }),
    ];
    check("several_players", &rules, world, &orders, &[(0, 0xa6be2ce7768d26c2), (500, 0xa1dcc6e04086671c), (1500, 0x8772b62c17323d44)]);
}

/// The language changes only text, not the game: a world plays the same in every language as
/// in French, to the byte (so players in different languages can play together).
#[test]
fn languages_play_alike() {
    let Some((files, french)) = load() else { return };
    let languages = sim::rules::languages(&files);
    assert!(languages.len() >= 2, "{languages:?}");
    for language in languages.iter().filter(|l| l.folder != "Francais") {
        let other = Rules::load(&files, &language.folder).unwrap();
        for setup in [WorldSetup { seed: 1, ..Default::default() }, crowded(&french)] {
            let mut worlds = [&french, &other].map(|r| World::create(r, &files, &setup).unwrap());
            for _ in 0..1500 {
                for (w, r) in worlds.iter_mut().zip([&french, &other]) {
                    w.step(r, &[]);
                }
                assert_eq!(worlds[0].digest(), worlds[1].digest(), "{}, seed {}: tick {}", language.folder, setup.seed, worlds[0].tick);
            }
            // the cases that once differed: a race's name, a tomb's inscription
            assert!(worlds[0].persos.iter().any(|p| p.name.is_empty()));
            if setup.seed == 3 {
                assert!(worlds[0].decors.iter().any(|d| d.epitaph.is_some()));
            }
        }
    }
}
