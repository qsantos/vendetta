//! While the player has dragged the map away from its character (not in the original, whose
//! camera always follows it but for the map window's): a button to follow it again, and an
//! arrow at the edge of the screen pointing at it when it is out of sight, which does too.

use super::{Hud, Input, LIGHT_GREY, outline};
use crate::layout::{screen_h, screen_w, touch};
use macroquad::prelude::*;

const YELLOW: Color = Color::new(1.0, 0.85, 0.2, 1.0);
/// The arrow's distance from the screen's edges, and its tap target's side.
const ARROW_MARGIN: f32 = 24.0;
const ARROW_HIT: i32 = 40;

/// The "Suivre" button, in the top middle under the epoch box: (x, y, w, h).
fn button() -> (i32, i32, i32, i32) {
    let (w, h) = if touch() { (88, 40) } else { (120, 24) };
    ((screen_w() - w) / 2, 44, w, h)
}

impl Hud {
    /// The camera is free (`Some`), with the character at this point of the screen (interface
    /// pixels), or follows the character (`None`).
    pub fn set_free_camera(&mut self, character: Option<(i32, i32)>) {
        self.free_camera = character;
    }

    /// The player asked to follow the character again since the last call.
    pub fn take_follow_request(&mut self) -> bool {
        std::mem::take(&mut self.follow_requested)
    }

    /// Where the arrow toward the character goes, and its direction, when it is out of sight.
    fn follow_arrow(&self) -> Option<((f32, f32), (f32, f32))> {
        let (px, py) = self.free_camera?;
        let (w, h) = (screen_w() as f32, screen_h() as f32);
        if px >= 0 && py >= 0 && (px as f32) < w && (py as f32) < h {
            return None;
        }
        let (cx, cy) = (w / 2.0, h / 2.0);
        let (dx, dy) = (px as f32 - cx, py as f32 - cy);
        let len = dx.hypot(dy).max(1.0);
        // where the line from the middle leaves the screen, kept off its edges (and at the top,
        // under the epoch box and the button)
        let (_, by, _, bh) = button();
        let top = (by + bh) as f32 + ARROW_MARGIN;
        let ky = if dy < 0.0 { (cy - top) / -dy } else { (cy - ARROW_MARGIN) / dy.max(1e-3) };
        let k = ((cx - ARROW_MARGIN) / dx.abs().max(1e-3)).min(ky);
        Some(((cx + dx * k, cy + dy * k), (dx / len, dy / len)))
    }

    /// Whether `m` is on the button or the arrow.
    pub(super) fn over_follow(&self, m: (i32, i32)) -> bool {
        if self.free_camera.is_none() {
            return false;
        }
        let (x, y, w, h) = button();
        let on_arrow = self.follow_arrow().is_some_and(|((ax, ay), _)| {
            let half = ARROW_HIT / 2;
            (m.0 - ax as i32).abs() <= half && (m.1 - ay as i32).abs() <= half
        });
        on_arrow || m.0 >= x && m.0 <= x + w && m.1 >= y && m.1 <= y + h
    }

    /// Draws the button and the arrow, and takes a click on them.
    pub(super) fn follow_controls(&mut self, input: &Input) {
        if self.free_camera.is_none() {
            return;
        }
        let (x, y, w, h) = button();
        let over = self.over_follow(input.mouse);
        draw_rectangle(x as f32, y as f32, w as f32, h as f32, Color::new(0.1, 0.1, 0.1, 0.85));
        outline(x, y, x + w, y + h, if over { YELLOW } else { LIGHT_GREY });
        let label = if touch() { crate::lang::tr(["Suivre", "Follow", "Seguir", "追従", "跟随"]) } else { crate::lang::tr(["Suivre (Espace)", "Follow (Space)", "Seguir (Espacio)", "追従 (Space)", "跟随 (空格)"]) };
        let tw = self.font.width(label);
        self.text(x + (w - tw) / 2, y + (h - 16) / 2, label, WHITE);
        if let Some(((ax, ay), (dx, dy))) = self.follow_arrow() {
            let s = if touch() { 16.0 } else { 12.0 };
            let (nx, ny) = (-dy, dx);
            let tip = vec2(ax + dx * s, ay + dy * s);
            let back = |side: f32| vec2(ax - dx * s * 0.6 + nx * s * side, ay - dy * s * 0.6 + ny * s * side);
            draw_triangle(tip, back(0.8), back(-0.8), YELLOW);
            draw_triangle_lines(tip, back(0.8), back(-0.8), 1.5, BLACK);
        }
        if over && input.left_pressed {
            self.follow_requested = true;
        }
    }
}
