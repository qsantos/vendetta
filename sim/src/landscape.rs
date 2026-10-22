//! Not in the original: random worlds with a lay of the land. The original's (`Generer_Monde`,
//! see `World::generate_terrain`) grows blobs of every terrain from scattered seeds, the same
//! size all over the map; castles go on a grid, resources anywhere. Here instead:
//! - a height and a moisture made of noise at several scales: low land under water, high land
//!   mountains, the rest earth, plain, grass or desert from dry to wet, in the shares the world
//!   type gives its terrains (`SurfaceTerrain`), so that "Grands Lacs" has its lakes and
//!   "Désert" its sands;
//! - rivers, running down from the hills to the water;
//! - castles as far from each other as the land allows, each on a clearing;
//! - roads that go round mountains and cross rivers where they are narrow, merging on their
//!   way;
//! - resources where they belong: trees in woods, ores by the mountains; and the same few
//!   around every castle, for fair starts.
//!
//! The noise is made of integer hashes and polynomials only (no `sin` or `exp`), so that every
//! build makes the same world, as network play needs.

use crate::rules::Rules;
use crate::terrain::{CELL, wrap_axis};
use crate::vb::clng;
use crate::world::{RESOURCE_SIZE, ResourceNode, Roads, World};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// How a random world's land is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Landscape {
    /// The original's blobs of terrain, castles on a grid.
    #[default]
    Original,
    /// Hills, lakes, rivers, woods and fairly placed castles (see the module).
    Natural,
}

// The terrains of `Terrain.ini` the land is made of.
const PLAIN: u8 = 0;
const GRASS: u8 = 1;
const EARTH: u8 = 2;
const DESERT: u8 = 3;
const MOUNTAIN: u8 = 4;
const WATER: u8 = 11;
/// The share of plain (which the original's worlds never grow: it is what is left), against
/// the world type's `SurfaceTerrain` of the others.
const PLAIN_SURFACE: f64 = 13.0;

/// Cells kept clear of water and mountains around a castle.
const CLEARING: i64 = 3;
/// The resources of the map (`TypeRessource`) that grow, in woods and meadows; the others are
/// deposits, by the mountains.
const WOOD: i64 = 3;
const APPLES: i64 = 2;
const HERBS: i64 = 1;
/// What every castle has around it (by `TypeRessource`), when the map has enough resources.
const START: [i64; 8] = [WOOD, WOOD, WOOD, APPLES, HERBS, 13, 13, 15];
/// Cells from the mountains within which deposits are found (mostly near).
const ORE_REACH: u8 = 10;

/// A hash of a lattice point, in [0, 1].
fn lattice(seed: u32, x: u32, y: u32) -> f64 {
    let mut h = seed ^ x.wrapping_mul(0x27d4_eb2d) ^ y.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h as f64 / u32::MAX as f64
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    smooth(((x - a) / (b - a)).clamp(0.0, 1.0))
}

/// Value noise over a map of `size` cells with `n` lattice steps across each axis, at cell
/// `(c, r)`: it wraps at the map's edges (seamless on a torus).
fn value(seed: u32, size: (usize, usize), n: (u32, u32), (c, r): (usize, usize)) -> f64 {
    let fx = (c as f64 + 0.5) / size.0 as f64 * n.0 as f64;
    let fy = (r as f64 + 0.5) / size.1 as f64 * n.1 as f64;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (smooth(fx - x0), smooth(fy - y0));
    let (x0, y0) = (x0 as u32 % n.0, y0 as u32 % n.1);
    let (x1, y1) = ((x0 + 1) % n.0, (y0 + 1) % n.1);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let top = lerp(lattice(seed, x0, y0), lattice(seed, x1, y0), tx);
    let bottom = lerp(lattice(seed, x0, y1), lattice(seed, x1, y1), tx);
    lerp(top, bottom, ty)
}

/// Noise at `octaves` scales from `wavelength` cells down, each half as strong as the one
/// before, in [0, 1]; `ridged` folds each scale into ridges (for mountain ranges).
fn fractal(seed: u32, size: (usize, usize), wavelength: f64, octaves: u32, ridged: bool, at: (usize, usize)) -> f64 {
    let (mut sum, mut total, mut amplitude, mut length) = (0.0, 0.0, 1.0, wavelength);
    for o in 0..octaves {
        let steps = |s: usize| ((s as f64 / length).round() as u32).max(1);
        let mut v = value(seed.wrapping_add(o.wrapping_mul(0x9e37_79b9)), size, (steps(size.0), steps(size.1)), at);
        if ridged {
            v = 1.0 - (2.0 * v - 1.0).abs();
        }
        sum += v * amplitude;
        total += amplitude;
        amplitude /= 2.0;
        length /= 2.0;
    }
    sum / total
}

