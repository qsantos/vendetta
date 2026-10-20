//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
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

/// Saving, loading and continuing gives exactly the game played without interruption.
#[test]
fn save_load_continue_is_seamless() {
    let Some((files, rules)) = load() else { return };
    let setup = WorldSetup { seed: 42, ..Default::default() };
    let mut straight = World::create(&rules, &files, &setup).unwrap();
    for _ in 0..3000 {
        straight.step(&rules, &[]);
    }
    let bytes = straight.save();
    eprintln!("save: {} bytes", bytes.len());
    let mut resumed = World::load(&rules, &bytes).unwrap();
    assert_eq!(resumed.save(), bytes);
    for _ in 0..3000 {
        straight.step(&rules, &[]);
        resumed.step(&rules, &[]);
    }
    assert_eq!(resumed.save(), straight.save());
}

#[test]
fn garbage_is_rejected() {
    let Some((_, rules)) = load() else { return };
    assert!(World::load(&rules, b"").is_err());
    assert!(World::load(&rules, b"VNDT\x00\x00\x00\x00junk").is_err());
}

/// The same with two human players giving orders, saved at various ticks and compared at
/// every tick after (network play reloads a save and replays the ticks since).
#[test]
fn save_load_continue_with_players_and_orders() {
    use sim::world::Order;
    let Some((files, rules)) = load() else { return };
    let setup = WorldSetup { seed: 7, players: vec![0, 0], ..Default::default() };
    let world = World::create(&rules, &files, &setup).unwrap();
    let humans: Vec<usize> = world.humans(&[0, 0]).into_iter().flatten().collect();
    let orders = |t: u64| -> Vec<Order> {
        let (a, b) = (humans[0], humans[1]);
        match t % 40 {
            3 => vec![Order::Move { perso: a, x: 300 + (t as i64 * 7) % 400, y: 300 + (t as i64 * 13) % 300 }],
            17 => vec![Order::Move { perso: b, x: 500 - (t as i64 * 11) % 300, y: 200 + (t as i64 * 5) % 400 }],
            29 => vec![Order::FindResource { perso: a, kind: 2 }],
            _ => vec![],
        }
    };
    for save_at in [1u64, 37, 102, 250] {
        let mut straight = World::create(&rules, &files, &setup).unwrap();
        while straight.tick < save_at {
            let o = orders(straight.tick);
            straight.step(&rules, &o);
        }
        let mut resumed = World::load(&rules, &straight.save()).unwrap();
        for _ in 0..400 {
            let o = orders(straight.tick);
            straight.step(&rules, &o);
            resumed.step(&rules, &o);
            assert_eq!(resumed.digest(), straight.digest(), "saved at {save_at}, differs at tick {}", straight.tick);
        }
    }
}
