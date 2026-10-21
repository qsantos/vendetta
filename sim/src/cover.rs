//! Not in the original: the buildings slow those who walk through them, like walls, but for
//! their own (see [`Free`]), with compact towns (`Towns::Compact`), whose lanes then make
//! streets. Which building stands on each cell, kept up to date as they are built and fall.

use crate::building::Building;
use crate::terrain::{CELL, Terrain, wrap_axis};

/// The speed through a building (that of the walls).
pub const BUILDING_SPEED: f64 = 0.2;

type Rect = (i32, i32, i32, i32);

/// Which building stands on each cell (none in the original, where they do not slow): a cell
/// is under a building when its centre is (castles and houses never overlap, so under one at
/// most). Not saved: made again from the buildings.
#[derive(Debug, Clone, Default)]
pub struct Cover {
    /// For each cell (as [`Terrain::index_at`] gives them), its building's number plus one, 0
    /// for none: the castles' first, then the houses'.
    under: Vec<u32>,
    /// The place of each building on `under`, None when not standing.
    placed: Vec<Option<Rect>>,
}

/// The buildings that do not slow a walker: its castle, its house and the one it goes to (or
/// none at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Free {
    everywhere: bool,
    buildings: [u32; 3],
}

impl Free {
    /// Nothing slows (no building does in the original).
    pub const EVERYWHERE: Free = Free { everywhere: true, buildings: [0; 3] };
}

impl Cover {
    /// Places the `castles` and `houses` standing, changing only where they changed since; or
    /// clears it all when `on` is false.
    pub fn update(&mut self, terrain: &Terrain, on: bool, castles: &[Building], houses: &[Building]) {
        if !on {
            *self = Cover::default();
            return;
        }
        let n = terrain.category().len();
        if self.under.len() != n {
            *self = Cover { under: vec![0; n], placed: Vec::new() };
        }
        self.placed.resize(castles.len() + houses.len(), None);
        for (k, b) in castles.iter().chain(houses).enumerate() {
            let now = b.visible().then_some((b.x, b.y, b.w, b.h));
            if self.placed[k] == now {
                continue;
            }
            let id = k as u32 + 1;
            if let Some(was) = self.placed[k] {
                for t in cells(terrain, was) {
                    if self.under[t] == id {
                        self.under[t] = 0;
                    }
                }
            }
            if let Some(now) = now {
                for t in cells(terrain, now) {
                    self.under[t] = id;
                }
            }
            self.placed[k] = now;
        }
    }

    /// Whether nothing slows (the original's way).
    pub fn is_off(&self) -> bool {
        self.under.is_empty()
    }

    /// What does not slow a walker of `fief` (whose house is `house`) going to `to` (a feet
    /// position).
    pub fn free(&self, terrain: &Terrain, castles: usize, fief: usize, house: usize, to: (i64, i64)) -> Free {
        if self.is_off() {
            return Free::EVERYWHERE;
        }
        let dest = self.at(terrain.index_at(to.0 as f64, to.1 as f64));
        Free { everywhere: false, buildings: [fief as u32 + 1, (castles + house) as u32 + 1, dest] }
    }

    /// The building on cell `t` plus one, 0 for none.
    fn at(&self, t: i64) -> u32 {
        usize::try_from(t).ok().and_then(|t| self.under.get(t)).copied().unwrap_or(0)
    }

    /// Whether a building that is not `free` stands on cell `t`.
    pub fn slows(&self, t: i64, free: Free) -> bool {
        if free.everywhere {
            return false;
        }
        let b = self.at(t);
        b != 0 && !free.buildings.contains(&b)
    }
}

/// The cells whose centre lies in `rect`.
fn cells(terrain: &Terrain, (x, y, w, h): Rect) -> impl Iterator<Item = usize> + '_ {
    let span = |from: i32, len: i32| (from + CELL - 1).div_euclid(CELL)..(from + len + CELL - 1).div_euclid(CELL);
    let (cols, rows) = (span(x, w), span(y, h));
    let (tw, th) = (terrain.width as i64, terrain.height as i64);
    rows.flat_map(move |r| cols.clone().map(move |c| (c as i64, r as i64))).filter_map(move |(c, r)| {
        if terrain.torus {
            Some((wrap_axis(c, tw) + wrap_axis(r, th) * tw) as usize)
        } else {
            ((0..tw).contains(&c) && (0..th).contains(&r)).then(|| (c + r * tw) as usize)
        }
    })
}
