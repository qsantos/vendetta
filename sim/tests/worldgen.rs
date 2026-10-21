//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::rules::Rules;
use sim::world::{FiefSetup, World, WorldSetup};
use std::path::Path;

fn load() -> Option<(Files, Rules)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    if !root.exists() {
        eprintln!("no assets, skipping");
        return None;
    }
    let files = Files::from_dir(&root).unwrap();
    let rules = Rules::load(&files, "Francais").unwrap();
    Some((files, rules))
}

fn setup(map: Option<&str>) -> WorldSetup {
    WorldSetup {
        seed: 1234,
        width: 128,
        height: 128,
        world_type: 1,
        fiefs: (0..4).map(|i| FiefSetup { people: i, team: i as i64 + 1, epoch: 0, citizens: 10 }).collect(),
        resources: 50,
        map: map.map(str::to_owned),
        ..Default::default()
    }
}

#[test]
fn random_world() {
    let Some((files, rules)) = load() else { return };
    let w = World::create(&rules, &files, &setup(None)).unwrap();
    assert_eq!(w.castles.len(), 4);
    assert_eq!(w.resources.len(), 51);
    let non_plain = w.terrain.category().iter().filter(|&&c| c != 0).count();
    eprintln!("non-plain cells: {non_plain}, castles: {:?}", w.castles.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>());
    assert!(non_plain > 0);
    // deterministic
    let w2 = World::create(&rules, &files, &setup(None)).unwrap();
    assert_eq!(w.terrain.category(), w2.terrain.category());
}

#[test]
fn all_maps_load() {
    let Some((files, rules)) = load() else { return };
    let mut maps: Vec<String> = files
        .list("Cartes")
        .iter()
        .filter_map(|p| p.strip_suffix("/terrain.png"))
        .map(|p| p.trim_start_matches("cartes/").to_owned())
        .collect();
    maps.sort();
    assert!(!maps.is_empty());
    for map in maps {
        let w = World::create(&rules, &files, &setup(Some(&map))).unwrap();
        eprintln!("{map}: {}x{}, {} castles, {} resources", w.terrain.width, w.terrain.height, w.castles.len(), w.resources.len());
        assert!(!w.castles.is_empty(), "{map}");
    }
}

#[test]
fn params_counts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
    if !root.exists() {
        return;
    }
    let files = Files::from_dir(&root).unwrap();
    use sim::params::{Kind, Params};
    let p = Params::load(&files, "Francais").unwrap();
    let counts: Vec<usize> = [
        Kind::Ressources, Kind::TerrainRessources, Kind::Batiments, Kind::CompetencesObjets,
        Kind::Objets, Kind::Peuples, Kind::Effets, Kind::Races, Kind::Decors,
        Kind::Resistances, Kind::Statuts,
    ]
    .iter()
    .map(|&k| p.list(k).len())
    .collect();
    assert_eq!(counts, [26, 8, 47, 28, 232, 13, 47, 106, 5, 8, 15]);
    let pe = Params::load(&files, "English").unwrap();
    eprintln!("race 1: {:?} / {:?}", p.list(Kind::Races)[1].get_l("Nom", "FR"), pe.list(Kind::Races)[1].get_l("Nom", "EN"));
}

/// The natural land (`Landscape::Natural`), for each world type, on a flat map and a torus:
/// the terrains in about the world type's shares, castles on cleared land far apart, roads,
/// all the resources; the same world twice.
#[test]
fn natural_world() {
    use sim::landscape::Landscape;
    let Some((files, rules)) = load() else { return };
    for world_type in 0..rules.world_types.len() {
        for torus in [false, true] {
            let s = WorldSetup { world_type, torus, landscape: Landscape::Natural, ..setup(None) };
            let w = World::create(&rules, &files, &s).unwrap();
            let cells = &w.terrain.category()[..w.terrain.width * w.terrain.height];
            let share = |t: u8| cells.iter().filter(|&&c| c == t).count() as f64 / cells.len() as f64;
            let name = &rules.world_types[world_type].name;
            eprintln!(
                "{name} (torus {torus}): water {:.2}, mountain {:.2}, desert {:.2}, road {:.2}; castles {:?}",
                share(11),
                share(4),
                share(3),
                share(5),
                w.castles.iter().map(|c| (c.x / 16, c.y / 16)).collect::<Vec<_>>()
            );
            let surface = &rules.world_types[world_type].surface;
            assert_eq!(share(11) > 0.0, surface[11] > 0, "{name}: water");
            assert_eq!(share(4) > 0.0, surface[4] > 0, "{name}: mountains");
            for (i, a) in w.castles.iter().enumerate() {
                assert!(w.buildable(&rules, a.x, a.y, a.w, a.h), "{name}: castle {i} on land");
                for b in &w.castles[i + 1..] {
                    let d = w.terrain.dist((a.x as i64, a.y as i64), (b.x as i64, b.y as i64));
                    assert!(d > 40.0 * 16.0, "{name}: castles {d} px apart");
                }
            }
            // (of the world type's road terrain: earth in the desert)
            let road = rules.world_types[world_type].road;
            assert!(road == 0 || share(road) > 0.0, "{name}: roads");
            assert_eq!(w.resources.len(), 51);
            assert!(w.resources.iter().all(|r| w.buildable(&rules, r.x, r.y, 32, 32)), "{name}: resources on land");
            let again = World::create(&rules, &files, &s).unwrap();
            assert_eq!(w.digest(), again.digest(), "{name}: deterministic");
        }
    }
}
