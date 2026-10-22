//! Not in the original: the houses slow those who walk through them, like walls, with compact
//! towns (`Towns::Compact`), whose streets then make the ways through a town (not the castles,
//! which everyone goes in and out of); but for a way in, from the door (the building type's
//! entrance) down to the front. Which house stands on each cell, kept up to date as they are
//! built and fall.

use crate::building::Building;
use crate::terrain::{CELL, Terrain, wrap_axis};

/// The speed through a house (that of the walls).
pub const BUILDING_SPEED: f64 = 0.2;

type Rect = (i32, i32, i32, i32);

/// Which house stands on each cell (none in the original, where they do not slow): a cell is
/// under a house when the feet of a walker whose speed is read there are (houses never
/// overlap, so under one at most). Not saved: made again from the houses.
#[derive(Debug, Clone, Default)]
pub struct Cover {
    /// For each cell (as [`Terrain::index_at`] gives them), its house's number plus one, 0 for
    /// none.
    under: Vec<u32>,
    /// The place of each house on `under`, and of its way in, None when not standing.
    placed: Vec<Option<(Rect, Rect)>>,
}

/// Whether the houses slow a walker: not one that flies, or where they do not slow at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Free {
    everywhere: bool,
}

impl Free {
    /// No house slows (none does in the original).
    pub const EVERYWHERE: Free = Free { everywhere: true };
    /// Every house slows.
    pub const NOWHERE: Free = Free { everywhere: false };
}

/// How far a walker's feet are from where its speed is read (the corner of its lowest cell,
/// see `Perso::speed_coef_effect`), give or take: the middle of a character, near its bottom.
const FEET: i32 = 12;

impl Cover {
    /// Places the `houses` standing, changing only where they changed since; or clears it all
    /// when `on` is false.
    pub fn update(&mut self, terrain: &Terrain, on: bool, houses: &[Building]) {
        if !on {
            *self = Cover::default();
            return;
        }
        let n = terrain.category().len();
        if self.under.len() != n {
            *self = Cover { under: vec![0; n], placed: Vec::new() };
        }
        self.placed.resize(houses.len(), None);
        for (k, b) in houses.iter().enumerate() {
            let now = b.visible().then(|| ((b.x, b.y, b.w, b.h), way_in(b)));
            if self.placed[k] == now {
                continue;
            }
            let id = k as u32 + 1;
            if let Some((was, _)) = self.placed[k] {
                for t in cells(terrain, was) {
                    if self.under[t] == id {
                        self.under[t] = 0;
                    }
                }
            }
            if let Some((now, way)) = now {
                for t in cells(terrain, now) {
                    self.under[t] = id;
                }
                for t in cells(terrain, way) {
                    if self.under[t] == id {
                        self.under[t] = 0;
                    }
                }
            }
            self.placed[k] = now;
        }
    }

    /// Whether nothing slows (the original's way).
    pub fn is_off(&self) -> bool {
        self.under.is_empty()
    }

    /// Whether a house stands on cell `t`, slowing a walker unless `free`.
    pub fn slows(&self, t: i64, free: Free) -> bool {
        !free.everywhere && usize::try_from(t).ok().and_then(|t| self.under.get(t)).is_some_and(|&b| b != 0)
    }
}

/// The way into house `b`: a cell wide, from its door (where those who go in stand, see
/// [`Building::entrance_for`]) down to its front, a cell deep at least.
fn way_in(b: &Building) -> Rect {
    let (x, y) = (b.x + b.w / 2 + b.entrance.0, b.y + b.h / 2 + b.entrance.1);
    let bottom = b.y + b.h;
    let top = y.min(bottom - CELL);
    (x - CELL / 2, top, CELL, bottom - top)
}

/// The cells whose middle lies in `rect` less [`FEET`] (cell `c` spans from `16c - 8` to
/// `16c + 8`, see [`Terrain::index_at`]).
fn cells(terrain: &Terrain, (x, y, w, h): Rect) -> impl Iterator<Item = usize> + '_ {
    let span = |from: i32, len: i32| (from - FEET + CELL - 1).div_euclid(CELL)..(from - FEET + len + CELL - 1).div_euclid(CELL);
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