/// The value of `values` below which a share `f` of them is (all of them from 1 on).
fn quantile(values: &[f64], f: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if f >= 1.0 { f64::INFINITY } else { sorted[((f * sorted.len() as f64) as usize).min(sorted.len() - 1)] }
}

impl World {
    /// [`Landscape::Natural`]: the land, the castles, the roads and the resources (as many as
    /// `self.resources` holds) of a random world.
    pub(crate) fn generate_natural(&mut self, rules: &Rules, world_type: usize, roads: Roads) {
        let (w, h) = (self.terrain.width, self.terrain.height);
        let size = (w, h);
        let seed = (self.rng.next() * 4_294_967_296.0) as u32;
        // Features about as far apart as the castles: a fief has hills or a lake of its own.
        let spacing = ((w * h) as f64 / self.castles.len().max(1) as f64).sqrt();
        let wavelength = spacing.clamp(32.0, 128.0);
        let cells: Vec<(usize, usize)> = (0..h).flat_map(|r| (0..w).map(move |c| (c, r))).collect();
        let height: Vec<f64> = cells
            .iter()
            .map(|&at| 0.65 * fractal(seed, size, wavelength, 5, false, at) + 0.35 * fractal(seed ^ 0x5bd1_e995, size, wavelength, 4, true, at))
            .collect();
        let moisture: Vec<f64> = cells.iter().map(|&at| fractal(seed ^ 0x68e3_1da4, size, wavelength / 2.0, 4, false, at)).collect();

        // The shares of the terrains, as the world type grows them.
        let wt = &rules.world_types[world_type];
        let surface = |t: u8| wt.surface.get(t as usize).copied().unwrap_or(0).max(0) as f64;
        let lands = [(DESERT, surface(DESERT)), (EARTH, surface(EARTH)), (PLAIN, PLAIN_SURFACE), (GRASS, surface(GRASS))];
        let total = surface(WATER) + surface(MOUNTAIN) + lands.iter().map(|l| l.1).sum::<f64>();
        let (water, mountain) = (surface(WATER) / total, surface(MOUNTAIN) / total);
        let sea = if water > 0.0 { quantile(&height, water) } else { f64::NEG_INFINITY };
        let peaks = if mountain > 0.0 { quantile(&height, 1.0 - mountain) } else { f64::INFINITY };
        let land: Vec<f64> = (0..w * h).filter(|&k| height[k] >= sea && height[k] < peaks).map(|k| moisture[k]).collect();
        let land_total: f64 = lands.iter().map(|l| l.1).sum();
        let mut bounds = Vec::new();
        let mut share = 0.0;
        for &(t, s) in &lands {
            share += s / land_total;
            bounds.push((t, if land.is_empty() { f64::INFINITY } else { quantile(&land, share) }));
        }
        let mut kinds: Vec<u8> = (0..w * h)
            .map(|k| match height[k] {
                v if v < sea => WATER,
                v if v >= peaks => MOUNTAIN,
                _ => bounds.iter().find(|b| moisture[k] < b.1).map_or(GRASS, |b| b.0),
            })
            .collect();
        if water > 0.0 {
            self.rivers(&height, &mut kinds, water);
        }
        self.place_castles_far(rules, &mut kinds);
        let category = self.terrain.category_mut();
        category[..w * h].copy_from_slice(&kinds);
        self.init_castles(rules);
        let road = rules.world_types[world_type].road;
        if road != 0 {
            self.lay_roads(rules, roads, road);
            self.pave_castles(road);
        }
        self.terrain.reshape();
        self.place_resources_natural(rules, seed, wavelength);
    }

    /// The cell `d` away from `(c, r)`: across the edges on a torus, None off a flat map.
    fn cell_toward(&self, (c, r): (i64, i64), (dc, dr): (i64, i64)) -> Option<(i64, i64)> {
        let (w, h) = (self.terrain.width as i64, self.terrain.height as i64);
        let (c, r) = (c + dc, r + dr);
        if self.terrain.torus {
            Some((wrap_axis(c, w), wrap_axis(r, h)))
        } else {
            (c >= 0 && r >= 0 && c < w && r < h).then_some((c, r))
        }
    }

