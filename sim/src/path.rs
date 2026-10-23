//! Not in the original: the fastest path, by A* over the terrain's cells, instead of
//! `Calculer_Chemin`'s greedy walk (which heads straight for the destination and only swerves
//! one cell at a time, so it wades through a lake it could have walked around).

use crate::cover::{BUILDING_SPEED, Free};
use crate::perso::speed_through;
use crate::rules::Rules;
use crate::terrain::{CELL, Terrain, wrap_axis};
use crate::vb::clng;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
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

/// Cells the searches a tick starts may look at, together: past them, the paths asked for
/// wait for the next ticks (see [`Planner`]). A search started finishes, so a tick takes up to
/// this and one search more.
pub const BUDGET: u64 = 1 << 16;

/// Not in the original: spreads the searches over the ticks, so that many asked for at once
/// (a troop sent far away) do not stall the game. A tick starts searches until they have looked
/// at [`BUDGET`] cells; past that, the characters asking for a path wait in a queue, standing,
/// served first at the next ticks, in the order they asked; and the other searches (the AI
/// choosing what to go for, heading off) are not made, which fall back on the original's ways.
/// Counted in cells, not time, it is the same on every copy of a network game.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Planner {
    /// The characters waiting for a path, oldest first.
    pub(crate) queue: VecDeque<usize>,
    /// [`looked`] when the tick started.
    #[serde(skip)]
    start: u64,
}

impl Planner {
    /// A new tick, with its budget.
    pub(crate) fn begin_tick(&mut self) {
        self.start = looked();
    }

    /// Whether this tick's budget is spent.
    pub(crate) fn spent(&self) -> bool {
        looked() - self.start >= BUDGET
    }

    /// Whether perso `i`, which needs a path, may plan it now: when nobody waits and the
    /// budget is not spent; otherwise it waits its turn (`queued`).
    pub(crate) fn admit(&mut self, i: usize, queued: &mut bool) -> bool {
        if *queued {
            return false;
        }
        if self.queue.is_empty() && !self.spent() {
            return true;
        }
        self.queue.push_back(i);
        *queued = true;
        false
    }
}

