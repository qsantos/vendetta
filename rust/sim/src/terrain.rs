//! The terrain grid: one terrain category and one tile shape ("apparence") per 16×16 cell.

use crate::vb::clng;

pub const CELL: i32 = 16;

#[derive(Debug, Clone)]
pub struct Terrain {
    pub width: usize,
    pub height: usize,
    /// Terrain type per cell (`Terrain.ini [Terrain_n]`), 0 = plain grass.
    pub category: Vec<u8>,
    /// Tileset column per cell, chosen from the neighbours by [`Terrain::reshape`].
    pub shape: Vec<u8>,
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
        }
    }

    /// `ClsJeuMonde.TerrainIndice`: pixel position to cell index, with banker's rounding (so
    /// logical cells are offset by half a cell from the drawn ones). May be out of range.
    pub fn index_at(&self, x: f64, y: f64) -> i64 {
        clng(x / CELL as f64) + clng(y / CELL as f64) * self.width as i64
    }

    /// `ClsJeuMonde.Reformer_Terrain`: picks each non-plain cell's shape from which of its
    /// four neighbours are non-plain (any type). Cells with no such neighbour become plain.
    ///
    /// Quirks kept from the original: a single sequential sweep (a cell turned plain
    /// affects later cells), and left/right neighbours computed on the flat array, so they
    /// wrap to the adjacent row at the map edges.
    pub fn reshape(&mut self) {
        let n = self.category.len() as isize;
        let w = self.width as isize;
        for i in 0..n {
            if self.category[i as usize] == 0 {
                continue;
            }
            let set = |j: isize| j >= 0 && j < n && self.category[j as usize] > 0;
            let (u, d, l, r) = (set(i - w), set(i + w), set(i - 1), set(i + 1));
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
