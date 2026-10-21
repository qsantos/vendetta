//! Not in the original: the fastest path, by A* over the terrain's cells, instead of
//! `Calculer_Chemin`'s greedy walk (which heads straight for the destination and only swerves
//! one cell at a time, so it wades through a lake it could have walked around).

use crate::perso::speed_coef;
use crate::rules::Rules;
use crate::terrain::{CELL, Terrain};
use crate::vb::clng;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

/// How characters find their way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Paths {
    /// The original's `Calculer_Chemin`: greedy, one cell at a time toward the destination.
    #[default]
    Greedy,
    /// The fastest path, by A* over the cells, for the terrains' speeds.
    Fastest,
}

/// Cells looked at before giving up on reaching the destination (then the path goes to the
/// cell nearest to it found so far, and straight on from there).
const MAX_CELLS: usize = 1 << 18;
/// The same while chasing a moving target, when the path is planned again every tick.
const MAX_CELLS_CHASE: usize = 1 << 11;

/// Time to cross one cell, straight or diagonally, in thousandths of `CELL` at speed 1.
const STRAIGHT: f64 = 1000.0;
const DIAGONAL: f64 = 1000.0 * std::f64::consts::SQRT_2;

/// The waypoints from `from` to `to` (feet positions, `y + h - 16`), the fastest way for the
/// terrains' speeds: the cells' centres where the direction changes, the last one being the
/// cell of `to` (which the caller replaces by `to` itself), or `to` itself after the cell
/// nearest to it found when the search gave up. On a torus, `to` is the copy of the
/// destination nearest to `from`, the one the path leads to.
pub fn fastest(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    to: (i64, i64),
    chase: bool,
) -> Vec<(i64, i64)> {
    let cell = |(x, y): (i64, i64)| (clng(x as f64 / CELL as f64), clng(y as f64 / CELL as f64));
    let (start, goal) = (cell(from), cell(to));
    if start == goal {
        return Vec::new();
    }
    let fastest_terrain = rules
        .terrains
        .iter()
        .map(|t| t.speed)
        .fold(1.0f64, f64::max);
    // While chasing (planned again every tick, and over short distances), as if roads were of
    // normal speed: much less to search, for paths that may only miss a road.
    let top = if chase { 1.0 } else { fastest_terrain };
    // Straight on when all the way is at that speed: no path over the cells is faster.
    if clear(rules, terrain, from, to, top) {
        return vec![to];
    }
    // On a flat map, the cells of the map, and those of the ends should they be off it.
    let bounds = (!terrain.torus).then(|| {
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        (
            start.0.min(goal.0).min(0),
            start.1.min(goal.1).min(0),
            start.0.max(goal.0).max(w - 1),
            start.1.max(goal.1).max(h - 1),
        )
    });
    // The time at that speed all the way: never more than the fastest path takes.
    let heuristic = |(c, r): (i64, i64)| {
        let (dx, dy) = ((c - goal.0).abs(), (r - goal.1).abs());
        let (diagonal, straight) = (dx.min(dy), dx.max(dy) - dx.min(dy));
        ((diagonal as f64 * DIAGONAL + straight as f64 * STRAIGHT) / top) as u64
    };
    let mut seen: Seen = HashMap::default();
    let mut open = BinaryHeap::new();
    seen.insert(start, (0, start));
    open.push(Reverse((heuristic(start), 0u64, start)));
    let mut nearest = (heuristic(start), start);
    let limit = if chase { MAX_CELLS_CHASE } else { MAX_CELLS };
    let mut looked = 0;
    while let Some(Reverse((_, g, node))) = open.pop() {
        if seen[&node].0 < g {
            continue;
        }
        if node == goal {
            nearest = (0, goal);
            break;
        }
        looked += 1;
        if looked > limit {
            break;
        }
        for (dc, dr) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let next = (node.0 + dc, node.1 + dr);
            if let Some((c0, r0, c1, r1)) = bounds
                && (next.0 < c0 || next.1 < r0 || next.0 > c1 || next.1 > r1)
            {
                continue;
            }
            let speed =
                speed_coef(rules, terrain, next.0 * CELL as i64, next.1 * CELL as i64).max(0.01);
            let g = g
                + ((if dc != 0 && dr != 0 {
                    DIAGONAL
                } else {
                    STRAIGHT
                }) / speed) as u64;
            if seen.get(&next).is_some_and(|&(known, _)| known <= g) {
                continue;
            }
            seen.insert(next, (g, node));
            let h = heuristic(next);
            nearest = nearest.min((h, next));
            open.push(Reverse((g + h, g, next)));
        }
    }
    // Back from the end to the start, keeping the cells where the direction changes.
    let mut cells = vec![nearest.1];
    while let Some(&last) = cells.last() {
        let parent = seen[&last].1;
        if parent == last {
            break;
        }
        cells.push(parent);
    }
    cells.reverse();
    let mut path: Vec<(i64, i64)> = Vec::new();
    for k in 1..cells.len() {
        let turn = k + 1 == cells.len() || {
            let (a, b, c) = (cells[k - 1], cells[k], cells[k + 1]);
            (b.0 - a.0, b.1 - a.1) != (c.0 - b.0, c.1 - b.1)
        };
        if turn {
            path.push((cells[k].0 * CELL as i64, cells[k].1 * CELL as i64));
        }
    }
    if nearest.1 != goal {
        path.push(to);
    }
    straighten(rules, terrain, from, path)
}

