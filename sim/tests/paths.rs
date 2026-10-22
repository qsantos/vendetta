//! The fastest paths (`sim::path`, not in the original).
//!
//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::cover::Free;
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
    let mut t = Terrain::with_torus(40, 40, torus);
    for r in 0..40 {
        for c in 0..40 {
            if water(c, r) {
                t.category_mut()[c + r * 40] = slowest(rules);
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
    let path = fastest(&rules, &t, Free::EVERYWHERE, from, to);
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
    assert_eq!(fastest(&rules, &t, Free::EVERYWHERE, from, to), vec![to]);
}

/// In the open, straight on.
#[test]
fn open_ground() {
    let Some(rules) = load() else { return };
    let t = map(&rules, false, |_, _| false);
    assert_eq!(
        fastest(&rules, &t, Free::EVERYWHERE, px(2, 2), px(22, 12)),
        vec![px(22, 12)]
    );
    assert_eq!(fastest(&rules, &t, Free::EVERYWHERE, px(2, 2), px(2, 2)), vec![]);
}

/// On a torus, across the edge, with the goal given as its copy nearest to the start.
#[test]
fn across_the_edge() {
    let Some(rules) = load() else { return };
    let t = map(&rules, true, |c, r| (10..30).contains(&c) && r != 39);
    let (from, to) = (px(5, 5), px(-5, 5));
    let path = fastest(&rules, &t, Free::EVERYWHERE, from, to);
    assert_eq!(path, vec![to]);
}

/// A destination that cannot be reached within the search's limit (across a sea): still
/// toward it.
#[test]
fn far_away() {
    let Some(rules) = load() else { return };
    let mut t = Terrain::with_torus(1024, 1024, true);
    t.category_mut().fill(slowest(&rules));
    let (from, to) = (px(1, 1), px(500, 300));
    let path = fastest(&rules, &t, Free::EVERYWHERE, from, to);
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
        t.category_mut()[c + 22 * 40] = road;
    }
    let (from, to) = (px(2, 20), px(38, 20));
    let path = fastest(&rules, &t, Free::EVERYWHERE, from, to);
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
        w.terrain.category_mut().fill(slowest(&rules));
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
                let k = (c + r * w.terrain.width as i64) as usize;
                w.terrain.category_mut()[k] = 1;
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
    w.terrain.category_mut().fill(plain);
    for c in 0..width {
        w.terrain.category_mut()[c + row * width] = road;
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

/// Heading off a target that walks past, ahead of the chaser and as fast: met on its way,
/// where going for where it is only trails behind it; with either way of finding paths.
#[test]
fn head_off() {
    use sim::orders::Order;
    use sim::path::{Chase, Paths};
    use sim::world::{World, WorldSetup};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let Some(rules) = load() else { return };
    let files = Files::from_dir(&root).unwrap();
    let plain = (0..rules.terrains.len()).find(|&k| rules.terrains[k].speed == 1.0).unwrap() as u8;
    let cell = CELL as i64;
    // the ticks until the chaser is within two cells of its target, if it ever is
    let caught = |paths: Paths, chase: Chase| {
        let setup = WorldSetup { seed: 7, paths, chase, players: vec![0, 1], ..Default::default() };
        let mut w = World::create(&rules, &files, &setup).unwrap();
        w.terrain.category_mut().fill(plain);
        let (i, j) = (w.persos.iter().position(|p| p.player).unwrap(), w.persos.iter().rposition(|p| p.player).unwrap());
        let at = |w: &mut World, k: usize, c: i64, r: i64| {
            let p = &mut w.persos[k];
            p.in_house = false;
            p.bonus_move = 4;
            (p.x, p.y) = (c * cell, r * cell);
        };
        at(&mut w, i, 30, 35);
        at(&mut w, j, 5, 20);
        w.step(&rules, &[Order::Move { perso: j, x: 70 * cell, y: 20 * cell }]);
        w.attack_perso(&rules, i, j);
        (0..400).find(|_| {
            w.step(&rules, &[]);
            w.terrain.dist((w.persos[i].x, w.persos[i].y), (w.persos[j].x, w.persos[j].y)) < 2.0 * cell as f64
        })
    };
    for paths in [Paths::Fastest, Paths::Greedy] {
        let (direct, ahead) = (caught(paths, Chase::Direct), caught(paths, Chase::Intercept));
        assert!(ahead.is_some_and(|a| direct.is_none_or(|d| a < d)), "{paths:?}: direct {direct:?}, heading off {ahead:?}");
    }
}

/// With compact towns, the houses slow those who walk through them, not the castles: a walker
/// goes round a house, through a castle; through both with the original's towns.
#[test]
fn around_a_house() {
    use sim::orders::Order;
    use sim::path::Paths;
    use sim::world::{Towns, World, WorldSetup};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let Some(rules) = load() else { return };
    let files = Files::from_dir(&root).unwrap();
    let plain = (0..rules.terrains.len()).find(|&k| rules.terrains[k].speed == 1.0).unwrap() as u8;
    // the ticks walking across the middle of another fief's castle, or of a house put there
    // instead, from its left to its right
    let inside = |towns: Towns, house: bool| {
        let setup = WorldSetup { seed: 7, paths: Paths::Fastest, towns, ..Default::default() };
        let mut w = World::create(&rules, &files, &setup).unwrap();
        w.terrain.category_mut().fill(plain);
        let i = w.persos.iter().position(|p| p.player).unwrap();
        let f = (w.persos[i].fief + 1) % w.castles.len();
        let (cx, cy, cw, ch) = (w.castles[f].x, w.castles[f].y, w.castles[f].w, w.castles[f].h);
        if house {
            let j = (0..w.houses.len()).find(|&j| j != i).unwrap();
            w.castles[f].life = 0.0;
            let m = &mut w.houses[j];
            (m.x, m.y, m.w, m.h, m.life) = (cx, cy, cw, ch, 100.0);
        }
        let (cx, cy, cw, ch) = (cx as i64, cy as i64, cw as i64, ch as i64);
        let cell = CELL as i64;
        let p = &mut w.persos[i];
        p.in_house = false;
        let y = cy + ch / 2 - (p.h as i64 - 16);
        (p.x, p.y) = (cx - 3 * cell, y);
        w.step(&rules, &[Order::Move { perso: i, x: cx + cw + 3 * cell, y }]);
        let mut inside = 0;
        for _ in 0..300 {
            w.step(&rules, &[]);
            let p = &w.persos[i];
            let feet = (p.x, p.y + p.h as i64 - 16);
            if (cx + cell..cx + cw - cell).contains(&feet.0) && (cy + cell..cy + ch - cell).contains(&feet.1) {
                inside += 1;
            }
        }
        inside
    };
    assert_eq!(inside(Towns::Compact, true), 0);
    assert!(inside(Towns::Compact, false) > 0);
    assert!(inside(Towns::Scattered, true) > 0);
}
