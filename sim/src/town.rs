//! Not in the original: what compact towns ([`Towns::Compact`]) build besides their houses, on
//! roads' land (a world type without roads builds none): a square before the castle's gate, a
//! street from each new house's door to the nearest road (which later houses may build over),
//! and walls round the town when its fief enters a new age.

use crate::building::Building;
use crate::rules::Rules;
use crate::terrain::{CELL, wrap_axis};
use crate::world::{Towns, World};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// How far a street goes looking for a road, in cells, and how much dearer a step is away from
/// the buildings (so that streets go along their fronts, in the ways left between them, rather
/// than across the land where more would be built).
const STREET_REACH: i64 = 24;
const STREET_OPEN: u32 = 3;
/// The square kept free before a castle's gate (wide, deep), in pixels.
pub(crate) const SQUARE: (i32, i32) = (8 * CELL, 5 * CELL);
/// The walls' terrain (`Muraille3`, a castle's brick wall, the same tile whatever its
/// neighbours: `Muraille1` has none for straight walls), and how far they stand off the fief's
/// buildings, in cells.
const WALL: u8 = 8;
const WALL_MARGIN: i64 = 3;
/// The walls' outline is smoothed over so many cells (grown, then shrunk back), and goes at
/// most so far from the castle (and no farther than halfway to the next one).
const WALL_SMOOTH: i64 = 3;
const WALL_REACH: i64 = 24;

/// The eight ways to a neighbouring cell.
const EIGHT: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Whether streets and walls may go on cells of terrain `k`: land, not a road (`road`) or a
/// wall already.
fn open_land(rules: &Rules, k: u8, road: u8) -> bool {
    !rules.terrains[k as usize].unbuildable && k != road
}

/// The cells under building `b`, as `(column, row)` (not wrapped).
fn cells_under(b: &Building) -> impl Iterator<Item = (i64, i64)> {
    let span = |from: i32, len: i32| (from + CELL / 2).div_euclid(CELL) as i64..=(from + len - 1 + CELL / 2).div_euclid(CELL) as i64;
    let (cols, rows) = (span(b.x, b.w), span(b.y, b.h));
    rows.flat_map(move |r| cols.clone().map(move |c| (c, r)))
}

impl World {
    /// Whether this world's towns build streets and walls.
    fn town_builds(&self) -> bool {
        self.towns == Towns::Compact && self.road != 0 && self.terrain.builds
    }

    /// The index of cell `(c, r)`, across the edges on a torus; None off a flat map.
    fn cell_index(&self, (c, r): (i64, i64)) -> Option<usize> {
        let (w, h) = (self.terrain.width as i64, self.terrain.height as i64);
        if self.terrain.torus {
            Some((wrap_axis(r, h) * w + wrap_axis(c, w)) as usize)
        } else {
            ((0..w).contains(&c) && (0..h).contains(&r)).then_some((r * w + c) as usize)
        }
    }

    /// Whether a building stands on each cell of the map (the castles, the houses).
    fn built_cells(&self) -> Vec<bool> {
        let mut built = vec![false; self.terrain.width * self.terrain.height];
        for b in self.castles.iter().chain(&self.houses).filter(|b| b.visible()) {
            for at in cells_under(b) {
                if let Some(k) = self.cell_index(at) {
                    built[k] = true;
                }
            }
        }
        built
    }

    /// Paves the square before each castle's gate (see [`SQUARE`]), on the land.
    pub(crate) fn pave_squares(&mut self, rules: &Rules) {
        if !self.town_builds() {
            return;
        }
        let mut cells = Vec::new();
        for c in &self.castles {
            let (x, y) = (c.x + c.w / 2 + c.entrance.0 - SQUARE.0 / 2, c.y + c.h);
            for r in (y + CELL / 2).div_euclid(CELL)..(y + SQUARE.1 + CELL / 2).div_euclid(CELL) {
                for col in (x + CELL / 2).div_euclid(CELL)..(x + SQUARE.0 + CELL / 2).div_euclid(CELL) {
                    if let Some(k) = self.cell_index((col as i64, r as i64))
                        && open_land(rules, self.terrain.category()[k], self.road)
                    {
                        cells.push((k, self.road));
                    }
                }
            }
        }
        self.terrain.build(rules, &cells);
    }

    /// Takes up the streets under house `h` (see [`World::lay_street`]): the land is as it was.
    pub(crate) fn clear_streets(&mut self, rules: &Rules, h: usize) {
        if self.streets.is_empty() {
            return;
        }
        let under: Vec<usize> = cells_under(&self.houses[h]).filter_map(|at| self.cell_index(at)).collect();
        let cells: Vec<(usize, u8)> = under.into_iter().filter_map(|k| self.streets.remove(&(k as u32)).map(|was| (k, was))).collect();
        self.terrain.build(rules, &cells);
    }

