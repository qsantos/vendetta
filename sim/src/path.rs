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

/// How a character chases a moving target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Chase {
    /// The original's: straight for where the target is, again every tick.
    #[default]
    Direct,
    /// Not in the original: where the target will be, met on its way (see [`intercept`] and
    /// [`intercept_straight`]).
    Intercept,
}

/// Cells looked at before giving up on reaching the destination (then the path goes to the
/// cell nearest to it found so far, and straight on from there).
const MAX_CELLS: usize = 1 << 18;

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
) -> Vec<(i64, i64)> {
    let top = top_speed(rules);
    // Straight on when all the way is at that speed: no path over the cells is faster.
    if cell(from) != cell(to) && clear(rules, terrain, from, to, top) {
        return vec![to];
    }
    let goals = [to];
    let (reached, mut path) = search(rules, terrain, from, &goals, &|_, time| Some(time), &aim(&goals, top), top);
    if reached.is_none() {
        path.push(to);
    }
    straighten(rules, terrain, from, path)
}

/// Which of `goals` (feet positions, the copies nearest to `from` on a torus) is the fastest
/// to reach from `from`, the first of them on a tie, and the waypoints there as
/// [`fastest`] gives them; None when none is within the search's reach.
pub fn nearest(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    goals: &[(i64, i64)],
) -> Option<(usize, Vec<(i64, i64)>)> {
    let top = top_speed(rules);
    let (reached, path) = search(
        rules,
        terrain,
        from,
        goals,
        &|_, time| Some(time),
        &aim(goals, top),
        top,
    );
    Some((reached?, straighten(rules, terrain, from, path)))
}

/// Where to meet a moving target, from `from` at `speed` (pixels a tick at speed 1): of
/// `trajectory`, the points (feet positions, the copies nearest to `from`) where the target
/// will be with the tick it will be there (the last one where it stops, should it `stay`),
/// the first on its way that can be reached by then, the fastest way; with the waypoints
/// there as [`fastest`] gives them.
/// None when none is within the search's reach.
pub fn intercept(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    speed: f64,
    trajectory: &[((i64, i64), f64)],
    stays: bool,
) -> Option<(usize, Vec<(i64, i64)>)> {
    let end = trajectory.len().checked_sub(1)?;
    // Of the points in a cell, the last: there until then.
    let mut last: HashMap<(i64, i64), usize, BuildHasherDefault<CellHasher>> = HashMap::default();
    for (k, &(p, _)) in trajectory.iter().enumerate() {
        last.insert(cell(p), k);
    }
    // (the search's time: thousandths of a cell at speed 1)
    let by = |k: usize| trajectory[k].1 * speed * STRAIGHT / CELL as f64;
    // Only those within reach in time, straight on at the top speed.
    let top = top_speed(rules);
    let start = cell(from);
    let mut kept: Vec<usize> = last
        .into_values()
        .filter(|&k| stays && k == end || octile(start, cell(trajectory[k].0)) / top <= by(k))
        .collect();
    kept.sort_unstable();
    let goals: Vec<(i64, i64)> = kept.iter().map(|&k| trajectory[k].0).collect();
    if goals.is_empty() {
        return None;
    }
    let cells: Vec<(i64, i64)> = goals.iter().map(|&g| cell(g)).collect();
    let times: Vec<f64> = kept.iter().map(|&k| by(k)).collect();
    let (reached, path) = search(
        rules,
        terrain,
        from,
        &goals,
        // the soonest they are both there
        &|g, time| {
            let (k, by) = (kept[g], by(kept[g]));
            (time as f64 <= by || stays && k == end).then(|| (time as f64).max(by) as u64)
        },
        &stretches(&cells, &times, stays, top),
        top,
    );
    let reached = reached?;
    Some((kept[reached], straighten(rules, terrain, from, path)))
}

/// Stretches of the trajectory that [`intercept`]'s search bounds its scores with.
const STRETCHES: usize = 32;

