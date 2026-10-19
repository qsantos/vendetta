//! Golden tests: fixed worlds played for a while must give exactly the same state, hashed at
//! checkpoints. They lock behaviour, so that optimisations change only speed. When a change
//! is meant to alter behaviour, run with `GOLDEN_PRINT=1` and update the hashes below in the
//! same commit.
//!
//! Needs the converted assets in `rust/assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::orders::Order;
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
    world.save().iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
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

/// The default random world, left to its villagers.
#[test]
fn default_world() {
    let Some((files, rules)) = load() else { return };
    let world = World::create(&rules, &files, &WorldSetup { seed: 1, ..Default::default() }).unwrap();
    check("default_world", &rules, world, &[], &[(0, 0xd730294707a0932c), (500, 0xd6625fe8ec3dec30), (1500, 0x04b35823b86e828a), (3000, 0xa3e9f60617c2d883)]);
}

/// A shipped map.
#[test]
fn shipped_map() {
    let Some((files, rules)) = load() else { return };
    let setup = WorldSetup { seed: 2, map: Some("(04) Delta".into()), ..Default::default() };
    let world = World::create(&rules, &files, &setup).unwrap();
    check("shipped_map", &rules, world, &[], &[(0, 0x9c7554d7145c4c88), (1000, 0x49e52458583246f9), (2500, 0xdbf8bfaf61b73747)]);
}

/// A crowded world with many fiefs and the nearest-castle roads.
#[test]
fn crowded_world() {
    let Some((files, rules)) = load() else { return };
    let d = WorldSetup::default();
    let setup = WorldSetup {
        seed: 3,
        width: 256,
        height: 256,
        resources: d.resources * 4,
        fiefs: (0..8).map(|i| FiefSetup { people: i % rules.peoples.len(), team: i as i64 + 1, epoch: 0, citizens: 60 }).collect(),
        roads: Roads::Nearest,
        ..d
    };
    let world = World::create(&rules, &files, &setup).unwrap();
    check("crowded_world", &rules, world, &[], &[(0, 0x7e31f4e6d21372d8), (500, 0x93c536068e369a30), (1500, 0xa4a6204674fcd368)]);
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
    check("player_orders", &rules, world, &orders, &[(0, 0xeaf239bcac412726), (300, 0x0de65c2be1d64f48), (800, 0xf5e71ea5039d520b), (2000, 0xa3f75fc7dedac398)]);
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
    check("large_world", &rules, world, &[], &[(0, 0x739e3eff6a37f16e), (1000, 0x31ed80675c8b0ff6), (2000, 0xf2d82360e771f049)]);
}
