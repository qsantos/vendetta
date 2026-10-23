// scratch: median distance of houses to their castle, as sim/tests/towns.rs
use sim::files::Files; use sim::landscape::Landscape; use sim::rules::Rules; use sim::world::{FiefSetup, Towns, World, WorldSetup}; use std::path::Path;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let files = Files::from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")).unwrap();
    let rules = Rules::load(&files, "Francais").unwrap();
    let towns = if a[2] == "c" { Towns::Compact } else { Towns::Scattered };
    let fiefs = (0..4).map(|i| FiefSetup { people: 0, team: i + 1, epoch: 0, citizens: 30 }).collect();
    let setup = WorldSetup { seed: a[1].parse().unwrap(), width: 192, height: 192, resources: 120, fiefs, towns, torus: a.len() > 3, landscape: Landscape::Natural, ..Default::default() };
    let mut w = World::create(&rules, &files, &setup).unwrap();
    for _ in 0..8000 { w.step(&rules, &[]); }
    let mut d: Vec<f64> = (0..w.houses.len()).filter(|&j| w.houses[j].visible()).map(|j| { let (m, c) = (&w.houses[j], &w.castles[w.persos[j].fief]); w.terrain.dist(((m.x + m.w / 2) as i64, (m.y + m.h / 2) as i64), ((c.x + c.w / 2) as i64, (c.y + c.h / 2) as i64)) / 16.0 }).collect();
    d.sort_by(f64::total_cmp);
    println!("{:.1}", d[d.len() / 2]);
}
