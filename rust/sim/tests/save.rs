//! Needs the converted assets in `rust/assets` (see `assetconv`); skipped otherwise.

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
