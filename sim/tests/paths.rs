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
    let path = fastest(&rules, &t, from, to);
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
    assert_eq!(fastest(&rules, &t, from, to), vec![to]);
}

/// In the open, straight on.
#[test]
fn open_ground() {
    let Some(rules) = load() else { return };
    let t = map(&rules, false, |_, _| false);
    assert_eq!(
        fastest(&rules, &t, px(2, 2), px(22, 12)),
        vec![px(22, 12)]
    );
    assert_eq!(fastest(&rules, &t, px(2, 2), px(2, 2)), vec![]);
}

/// On a torus, across the edge, with the goal given as its copy nearest to the start.
#[test]
fn across_the_edge() {
    let Some(rules) = load() else { return };
    let t = map(&rules, true, |c, r| (10..30).contains(&c) && r != 39);
    let (from, to) = (px(5, 5), px(-5, 5));
    let path = fastest(&rules, &t, from, to);
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
    let path = fastest(&rules, &t, from, to);
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
    let path = fastest(&rules, &t, from, to);
    assert!(
        time(&rules, &t, from, &path) < time(&rules, &t, from, &[to]) * 0.9,
        "{path:?}"
    );
    assert!(path.iter().any(|&(_, y)| y == 22 * CELL as i64), "{path:?}");
}

/// "The nearest" is the fastest to reach: behind slow ground, a nearer resource loses to one
/// farther away along clear ground (and the character walks the path found choosing it).
#[test]
fn nearest_by_time() {
    use sim::path::Paths;
    use sim::world::{World, WorldSetup};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let Some(rules) = load() else { return };
    let files = Files::from_dir(&root).unwrap();
    for paths in [Paths::Greedy, Paths::Fastest] {
        let mut w = World::create(
            &rules,
            &files,
            &WorldSetup {
                seed: 7,
                paths,
                ..Default::default()
            },
        )
        .unwrap();
        let i = 0;
        let (px, py) = (w.persos[i].x, w.persos[i].y);
        let kind = w.resources[0].kind;
        let mut nodes: Vec<usize> = (0..w.resources.len())
            .filter(|&k| w.resources[k].kind == kind)
            .collect();
        let dist = |w: &World, k: usize| {
            w.terrain
                .dist((px, py), (w.resources[k].x as i64, w.resources[k].y as i64))
        };
        nodes.sort_by(|&a, &b| dist(&w, a).total_cmp(&dist(&w, b)));
        let (near, far) = (nodes[0], nodes[1]);
        assert!(dist(&w, far) < 4.0 * dist(&w, near), "pick another seed");
        // all slow, but for a corridor to the farther node
        w.terrain.category.fill(slowest(&rules));
        let (fx, fy) = (
            w.resources[far].x as i64 + 16,
            w.resources[far].y as i64 + 16,
        );
        let steps = 200;
        for s in 0..=steps {
            let (x, y) = (px + (fx - px) * s / steps, py + (fy - py) * s / steps);
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (0, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
                (0, 2),
                (1, 2),
                (-1, 2),
            ] {
                let c = (x / CELL as i64 + dx).rem_euclid(w.terrain.width as i64);
                let r = (y / CELL as i64 + dy).rem_euclid(w.terrain.height as i64);
                w.terrain.category[(c + r * w.terrain.width as i64) as usize] = 1;
            }
        }
        w.harvest_nearest(&rules, i, kind as i64);
        let want = if paths == Paths::Fastest { far } else { near };
        assert_eq!(w.persos[i].resource_target, want, "{paths:?}");
    }
}

/// Chasing a target that walks away along a road, from beside it: onto the road, where the
/// chaser is faster, rather than heading for the target across the grass (the path is planned
/// once, its end following the target, rather than again every tick).
#[test]
fn chase_along_a_road() {
    use sim::orders::Order;
    use sim::path::Paths;
    use sim::world::{World, WorldSetup};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let Some(rules) = load() else { return };
    let files = Files::from_dir(&root).unwrap();
    let setup = WorldSetup { seed: 7, paths: Paths::Fastest, players: vec![0, 1], ..Default::default() };
    let mut w = World::create(&rules, &files, &setup).unwrap();
    let speed = |k: usize| rules.terrains[k].speed;
    let road = (0..rules.terrains.len()).max_by(|&a, &b| speed(a).total_cmp(&speed(b))).unwrap() as u8;
    let plain = (0..rules.terrains.len()).find(|&k| speed(k) == 1.0).unwrap() as u8;
    let width = w.terrain.width;
    let row = 20;
    w.terrain.category.fill(plain);
    for c in 0..width {
        w.terrain.category[c + row * width] = road;
    }
    let (i, j) = (w.persos.iter().position(|p| p.player).unwrap(), w.persos.iter().rposition(|p| p.player).unwrap());
    assert_ne!(w.persos[i].team, w.persos[j].team);
    let cell = CELL as i64;
    // (positions of the feet, `y + h - 16`)
    let at = |w: &mut World, k: usize, c: i64, r: i64| {
        let p = &mut w.persos[k];
        p.in_house = false;
        (p.x, p.y) = (c * cell, r * cell - (p.h as i64 - 16));
    };
    at(&mut w, i, 4, row as i64 + 8);
    at(&mut w, j, 24, row as i64);
    let far = (width as i64 - 4) * cell;
    let jy = w.persos[j].y;
    w.step(&rules, &[Order::Move { perso: j, x: far, y: jy }]);
    w.attack_perso(&rules, i, j);
    let mut on_road = 0;
    let ticks = 150;
    for _ in 0..ticks {
        w.step(&rules, &[]);
        let p = &w.persos[i];
        if sim::perso::speed_coef(&rules, &w.terrain, p.x, p.y + p.h as i64 - 16) == speed(road as usize) {
            on_road += 1;
        }
    }
    assert!(on_road > ticks / 2, "{on_road} ticks of {ticks} on the road");
}
