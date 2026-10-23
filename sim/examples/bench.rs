//! Simulation benchmark: `cargo run --release -p sim --example bench [preset...] [ticks=N]
//! [fastest] [intercept] [torus] [compact] [natural] [default] [size=N] [fiefs=N] [villagers=N]
//! [every=N]`.
//! Presets: `small` (the default world), `large` (512², 16 fiefs of 150), `huge` (1024², 30
//! fiefs of 500, about where Firefox gave up); all by default. Reports world creation, time
//! per tick (average and worst, after a warm-up for the villagers to leave their castles) and
//! saving and loading. `fastest`, `intercept` and `torus` play as the game does by default:
//! the fastest paths, heading off the targets, on a toroidal map (the simulation's default is
//! the original's); `compact` builds compact towns, `natural` makes natural land; `default` all
//! five, as the game's menu does by default. `size`, `fiefs` and `villagers` change a preset's
//! world, as the URL options do; `every=N` also reports the time per tick of each N ticks, to
//! see it change as the game goes on.

use sim::files::Files;
use sim::landscape::Landscape;
use sim::path::{Chase, Paths};
use sim::rules::Rules;
use sim::world::{FiefSetup, Roads, Towns, World, WorldSetup};
use std::path::Path;
use std::time::{Duration, Instant};

fn setup(rules: &Rules, size: usize, fiefs: usize, citizens: usize) -> WorldSetup {
    let d = WorldSetup::default();
    WorldSetup {
        seed: 42,
        width: size,
        height: size,
        resources: d.resources * size * size / (d.width * d.height),
        fiefs: (0..fiefs).map(|i| FiefSetup { people: i % rules.peoples.len(), team: i as i64 + 1, epoch: 0, citizens }).collect(),
        roads: Roads::Nearest,
        ..d
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let files = Files::from_dir(&root).expect("converted assets in assets/");
    let rules = Rules::load(&files, "Francais").unwrap();
    let mut ticks = 1000;
    let mut presets = Vec::new();
    let (mut paths, mut chase, mut torus) = (Paths::Greedy, Chase::Direct, false);
    let mut towns = Towns::Scattered;
    let mut landscape = Landscape::Original;
    let (mut size, mut fiefs, mut villagers, mut every) = (None, None, None, None);
    for arg in std::env::args().skip(1) {
        let number = |name: &str| arg.strip_prefix(name).and_then(|n| n.strip_prefix('=')).map(|n| n.parse::<usize>().expect("N"));
        if let Some(n) = number("ticks") {
            ticks = n;
        } else if let Some(n) = number("size") {
            size = Some(n);
        } else if let Some(n) = number("fiefs") {
            fiefs = Some(n);
        } else if let Some(n) = number("villagers") {
            villagers = Some(n);
        } else if let Some(n) = number("every") {
            every = Some(n);
        } else {
            match arg.as_str() {
                "fastest" => paths = Paths::Fastest,
                "intercept" => chase = Chase::Intercept,
                "torus" => torus = true,
                "compact" => towns = Towns::Compact,
                "natural" => landscape = Landscape::Natural,
                "default" => (paths, chase, torus, towns, landscape) = (Paths::Fastest, Chase::Intercept, true, Towns::Compact, Landscape::Natural),
                _ => presets.push(arg),
            }
        }
    }
    if presets.is_empty() {
        presets = ["small", "large", "huge"].map(String::from).to_vec();
    }
    for name in presets {
        let (s, f, v) = match name.as_str() {
            "small" => {
                let d = WorldSetup::default();
                (d.width, d.fiefs.len(), d.fiefs[0].citizens)
            }
            "large" => (512, 16, 150),
            "huge" => (1024, 30, 500),
            _ => panic!("unknown preset {name}"),
        };
        let setup = if name == "small" && (size, fiefs, villagers) == (None, None, None) {
            WorldSetup { seed: 42, ..Default::default() }
        } else {
            setup(&rules, size.unwrap_or(s), fiefs.unwrap_or(f), villagers.unwrap_or(v))
        };
        let setup = WorldSetup { paths, chase, torus, towns, landscape, ..setup };
        let t = Instant::now();
        let mut world = World::create(&rules, &files, &setup).unwrap();
        let created = t.elapsed();
        let report = |tick: usize, window: Duration, world: &World| {
            if let Some(n) = every.filter(|n| tick.is_multiple_of(*n)) {
                let alive = world.persos.iter().filter(|p| p.vitality > 0.0).count();
                println!("  tick {tick}: {:.2?} per tick, {} characters ({alive} alive)", window / n as u32, world.persos.len());
            }
        };
        let mut window = Duration::ZERO;
        for tick in 1..=ticks / 2 {
            let t = Instant::now();
            world.step(&rules, &[]);
            window += t.elapsed();
            report(tick, window, &world);
            if every.is_some_and(|n| tick.is_multiple_of(n)) {
                window = Duration::ZERO;
            }
        }
        let (mut total, mut worst) = (Duration::ZERO, Duration::ZERO);
        for tick in ticks / 2 + 1..=ticks / 2 + ticks {
            let t = Instant::now();
            world.step(&rules, &[]);
            let d = t.elapsed();
            total += d;
            window += d;
            worst = worst.max(d);
            report(tick, window, &world);
            if every.is_some_and(|n| tick.is_multiple_of(n)) {
                window = Duration::ZERO;
            }
        }
        let t = Instant::now();
        let bytes = world.save();
        let saved = t.elapsed();
        let t = Instant::now();
        World::load(&rules, &bytes).unwrap();
        let loaded = t.elapsed();
        println!(
            "{name}: {} characters, created in {created:.0?}; tick {:.2?} average, {worst:.2?} worst; save {:.1} MB in {saved:.0?}, load {loaded:.0?}",
            world.persos.len(),
            total / ticks as u32,
            bytes.len() as f64 / 1e6,
        );
    }
}