thread_local! {
    /// Cells the searches of this thread have looked at, ever (see [`Planner`]).
    static LOOKED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Cells the searches of this thread have looked at so far.
pub fn looked() -> u64 {
    LOOKED.get()
}

/// Time to cross one cell, straight or diagonally, in thousandths of `CELL` at speed 1.
const STRAIGHT: f64 = 1000.0;
const DIAGONAL: f64 = 1000.0 * std::f64::consts::SQRT_2;

/// The waypoints from `from` to `to` (feet positions, `y + h - 16`), the fastest way for the
/// terrains' speeds: the cells' centres where the direction changes, the last one being the
/// cell of `to` (which the caller replaces by `to` itself), or `to` itself after the cell
/// nearest to it found when the search gave up. On a torus, `to` is the copy of the
/// destination nearest to `from`, the one the path leads to. The buildings slow the way but
/// for those `free` (see [`crate::cover`]).
pub fn fastest(
    rules: &Rules,
    terrain: &Terrain,
    free: Free,
    from: (i64, i64),
    to: (i64, i64),
) -> Vec<(i64, i64)> {
    let top = top_speed(rules);
    // Straight on when all the way is at that speed: no path over the cells is faster.
    if cell(from) != cell(to) && clear(rules, terrain, free, from, to, top) {
        return vec![to];
    }
    let goals = [to];
    let landmarks = Landmarks::of(rules, terrain, cell(from), &[cell(to)]);
    let (reached, mut path) = search(rules, terrain, free, from, &goals, &|_, time| Some(time), &aim(terrain, landmarks, &goals, top));
    if reached.is_none() {
        path.push(to);
    }
    straighten(rules, terrain, free, from, path)
}

/// How long walking from `from` to `to` (feet positions) takes, and how long crossing straight at
/// the top speed would, both in cells; the first more round a lake or over slow ground. None
/// when the search finds no way within its reach (across the sea, say).
pub fn detour(rules: &Rules, terrain: &Terrain, free: Free, from: (i64, i64), to: (i64, i64)) -> Option<(f64, f64)> {
    let top = top_speed(rules);
    let (_, time, _) = among(rules, terrain, free, from, &[to], top)?;
    Some((time as f64 / STRAIGHT, octile(cell(from), cell(to)) / top / STRAIGHT))
}

/// Which of `goals` (feet positions, the copies nearest to `from` on a torus) is the fastest
/// to reach from `from`, the first of them on a tie, and the waypoints there as
/// [`fastest`] gives them; None when none is within the search's reach.
pub fn nearest(
    rules: &Rules,
    terrain: &Terrain,
    free: Free,
    from: (i64, i64),
    goals: &[(i64, i64)],
) -> Option<(usize, Vec<(i64, i64)>)> {
    let top = top_speed(rules);
    // Beyond a few goals, the search can neither aim nor use the landmarks, and spreads evenly
    // until it meets one. But once the time to one of them is known, none is faster to reach
    // that is slower straight on at the top speed: the search is among the others only (the
    // same goal, but on a tie), often the one alone.
    if goals.len() > MAX_LANDMARKED {
        let start = cell(from);
        // (a little under: the search's steps are rounded down, by up to a 2000th of one)
        let least = |g: (i64, i64)| (octile(start, cell(g)) / top * 0.999) as u64;
        let first = (0..goals.len()).min_by_key(|&k| least(goals[k]))?;
        if let Some((_, time, path)) = among(rules, terrain, free, from, &goals[first..=first], top) {
            let kept: Vec<usize> = (0..goals.len()).filter(|&k| least(goals[k]) <= time).collect();
            if kept.len() == 1 {
                return Some((first, straighten(rules, terrain, free, from, path)));
            }
            let points: Vec<(i64, i64)> = kept.iter().map(|&k| goals[k]).collect();
            if let Some((k, _, path)) = among(rules, terrain, free, from, &points, top) {
                return Some((kept[k], straighten(rules, terrain, free, from, path)));
            }
        }
    }
    let (k, _, path) = among(rules, terrain, free, from, goals, top)?;
    Some((k, straighten(rules, terrain, free, from, path)))
}

/// A goal reached: which, the time to reach it and the cells where the direction changes on
/// the way.
type Reached = (usize, u64, Vec<(i64, i64)>);

/// [`nearest`] by a single search.
fn among(rules: &Rules, terrain: &Terrain, free: Free, from: (i64, i64), goals: &[(i64, i64)], top: f64) -> Option<Reached> {
    let cells: Vec<(i64, i64)> = goals.iter().take(MAX_LANDMARKED + 1).map(|&g| cell(g)).collect();
    let landmarks = (goals.len() <= MAX_LANDMARKED).then(|| Landmarks::of(rules, terrain, cell(from), &cells)).flatten();
    let (reached, path) = search(rules, terrain, free, from, goals, &|_, time| Some(time), &aim(terrain, landmarks, goals, top));
    let k = reached?;
    Some((k, SEEN.with_borrow(|seen| seen.time(cell(goals[k]))), path))
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
    free: Free,
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
    let points: Vec<(i64, i64)> = trajectory.iter().map(|&(p, _)| cell(p)).collect();
    let landmarks = Landmarks::of(rules, terrain, start, &points);
    let (here, marks) = (landmarks.and_then(|l| l.at(terrain, start)), landmarks.map(|l| |c| l.at(terrain, c).unwrap()));
    let least = |k: usize| {
        let straight = (octile(start, points[k]) / top) as u64;
        here.zip(marks).map_or(straight, |(a, marks)| straight.max(apart(a, marks(points[k]))))
    };
    let mut kept: Vec<usize> = last
        .into_values()
        .filter(|&k| stays && k == end || least(k) as f64 <= by(k))
        .collect();
    kept.sort_unstable();
    let goals: Vec<(i64, i64)> = kept.iter().map(|&k| trajectory[k].0).collect();
    if goals.is_empty() {
        return None;
    }
    // The first of them is the soonest place to meet, should it be reached by then: tried
    // alone first, a search that can aim (and use the landmarks).
    let first = kept[0];
    if let Some((_, time, path)) = among(rules, terrain, free, from, &goals[..1], top)
        && (time as f64 <= by(first) || stays && first == end)
    {
        return Some((first, straighten(rules, terrain, free, from, path)));
    }
    let cells: Vec<(i64, i64)> = goals.iter().map(|&g| cell(g)).collect();
    let times: Vec<f64> = kept.iter().map(|&k| by(k)).collect();
    let (reached, path) = search(
        rules,
        terrain,
        free,
        from,
        &goals,
        // the soonest they are both there
        &|g, time| {
            let (k, by) = (kept[g], by(kept[g]));
            (time as f64 <= by || stays && k == end).then(|| (time as f64).max(by) as u64)
        },
        &stretches(terrain, landmarks, &cells, &times, stays, top),
    );
    let reached = reached?;
    Some((kept[reached], straighten(rules, terrain, free, from, path)))
}

/// Stretches of the trajectory that [`intercept`]'s search bounds its scores with.
const STRETCHES: usize = 32;

/// For [`intercept`]'s search, of the `goals` (cells, in the order of the trajectory) and
/// when the target leaves each (`by`, increasing; the last one never should it `stay`): a
/// bound on the score through a cell at a time, by stretches of consecutive goals (each
/// within its box of cells, left between the first's time and the last's); None when no
/// goal can be reached through it before the target has left.
fn stretches<'a>(
    terrain: &'a Terrain,
    landmarks: Option<&'a Landmarks>,
    goals: &[(i64, i64)],
    by: &[f64],
    stays: bool,
    top: f64,
) -> impl Fn((i64, i64), u64) -> Option<u64> + 'a {
    let size = goals.len().div_ceil(STRETCHES);
    let boxes: Vec<_> = (0..goals.len())
        .step_by(size)
        .map(|k| {
            let end = (k + size).min(goals.len());
            let (mut c0, mut r0, mut c1, mut r1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            // the least and most times from each landmark
            let mut marks = [(u32::MAX, 0); LANDMARKS];
            for &(c, r) in &goals[k..end] {
                (c0, r0, c1, r1) = (c0.min(c), r0.min(r), c1.max(c), r1.max(r));
                if let Some(at) = landmarks.and_then(|l| l.at(terrain, (c, r))) {
                    for (m, &t) in marks.iter_mut().zip(at) {
                        *m = (m.0.min(t), m.1.max(t));
                    }
                }
            }
            let last = if stays && end == goals.len() { f64::INFINITY } else { by[end - 1] };
            ((c0, r0, c1, r1), marks, by[k], last)
        })
        .collect();
    move |(c, r), time| {
        let at = landmarks.and_then(|l| l.at(terrain, (c, r)));
        let mut best: Option<f64> = None;
        // (in the order of the trajectory: none later scores below the time it is left)
        for &((c0, r0, c1, r1), marks, first, last) in &boxes {
            if best.is_some_and(|b| first >= b) {
                break;
            }
            let mut apart = octile((c, r), (c.clamp(c0, c1), r.clamp(r0, r1))) / top;
            if let Some(at) = at {
                for (&(least, most), &t) in marks.iter().zip(at) {
                    apart = apart.max(least.saturating_sub(t).max(t.saturating_sub(most)) as f64);
                }
            }
            let at = time as f64 + apart;
            if at <= last {
                best = Some(best.map_or(at.max(first), |b| b.min(at.max(first))));
            }
        }
        best.map(|b| b as u64)
    }
}