    /// Rivers from the hills down to the water, more with more water (one that cannot reach it,
    /// stuck in a hollow, is not made).
    fn rivers(&mut self, height: &[f64], kinds: &mut [u8], water: f64) {
        let (w, h) = (self.terrain.width, self.terrain.height);
        let count = ((w * h) as f64 / 4000.0 * (water * 4.0).min(1.0)).round() as usize;
        let hills = quantile(height, 0.85);
        let mut seen = vec![usize::MAX; w * h];
        for river in 0..count {
            // a spring on the hills (a few tries)
            let spring = (0..100).find_map(|_| {
                let k = (self.rng.next() * (w * h) as f64) as usize % (w * h);
                (height[k] >= hills && kinds[k] != WATER).then_some(k)
            });
            let Some(mut k) = spring else { continue };
            let mut bed = Vec::new();
            let reached = loop {
                let at = ((k % w) as i64, (k / w) as i64);
                seen[k] = river;
                // two cells wide, three downstream
                bed.push(k);
                for d in if bed.len() > 60 { &[(1, 0), (0, 1)][..] } else { &[(1, 0)][..] } {
                    if let Some((c, r)) = self.cell_toward(at, *d) {
                        bed.push(r as usize * w + c as usize);
                    }
                }
                let lower = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                    .into_iter()
                    .filter_map(|d| self.cell_toward(at, d))
                    .map(|(c, r)| r as usize * w + c as usize)
                    .filter(|&j| seen[j] != river)
                    .min_by(|&a, &b| height[a].total_cmp(&height[b]));
                match lower {
                    Some(j) if kinds[j] == WATER => break true,
                    Some(j) if height[j] < height[k] => k = j,
                    _ => break false,
                }
            };
            if reached {
                for k in bed {
                    kinds[k] = WATER;
                }
            }
        }
    }

    /// The castles one after the other, each as far from those placed as the land allows (the
    /// farthest places, one of them by chance), off water and mountains with a clearing around,
    /// which is then made.
    fn place_castles_far(&mut self, rules: &Rules, kinds: &mut [u8]) {
        let (w, h) = (self.terrain.width as i64, self.terrain.height as i64);
        let n = self.castles.len();
        if n == 0 {
            return;
        }
        // the castles' size in cells, the clearing included
        let span = |c: &crate::building::Building| ((c.w / CELL) as i64 + 1 + 2 * CLEARING, (c.h / CELL) as i64 + 1 + 2 * CLEARING);
        let (sw, sh) = self.castles.iter().map(span).fold((0, 0), |a, b| (a.0.max(b.0), a.1.max(b.1)));
        let buildable = |t: u8| !rules.terrains[t as usize].unbuildable;
        let fits = |c: i64, r: i64, kinds: &[u8]| {
            (0..sh).all(|dr| {
                (0..sw).all(|dc| {
                    let (cc, rr) = (wrap_axis(c + dc, w), wrap_axis(r + dr, h));
                    buildable(kinds[(rr * w + cc) as usize])
                })
            })
        };
        // top-left corners of the clearings, every other cell; on a flat map, inside it
        let (cmax, rmax) = if self.terrain.torus { (w, h) } else { (w - sw, h - sh) };
        let all: Vec<(i64, i64)> = (0..rmax.max(1)).step_by(2).flat_map(|r| (0..cmax.max(1)).step_by(2).map(move |c| (c, r))).collect();
        let mut candidates: Vec<(i64, i64)> = all.iter().copied().filter(|&(c, r)| fits(c, r, kinds)).collect();
        if candidates.len() < n {
            candidates = all;
        }
        let gap = |a: i64, b: i64, size: i64| {
            let d = (a - b).abs();
            if self.terrain.torus { d.min(size - d) } else { d }
        };
        let mut nearest = vec![i64::MAX; candidates.len()];
        let mut placed = Vec::with_capacity(n);
        for k in 0..n {
            let pick = if k == 0 {
                (self.rng.next() * candidates.len() as f64) as usize % candidates.len()
            } else {
                // one of the places at least nine tenths as far as the farthest
                let far = *nearest.iter().max().unwrap();
                let choices: Vec<usize> = (0..candidates.len()).filter(|&j| nearest[j] * 10 >= far * 9).collect();
                choices[(self.rng.next() * choices.len() as f64) as usize % choices.len()]
            };
            let (c, r) = candidates[pick];
            placed.push((c, r));
            for (j, &(cj, rj)) in candidates.iter().enumerate() {
                let (dc, dr) = (gap(cj, c, w), gap(rj, r, h));
                nearest[j] = nearest[j].min(dc * dc + dr * dr);
            }
        }
        for (castle, (c, r)) in self.castles.iter_mut().zip(placed) {
            let (cw, ch) = span(castle);
            for dr in 0..ch {
                for dc in 0..cw {
                    let (cc, rr) = (wrap_axis(c + dc, w), wrap_axis(r + dr, h));
                    let t = &mut kinds[(rr * w + cc) as usize];
                    if !buildable(*t) {
                        *t = PLAIN;
                    }
                }
            }
            castle.x = (wrap_axis(c + CLEARING, w) * CELL as i64) as i32;
            castle.y = (wrap_axis(r + CLEARING, h) * CELL as i64) as i32;
        }
    }

