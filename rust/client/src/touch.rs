//! Fingers for the touch interface (see `layout::touch`): taps, long presses, drags and pinches,
//! made from macroquad's touches, or from the mouse's left button for trying it on a desktop.
//! Not in the original.

use macroquad::input::{Touch, TouchPhase, is_mouse_button_down, mouse_position, touches};
use macroquad::time::get_time;

/// How far a finger may wander (in CSS pixels) and still tap.
const SLOP: f32 = 12.0;
/// A finger held still this long (in seconds) presses long: no tap on release.
pub const LONG: f64 = 0.5;

struct Press {
    start: (f32, f32),
    time: f64,
    moved: bool,
    /// A second finger came: a pinch, not a tap.
    cancelled: bool,
}

#[derive(Default)]
pub struct Gestures {
    press: Option<Press>,
    /// Where the finger is, or was last, in frame pixels.
    pos: (f32, f32),
    /// The two fingers' distance in the previous frame of a pinch.
    pinch: Option<f32>,
}

/// One frame of fingers (none by default).
#[derive(Clone, Copy)]
pub struct Frame {
    /// Where the finger is, or was last, in frame pixels.
    pub pos: (f32, f32),
    /// A finger is on the screen.
    pub down: bool,
    /// A finger came off after a short, still press.
    pub tap: bool,
    /// A finger has stayed still for [`LONG`] seconds (and is still down).
    pub long: bool,
    /// The finger moved this far this frame (frame pixels), once out of tapping range.
    pub drag: Option<(f32, f32)>,
    /// Two fingers moved apart by this factor this frame.
    pub pinch: f32,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame { pos: (0.0, 0.0), down: false, tap: false, long: false, drag: None, pinch: 1.0 }
    }
}

impl Gestures {
    /// `dpi`: frame pixels per CSS pixel.
    pub fn poll(&mut self, dpi: f32) -> Frame {
        // Touches are in frame pixels; the mouse in CSS pixels.
        let mut points: Vec<Touch> = touches();
        points.sort_by_key(|t| t.id);
        if points.is_empty() && is_mouse_button_down(macroquad::input::MouseButton::Left) {
            let (x, y) = mouse_position();
            points.push(Touch { id: 0, phase: TouchPhase::Moved, position: macroquad::math::vec2(x * dpi, y * dpi) });
        }
        let ended = |t: &Touch| matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
        let active = points.iter().filter(|t| !ended(t)).count();
        let mut frame = Frame::default();

        if active >= 2 {
            let (a, b) = (points[0].position, points[1].position);
            let d = a.distance(b);
            if let Some(prev) = self.pinch.filter(|&p| p > 0.0) {
                frame.pinch = d / prev;
            }
            self.pinch = Some(d);
            if let Some(press) = &mut self.press {
                press.cancelled = true;
            }
        } else {
            self.pinch = None;
        }

        if let Some(first) = points.first() {
            let p = (first.position.x, first.position.y);
            if self.press.is_none() {
                self.pos = p;
            }
            let press = self.press.get_or_insert(Press { start: p, time: get_time(), moved: false, cancelled: false });
            // (once out of tapping range, the drag counts from where the finger came down)
            let from = if press.moved { self.pos } else { press.start };
            if (p.0 - press.start.0).hypot(p.1 - press.start.1) > SLOP * dpi {
                press.moved = true;
            }
            if press.moved && active == 1 {
                frame.drag = Some((p.0 - from.0, p.1 - from.1));
            }
            if points.iter().any(|t| t.phase == TouchPhase::Cancelled) {
                press.cancelled = true;
            }
            self.pos = p;
        }

        frame.pos = self.pos;
        let held = self.press.as_ref().map(|p| get_time() - p.time);
        let still = self.press.as_ref().is_some_and(|p| !p.moved && !p.cancelled);
        if active == 0 {
            if let Some(press) = self.press.take() {
                frame.tap = !press.moved && !press.cancelled && held.unwrap_or(0.0) < LONG;
            }
        } else {
            frame.down = true;
            frame.long = still && held.is_some_and(|h| h >= LONG);
        }
        frame
    }
}
