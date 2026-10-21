//! Where the AI builds (`WorldSetup::towns`).
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::landscape::Landscape;
use sim::rules::Rules;
use sim::world::{FiefSetup, Towns, World, WorldSetup};
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

/// The houses standing after a while, by distance (in cells) to their fief's castle; and how
/// many lie less than a cell from another house of their fief.
fn towns(files: &Files, rules: &Rules, towns: Towns, torus: bool) -> (Vec<f64>, usize) {
    let fiefs = (0..4).map(|i| FiefSetup { people: 0, team: i + 1, epoch: 0, citizens: 30 }).collect();
    let setup = WorldSetup { seed: 3, width: 192, height: 192, resources: 120, fiefs, towns, torus, landscape: Landscape::Natural, ..Default::default() };
    let mut w = World::create(rules, files, &setup).unwrap();
    for _ in 0..8000 {
        w.step(rules, &[]);
    }
    let standing: Vec<usize> = (0..w.houses.len()).filter(|&j| w.houses[j].visible()).collect();
    let mut distances: Vec<f64> = standing
        .iter()
        .map(|&j| {
            let (m, c) = (&w.houses[j], &w.castles[w.persos[j].fief]);
            w.terrain.dist(((m.x + m.w / 2) as i64, (m.y + m.h / 2) as i64), ((c.x + c.w / 2) as i64, (c.y + c.h / 2) as i64)) / 16.0
        })
        .collect();
    distances.sort_by(f64::total_cmp);
    let cramped = standing
        .iter()
        .filter(|&&j| {
            let m = &w.houses[j];
            standing.iter().any(|&k| {
                let o = &w.houses[k];
                k != j && w.persos[k].fief == w.persos[j].fief && w.terrain.overlaps((m.x - 15, m.y - 15, m.w + 30, m.h + 30), (o.x, o.y, o.w, o.h))
            })
        })
        .count();
    (distances, cramped)
}

/// Compact towns keep closer to their castle than the original's, and leave lanes between
/// their houses.
#[test]
fn compact_towns() {
    let Some((files, rules)) = load() else { return };
    for torus in [false, true] {
        let (scattered, _) = towns(&files, &rules, Towns::Scattered, torus);
        let (compact, cramped) = towns(&files, &rules, Towns::Compact, torus);
        let median = |d: &[f64]| d[d.len() / 2];
        eprintln!("torus {torus}: {} houses scattered, median {:.1} cells; {} compact, median {:.1}, {cramped} cramped", scattered.len(), median(&scattered), compact.len(), median(&compact));
        assert!(compact.len() >= 40);
        assert!(median(&compact) < 0.9 * median(&scattered));
        assert!(cramped * 10 <= compact.len());
    }
}
