//! Simulation benchmark: `cargo run --release -p sim --example bench [preset...] [ticks=N]
//! [fastest] [intercept] [torus]`.
//! Presets: `small` (the default world), `large` (512², 16 fiefs of 150), `huge` (1024², 30
//! fiefs of 500, about where Firefox gave up); all by default. Reports world creation, time
//! per tick (average and worst, after a warm-up for the villagers to leave their castles) and
//! saving and loading. `fastest`, `intercept` and `torus` play as the game does by default:
//! the fastest paths, heading off the targets, on a toroidal map (the simulation's default is
//! the original's).

use sim::files::Files;
use sim::path::{Chase, Paths};
use sim::rules::Rules;
use sim::world::{FiefSetup, Roads, World, WorldSetup};
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
    for arg in std::env::args().skip(1) {
        match (arg.as_str(), arg.strip_prefix("ticks=")) {
            (_, Some(n)) => ticks = n.parse().expect("ticks=N"),
            ("fastest", _) => paths = Paths::Fastest,
            ("intercept", _) => chase = Chase::Intercept,
            ("torus", _) => torus = true,
            _ => presets.push(arg),
        }
    }
    if presets.is_empty() {
        presets = ["small", "large", "huge"].map(String::from).to_vec();
    }
    for name in presets {
        let setup = match name.as_str() {
            "small" => WorldSetup { seed: 42, ..Default::default() },
            "large" => setup(&rules, 512, 16, 150),
            "huge" => setup(&rules, 1024, 30, 500),
            _ => panic!("unknown preset {name}"),
        };
        let setup = WorldSetup { paths, chase, torus, ..setup };
        let t = Instant::now();
        let mut world = World::create(&rules, &files, &setup).unwrap();
        let created = t.elapsed();
        for _ in 0..ticks / 2 {
            world.step(&rules, &[]);
        }
        let (mut total, mut worst) = (Duration::ZERO, Duration::ZERO);
        for _ in 0..ticks {
            let t = Instant::now();
            world.step(&rules, &[]);
            let d = t.elapsed();
            total += d;
            worst = worst.max(d);
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