    /// Roads between the castles (as `roads` says which), each the cheapest way for a road
    /// builder: round mountains, over rivers where they are narrow, along the roads already
    /// laid.
    fn lay_roads(&mut self, rules: &Rules, roads: Roads, road: u8) {
        let nf = self.castles.len();
        let links: Vec<(usize, usize)> = match roads {
            Roads::Ring => (0..nf).map(|i| (i, (i + 1) % nf)).filter(|&(i, j)| i != j).collect(),
            Roads::Nearest => self.nearest_links(),
        };
        let (w, h) = (self.terrain.width, self.terrain.height);
        // (on a torus, a castle may reach over the edge)
        let centre = |c: &crate::building::Building| {
            (wrap_axis(((c.x + c.w / 2) / CELL) as i64, w as i64), wrap_axis(((c.y + c.h / 2) / CELL) as i64, h as i64))
        };
        // tenths of a plain cell's cost
        let cost = |t: u8| match t {
            t if t == road => 6,
            WATER => 50,
            MOUNTAIN => 30,
            t if rules.terrains[t as usize].unbuildable => 50,
            DESERT => 13,
            _ => 10,
        };
        let mut best = vec![u64::MAX; w * h];
        let mut from = vec![usize::MAX; w * h];
        let mut touched = Vec::new();
        for (a, b) in links {
            let (start, goal) = (centre(&self.castles[a]), centre(&self.castles[b]));
            let index = |(c, r): (i64, i64)| r as usize * w + c as usize;
            // a greedy estimate (twice the least cost straight on): roads need not be the
            // cheapest, and the search stays narrow
            let estimate = |(c, r): (i64, i64)| {
                let (dx, dy) = ((c - goal.0).abs(), (r - goal.1).abs());
                let (dx, dy) = if self.terrain.torus { (dx.min(w as i64 - dx), dy.min(h as i64 - dy)) } else { (dx, dy) };
                (dx.max(dy) * 10 + dx.min(dy) * 4) as u64 * 6 / 10 * 2
            };
            for k in touched.drain(..) {
                best[k] = u64::MAX;
                from[k] = usize::MAX;
            }
            let mut open = BinaryHeap::new();
            best[index(start)] = 0;
            touched.push(index(start));
            open.push(Reverse((estimate(start), 0u64, start)));
            let mut reached = false;
            while let Some(Reverse((_, g, at))) = open.pop() {
                if at == goal {
                    reached = true;
                    break;
                }
                if g > best[index(at)] {
                    continue;
                }
                for d in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let Some(next) = self.cell_toward(at, d) else { continue };
                    let step = cost(self.terrain.category()[index(next)]) * if d.0 != 0 && d.1 != 0 { 14 } else { 10 };
                    let g = g + step as u64;
                    if g < best[index(next)] {
                        if best[index(next)] == u64::MAX {
                            touched.push(index(next));
                        }
                        best[index(next)] = g;
                        from[index(next)] = index(at);
                        open.push(Reverse((g + estimate(next), g, next)));
                    }
                }
            }
            if !reached {
                continue;
            }
            // Back to the start, two cells wide like the original's roads, without gaps on the
            // diagonals.
            let mut k = index(goal);
            let category = self.terrain.category_mut();
            loop {
                let at = ((k % w) as i64, (k / w) as i64);
                let prev = from[k];
                for d in [(0, 0), (1, 1)] {
                    let (c, r) = (at.0 + d.0, at.1 + d.1);
                    let (c, r) = (wrap_axis(c, w as i64), wrap_axis(r, h as i64));
                    category[r as usize * w + c as usize] = road;
                }
                if prev == usize::MAX {
                    break;
                }
                let p = ((prev % w) as i64, (prev / w) as i64);
                if p.0 != at.0 && p.1 != at.1 {
                    category[(prev / w) * w + k % w] = road;
                }
                k = prev;
            }
        }
    }

    /// The resources: first the same few around every castle (when there are enough of them),
    /// then each of the others of a random kind like the original, but more likely where it
    /// belongs: what grows in woods and meadows, deposits by the mountains.
    fn place_resources_natural(&mut self, rules: &Rules, seed: u32, wavelength: f64) {
        let (w, h) = (self.terrain.width, self.terrain.height);
        let kinds = rules.terrain_resources.len();
        if kinds == 0 {
            return;
        }
        let of_type = |t: i64| rules.terrain_resources.iter().position(|r| r.resource == t);
        // cells to the nearest mountain, up to `ORE_REACH`
        let mut ore = vec![u8::MAX; w * h];
        let mut queue = VecDeque::new();
        for (k, (&t, d)) in self.terrain.category().iter().zip(&mut ore).enumerate() {
            if t == MOUNTAIN {
                *d = 0;
                queue.push_back(k);
            }
        }
        let mountains = !queue.is_empty();
        while let Some(k) = queue.pop_front() {
            if ore[k] >= ORE_REACH {
                continue;
            }
            for d in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
                if let Some((c, r)) = self.cell_toward(((k % w) as i64, (k / w) as i64), d) {
                    let j = r as usize * w + c as usize;
                    if ore[j] == u8::MAX {
                        ore[j] = ore[k] + 1;
                        queue.push_back(j);
                    }
                }
            }
        }
        let woods = |c: usize, r: usize| fractal(seed ^ 0x2545_f491, (w, h), wavelength / 3.0, 3, false, (c, r));
        let fitness = |world: &World, kind: usize, x: i32, y: i32| {
            let k = world.terrain.index_at((x + RESOURCE_SIZE / 2) as f64, (y + RESOURCE_SIZE / 2) as f64);
            if k < 0 || k as usize >= w * h {
                return 1.0;
            }
            let (c, r) = (k as usize % w, k as usize / w);
            let dry = if world.terrain.category()[k as usize] == DESERT { 0.15 } else { 1.0 };
            match rules.terrain_resources[kind].resource {
                WOOD => dry * (0.05 + 0.95 * smoothstep(0.45, 0.7, woods(c, r))),
                APPLES => dry * (0.15 + 0.6 * smoothstep(0.35, 0.6, woods(c, r))),
                HERBS => dry * 0.5,
                _ if !mountains => 1.0,
                _ => match ore[k as usize] {
                    d if d <= ORE_REACH => 0.01 + 0.99 * (1.0 - d as f64 / ORE_REACH as f64),
                    _ => 0.01,
                },
            }
        };
        let mut next = 0;
        let starts: Vec<usize> = START.iter().filter_map(|&t| of_type(t)).collect();
        if self.resources.len() >= 2 * starts.len() * self.castles.len() {
            for i in 0..self.castles.len() {
                let c = &self.castles[i];
                let (cx, cy) = ((c.x + c.w / 2) as f64, (c.y + c.h / 2) as f64);
                let inner = (c.w.max(c.h) / 2 + 3 * CELL) as f64;
                for &kind in &starts {
                    // somewhere in a ring round the castle, out of the way
                    let spot = (0..100).find_map(|_| {
                        let (dx, dy) = (self.rng.next() * 2.0 - 1.0, self.rng.next() * 2.0 - 1.0);
                        let l = (dx * dx + dy * dy).sqrt();
                        if l == 0.0 || l > 1.0 {
                            return None;
                        }
                        let reach = inner + self.rng.next() * 8.0 * CELL as f64;
                        let x = clng(cx + dx / l * reach) as i32 - RESOURCE_SIZE / 2;
                        let y = clng(cy + dy / l * reach) as i32 - RESOURCE_SIZE / 2;
                        let (x, y) = self.terrain.wrap32((x, y));
                        (self.obstacle(rules, 0, x, y, RESOURCE_SIZE, RESOURCE_SIZE) == 0).then_some((x, y))
                    });
                    if let Some((x, y)) = spot {
                        self.resources[next] = ResourceNode { x, y, kind };
                        next += 1;
                    }
                }
            }
        }
        let (mw, mh) = ((w as i32 * CELL - RESOURCE_SIZE) as f64, (h as i32 * CELL - RESOURCE_SIZE) as f64);
        for i in next..self.resources.len() {
            let kind = (clng(self.rng.next() * kinds as f64) as usize).min(kinds - 1);
            let mut tries = 0;
            let (x, y) = loop {
                let x = clng(self.rng.next() * mw) as i32;
                let y = clng(self.rng.next() * mh) as i32;
                tries += 1;
                if self.obstacle(rules, 0, x, y, RESOURCE_SIZE, RESOURCE_SIZE) == 0 && (tries > 64 || self.rng.next() < fitness(self, kind, x, y)) {
                    break (x, y);
                }
            };
            self.resources[i] = ResourceNode { x, y, kind };
        }
    }
}
