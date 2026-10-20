//! Grids of the persos, houses and resource nodes by position, so that `obstacle_at` finds the
//! first one overlapping a box without scanning them all, with exactly the same result.
//!
//! The grids are rebuilt at the start of each tick. During the tick, whatever moves farther than
//! the grids allow for, or is created, is checked one by one: walking reports persos that drifted
//! more than `DRIFT` from their cell (`World::move_step`), teleports and reused slots report
//! themselves, houses report when built somewhere (`create_house`), and items appended after the
//! rebuild are past the grid's length. Debug builds compare every answer with a full scan.

/// Side of a cell, in pixels.
const CELL: i32 = 64;
/// How far a perso may walk from where its grid has it before being checked one by one.
pub(crate) const DRIFT: i64 = 32;

/// Items by the cell of their top-left corner, as a counting sort: the items of cell `c` are
/// `items[start[c]..start[c + 1]]`, in index order.
#[derive(Default)]
pub(crate) struct Grid {
    cols: i32,
    rows: i32,
    start: Vec<u32>,
    items: Vec<u32>,
    /// Where each item was when indexed.
    pub(crate) at: Vec<(i32, i32)>,
    /// Items indexed; later ones are checked one by one.
    len: usize,
    /// Items that moved since, checked one by one.
    pub(crate) moved: Vec<usize>,
    /// On a torus, the map's size in pixels: a box crossing an edge continues on the other side.
    wrap: Option<(i32, i32)>,
}

impl Grid {
    /// Indexes items at `positions` on a map of `width`×`height` pixels (a torus if `torus`).
    pub(crate) fn build(&mut self, positions: impl Iterator<Item = (i32, i32)>, width: i32, height: i32, torus: bool) {
        self.wrap = torus.then_some((width, height));
        self.at.clear();
        self.at.extend(positions);
        self.cols = width / CELL + 1;
        self.rows = height / CELL + 1;
        let cells = (self.cols * self.rows) as usize;
        self.start.clear();
        self.start.resize(cells + 1, 0);
        for k in 0..self.at.len() {
            let c = self.cell(self.at[k].0, self.at[k].1);
            self.start[c + 1] += 1;
        }
        for c in 0..cells {
            self.start[c + 1] += self.start[c];
        }
        let mut next = self.start.clone();
        self.items.clear();
        self.items.resize(self.at.len(), 0);
        for (i, &(x, y)) in self.at.iter().enumerate() {
            let c = self.cell(x, y);
            self.items[next[c] as usize] = i as u32;
            next[c] += 1;
        }
        self.len = self.at.len();
        self.moved.clear();
    }

    /// Item `i` moved to (x, y): checked one by one from now on if out of `drift` of its cell.
    pub(crate) fn moved_to(&mut self, i: usize, x: i64, y: i64, drift: i64) {
        let delta = |from: i64, to: i64, size: Option<i32>| match size {
            Some(s) => (to - from).rem_euclid(s as i64).min((from - to).rem_euclid(s as i64)),
            None => (to - from).abs(),
        };
        if let Some(&(ax, ay)) = self.at.get(i).filter(|_| i < self.len)
            && (delta(ax as i64, x, self.wrap.map(|w| w.0)) > drift || delta(ay as i64, y, self.wrap.map(|w| w.1)) > drift)
        {
            self.moved.push(i);
        }
    }

    fn cell(&self, x: i32, y: i32) -> usize {
        let (cx, cy) = ((x / CELL).clamp(0, self.cols - 1), (y / CELL).clamp(0, self.rows - 1));
        (cy * self.cols + cx) as usize
    }

    /// The lowest index below `n` satisfying `matches` among the items indexed with their
    /// top-left corner in `x0..=x1`×`y0..=y1`, the items that moved and those not indexed yet.
    /// On a torus, the parts of the box past an edge are looked for on the other side.
    pub(crate) fn first(&self, n: usize, (x0, y0, x1, y1): (i32, i32, i32, i32), matches: impl Fn(usize) -> bool) -> Option<usize> {
        let mut best: Option<usize> = None;
        let mut consider = |j: usize| {
            if j < n && best.is_none_or(|b| j < b) && matches(j) {
                best = Some(j);
            }
        };
        if self.len > 0 {
            // the box, and its copies one map away when it sticks out of the map on a torus
            let shifts = |lo: i32, hi: i32, size: Option<i32>| -> Vec<i32> {
                match size {
                    Some(s) => [0, -s, s].into_iter().filter(|&d| d == 0 || (d < 0 && hi >= s) || (d > 0 && lo < 0)).collect(),
                    None => vec![0],
                }
            };
            for dy in shifts(y0, y1, self.wrap.map(|w| w.1)) {
                for dx in shifts(x0, x1, self.wrap.map(|w| w.0)) {
                    let (x0, y0, x1, y1) = (x0 + dx, y0 + dy, x1 + dx, y1 + dy);
                    let (cx0, cy0) = ((x0 / CELL).clamp(0, self.cols - 1), (y0 / CELL).clamp(0, self.rows - 1));
                    let (cx1, cy1) = ((x1 / CELL).clamp(0, self.cols - 1), (y1 / CELL).clamp(0, self.rows - 1));
                    for cy in cy0..=cy1 {
                        let row = (cy * self.cols) as usize;
                        let (from, to) = (self.start[row + cx0 as usize] as usize, self.start[row + cx1 as usize + 1] as usize);
                        for &j in &self.items[from..to] {
                            let (x, y) = self.at[j as usize];
                            if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                                consider(j as usize);
                            }
                        }
                    }
                }
            }
        }
        for &j in &self.moved {
            consider(j);
        }
        for j in self.len..n {
            consider(j);
        }
        best
    }
}

/// The grids `obstacle_at` uses (not saved; empty until the first tick, then everything is
/// checked one by one).
#[derive(Default)]
pub(crate) struct Spatial {
    pub(crate) persos: Grid,
    pub(crate) houses: Grid,
    pub(crate) resources: Grid,
    /// The largest side of any perso, house and resource node, which bounds how far from a box
    /// the corner of something overlapping it can be.
    pub(crate) perso_reach: i32,
    pub(crate) house_reach: i32,
}