    /// A street from the door of house `h` (the cell below it, in line with its entrance) to
    /// the nearest road, the shortest way between the buildings over the land (mostly along
    /// them), if there is one near enough.
    pub(crate) fn lay_street(&mut self, rules: &Rules, h: usize) {
        if !self.town_builds() {
            return;
        }
        let m = &self.houses[h];
        let door = m.x + m.w / 2 + m.entrance.0;
        let start = ((door + CELL / 2).div_euclid(CELL) as i64, ((m.y + m.h - 1 + CELL / 2).div_euclid(CELL) + 1) as i64);
        let built = self.built_cells();
        let road = self.road;
        let category = self.terrain.category();
        let free = |at: (i64, i64)| self.cell_index(at).filter(|&k| !built[k] && (category[k] == road || open_land(rules, category[k], road)));
        if free(start).is_none() {
            return;
        }
        // Dijkstra over the cells near the door, in a box of offsets from it
        let side = (2 * STREET_REACH + 1) as usize;
        let local = |(c, r): (i64, i64)| {
            let (dc, dr) = (c - start.0 + STREET_REACH, r - start.1 + STREET_REACH);
            ((0..side as i64).contains(&dc) && (0..side as i64).contains(&dr)).then(|| dr as usize * side + dc as usize)
        };
        let mut came = vec![usize::MAX; side * side];
        let mut cost = vec![u32::MAX; side * side];
        let mut heap = BinaryHeap::new();
        let s = local(start).unwrap();
        cost[s] = 0;
        heap.push(Reverse((0u32, s)));
        let mut reached = None;
        while let Some(Reverse((t, l))) = heap.pop() {
            if t > cost[l] {
                continue;
            }
            let at = ((l % side) as i64 + start.0 - STREET_REACH, (l / side) as i64 + start.1 - STREET_REACH);
            if free(at).is_some_and(|k| category[k] == road) {
                reached = Some(l);
                break;
            }
            for d in [(0, 1), (-1, 0), (1, 0), (0, -1)] {
                let next = (at.0 + d.0, at.1 + d.1);
                let (Some(j), Some(k)) = (local(next), free(next)) else { continue };
                let against = category[k] != road && [(0, -1), (-1, 0), (1, 0), (0, 1)].iter().any(|&(dc, dr)| self.cell_index((next.0 + dc, next.1 + dr)).is_some_and(|k| built[k]));
                let t = t + if against { STREET_OPEN } else { 1 };
                if t < cost[j] {
                    cost[j] = t;
                    came[j] = l;
                    heap.push(Reverse((t, j)));
                }
            }
        }
        let Some(mut l) = reached else { return };
        let mut cells = Vec::new();
        while l != s {
            l = came[l];
            let at = ((l % side) as i64 + start.0 - STREET_REACH, (l / side) as i64 + start.1 - STREET_REACH);
            cells.extend(self.cell_index(at));
        }
        cells.retain(|&k| category[k] != road);
        let cells: Vec<(usize, u8)> = cells.into_iter().map(|k| (k, road)).collect();
        for &(k, _) in &cells {
            self.streets.insert(k as u32, category[k]);
        }
        self.terrain.build(rules, &cells);
    }