/// The search's usual bound: the time to reach the nearest goal, straight on at speed `top`
/// or by the `landmarks` (for up to [`MAX_LANDMARKED`] goals), whichever is the longer (none
/// beyond [`MAX_AIMED`] goals: spreading evenly until it meets one).
fn aim<'a>(terrain: &'a Terrain, landmarks: Option<&'a Landmarks>, goals: &[(i64, i64)], top: f64) -> impl Fn((i64, i64), u64) -> Option<u64> + 'a {
    let aimed: Vec<(i64, i64)> = if goals.len() <= MAX_AIMED { goals.iter().map(|&g| cell(g)).collect() } else { Vec::new() };
    let landmarks = landmarks.filter(|_| goals.len() <= MAX_LANDMARKED);
    let marks: Vec<[u32; LANDMARKS]> = landmarks.map_or(Vec::new(), |l| aimed.iter().map(|&g| *l.at(terrain, g).unwrap()).collect());
    move |n, time| {
        let at = landmarks.and_then(|l| l.at(terrain, n));
        let bound = |k: usize| {
            let straight = (octile(n, aimed[k]) / top) as u64;
            at.map_or(straight, |a| straight.max(apart(a, &marks[k])))
        };
        Some(time + (0..aimed.len()).map(bound).min().unwrap_or(0))
    }
}

