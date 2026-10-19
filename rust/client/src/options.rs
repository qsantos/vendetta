//! Testing options, from the URL's hash (`#god&size=512&seed=7&fiefs=8&villagers=100`) or the command line
//! (`client god size=512x256`): not in the original, which set worlds up in its menus.

use crate::storage;
use std::sync::atomic::{AtomicBool, Ordering};

static TEST: AtomicBool = AtomicBool::new(false);

/// `test`: a still, reproducible scene for the rendering tests (`web-tests/`): the world does
/// not advance, nothing is read from or written to storage, no FPS counter, no music.
pub fn test_mode() -> bool {
    TEST.load(Ordering::Relaxed)
}

#[derive(Default)]
pub struct Options {
    /// `god`: the player cannot die and never runs short (see `World::god`).
    pub god: bool,
    /// `size=N` or `size=WxH`: the map's size in cells (the original's default is 128).
    pub size: Option<(usize, usize)>,
    /// `seed=N`: a reproducible world.
    pub seed: Option<u32>,
    /// `fiefs=N`: number of fiefs, 2 to 200 (the original's setup screen allows 16).
    pub fiefs: Option<usize>,
    /// `villagers=N`: characters per fief, 1 to 2000 (the original allows 250).
    pub villagers: Option<usize>,
    /// `roads=ring`: the original's ring of roads in fief order, instead of joining each
    /// castle to its nearest ones.
    pub ring_roads: bool,
    /// `perf`: show where the time goes (see `perf.rs`).
    pub perf: bool,
    /// `new`: a new world, not the saved one (the page then takes `new` out of the address).
    pub new: bool,
    /// `effects=148:3+112`: give the player these temporary effects (object index, then the
    /// seconds left if not the object's whole duration), to show their icons.
    pub effects: Vec<(usize, Option<i64>)>,
    /// `recharges=40:5+41:25`: give the player these objects, recharging for so many seconds,
    /// bound to the shortcut keys 1, 2…, to show their recharge clocks.
    pub recharges: Vec<(usize, i64)>,
    /// `hold=N`: the player holds object N on the cursor.
    pub hold: Option<usize>,
    /// `emperor`: the player rules its fief.
    pub emperor: bool,
    /// `advance=N`: the player's fief is N % into changing epoch.
    pub advance: Option<f64>,
}

/// Map sizes accepted, in cells per side (a 1024² save is about 3 MB in local storage, which
/// holds about 5).
const SIZES: std::ops::RangeInclusive<usize> = 16..=1024;

impl Options {
    pub fn load() -> Options {
        let mut o = Options::default();
        for item in storage::options_text().split(['&', ',', ' ']).filter(|s| !s.is_empty()) {
            let (key, value) = item.split_once('=').unwrap_or((item, ""));
            match key.to_ascii_lowercase().as_str() {
                "god" => o.god = true,
                "perf" => o.perf = true,
                "new" => o.new = true,
                "emperor" => o.emperor = true,
                "hold" => o.hold = value.parse().ok(),
                "advance" => o.advance = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 100.0)),
                "test" => TEST.store(true, Ordering::Relaxed),
                "size" => {
                    let (w, h) = value.split_once(['x', 'X']).unwrap_or((value, value));
                    if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                        o.size = Some((clamp_size(w), clamp_size(h)));
                    }
                }
                "seed" => o.seed = value.parse().ok(),
                "roads" => o.ring_roads = value.eq_ignore_ascii_case("ring"),
                "fiefs" => o.fiefs = value.parse::<usize>().ok().map(|n| n.clamp(2, 200)),
                "villagers" => o.villagers = value.parse::<usize>().ok().map(|n| n.clamp(1, 2000)),
                "effects" => {
                    for e in value.split('+') {
                        let (object, left) = e.split_once(':').unwrap_or((e, ""));
                        if let Ok(object) = object.parse() {
                            o.effects.push((object, left.parse().ok()));
                        }
                    }
                }
                "recharges" => {
                    for e in value.split('+') {
                        if let Some((Ok(object), Ok(left))) = e.split_once(':').map(|(a, b)| (a.parse(), b.parse())) {
                            o.recharges.push((object, left));
                        }
                    }
                }
                _ => {}
            }
        }
        o
    }

    /// The options that shape a new world: a saved game made with others is not resumed.
    pub fn setup_key(&self) -> String {
        format!("{:?} {:?} {:?} {:?}{}", self.size, self.seed, self.fiefs, self.villagers, if self.ring_roads { " ring" } else { "" })
    }
}

fn clamp_size(v: usize) -> usize {
    v.clamp(*SIZES.start(), *SIZES.end())
}
