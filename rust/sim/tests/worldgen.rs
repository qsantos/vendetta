//! Needs the converted assets in `rust/assets` (see `assetconv`); skipped otherwise.

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
    let non_plain = w.terrain.category.iter().filter(|&&c| c != 0).count();
    eprintln!("non-plain cells: {non_plain}, castles: {:?}", w.castles.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>());
    assert!(non_plain > 0);
    // deterministic
    let w2 = World::create(&rules, &files, &setup(None)).unwrap();
    assert_eq!(w.terrain.category, w2.terrain.category);
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