/// Goals beyond which [`aim`] leaves the landmarks out (the bound would cost more than it
/// saves).
const MAX_LANDMARKED: usize = 4;

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
    free: Free,
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
            time += len / n / (speed * speed_through(rules, terrain, free, clng(x), clng(y)).max(0.01));
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
    free: Free,
    from: (i64, i64),
    goals: &[(i64, i64)],
    accept: &dyn Fn(usize, u64) -> Option<u64>,
    bound: &dyn Fn((i64, i64), u64) -> Option<u64>,
) -> (Option<usize>, Vec<(i64, i64)>) {
    let top = top_speed(rules);
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
    let steps = steps(rules, terrain);
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
                let (straight, diagonal) = step_time(steps, terrain, free, next);
                let g = g + if dc != 0 && dr != 0 { diagonal } else { straight } as u64;
                if seen.entry(next).is_some_and(|(known, _)| known <= g) {
                    continue;
                }
                let Some(f) = bound(next, g) else { continue };
                seen.insert(next, (g, node));
                nearest = nearest.min((estimate(next, first), next));
                open.push(Reverse((f, g, next)));
            }
        }
        LOOKED.set(LOOKED.get() + looked as u64);
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

/// What the search makes of a [`Terrain`]'s `category` when it first needs it (again after
/// it changes, see [`Terrain::category_mut`]): the time to step into each cell (as
/// [`speed_coef`] gives its speed at its centre), straight and diagonally; and the
/// [`Landmarks`].
#[derive(Clone, Default)]
pub struct Prepared {
    steps: std::sync::OnceLock<Vec<(u32, u32)>>,
    // (shared by copies of the terrain, rather than copied)
    landmarks: std::sync::OnceLock<std::sync::Arc<Landmarks>>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Prepared")
    }
}

/// The time to step into a cell of `speed`, straight and diagonally.
fn step_times(speed: f64) -> (u32, u32) {
    let speed = speed.max(0.01);
    ((STRAIGHT / speed) as u32, (DIAGONAL / speed) as u32)
}

/// Makes what the search needs of `terrain` ([`Prepared`]) now rather than at the first
/// search, which would take that much longer (a third of a second on a 1024 × 1024 map).
pub fn prepare(rules: &Rules, terrain: &Terrain) {
    // (a search from a corner of the map to itself would use them)
    Landmarks::of(rules, terrain, (0, 0), &[(0, 0)]);
}

impl Prepared {
    /// Cells `cells` of `category` changed: their times again, if they were made.
    pub(crate) fn rebuilt(&mut self, rules: &Rules, category: &[u8], cells: &[usize]) {
        if let Some(steps) = self.steps.get_mut() {
            for &k in cells {
                steps[k] = step_times(rules.terrains[category[k] as usize].speed);
            }
        }
    }
}

/// Whether a cell of terrain `k` may become a street or a wall as the game goes (see
/// [`Terrain::builds`]): of the land, but water and mountains.
fn buildable_land(rules: &Rules, k: u8) -> bool {
    !rules.terrains[k as usize].unbuildable || (6..=9).contains(&k)
}

