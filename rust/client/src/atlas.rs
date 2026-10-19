//! Packing many small images into a few big textures, so that drawing them switches textures
//! rarely and batches well (WebGL calls are expensive, especially in Firefox).

/// Places rectangles in a `size`×`size` area, row by row, a pixel apart.
pub struct Shelf {
    size: u16,
    x: u16,
    y: u16,
    /// Height of the current row.
    row: u16,
}

impl Shelf {
    pub fn new(size: u16) -> Shelf {
        Shelf { size, x: 0, y: 0, row: 0 }
    }

    /// Room for a `w`×`h` rectangle, or None when the area is full.
    pub fn place(&mut self, w: u16, h: u16) -> Option<(u16, u16)> {
        if w + 1 > self.size {
            return None;
        }
        if self.x + w + 1 > self.size {
            (self.x, self.y, self.row) = (0, self.y + self.row + 1, 0);
        }
        if self.y + h + 1 > self.size {
            return None;
        }
        let at = (self.x, self.y);
        self.x += w + 1;
        self.row = self.row.max(h);
        Some(at)
    }
}
