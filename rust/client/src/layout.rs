//! Screen size and HUD anchoring. The game draws at the window's size; HUD positions are
//! the original's 1024×768 ones, moved with the nearest screen edge.

use std::sync::atomic::{AtomicI32, Ordering};

static W: AtomicI32 = AtomicI32::new(1024);
static H: AtomicI32 = AtomicI32::new(768);

/// Smallest logical screen; smaller windows are scaled down to fit it.
pub const MIN_W: f32 = 800.0;
pub const MIN_H: f32 = 600.0;

pub fn screen_w() -> i32 {
    W.load(Ordering::Relaxed)
}

pub fn screen_h() -> i32 {
    H.load(Ordering::Relaxed)
}

pub fn set_screen(w: i32, h: i32) {
    W.store(w, Ordering::Relaxed);
    H.store(h, Ordering::Relaxed);
}

/// A 1024-wide HUD x: elements in the right half stick to the right edge.
pub fn ax(x: i32) -> i32 {
    if x >= 512 { x + screen_w() - 1024 } else { x }
}

/// A 768-high HUD y: elements in the bottom half stick to the bottom edge.
pub fn ay(y: i32) -> i32 {
    if y >= 384 { y + screen_h() - 768 } else { y }
}

/// A 1024-wide HUD x for centred elements (the epoch box).
pub fn cx(x: i32) -> i32 {
    x + (screen_w() - 1024) / 2
}