/// The time to step into each cell of `terrain` (see [`Prepared`]).
fn steps<'a>(rules: &Rules, terrain: &'a Terrain) -> &'a [(u32, u32)] {
    terrain.prepared.steps.get_or_init(|| {
        terrain.category().iter().map(|&k| step_times(rules.terrains[k as usize].speed)).collect()
    })
}

/// The time to step into cell `(c, r)`, straight and diagonally, from `steps`: as at the
/// speed [`speed_through`] gives at its centre, keeping the original's index (rows of `width`
/// without bounds on the column, plain beyond the ends).
fn step_time(steps: &[(u32, u32)], terrain: &Terrain, free: Free, (c, r): (i64, i64)) -> (u32, u32) {
    let (w, h) = (terrain.width as i64, terrain.height as i64);
    let t = if !terrain.torus || (0..w).contains(&c) && (0..h).contains(&r) {
        c + r * w
    } else {
        wrap_axis(c, w) + wrap_axis(r, h) * w
    };
    let times = if t > 0 && (t as usize) < steps.len() - 1 { steps[t as usize] } else { step_times(1.0) };
    if terrain.cover.slows(t, free) {
        let slow = step_times(BUILDING_SPEED);
        (times.0.max(slow.0), times.1.max(slow.1))
    } else {
        times
    }
}

/// Landmarks spread over the map, each as far as can be from the others.
const LANDMARKS: usize = 8;

/// For each cell of the map, the time from each of [`LANDMARKS`] cells, each step taken at
/// the cheaper of its two ways: the time between two cells is at least the difference of
/// theirs from a landmark (the triangle inequality), a bound on the search's time much closer
/// than the straight line at the top speed when slow ground lies between. None on a flat map
/// when the search may leave it (its ends off it), where the landmarks know nothing.
pub(crate) struct Landmarks {
    times: Vec<[u32; LANDMARKS]>,
}

impl Landmarks {
    /// The times of cell `(c, r)` from the landmarks, None off a flat map.
    fn at(&self, terrain: &Terrain, (c, r): (i64, i64)) -> Option<&[u32; LANDMARKS]> {
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        let k = if (0..w).contains(&c) && (0..h).contains(&r) {
            c + r * w
        } else if terrain.torus {
            wrap_axis(c, w) + wrap_axis(r, h) * w
        } else {
            return None;
        };
        Some(&self.times[k as usize])
    }

