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
    /// `edges=closed`: the original's map, with edges, instead of one whose opposite edges
    /// join (a torus: going off one side comes back on the other).
    pub closed_edges: bool,
    /// `world=N`: the world type, `Monde_N` of `Terrain.ini` (`TerrainChoisi`); by default
    /// one by chance on the natural land, the original's ("Plaines") on its own.
    pub world: Option<usize>,
    /// `shape=continent`: the overall shape of the natural land (see `sim::landscape::Shape`),
    /// by default one by chance.
    pub shape: Option<sim::landscape::Shape>,
    /// `land=original`: the original's random land (blobs of terrain, castles on a grid),
    /// instead of hills, lakes, rivers and fairly placed castles (see `sim::landscape`).
    pub original_land: bool,
    /// `towns=scattered`: the AI builds anywhere around its castle as in the original, instead
    /// of next to its buildings, a cell apart.
    pub scattered_towns: bool,
    /// `monsters=wait`: monsters with nobody to attack in reach mostly stand about, as in the
    /// original, instead of roaming toward the nearest enemy castle.
    pub waiting_monsters: bool,
    /// `paths=greedy`: the original's way of walking, straight for the destination one cell at
    /// a time, instead of the fastest path.
    pub greedy_paths: bool,
    /// `chase=direct`: the original's way of chasing, straight for where the target is,
    /// instead of heading it off.
    pub direct_chase: bool,
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
    /// `players=1+1+3`: one human player per entry, in that fief (numbered from 1, as listed at
    /// the bottom right); several may share a fief. One in the first fief by default.
    pub players: Vec<usize>,
    /// `names=Alice+Bob`: the human players' names, in the order of `players`, each
    /// percent-encoded (`encodeURIComponent`).
    pub names: Vec<String>,
    /// `characters=…`: the human players' characters, in the order of `players`, each
    /// percent-encoded `race:f|m:file,file…` (the race from 0, in `Parametres_Races.ini`'s
    /// order; the look's files as `Files` lists them); an empty one for none.
    pub characters: Vec<Option<sim::world::CharacterSetup>>,
    /// `me=k`: network play (see `net.rs`), as the k-th player of `players` (from 0).
    pub me: Option<usize>,
    /// `room=CODE`: the network game's room, whose world is saved apart from the solo one.
    pub room: String,
    /// `resume`: back in a network game in progress, from its save if there is one.
    pub resume: bool,
    /// `inventory=Hache+40:3`: objects in the player's bag, by name (`Parametres_Objets.ini`,
    /// any case) or index, with a count after `:`. Only for a new world, not a resumed one.
    pub inventory: Vec<(String, i64)>,
    /// `speed=N`: the game's speed in ticks per second (the original's `Vitesse`, 5 to 30, 10 by
    /// default); also a resumed solo game's.
    pub speed: Option<i64>,
    /// `lang=EN`: the language, by its `Langues.ini` suffix (the page's menu setting comes
    /// first; see `lang.rs`).
    pub lang: Option<String>,
    /// `layout=touch` or `layout=desktop`: the touch interface or the original one, whatever
    /// the screen (by default, the touch one on touch screens).
    pub touch: Option<bool>,
}

/// The game speeds the original's slider allowed (`VitesseMin`, `VitesseMax`).
pub const SPEEDS: (i64, i64) = (5, 30);

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
                "speed" => o.speed = value.parse::<i64>().ok().map(|v| v.clamp(SPEEDS.0, SPEEDS.1)),
                "roads" => o.ring_roads = value.eq_ignore_ascii_case("ring"),
                "edges" => o.closed_edges = value.eq_ignore_ascii_case("closed"),
                "world" => o.world = value.parse::<usize>().ok().filter(|&n| n >= 1),
                "shape" => o.shape = sim::landscape::Shape::from_name(value),
                "land" => o.original_land = value.eq_ignore_ascii_case("original"),
                "towns" => o.scattered_towns = value.eq_ignore_ascii_case("scattered"),
                "monsters" => o.waiting_monsters = value.eq_ignore_ascii_case("wait"),
                "paths" => o.greedy_paths = value.eq_ignore_ascii_case("greedy"),
                "chase" => o.direct_chase = value.eq_ignore_ascii_case("direct"),
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
                "me" => o.me = value.parse().ok(),
                "names" => o.names = value.split('+').map(percent_decode).collect(),
                "characters" => o.characters = value.split('+').map(|c| character(&percent_decode(c))).collect(),
                "room" => o.room = value.to_uppercase(),
                "resume" => o.resume = true,
                "lang" => o.lang = Some(value.to_uppercase()),
                "layout" => {
                    o.touch = match value.to_ascii_lowercase().as_str() {
                        "touch" => Some(true),
                        "desktop" => Some(false),
                        _ => None,
                    }
                }
                "players" => {
                    o.players = value.split('+').filter_map(|f| f.parse::<usize>().ok()).filter(|&f| f >= 1).map(|f| f - 1).collect();
                }
                "inventory" => {
                    for e in value.split('+').filter(|e| !e.is_empty()) {
                        let (object, count) = e.split_once(':').unwrap_or((e, "1"));
                        o.inventory.push((object.to_owned(), count.parse().unwrap_or(1)));
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
        // In network play every copy of the world must be the same: no cheats.
        if o.me.is_some() {
            let me = o.me;
            o = Options { size: o.size, seed: o.seed, speed: o.speed, fiefs: o.fiefs, villagers: o.villagers, ring_roads: o.ring_roads, closed_edges: o.closed_edges, world: o.world, shape: o.shape, original_land: o.original_land, scattered_towns: o.scattered_towns, waiting_monsters: o.waiting_monsters, greedy_paths: o.greedy_paths, direct_chase: o.direct_chase, players: o.players, names: o.names, characters: o.characters, perf: o.perf, touch: o.touch, me, room: o.room, resume: o.resume, ..Default::default() };
            o.new = true;
        }
        o
    }

    /// The options that shape a new world: a saved game made with others is not resumed.
    pub fn setup_key(&self) -> String {
        let (ring, closed) = (if self.ring_roads { " ring" } else { "" }, if self.closed_edges { " closed" } else { "" });
        let greedy = if self.greedy_paths { " greedy" } else { "" };
        let direct = if self.direct_chase { " direct" } else { "" };
        let land = if self.original_land { " original-land" } else { "" };
        let towns = if self.scattered_towns { " scattered" } else { "" };
        let monsters = if self.waiting_monsters { " waiting" } else { "" };
        format!("{:?} {:?} {:?} {:?} {:?} {:?}{ring}{closed}{land}{towns}{monsters}{greedy}{direct} {:?}", self.size, self.world, self.shape, self.seed, self.fiefs, self.villagers, self.players)
    }
}

fn clamp_size(v: usize) -> usize {
    v.clamp(*SIZES.start(), *SIZES.end())
}

/// A character of `characters=`, None when it is not one.
fn character(s: &str) -> Option<sim::world::CharacterSetup> {
    let mut parts = s.splitn(3, ':');
    let race = parts.next()?.parse().ok()?;
    let female = match parts.next()? {
        "f" => true,
        "m" => false,
        _ => return None,
    };
    let look = parts.next()?.split(',').filter(|f| !f.is_empty()).map(str::to_owned).collect();
    Some(sim::world::CharacterSetup { race, female, look })
}

/// Undoes `encodeURIComponent` (`%XX` bytes, UTF-8).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok())) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
