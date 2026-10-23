// scratch: where a walker's feet are, at each building's door
use sim::files::Files; use sim::rules::Rules; use sim::world::{World, WorldSetup, Towns}; use std::path::Path;
fn main() {
    let files = Files::from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")).unwrap();
    let rules = Rules::load(&files, "Francais").unwrap();
    let mut w = World::create(&rules, &files, &WorldSetup { seed: 7, towns: Towns::Compact, ..Default::default() }).unwrap();
    let i = w.player.unwrap();
    let (pw, ph) = (w.persos[i].w, w.persos[i].h);
    println!("walker {pw}x{ph}; races {:?}", rules.races.iter().map(|r| r.size).collect::<Vec<_>>());
    let j = (0..w.houses.len()).find(|&j| j != i).unwrap();
    for k in 0..rules.buildings.len() {
        w.houses[j].set_kind(&rules, k);
        let m = &mut w.houses[j]; (m.x, m.y) = (1000, 1000);
        let (_, dy) = m.entrance_for(pw, ph);
        let feet = dy + ph as i64 - 16;
        println!("{:28} h {:3} entrance.y {:3}: feet {:+4} px from the bottom", rules.buildings[k].name, m.h, m.entrance.1, feet - (m.y + m.h) as i64);
    }
}
