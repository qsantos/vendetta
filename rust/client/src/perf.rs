//! `#perf`: where the time goes, per second, under the FPS counter: the simulation's ticks,
//! the frame's drawing and input, the autosave, and the time between frames.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    /// `performance.now()` in seconds.
    fn vendetta_now() -> f64;
}

/// Seconds, precise to well under a millisecond (macroquad's `get_time` in the browser is
/// only to the millisecond).
pub fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: a plain call into the page.
    return unsafe { vendetta_now() };
    #[cfg(not(target_arch = "wasm32"))]
    macroquad::prelude::get_time()
}

#[derive(Clone, Copy)]
pub enum Part {
    Tick,
    Frame,
    Save,
    Interval,
}

const NAMES: [&str; 4] = ["tick", "frame", "save", "interval"];

#[derive(Default, Clone, Copy)]
struct Stat {
    total: f64,
    count: u32,
    max: f64,
}

#[derive(Default)]
pub struct Perf {
    current: [Stat; 4],
    /// The last full second, shown.
    shown: [Stat; 4],
    since: f64,
    last_frame: f64,
}

impl Perf {
    /// Times `f` as `part`.
    pub fn time<T>(&mut self, part: Part, f: impl FnOnce() -> T) -> T {
        let start = now();
        let result = f();
        self.add(part, now() - start);
        result
    }

    pub fn add(&mut self, part: Part, seconds: f64) {
        let s = &mut self.current[part as usize];
        s.total += seconds;
        s.count += 1;
        s.max = s.max.max(seconds);
    }

    /// Once per frame: the time since the previous one, and the per-second roll-over.
    pub fn frame(&mut self) {
        let now = now();
        if self.last_frame > 0.0 {
            self.add(Part::Interval, now - self.last_frame);
        }
        self.last_frame = now;
        if now - self.since >= 1.0 {
            self.shown = std::mem::take(&mut self.current);
            self.since = now;
        }
    }

    /// "tick 12.3 ms (max 40.1) ×10" per part that ran in the last second.
    pub fn lines(&self) -> Vec<String> {
        (self.shown.iter().zip(NAMES))
            .filter(|(s, _)| s.count > 0)
            .map(|(s, name)| {
                let avg = s.total / s.count as f64 * 1000.0;
                format!("{name} {avg:.1} ms (max {:.1}) ×{}", s.max * 1000.0, s.count)
            })
            .collect()
    }
}
