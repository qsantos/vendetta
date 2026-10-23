// scratch: compare where the AI builds
use sim::files::Files;
use sim::landscape::Landscape;
use sim::path::{Chase, Paths};
use sim::rules::Rules;
use sim::world::{Towns, World, WorldSetup};
use std::io::Write;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ticks: u64 = a[1].parse().unwrap();
    let files = Files::from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")).unwrap();
    let rules = Rules::load(&files, "Francais").unwrap();
    for towns in [Towns::Scattered, Towns::Compact] {
        let setup = WorldSetup { seed: a[5].parse().unwrap(), fiefs: (0..4).map(|i| sim::world::FiefSetup { people: 0, team: i as i64 + 1, epoch: 0, citizens: a[4].parse().unwrap() }).collect(), resources: 200, width: a[3].parse().unwrap(), height: a[3].parse().unwrap(), towns, landscape: Landscape::Natural, paths: Paths::Fastest, chase: Chase::Intercept, torus: true, ..Default::default() };
        let mut w = World::create(&rules, &files, &setup).unwrap();
        let t = std::time::Instant::now();
        for t in 0..ticks { w.step(&rules, &[]);  }
        let el = t.elapsed();
        let mut d = Vec::new();
        for (j, m) in w.houses.iter().enumerate() {
            if !m.visible() { continue; }
            let c = &w.castles[w.persos[j].fief];
            d.push(w.terrain.dist(((m.x + m.w / 2) as i64, (m.y + m.h / 2) as i64), ((c.x + c.w / 2) as i64, (c.y + c.h / 2) as i64)) / 16.0);
        }
        d.sort_by(f64::total_cmp);
        let gold: i64 = w.fiefs.iter().map(|f| f.money as i64).sum(); let alive = w.persos.iter().filter(|p| p.vitality > 0.0).count(); println!("epochs {:?}; gold {gold}", w.fiefs.iter().map(|f| f.epoch).collect::<Vec<_>>()); println!("gold {gold}, {alive} alive of {}", w.persos.len());
        println!("{towns:?}: {} houses, distance to castle in cells: median {:.1}, max {:.1}; {:.2?} per tick", d.len(), d[d.len() / 2], d[d.len() - 1], el / ticks as u32);
        let size = w.terrain.width;
        let colour = |c: u8| match c { 0 => [140, 190, 90], 1 => [80, 160, 60], 2 => [150, 120, 80], 3 => [230, 210, 140], 4 => [120, 110, 100], 5 => [200, 180, 150], 11 => [50, 100, 200], 6 => [40, 40, 50], 10 => [250, 250, 255], _ => [255, 0, 255] };
        let mut px: Vec<[u8; 3]> = (0..size * size).map(|k| colour(w.terrain.category()[k])).collect();
        let mut rect = |x: i32, y: i32, ww: i32, hh: i32, col: [u8; 3]| for dy in 0..hh / 16 { for dx in 0..ww / 16 { px[((y / 16 + dy) as usize % size) * size + (x / 16 + dx) as usize % size] = col; } };
        for c in &w.castles { rect(c.x, c.y, c.w, c.h, [255, 0, 0]); }
        for m in w.houses.iter().filter(|m| m.visible()) { rect(m.x, m.y, m.w, m.h, [0, 0, 0]); }
        {
            // crop around castle 0 at half scale, exact rects
            let c = &w.castles[0]; let (cx, cy) = (c.x as i64 - 600, c.y as i64 - 600); let n = 1328usize; let sc = 2i64;
            let m = (n as i64 / sc) as usize; let mut img = vec![[0u8; 3]; m * m];
            for yy in 0..m { for xx in 0..m { let (px, py) = w.terrain.wrap((cx + xx as i64 * sc, cy + yy as i64 * sc)); let k = w.terrain.index_at(px as f64, py as f64) as usize; img[yy * m + xx] = colour(w.terrain.category()[k]); } }
            let mut put = |x: i32, y: i32, ww: i32, hh: i32, col: [u8; 3]| { let dx = w.terrain.dx(cx, x as i64); let dy = w.terrain.dy(cy, y as i64); for yy in 0..hh as i64 / sc { for xx in 0..ww as i64 / sc { let (a, b) = (dx / sc + xx, dy / sc + yy); if a >= 0 && b >= 0 && (a as usize) < m && (b as usize) < m { let edge = xx == 0 || yy == 0 || xx == ww as i64 / sc - 1 || yy == hh as i64 / sc - 1; img[b as usize * m + a as usize] = if edge { [255, 255, 255] } else { col }; } } } };
            for c in &w.castles { put(c.x, c.y, c.w, c.h, [255, 0, 0]); }
            for m in w.houses.iter().filter(|m| m.visible()) { put(m.x, m.y, m.w, m.h, [0, 0, 0]); }
            let mut f = std::fs::File::create(format!("{}/{towns:?}-town.ppm", a[2])).unwrap();
            write!(f, "P6 {m} {m} 255\n").unwrap();
            for p in img { f.write_all(&p).unwrap(); }
        }
        let mut f = std::fs::File::create(format!("{}/{towns:?}.ppm", a[2])).unwrap();
        write!(f, "P6 {size} {size} 255\n").unwrap();
        for p in px { f.write_all(&p).unwrap(); }
    }
}
// (appended) nothing
