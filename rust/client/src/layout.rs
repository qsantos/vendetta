//! Screen size and HUD anchoring. The game draws at the window's size; HUD positions are
//! the original's 1024×768 ones, moved with the nearest screen edge.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

static W: AtomicI32 = AtomicI32::new(1024);
static H: AtomicI32 = AtomicI32::new(768);
static FULL_W: AtomicI32 = AtomicI32::new(1024);
static FULL_H: AtomicI32 = AtomicI32::new(768);
static TOUCH: AtomicBool = AtomicBool::new(false);

/// Smallest logical screen; smaller windows are scaled down to fit it.
pub const MIN_W: f32 = 800.0;
pub const MIN_H: f32 = 600.0;

pub fn screen_w() -> i32 {
    W.load(Ordering::Relaxed)
}

pub fn screen_h() -> i32 {
    H.load(Ordering::Relaxed)
}

/// The interface's size. With the touch interface, the bar of big buttons takes [`BAR`] off
/// one side, and the rest of the interface keeps to the rest of the screen.
pub fn set_screen(w: i32, h: i32) {
    FULL_W.store(w, Ordering::Relaxed);
    FULL_H.store(h, Ordering::Relaxed);
    let (w, h) = match (touch(), bar_vertical()) {
        (false, _) => (w, h),
        (true, true) => (w - BAR, h),
        (true, false) => (w, h - BAR),
    };
    W.store(w, Ordering::Relaxed);
    H.store(h, Ordering::Relaxed);
}

/// The touch interface's bar of big buttons: its thickness.
pub const BAR: i32 = 64;
/// The interface's shorter side with the touch interface, in interface pixels: a window
/// (368 high) and its margins.
pub const TOUCH_SIDE: f32 = 400.0;
/// …but no bigger than this, in CSS pixels per interface pixel (on tablets).
pub const TOUCH_MAX_SCALE: f32 = 1.3;

/// The whole interface's size, the touch interface's bar included.
pub fn full_size() -> (i32, i32) {
    (FULL_W.load(Ordering::Relaxed), FULL_H.load(Ordering::Relaxed))
}

/// The bar runs down the right edge on a wide screen, along the bottom on a tall one.
pub fn bar_vertical() -> bool {
    let (w, h) = full_size();
    w > h
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

/// The touch interface (phones and tablets): big buttons instead of the original's trays and
/// keys, one window at a time. Not in the original, whose interface stays the desktop one.
pub fn touch() -> bool {
    TOUCH.load(Ordering::Relaxed)
}

pub fn set_touch(on: bool) {
    TOUCH.store(on, Ordering::Relaxed);
}
