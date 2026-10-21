//! The terrain grid: one terrain category and one tile shape ("apparence") per 16×16 cell.

use serde::{Deserialize, Serialize};
use crate::vb::clng;

pub const CELL: i32 = 16;

/// `v.rem_euclid(size)`, without dividing when `v` is less than a turn off `[0, size)` (as
/// positions on a torus mostly are): a division took much of the time spent measuring
/// distances.
pub fn wrap_axis(v: i64, size: i64) -> i64 {
    if (0..size).contains(&v) {
        v
    } else if (-size..0).contains(&v) {
        v + size
    } else if (size..2 * size).contains(&v) {
        v - size
    } else {
        v.rem_euclid(size)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terrain {
    pub width: usize,
    pub height: usize,
    /// Terrain type per cell (`Terrain.ini [Terrain_n]`), 0 = plain grass: see
    /// [`Terrain::category`].
    category: Vec<u8>,
    /// Tileset column per cell, chosen from the neighbours by [`Terrain::reshape`].
    pub shape: Vec<u8>,
    /// Not in the original: opposite edges join (a torus), going off one side comes back on
    /// the other. Positions then stay on the map, and two positions are compared by their
    /// shortest difference ([`Terrain::near`]); on a flat map, every helper here does just what
    /// the original does.
    pub torus: bool,
    /// Not in the original: what the path search makes of `category`, when first needed and
    /// again after it changes.
    #[serde(skip)]
    pub(crate) prepared: crate::path::Prepared,
    /// Not in the original: the buildings, which slow walkers with compact towns.
    #[serde(skip)]
    pub(crate) cover: crate::cover::Cover,
}

impl Terrain {
    /// Like the original, the arrays hold `(width+1)*(height+1)+1` cells but rows have a
    /// stride of `width`, so the tail past `width*height` is never drawn.
    pub fn new(width: usize, height: usize) -> Terrain {
        let n = (width + 1) * (height + 1) + 1;
        Terrain {
            width,
            height,
            category: vec![0; n],
            shape: vec![0; n],
            torus: false,
            prepared: Default::default(),
            cover: Default::default(),
        }
    }

    /// [`Terrain::new`], a torus or not.
    pub fn with_torus(width: usize, height: usize, torus: bool) -> Terrain {
        Terrain { torus, ..Terrain::new(width, height) }
    }

    /// The terrain type per cell (`Terrain.ini [Terrain_n]`), 0 = plain grass.
    pub fn category(&self) -> &[u8] {
        &self.category
    }

    /// [`Terrain::category`], to change it.
    pub fn category_mut(&mut self) -> &mut [u8] {
        self.prepared = Default::default();
        &mut self.category
    }

    /// The map's size, in pixels.
    pub fn size_px(&self) -> (i64, i64) {
        (self.width as i64 * CELL as i64, self.height as i64 * CELL as i64)
    }

    /// `to - from` on an axis of `size` pixels: on a torus, the short way round.
    fn delta(&self, from: i64, to: i64, size: i64) -> i64 {
        let d = to - from;
        if !self.torus || size <= 0 {
            return d;
        }
        let d = wrap_axis(d, size);
        if d * 2 >= size { d - size } else { d }
    }

    /// `to - from`, the short way round on a torus.
    pub fn dx(&self, from: i64, to: i64) -> i64 {
        self.delta(from, to, self.size_px().0)
    }

    pub fn dy(&self, from: i64, to: i64) -> i64 {
        self.delta(from, to, self.size_px().1)
    }

    /// `to - from` on an axis of `size` pixels, in fractional pixels: on a torus, the short way
    /// round.
    fn delta_f(&self, from: f64, to: f64, size: i64) -> f64 {
        let d = to - from;
        if !self.torus || size <= 0 {
            return d;
        }
        let s = size as f64;
        let d = d.rem_euclid(s);
        if d * 2.0 >= s { d - s } else { d }
    }

    /// [`Terrain::dx`] in fractional pixels.
    pub fn dxf(&self, from: f64, to: f64) -> f64 {
        self.delta_f(from, to, self.size_px().0)
    }

    pub fn dyf(&self, from: f64, to: f64) -> f64 {
        self.delta_f(from, to, self.size_px().1)
    }

    /// The copy of `b` nearest to `a`: `b` itself on a flat map. Comparing `a` with it, with
    /// the original's formulas, measures across the edges on a torus.
    pub fn near(&self, a: (i64, i64), b: (i64, i64)) -> (i64, i64) {
        (a.0 + self.dx(a.0, b.0), a.1 + self.dy(a.1, b.1))
    }

    /// [`Terrain::near`] for `i32` positions.
    pub fn near32(&self, a: (i32, i32), b: (i32, i32)) -> (i32, i32) {
        let (x, y) = self.near((a.0 as i64, a.1 as i64), (b.0 as i64, b.1 as i64));
        (x as i32, y as i32)
    }

    /// A position brought back onto the map on a torus (unchanged on a flat map).
    pub fn wrap(&self, (x, y): (i64, i64)) -> (i64, i64) {
        if !self.torus {
            return (x, y);
        }
        let (w, h) = self.size_px();
        (wrap_axis(x, w), wrap_axis(y, h))
    }

    /// [`Terrain::wrap`] for `i32` positions.
    pub fn wrap32(&self, (x, y): (i32, i32)) -> (i32, i32) {
        let (x, y) = self.wrap((x as i64, y as i64));
        (x as i32, y as i32)
    }

    /// The distance between two positions, across the edges on a torus.
    pub fn dist(&self, a: (i64, i64), b: (i64, i64)) -> f64 {
        let (dx, dy) = (self.dx(a.0, b.0) as f64, self.dy(a.1, b.1) as f64);
        (dx * dx + dy * dy).sqrt()
    }

    /// [`crate::world::overlaps`] (`Est_Superpose`, quirks included) with `b`'s copy nearest
    /// to `a`.
    pub fn overlaps(&self, a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        let (x, y) = self.near32((a.0, a.1), (b.0, b.1));
        crate::world::overlaps(a, (x, y, b.2, b.3))
    }

    /// `ClsJeuMonde.TerrainIndice`: pixel position to cell index, with banker's rounding (so
    /// logical cells are offset by half a cell from the drawn ones). May be out of range.
    /// On a torus, the cell is taken modulo the map's size.
    pub fn index_at(&self, x: f64, y: f64) -> i64 {
        let (c, r) = (clng(x / CELL as f64), clng(y / CELL as f64));
        if self.torus {
            return wrap_axis(c, self.width as i64) + wrap_axis(r, self.height as i64) * self.width as i64;
        }
        c + r * self.width as i64
    }

    /// The flat index of the cell `d` = (columns, rows) away from cell `i`: on a torus, across
    /// the edges; otherwise on the flat array like the original (left and right wrap to the
    /// adjacent row), `None` past its ends.
    pub fn neighbour(&self, i: isize, d: (isize, isize)) -> Option<usize> {
        let w = self.width as isize;
        if self.torus {
            let h = self.height as isize;
            if i >= w * h {
                return None;
            }
            let (c, r) = ((i % w + d.0).rem_euclid(w), (i / w + d.1).rem_euclid(h));
            return Some((r * w + c) as usize);
        }
        let j = i + d.1 * w + d.0;
        (j >= 0 && j < self.category.len() as isize).then_some(j as usize)
    }

    /// `ClsJeuMonde.Reformer_Terrain`: picks each non-plain cell's shape from which of its
    /// four neighbours are non-plain (any type). Cells with no such neighbour become plain.
    ///
    /// Quirks kept from the original: a single sequential sweep (a cell turned plain
    /// affects later cells), and left/right neighbours computed on the flat array, so they
    /// wrap to the adjacent row at the map edges. On a torus, the neighbours are across the
    /// edges, so that patches continue seamlessly.
    pub fn reshape(&mut self) {
        self.prepared = Default::default();
        let n = self.category.len() as isize;
        for i in 0..n {
            if self.category[i as usize] == 0 {
                continue;
            }
            let set = |d: (isize, isize)| self.neighbour(i, d).is_some_and(|j| self.category[j] > 0);
            let (u, d, l, r) = (set((0, -1)), set((0, 1)), set((-1, 0)), set((1, 0)));
            let shape = match (u, d, l, r) {
                (true, true, true, true) => 1,
                (false, true, false, false) => 2,
                (true, false, false, false) => 3,
                (false, false, false, true) => 4,
                (false, false, true, false) => 5,
                (true, true, false, false) => 6,
                (false, false, true, true) => 7,
                (false, true, true, true) => 8,
                (true, false, true, true) => 9,
                (true, true, false, true) => 10,
                (true, true, true, false) => 11,
                (false, true, false, true) => 12,
                (false, true, true, false) => 13,
                (true, false, false, true) => 14,
                (true, false, true, false) => 15,
                (false, false, false, false) => {
                    self.category[i as usize] = 0;
                    0
                }
            };
            self.shape[i as usize] = shape;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshape_line_and_isolated_cell() {
        let mut t = Terrain::new(5, 3);
        // horizontal road on the middle row, columns 1..=3, and an isolated cell at (0,0)
        t.category[0] = 5;
        for x in 1..=3 {
            t.category[5 + x] = 5;
        }
        t.reshape();
        assert_eq!(t.category[0], 0);
        assert_eq!(&t.shape[6..9], &[4, 7, 5]);
    }
}