    /// The landmarks of `terrain`, for a search from `from` to `goals` (cells); None on a
    /// flat map when one of them is off it.
    fn of<'a>(rules: &Rules, terrain: &'a Terrain, from: (i64, i64), goals: &[(i64, i64)]) -> Option<&'a Landmarks> {
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        let on = |(c, r): (i64, i64)| (0..w).contains(&c) && (0..h).contains(&r);
        if !terrain.torus && !(on(from) && goals.iter().all(|&g| on(g))) {
            return None;
        }
        Some(terrain.prepared.landmarks.get_or_init(|| std::sync::Arc::new(Landmarks::new(rules, terrain))))
    }

    fn new(rules: &Rules, terrain: &Terrain) -> Landmarks {
        let steps = steps(rules, terrain);
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        let n = (w * h) as usize;
        let mut cost: Vec<(u32, u32)> = (0..n as i64).map(|k| step_time(steps, terrain, Free::EVERYWHERE, (k % w, k / w))).collect();
        // On land that changes as the game goes, any cell that may become a street or a wall
        // counts as the fastest of the land, so that the times from the landmarks stay a lower
        // bound whatever is built, and are the same made before or after.
        if terrain.builds {
            let fastest = (0..rules.terrains.len() as u8).filter(|&k| buildable_land(rules, k)).map(|k| rules.terrains[k as usize].speed).fold(1.0, f64::max);
            for (k, c) in cost.iter_mut().enumerate() {
                if buildable_land(rules, terrain.category()[k]) {
                    *c = step_times(fastest);
                }
            }
        }
        let mut times = vec![[0; LANDMARKS]; n];
        // the time to the nearest landmark so far, the next being the farthest cell
        let mut nearest = vec![u32::MAX; n];
        let mut at = 0;
        for l in 0..LANDMARKS {
            let time = Self::spread(terrain, &cost, at);
            for ((marks, near), &t) in times.iter_mut().zip(&mut nearest).zip(&time) {
                marks[l] = t;
                *near = (*near).min(t);
            }
            at = (0..n).max_by_key(|&k| nearest[k]).unwrap_or(0);
        }
        Landmarks { times }
    }

    /// Dijkstra over the whole map from cell `from`, each step at the cheaper of its two
    /// ways (stepping into either cell), with buckets (a ring of times, longer than a step).
    fn spread(terrain: &Terrain, cost: &[(u32, u32)], from: usize) -> Vec<u32> {
        let (w, h) = (terrain.width as i64, terrain.height as i64);
        let ring = (cost.iter().map(|c| c.1).max().unwrap_or(1) as usize + 1).next_power_of_two();
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); ring];
        let mut time = vec![u32::MAX; cost.len()];
        time[from] = 0;
        buckets[0].push(from as u32);
        let (mut queued, mut now) = (1usize, 0u32);
        while queued > 0 {
            let Some(k) = buckets[now as usize & (ring - 1)].pop() else {
                now += 1;
                continue;
            };
            queued -= 1;
            let k = k as usize;
            if time[k] < now {
                continue;
            }
            let (c, r) = (k as i64 % w, k as i64 / w);
            for (dc, dr) in NEIGHBOURS {
                let (nc, nr) = (c + dc, r + dr);
                let j = if (0..w).contains(&nc) && (0..h).contains(&nr) {
                    nc + nr * w
                } else if terrain.torus {
                    wrap_axis(nc, w) + wrap_axis(nr, h) * w
                } else {
                    continue;
                } as usize;
                let diagonal = dc != 0 && dr != 0;
                let step = |(s, d): (u32, u32)| if diagonal { d } else { s };
                let t = now + step(cost[k]).min(step(cost[j]));
                if t < time[j] {
                    time[j] = t;
                    buckets[t as usize & (ring - 1)].push(j as u32);
                    queued += 1;
                }
            }
        }
        time
    }
}

/// The bound [`Landmarks`] give on the time between two cells of times `a` and `b` from them.
fn apart(a: &[u32; LANDMARKS], b: &[u32; LANDMARKS]) -> u64 {
    a.iter().zip(b).map(|(&a, &b)| a.abs_diff(b)).max().unwrap_or(0) as u64
}

/// `path` without the waypoints that a straight line can skip: one all at least as fast as
/// the fastest terrain of the stretch of `path` it replaces (and so shorter, and no slower).
/// It removes the zigzags of paths over cells, keeping the turns that follow faster terrain.
fn straighten(
    rules: &Rules,
    terrain: &Terrain,
    free: Free,
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
            fastest = fastest.max(max_speed(rules, terrain, free, prev, p));
            prev = p;
            if j == k || clear(rules, terrain, free, at, p, fastest) {
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
fn max_speed(rules: &Rules, terrain: &Terrain, free: Free, from: (i64, i64), to: (i64, i64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = (dx.abs().max(dy.abs()) * 2 / CELL as i64).max(1);
    (0..=steps)
        .map(|k| {
            speed_through(
                rules,
                terrain,
                free,
                from.0 + dx * k / steps,
                from.1 + dy * k / steps,
            )
        })
        .fold(0.0, f64::max)
}

/// Whether the straight line from `from` to `to` is all at speed `speed` or faster.
fn clear(rules: &Rules, terrain: &Terrain, free: Free, from: (i64, i64), to: (i64, i64), speed: f64) -> bool {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    // every half cell
    let steps = (dx.abs().max(dy.abs()) * 2 / CELL as i64).max(1);
    (0..=steps).all(|k| {
        let (x, y) = (from.0 + dx * k / steps, from.1 + dy * k / steps);
        speed_through(rules, terrain, free, x, y) >= speed
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