/// `path` without the waypoints that a straight line can skip: one all at least as fast as
/// the fastest terrain of the stretch of `path` it replaces (and so shorter, and no slower).
/// It removes the zigzags of paths over cells, keeping the turns that follow faster terrain.
fn straighten(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    path: Vec<(i64, i64)>,
) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut at = from;
    let mut k = 0;
    while k < path.len() {
        // the farthest waypoint reachable straight on
        let mut fastest = 0.0f64;
        let mut prev = at;
        let mut reach = k;
        for (j, &p) in path.iter().enumerate().skip(k) {
            fastest = fastest.max(max_speed(rules, terrain, prev, p));
            prev = p;
            if j == k || clear(rules, terrain, at, p, fastest) {
                reach = j;
            }
        }
        out.push(path[reach]);
        at = path[reach];
        k = reach + 1;
    }
    out
}

/// The fastest terrain on the straight line from `from` to `to`.
fn max_speed(rules: &Rules, terrain: &Terrain, from: (i64, i64), to: (i64, i64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = (dx.abs().max(dy.abs()) * 2 / CELL as i64).max(1);
    (0..=steps)
        .map(|k| {
            speed_coef(
                rules,
                terrain,
                from.0 + dx * k / steps,
                from.1 + dy * k / steps,
            )
        })
        .fold(0.0, f64::max)
}

/// Whether the straight line from `from` to `to` is all at speed `speed` or faster.
fn clear(rules: &Rules, terrain: &Terrain, from: (i64, i64), to: (i64, i64), speed: f64) -> bool {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    // every half cell
    let steps = (dx.abs().max(dy.abs()) * 2 / CELL as i64).max(1);
    (0..=steps).all(|k| {
        let (x, y) = (from.0 + dx * k / steps, from.1 + dy * k / steps);
        speed_coef(rules, terrain, x, y) >= speed
    })
}

/// For each cell reached: the time to reach it, and the cell it is reached from.
type Seen = HashMap<(i64, i64), (u64, (i64, i64)), BuildHasherDefault<CellHasher>>;

/// A hasher for cells, much faster than the default one (which resists attacks that do not
/// concern positions on a map): FxHash's multiply and rotate.
#[derive(Default)]
struct CellHasher(u64);

impl Hasher for CellHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }

    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