/// For [`intercept`]'s search, of the `goals` (cells, in the order of the trajectory) and
/// when the target leaves each (`by`, increasing; the last one never should it `stay`): a
/// bound on the score through a cell at a time, by stretches of consecutive goals (each
/// within its box of cells, left between the first's time and the last's); None when no
/// goal can be reached through it before the target has left.
fn stretches(goals: &[(i64, i64)], by: &[f64], stays: bool, top: f64) -> impl Fn((i64, i64), u64) -> Option<u64> {
    let size = goals.len().div_ceil(STRETCHES);
    let boxes: Vec<_> = (0..goals.len())
        .step_by(size)
        .map(|k| {
            let end = (k + size).min(goals.len());
            let (mut c0, mut r0, mut c1, mut r1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for &(c, r) in &goals[k..end] {
                (c0, r0, c1, r1) = (c0.min(c), r0.min(r), c1.max(c), r1.max(r));
            }
            let last = if stays && end == goals.len() { f64::INFINITY } else { by[end - 1] };
            ((c0, r0, c1, r1), by[k], last)
        })
        .collect();
    move |(c, r), time| {
        boxes
            .iter()
            .filter_map(|&((c0, r0, c1, r1), first, last)| {
                let near = (c.clamp(c0, c1), r.clamp(r0, r1));
                let at = time as f64 + octile((c, r), near) / top;
                (at <= last).then(|| at.max(first) as u64)
            })
            .min()
    }
}

/// The search's usual bound: the time to reach the nearest goal, straight on at speed `top`
/// (none beyond [`MAX_AIMED`] goals: spreading evenly until it meets one).
fn aim(goals: &[(i64, i64)], top: f64) -> impl Fn((i64, i64), u64) -> Option<u64> {
    let aimed: Vec<(i64, i64)> = if goals.len() <= MAX_AIMED { goals.iter().map(|&g| cell(g)).collect() } else { Vec::new() };
    move |n, time| Some(time + aimed.iter().map(|&g| (octile(n, g) / top) as u64).min().unwrap_or(0))
}

/// Above the terrains' speeds, to be sure to find the fastest path.
fn top_speed(rules: &Rules) -> f64 {
    rules
        .terrains
        .iter()
        .map(|t| t.speed)
        .fold(1.0f64, f64::max)
}

/// The time between two cells at speed 1 without obstacles (eight directions).
fn octile((c, r): (i64, i64), to: (i64, i64)) -> f64 {
    let (dx, dy) = ((c - to.0).abs(), (r - to.1).abs());
    let (diagonal, straight) = (dx.min(dy), dx.max(dy) - dx.min(dy));
    diagonal as f64 * DIAGONAL + straight as f64 * STRAIGHT
}

fn cell((x, y): (i64, i64)) -> (i64, i64) {
    (clng(x as f64 / CELL as f64), clng(y as f64 / CELL as f64))
}

/// Goals beyond which the search does not aim (the estimate would cost more than it saves),
/// spreading evenly until it meets one.
const MAX_AIMED: usize = 64;

/// [`intercept`] for a chaser walking straight on (the original's paths): of `trajectory`,
/// the first point reached by then along the straight line, at the terrains' speeds on it.
pub fn intercept_straight(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    speed: f64,
    trajectory: &[((i64, i64), f64)],
    stays: bool,
) -> Option<usize> {
    let top = top_speed(rules);
    let end = trajectory.len().checked_sub(1)?;
    trajectory.iter().enumerate().position(|(k, &((x, y), by))| {
        let by = if stays && k == end { f64::INFINITY } else { by };
        let (dx, dy) = ((x - from.0) as f64, (y - from.1) as f64);
        let len = (dx * dx + dy * dy).sqrt();
        // (not even at the top speed)
        if len / (speed * top) > by {
            return false;
        }
        // every half cell
        let n = (len / (CELL as f64 / 2.0)).ceil().max(1.0);
        let mut time = 0.0;
        (0..n as usize).all(|k| {
            let f = (k as f64 + 0.5) / n;
            let (x, y) = (from.0 as f64 + dx * f, from.1 as f64 + dy * f);
            time += len / n / (speed * speed_coef(rules, terrain, clng(x), clng(y)).max(0.01));
            time <= by
        })
    })
}

/// A* from `from` to the goal that `accept` gives the lowest score (given the goal and the
/// time to reach it; None when it does not take it, and never below that time), the first
/// reached of those with the same, looking at no more than [`MAX_CELLS`] cells: the goal reached,
/// and the cells where the direction changes up to it, or up to the cell nearest to the
/// (first) goal when none was reached. `bound` gives, for a cell reached at a time, a score
/// no goal taken through it is below (None when none can be taken through it, which leaves
/// the cell out): the A* estimate.
fn search(
    rules: &Rules,
    terrain: &Terrain,
    from: (i64, i64),
    goals: &[(i64, i64)],
    accept: &dyn Fn(usize, u64) -> Option<u64>,
    bound: &dyn Fn((i64, i64), u64) -> Option<u64>,
    top: f64,
) -> (Option<usize>, Vec<(i64, i64)>) {
    let start = cell(from);
    let mut goal_cells: HashMap<(i64, i64), usize, BuildHasherDefault<CellHasher>> =
        HashMap::default();
    for (k, &g) in goals.iter().enumerate().rev() {
        goal_cells.insert(cell(g), k);
    }
    // On a flat map, the cells of the map, and those of the ends should they be off it.
    let bounds = (!terrain.torus).then(|| {
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        goal_cells.keys().fold(
            (
                start.0.min(0),
                start.1.min(0),
                start.0.max(w - 1),
                start.1.max(h - 1),
            ),
            |(c0, r0, c1, r1), &(c, r)| (c0.min(c), r0.min(r), c1.max(c), r1.max(r)),
        )
    });
    let first = cell(goals[0]);
    let estimate = |n: (i64, i64), to: (i64, i64)| (octile(n, to) / top) as u64;
    let (reached, cells) = SEEN.with_borrow_mut(|seen| {
        seen.reset(start);
        let mut open = BinaryHeap::new();
        seen.insert(start, (0, start));
        open.push(Reverse((bound(start, 0).unwrap_or(0), 0u64, start)));
        let mut nearest = (estimate(start, first), start);
        let mut reached = None;
        // the best score of a goal taken: none better is left once every path still open takes
        // that long (a goal's score is never below the time to reach it)
        let mut best = None;
        let mut looked = 0;
        while let Some(Reverse((f, g, node))) = open.pop() {
            if best.is_some_and(|b| f >= b) {
                break;
            }
            if seen.time(node) < g {
                continue;
            }
            if let Some(&k) = goal_cells.get(&node)
                && let Some(score) = accept(k, g)
                && best.is_none_or(|b| score < b)
            {
                (best, reached, nearest.1) = (Some(score), Some(k), node);
                if score <= f {
                    break;
                }
            }
            looked += 1;
            if looked > MAX_CELLS {
                break;
            }
            for (dc, dr) in NEIGHBOURS {
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
                if seen.entry(next).is_some_and(|(known, _)| known <= g) {
                    continue;
                }
                let Some(f) = bound(next, g) else { continue };
                seen.insert(next, (g, node));
                nearest = nearest.min((estimate(next, first), next));
                open.push(Reverse((f, g, next)));
            }
        }
        // Back from the end to the start, keeping the cells where the direction changes.
        let mut cells = vec![nearest.1];
        while let Some(&last) = cells.last() {
            let parent = seen.parent(last);
            if parent == last {
                break;
            }
            cells.push(parent);
        }
        cells.reverse();
        (reached, cells)
    });
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
    (reached, path)
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

/// The neighbours of a cell, in the order the search looks at them.
const NEIGHBOURS: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Cells from the start, either way, that [`Seen`] keeps in its window.
const WINDOW: i64 = 256;

/// For each cell reached by a search: the time to reach it, and the cell it is reached from.
/// In a window around the start, reused from search to search (a hash map took most of the
/// search's time), beyond it in a hash map.
struct Seen {
    start: (i64, i64),
    /// The search the window's entries are from: older ones are not reached.
    search: u32,
    window: Vec<(u32, u64, u8)>,
    beyond: HashMap<(i64, i64), (u64, u8), BuildHasherDefault<CellHasher>>,
}

thread_local! {
    static SEEN: std::cell::RefCell<Seen> = std::cell::RefCell::new(Seen {
        start: (0, 0),
        search: 0,
        window: Vec::new(),
        beyond: HashMap::default(),
    });
}

impl Seen {
    /// For a new search from `start`.
    fn reset(&mut self, start: (i64, i64)) {
        let side = (2 * WINDOW) as usize;
        if self.window.is_empty() || self.search == u32::MAX {
            self.window = vec![(0, 0, 0); side * side];
            self.search = 0;
        }
        self.search += 1;
        self.start = start;
        self.beyond.clear();
    }

    fn index(&self, (c, r): (i64, i64)) -> Option<usize> {
        let (dc, dr) = (c - self.start.0 + WINDOW, r - self.start.1 + WINDOW);
        let side = 2 * WINDOW;
        ((0..side).contains(&dc) && (0..side).contains(&dr)).then(|| (dc + dr * side) as usize)
    }

    /// The time to reach a cell and the neighbour it is reached from (an index into
    /// [`NEIGHBOURS`], or 8 for the start), if reached.
    fn entry(&self, cell: (i64, i64)) -> Option<(u64, u8)> {
        match self.index(cell) {
            Some(k) => {
                let (search, time, from) = self.window[k];
                (search == self.search).then_some((time, from))
            }
            None => self.beyond.get(&cell).copied(),
        }
    }

    fn time(&self, cell: (i64, i64)) -> u64 {
        self.entry(cell).unwrap().0
    }

    /// The cell `cell` is reached from (itself for the start).
    fn parent(&self, cell: (i64, i64)) -> (i64, i64) {
        match self.entry(cell).unwrap().1 {
            8 => cell,
            k => (cell.0 - NEIGHBOURS[k as usize].0, cell.1 - NEIGHBOURS[k as usize].1),
        }
    }

    fn insert(&mut self, cell: (i64, i64), (time, parent): (u64, (i64, i64))) {
        let step = (cell.0 - parent.0, cell.1 - parent.1);
        let from = NEIGHBOURS.iter().position(|&n| n == step).unwrap_or(8) as u8;
        match self.index(cell) {
            Some(k) => self.window[k] = (self.search, time, from),
            None => {
                self.beyond.insert(cell, (time, from));
            }
        }
    }
}

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
