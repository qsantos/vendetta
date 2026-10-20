//! VB6 numeric semantics that affect gameplay.

use serde::{Deserialize, Serialize};

/// `CLng`/`CInt`/implicit Double→Long conversion: round half to even.
pub fn clng(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// VB6's `Rnd`: a 24-bit linear congruential generator, `x = (x * 0x43FD43FD + 0xC39EC3)
/// mod 2^24`, returning `x / 2^24` in `[0, 1)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rnd {
    state: u32,
}

impl Rnd {
    /// VB6's initial state before any `Randomize`.
    pub const DEFAULT_SEED: u32 = 0x50000;

    pub fn new(seed: u32) -> Rnd {
        Rnd { state: seed & 0xFF_FFFF }
    }

    pub fn next(&mut self) -> f64 {
        self.state = (self.state.wrapping_mul(0x43FD43FD).wrapping_add(0xC39EC3)) & 0xFF_FFFF;
        self.state as f64 / 16_777_216.0
    }

    /// `Int(Rnd * n)`
    pub fn below(&mut self, n: i64) -> i64 {
        (self.next() * n as f64).floor() as i64
    }
}

impl Default for Rnd {
    fn default() -> Rnd {
        Rnd::new(Rnd::DEFAULT_SEED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bankers_rounding() {
        assert_eq!(clng(0.5), 0);
        assert_eq!(clng(1.5), 2);
        assert_eq!(clng(2.5), 2);
        assert_eq!(clng(-0.5), 0);
        assert_eq!(clng(2.6), 3);
    }

    #[test]
    fn rnd_first_values_match_vb6() {
        // VB6 without Randomize: Rnd = 0.7055475, 0.533424, 0.5795186
        let mut r = Rnd::default();
        let v: Vec<f64> = (0..3).map(|_| r.next()).collect();
        assert!((v[0] - 0.7055475).abs() < 1e-6, "{v:?}");
        assert!((v[1] - 0.533424).abs() < 1e-6, "{v:?}");
        assert!((v[2] - 0.5795186).abs() < 1e-6, "{v:?}");
    }
}
