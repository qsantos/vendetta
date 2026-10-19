//! Testing options, from the URL's hash (`#god&size=512&seed=7&fiefs=8&villagers=100`) or the command line
//! (`client god size=512x256`): not in the original, which set worlds up in its menus.

use crate::storage;

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
