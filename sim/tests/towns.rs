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

/// The houses standing after a while, by distance (in cells) to their fief's castle; how many
/// stand against another house of their fief; and how many pairs of them leave a way too
/// narrow to walk between them (less than two cells; but for the summons', placed as in the
/// original).
fn towns(files: &Files, rules: &Rules, towns: Towns, torus: bool) -> (Vec<f64>, usize, usize) {
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
    let against = standing
        .iter()
        .filter(|&&j| {
            let m = &w.houses[j];
            standing.iter().any(|&k| {
                let o = &w.houses[k];
                k != j && w.persos[k].fief == w.persos[j].fief && w.terrain.overlaps((m.x - 1, m.y - 1, m.w + 2, m.h + 2), (o.x, o.y, o.w, o.h))
            })
        })
        .count();
    let narrow = standing
        .iter()
        .flat_map(|&j| standing.iter().map(move |&k| (j, k)))
        .filter(|&(j, k)| {
            let (m, o) = (&w.houses[j], &w.houses[k]);
            // (the gaps between them along each axis, negative when they overlap along it)
            let (dx, dy) = (w.terrain.dx(m.x as i64, o.x as i64) as i32, w.terrain.dy(m.y as i64, o.y as i64) as i32);
            let (gx, gy) = ((dx - m.w).max(-dx - o.w), (dy - m.h).max(-dy - o.h));
            let against = gx == 0 && gy < 0 || gy == 0 && gx < 0;
            let built = |i: usize| !w.persos[i].invocation();
            j < k && w.persos[k].fief == w.persos[j].fief && built(j) && built(k) && !against && gx.max(gy) < 32
        })
        .count();
    (distances, against, narrow)
}

/// Compact towns keep closer to their castle than the original's, many of their houses side by
/// side, with no way between them too narrow to walk.
#[test]
fn compact_towns() {
    let Some((files, rules)) = load() else { return };
    for torus in [false, true] {
        let (scattered, apart, _) = towns(&files, &rules, Towns::Scattered, torus);
        let (compact, against, narrow) = towns(&files, &rules, Towns::Compact, torus);
        let median = |d: &[f64]| d[d.len() / 2];
        eprintln!(
            "torus {torus}: {} houses scattered, median {:.1} cells, {apart} against another; {} compact, median {:.1}, {against} against another, {narrow} narrow ways",
            scattered.len(),
            median(&scattered),
            compact.len(),
            median(&compact)
        );
        assert!(compact.len() >= 40);
        assert!(median(&compact) < median(&scattered));
        assert!(against * 3 >= compact.len() && apart * 5 <= scattered.len());
        assert_eq!(narrow, 0);
    }
}
