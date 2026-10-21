//! The fastest paths (`sim::path`, not in the original).
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::path::fastest;
use sim::rules::Rules;
use sim::terrain::{CELL, Terrain};
use std::path::Path;

fn load() -> Option<Rules> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    if !root.exists() {
        eprintln!("no assets, skipping");
        return None;
    }
    Some(Rules::load(&Files::from_dir(&root).unwrap(), "Francais").unwrap())
}

/// The slowest terrain (water).
fn slowest(rules: &Rules) -> u8 {
    (0..rules.terrains.len())
        .min_by(|&a, &b| rules.terrains[a].speed.total_cmp(&rules.terrains[b].speed))
        .unwrap() as u8
}

/// A 40 × 40 map with the cells for which `water` holds made of the slowest terrain.
fn map(rules: &Rules, torus: bool, water: impl Fn(usize, usize) -> bool) -> Terrain {
    let mut t = Terrain {
        torus,
        ..Terrain::new(40, 40)
    };
    for r in 0..40 {
        for c in 0..40 {
            if water(c, r) {
                t.category[c + r * 40] = slowest(rules);
            }
        }
    }
    t
}

fn px(c: i64, r: i64) -> (i64, i64) {
    (c * CELL as i64, r * CELL as i64)
}

/// The time to walk from `from` through `path`, a pixel at a time, in pixels at speed 1.
fn time(rules: &Rules, t: &Terrain, from: (i64, i64), path: &[(i64, i64)]) -> f64 {
    let mut total = 0.0;
    let mut at = (from.0 as f64, from.1 as f64);
    for &(x, y) in path {
        let (dx, dy) = (x as f64 - at.0, y as f64 - at.1);
        let steps = dx.abs().max(dy.abs()).max(1.0) as usize;
        for _ in 0..steps {
            at = (at.0 + dx / steps as f64, at.1 + dy / steps as f64);
            let len = (dx * dx + dy * dy).sqrt() / steps as f64;
            total +=
                len / sim::perso::speed_coef(rules, t, at.0.round() as i64, at.1.round() as i64);
        }
    }
    total
}

/// Around a lake rather than through it, when that is faster.
#[test]
fn around_a_lake() {
    let Some(rules) = load() else { return };
    let t = map(&rules, false, |c, r| {
        (15..25).contains(&c) && (8..32).contains(&r)
    });
    let (from, to) = (px(5, 20), px(35, 20));
    let path = fastest(&rules, &t, from, to, false);
    let straight = time(&rules, &t, from, &[to]);
    let around = time(&rules, &t, from, &path);
    assert!(
        around < straight * 0.6,
        "{around} vs {straight} straight: {path:?}"
    );
    assert_eq!(*path.last().unwrap(), to);
}

/// Through the water when it is narrow enough to be the fastest way.
#[test]
fn across_a_stream() {
    let Some(rules) = load() else { return };
    let t = map(&rules, false, |c, _| c == 20);
    let (from, to) = (px(5, 20), px(35, 20));
    assert_eq!(fastest(&rules, &t, from, to, false), vec![to]);
}

/// In the open, straight on.
#[test]
fn open_ground() {
    let Some(rules) = load() else { return };
    let t = map(&rules, false, |_, _| false);
    assert_eq!(
        fastest(&rules, &t, px(2, 2), px(22, 12), false),
        vec![px(22, 12)]
    );
    assert_eq!(fastest(&rules, &t, px(2, 2), px(2, 2), false), vec![]);
}

/// On a torus, across the edge, with the goal given as its copy nearest to the start.
#[test]
fn across_the_edge() {
    let Some(rules) = load() else { return };
    let t = map(&rules, true, |c, r| (10..30).contains(&c) && r != 39);
    let (from, to) = (px(5, 5), px(-5, 5));
    let path = fastest(&rules, &t, from, to, false);
    assert_eq!(path, vec![to]);
}

/// A destination that cannot be reached within the search's limit (across a sea): still
/// toward it.
#[test]
fn far_away() {
    let Some(rules) = load() else { return };
    let mut t = Terrain {
        torus: true,
        ..Terrain::new(1024, 1024)
    };
    t.category.fill(slowest(&rules));
    let (from, to) = (px(1, 1), px(500, 300));
    let path = fastest(&rules, &t, from, to, true);
    // (straight on, across a uniform sea)
    assert_eq!(path, vec![to]);
}

/// Along a road rather than beside it, when going the same way.
#[test]
fn along_a_road() {
    let Some(rules) = load() else { return };
    let road = (0..rules.terrains.len())
        .max_by(|&a, &b| rules.terrains[a].speed.total_cmp(&rules.terrains[b].speed))
        .unwrap() as u8;
    let mut t = map(&rules, false, |_, _| false);
    for c in 0..40 {
        t.category[c + 22 * 40] = road;
    }
    let (from, to) = (px(2, 20), px(38, 20));
    let path = fastest(&rules, &t, from, to, false);
    assert!(
        time(&rules, &t, from, &path) < time(&rules, &t, from, &[to]) * 0.9,
        "{path:?}"
    );
    assert!(path.iter().any(|&(_, y)| y == 22 * CELL as i64), "{path:?}");
}
