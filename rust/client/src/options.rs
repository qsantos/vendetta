//! Testing options, from the URL's hash (`#god&size=512&seed=7`) or the command line
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
                _ => {}
            }
        }
        o
    }

    /// The options that shape a new world: a saved game made with others is not resumed.
    pub fn setup_key(&self) -> String {
        format!("{:?} {:?}", self.size, self.seed)
    }
}

fn clamp_size(v: usize) -> usize {
    v.clamp(*SIZES.start(), *SIZES.end())
}