    /// Walls round the town of fief `f`: a smooth ring of [`WALL`] about `WALL_MARGIN` cells off
    /// its castle and the houses near it that stand together with it (those farther out are
    /// left outside, its suburbs), but where roads and streets pass (its gates), and on water
    /// and mountains.
    pub fn build_walls(&mut self, rules: &Rules, f: usize) {
        if !self.town_builds() || !self.castles[f].visible() {
            return;
        }
        let castle = &self.castles[f];
        let centre = |b: &Building| ((b.x + b.w / 2) as i64, (b.y + b.h / 2) as i64);
        let next = (self.castles.iter().enumerate())
            .filter(|&(g, c)| g != f && c.visible())
            .map(|(_, c)| self.terrain.dist(centre(castle), centre(c)))
            .fold(f64::INFINITY, f64::min);
        let reach = (next / 2.0).min((WALL_REACH * CELL as i64) as f64);
        let core = |b: &&Building| b.visible() && self.terrain.dist(centre(castle), centre(b)) <= reach;
        let fief_houses = (0..self.houses.len()).filter(|&j| self.persos[j].fief == f).map(|j| &self.houses[j]);
        let town: Vec<(i64, i64)> = std::iter::once(castle).chain(fief_houses).filter(core).flat_map(cells_under).collect();
        // the cells within the margin of the town (by offset from the castle, so that a town
        // across the edges of a torus stays in one piece), then their outline
        let (oc, or) = cells_under(&self.castles[f]).next().unwrap();
        let wrap = |(c, r): (i64, i64)| -> (i64, i64) {
            if self.terrain.torus {
                let (w, h) = (self.terrain.width as i64, self.terrain.height as i64);
                let d = |v: i64, o: i64, n: i64| {
                    let d = wrap_axis(v - o, n);
                    if 2 * d >= n { d - n } else { d }
                };
                (d(c, oc, w), d(r, or, h))
            } else {
                (c - oc, r - or)
            }
        };
        let offsets: Vec<(i64, i64)> = town.iter().map(|&at| wrap(at)).collect();
        let (c0, r0) = offsets.iter().fold((i64::MAX, i64::MAX), |a, b| (a.0.min(b.0), a.1.min(b.1)));
        let (c1, r1) = offsets.iter().fold((i64::MIN, i64::MIN), |a, b| (a.0.max(b.0), a.1.max(b.1)));
        let m = WALL_MARGIN + WALL_SMOOTH + 1;
        let (c0, r0, c1, r1) = (c0 - m, r0 - m, c1 + m, r1 + m);
        let (cols, rows) = ((c1 - c0 + 1) as usize, (r1 - r0 + 1) as usize);
        let at = |c: i64, r: i64| ((c0..=c1).contains(&c) && (r0..=r1).contains(&r)).then(|| (r - r0) as usize * cols + (c - c0) as usize);
        // distance (in steps, any of the eight ways) from the town, up to the margin
        let mut near = vec![u8::MAX; cols * rows];
        let mut todo = std::collections::VecDeque::new();
        for &(c, r) in &offsets {
            let k = at(c, r).unwrap();
            if near[k] != 0 {
                near[k] = 0;
                todo.push_back((c, r));
            }
        }
        while let Some((c, r)) = todo.pop_front() {
            let d = near[at(c, r).unwrap()];
            if d as i64 >= WALL_MARGIN + WALL_SMOOTH {
                continue;
            }
            for (dc, dr) in EIGHT {
                if let Some(k) = at(c + dc, r + dr)
                    && near[k] == u8::MAX
                {
                    near[k] = d + 1;
                    todo.push_back((c + dc, r + dr));
                }
            }
        }
        // the town: what lies within the margin together with the castle (eight ways), and
        // the holes in it
        let within = |k: usize| near[k] != u8::MAX;
        let mut town_cells = vec![false; cols * rows];
        let mut todo: Vec<(i64, i64)> = cells_under(&self.castles[f]).map(wrap).collect();
        while let Some((c, r)) = todo.pop() {
            let Some(k) = at(c, r) else { continue };
            if town_cells[k] || !within(k) {
                continue;
            }
            town_cells[k] = true;
            todo.extend(EIGHT.map(|(dc, dr)| (c + dc, r + dr)));
        }
        let mut outside = vec![false; cols * rows];
        let mut todo: Vec<(i64, i64)> = (c0..=c1).flat_map(|c| [(c, r0), (c, r1)]).chain((r0..=r1).flat_map(|r| [(c0, r), (c1, r)])).collect();
        while let Some((c, r)) = todo.pop() {
            let Some(k) = at(c, r) else { continue };
            if outside[k] || town_cells[k] {
                continue;
            }
            outside[k] = true;
            todo.extend([(0, -1), (-1, 0), (1, 0), (0, 1)].map(|(dc, dr)| (c + dc, r + dr)));
        }
        // shrunk back: the cells farther than the smoothing from any outside
        let mut depth = vec![u8::MAX; cols * rows];
        let mut todo = std::collections::VecDeque::new();
        for (k, &o) in outside.iter().enumerate() {
            if o {
                depth[k] = 0;
                todo.push_back(((k % cols) as i64 + c0, (k / cols) as i64 + r0));
            }
        }
        while let Some((c, r)) = todo.pop_front() {
            let d = depth[at(c, r).unwrap()];
            for (dc, dr) in EIGHT {
                if let Some(k) = at(c + dc, r + dr)
                    && depth[k] == u8::MAX
                {
                    depth[k] = d + 1;
                    todo.push_back((c + dc, r + dr));
                }
            }
        }
        let outside: Vec<bool> = depth.iter().map(|&d| d as i64 <= WALL_SMOOTH).collect();
        // the ring: the cells outside beside one within (any of the eight ways: so that the
        // wall goes on round its corners, cell to cell)
        let built = self.built_cells();
        let (road, category) = (self.road, self.terrain.category());
        let mut ring = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                let inside = |c: i64, r: i64| at(c, r).is_some_and(|k| !outside[k]);
                if inside(c, r) || !EIGHT.iter().any(|&(dc, dr)| inside(c + dc, r + dr)) {
                    continue;
                }
                let Some(k) = self.cell_index((c + oc, r + or)) else { continue };
                if !built[k] && open_land(rules, category[k], road) {
                    ring.push((k, WALL));
                }
            }
        }
        self.terrain.build(rules, &ring);
    }
}
