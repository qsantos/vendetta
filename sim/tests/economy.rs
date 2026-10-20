//! Needs the converted assets in `assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::perso::action;
use sim::rules::Rules;
use sim::world::{Order, World, WorldSetup};
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

fn world(files: &Files, rules: &Rules) -> World {
    World::create(rules, files, &WorldSetup { seed: 42, ..Default::default() }).unwrap()
}

/// The local player, standing still outside its castle; the other characters frozen.
fn player(w: &mut World) -> usize {
    let i = w.player.unwrap();
    for p in &mut w.persos {
        p.ai = false;
    }
    let p = &mut w.persos[i];
    p.set_visible(true);
    i
}

fn run(w: &mut World, rules: &Rules, orders: &[Order]) {
    w.step(rules, orders);
}

/// A free site for a house of type `kind` near the player's castle.
fn site(w: &World, rules: &Rules, i: usize, kind: usize) -> (i32, i32) {
    let (bw, bh) = rules.buildings[kind].size;
    let c = &w.castles[w.persos[i].fief];
    for r in (100..2000).step_by(16) {
        for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r)] {
            let (x, y) = (c.x + dx, c.y + dy);
            if x > 0 && y > 0 && w.obstacle(rules, i, x, y, bw, bh) == 0 {
                return (x, y);
            }
        }
    }
    panic!("no site");
}

#[test]
fn harvest_wood_rate() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let wood = 2usize;
    let node = w.resources.iter().position(|n| rules.terrain_resources[n.kind].resource == wood as i64).unwrap();
    // Stand where `Aller_Ressource` leads, so the harvest starts at once.
    let n = &w.resources[node];
    let p = &mut w.persos[i];
    p.x = (n.x as f64 + 16.0 - p.w as f64 / 2.0).round_ties_even() as i64;
    p.y = (n.y as f64 + 16.0 - p.h as f64 / 2.0).round_ties_even() as i64;
    run(&mut w, &rules, &[Order::Harvest { perso: i, node }]);
    assert_eq!(w.persos[i].action, action::HARVEST);
    let p = &w.persos[i];
    assert_eq!(p.vitality, 1.0);
    // Recolter_Ressources: VitesseExtraction × 1/400 × Vitalite × (level + 20).
    let rate = rules.resources[wood].speed * (1.0 / 400.0 * (p.harvest_level(wood) as f64 + 20.0));
    let start = p.resource(wood);
    assert!((start - rate).abs() < 1e-12, "{start} {rate}");
    for _ in 0..99 {
        run(&mut w, &rules, &[]);
    }
    let got = w.persos[i].resource(wood);
    assert!((got - 100.0 * rate).abs() < 1e-9, "{got} vs {}", 100.0 * rate);
    // Harvesting tires: 0.0025 × (level + 20) Energie per slow tick.
    assert!(w.persos[i].bars[1] < w.persos[i].bonus_max[1] as f64);
}

#[test]
fn house_construction_and_production() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let farm = 0; // Ferme de blé: 5 wood, 500 HP, makes wheat
    let t = &rules.buildings[farm];
    assert_eq!(t.product, 7);
    w.persos[i].set_resource(2, 5.0);
    assert!(w.can_build(&rules, i, farm));
    let (x, y) = site(&w, &rules, i, farm);
    run(&mut w, &rules, &[Order::BuildHouse { perso: i, x, y, kind: farm }]);
    assert_eq!(w.persos[i].resource(2), 0.0, "the cost is paid");
    let m = &w.houses[i];
    assert!(m.visible() && !m.built);
    assert_eq!(m.max_life, 500);
    assert_eq!(m.life, 500.0 * 0.09);
    let mut ticks = 0;
    let mut building = 0;
    while !w.houses[i].built {
        run(&mut w, &rules, &[]);
        if w.persos[i].action == action::BUILD || w.houses[i].built {
            building += 1;
        }
        ticks += 1;
        assert!(ticks < 5000, "never built");
    }
    // Batir_Maison: 3 × Vitalite × (20 + 20) / 20 = 6 HP per tick.
    assert_eq!(building, ((500.0 - 45.0) / 6.0f64).ceil() as i32);
    assert_eq!(w.persos[i].action, action::GO_HOME);
    // Completing a building of the current age starts the next one.
    let f = &w.fiefs[w.persos[i].fief];
    assert!(f.changing_epoch);
    assert_eq!(f.epoch_timer, rules.epochs[1].duration);

    // The owner goes in and produces wheat, into his bag then into the house stock.
    run(&mut w, &rules, &[]);
    assert_eq!(w.persos[i].action, action::WORK_HOME);
    assert!(w.persos[i].in_own_house);
    let rate = rules.resources[7].speed / 400.0 * 20.0;
    let before = w.persos[i].resource(7);
    run(&mut w, &rules, &[]);
    assert!((w.persos[i].resource(7) - before - rate).abs() < 1e-12);
    while w.persos[i].resource(7) < w.persos[i].max_resource(7) {
        run(&mut w, &rules, &[]);
    }
    for _ in 0..10 {
        run(&mut w, &rules, &[]);
    }
    assert!(w.houses[i].stock() > 0.0, "shop stock");
    assert_eq!(w.houses[i].state(&rules), rules.buildings[farm].build_stages - 1);
}

#[test]
fn castle_trade() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let f = w.persos[i].fief;
    w.persos[i].set_resource(2, 5.0);
    run(&mut w, &rules, &[Order::SellResource { perso: i, resource: 2, quantity: 3.5 }]);
    assert_eq!(w.persos[i].money(), 3.0 * 4.0, "3 wood at 4");
    assert_eq!(w.persos[i].resource(2), 2.0);
    // The castle market then moves 0.00125 × citizens from its most abundant good.
    assert!((w.fiefs[f].resources[2] - (3.0 - 0.00125 * w.fiefs[f].citizens as f64)).abs() < 1e-9);
    // Buying costs twice the price; limited by money.
    let stone = 12;
    w.fiefs[f].resources[stone] = 100.0;
    run(&mut w, &rules, &[Order::BuyResource { perso: i, resource: stone, quantity: 5.0 }]);
    assert_eq!(w.persos[i].resource(stone), 1.0, "12 gold buys one stone at 10");
    assert_eq!(w.persos[i].money(), 2.0);
    // Selling the selection at the castle.
    w.persos[i].set_resource(2, 10.0);
    let orders = [
        Order::SelectResource { perso: i, resource: 2, quantity: 4.0 },
        Order::SellSelection { perso: i, fief: f },
    ];
    run(&mut w, &rules, &orders);
    assert_eq!(w.persos[i].resource(2), 6.0);
    assert_eq!(w.persos[i].money(), 2.0 + 16.0);
    assert!(!w.persos[i].holding());
    // A failing command drops the rest of the queue (`Lire_Commandes`).
    let orders = [
        Order::Harvest { perso: i, node: 1_000_000 },
        Order::SellResource { perso: i, resource: 2, quantity: 1.0 },
    ];
    run(&mut w, &rules, &orders);
    assert_eq!(w.persos[i].resource(2), 6.0);
}

#[test]
fn epoch_advance() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let f = 0;
    let life = w.castles[f].max_life;
    w.fiefs[f].change_epoch(&rules, &mut w.castles[f], 0.5);
    assert_eq!(w.castles[f].max_life, life + rules.epochs[1].castle_life_bonus);
    assert_eq!(w.fiefs[f].epoch_timer, (rules.epochs[1].duration as f64 * 0.5) as i64);
    for _ in 1..w.fiefs[f].epoch_timer {
        run(&mut w, &rules, &[]);
    }
    assert_eq!(w.fiefs[f].epoch, 0);
    run(&mut w, &rules, &[]);
    assert_eq!(w.fiefs[f].epoch, 1);
    assert!(!w.fiefs[f].changing_epoch);
    // The castle heals the bonus at 0.1 HP per tick.
    assert!(w.castles[f].life < w.castles[f].max_life as f64);
    assert!(w.castles[f].life > life as f64 + 200.0);
}

#[test]
fn demolition_refund_and_treasure() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let farm = 0;
    w.persos[i].set_resource(2, 5.0);
    let (x, y) = site(&w, &rules, i, farm);
    run(&mut w, &rules, &[Order::BuildHouse { perso: i, x, y, kind: farm }]);
    while !w.houses[i].built {
        run(&mut w, &rules, &[]);
    }
    w.houses[i].money = 50.0;
    // Walk away so the till falls into a treasure.
    let (px, py) = (w.persos[i].x, w.persos[i].y);
    w.persos[i].x = px + 500;
    w.persos[i].y = py + 500;
    w.persos[i].in_own_house = false;
    run(&mut w, &rules, &[Order::Demolish { perso: i }]);
    assert!(!w.houses[i].visible());
    // Half the cost back at full HP: 2.5 wood.
    assert_eq!(w.persos[i].resource(2), 2.5);
    assert_eq!(w.treasures.len(), 1);
    assert_eq!(w.treasures[0].money(), 50.0);
    assert_eq!(w.treasures[0].tier, 0);
    // Pick it up.
    run(&mut w, &rules, &[Order::FindTreasure { perso: i }]);
    assert_eq!(w.persos[i].action, action::GO_TREASURE);
    let mut n = 0;
    while !w.treasures.is_empty() {
        run(&mut w, &rules, &[]);
        n += 1;
        assert!(n < 2000);
    }
    assert_eq!(w.persos[i].money(), 50.0);
    assert_eq!(w.persos[i].action, action::IDLE);
}

#[test]
fn treasures_merge() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    w.persos[i].set_money(300.0);
    w.persos[i].set_resource(2, 10.0);
    run(&mut w, &rules, &[Order::SelectMoney { perso: i, quantity: 120.0 }, Order::DropSelection { perso: i }]);
    assert_eq!(w.treasures.len(), 1);
    assert_eq!(w.treasures[0].tier, 1);
    run(&mut w, &rules, &[Order::SelectResource { perso: i, resource: 2, quantity: 5.0 }, Order::DropSelection { perso: i }]);
    assert_eq!(w.treasures.len(), 1, "dropped on the same spot: merged");
    assert_eq!(w.treasures[0].resource(2), 5.0);
    assert_eq!(w.treasures[0].fortune(), 120.0 + 5.0 * 4.0);
    assert_eq!(w.persos[i].money(), 180.0);
    assert_eq!(w.persos[i].resource(2), 5.0);
}

#[test]
fn emperor_and_eating() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let f = w.persos[i].fief;
    let price = rules.epochs[0].emperor_price as f64;
    w.persos[i].set_money(price + 10.0);
    assert!(w.can_become_emperor(&rules, i));
    run(&mut w, &rules, &[Order::BecomeEmperor { perso: i }]);
    assert_eq!(w.fiefs[f].emperor, i as i64);
    assert!(w.persos[i].emperor);
    assert_eq!(w.persos[i].money(), 10.0);
    let crown = rules.objects.iter().position(|o| o.emperor).unwrap();
    assert!(w.persos[i].object_count(crown) > 0);

    // Eating bread (+10 Energie) fills the recovery pool until 20 - 10 is reached.
    let bread = 8;
    w.persos[i].bars[1] = 5.0;
    w.persos[i].set_resource(bread, 3.0);
    run(&mut w, &rules, &[Order::EatFor { perso: i, bar: 1 }]);
    assert_eq!(w.persos[i].resource(bread), 2.0);
    assert_eq!(w.persos[i].recovery[1], 10.0);
    // The pool drains 0.05 × max per slow tick.
    while w.slow_phase != 0 {
        run(&mut w, &rules, &[]);
    }
    assert!((w.persos[i].recovery[1] - 9.0).abs() < 1e-9, "{}", w.persos[i].recovery[1]);
}

#[test]
fn villagers_run_deterministically() {
    let Some((files, rules)) = load() else { return };
    let state = |w: &World| {
        w.persos.iter().map(|p| (p.x, p.y, p.action, p.money().to_bits(), p.bars.map(f64::to_bits))).collect::<Vec<_>>()
    };
    let mut a = world(&files, &rules);
    let mut b = world(&files, &rules);
    for _ in 0..1500 {
        a.step(&rules, &[]);
        b.step(&rules, &[]);
    }
    assert_eq!(state(&a), state(&b));
    assert_eq!(a.houses.iter().filter(|m| m.visible()).count(), b.houses.iter().filter(|m| m.visible()).count());
}

#[test]
fn villagers_build_and_trade() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    for _ in 0..6000 {
        w.step(&rules, &[]);
    }
    let built = w.houses.iter().filter(|m| m.built).count();
    let money: f64 = w.persos.iter().map(|p| p.money()).sum();
    eprintln!("built {built}, visible {}, money {money}, epochs {:?}", w.houses.iter().filter(|m| m.visible()).count(), w.fiefs.iter().map(|f| f.epoch).collect::<Vec<_>>());
    assert!(built > 0);
}

#[test]
fn crafting_and_selling_an_object() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = player(&mut w);
    let workshop = 9; // Atelier
    let t = &rules.buildings[workshop];
    for (r, &q) in t.price_resources.iter().enumerate() {
        w.persos[i].set_resource(r, q as f64);
    }
    let (x, y) = site(&w, &rules, i, workshop);
    run(&mut w, &rules, &[Order::BuildHouse { perso: i, x, y, kind: workshop }]);
    let max = w.houses[i].max_life as f64;
    w.set_house_life(&rules, i, max);
    while w.persos[i].action != action::AT_HOME {
        run(&mut w, &rules, &[]);
    }
    let o = t.objects[0];
    let obj = &rules.objects[o];
    for (r, &q) in obj.price_resources.iter().enumerate() {
        w.persos[i].set_resource(r, q as f64);
    }
    run(&mut w, &rules, &[Order::Craft { perso: i, slot: 0 }]);
    let mut ticks = 1;
    while w.houses[i].object_stock(0) < 1.0 {
        assert_eq!(w.persos[i].action, action::CRAFT);
        run(&mut w, &rules, &[]);
        ticks += 1;
    }
    // Fabriquer_Objet: 0.1 × Vitalite × skill level (20) per tick, paid at the end.
    let skill = w.persos[i].objects[obj.skill].bonus as f64;
    assert_eq!(ticks, (obj.craft_time as f64 / (0.1 * skill)).ceil() as i32);
    assert!(obj.price_resources.iter().enumerate().all(|(r, &q)| q == 0 || w.persos[i].resource(r) == 0.0));
    assert_eq!(w.persos[i].action, action::AT_HOME);
    assert_eq!(w.houses[i].stock(), 1.0);
    // Take it, then sell it to the castle, which recycles its resources.
    let f = w.persos[i].fief;
    let stock_before: Vec<f64> = w.fiefs[f].resources.clone();
    run(&mut w, &rules, &[Order::TakeObjects { perso: i, slot: 0, count: 1 }]);
    assert_eq!(w.persos[i].object_count(o), 1);
    // `IA_Equiper` characters (the player included) wear a new object when its slot is free.
    let in_bag = |w: &World| w.persos[i].inventory.iter().position(|s| s.is_some_and(|it| it.object == o));
    if in_bag(&w).is_none() {
        let e = w.persos[i].equipped.iter().position(|s| s.is_some_and(|it| it.object == o)).unwrap();
        run(&mut w, &rules, &[Order::StoreEquipped { perso: i, slot: e }]);
    }
    let money = w.persos[i].money();
    let slot = in_bag(&w).unwrap();
    run(&mut w, &rules, &[Order::SellObject { perso: i, object: slot, from_inventory: true }]);
    assert_eq!(w.persos[i].money(), money + obj.price as f64);
    assert_eq!(w.persos[i].object_count(o), 0);
    let gained: f64 = w.fiefs[f].resources.iter().sum::<f64>() - stock_before.iter().sum::<f64>();
    let cost: i64 = obj.price_resources.iter().sum();
    assert!((gained - cost as f64).abs() < 1e-9, "{gained} vs {cost}");
}

/// Holding an inventory object takes it out of the bag; unselecting puts it back.
#[test]
fn hold_and_put_back_an_object() {
    let Some((files, rules)) = load() else { return };
    let mut w = World::create(&rules, &files, &WorldSetup { seed: 3, ..Default::default() }).unwrap();
    let p = w.persos.iter().position(|p| p.player).unwrap();
    let o = rules.buildings.iter().find(|b| b.factory).unwrap().objects[0];
    w.persos[p].add_object(&rules, o);
    let slot = w.persos[p].inventory.iter().position(|s| s.is_some_and(|it| it.object == o)).unwrap();
    w.step(&rules, &[Order::HoldObject { perso: p, slot }]);
    assert_eq!(w.persos[p].selected_object, o as i64);
    assert!(w.persos[p].inventory.iter().all(|s| s.is_none_or(|it| it.object != o)));
    w.step(&rules, &[Order::Unselect { perso: p }]);
    assert_eq!(w.persos[p].selected_object, -1);
    assert!(w.persos[p].inventory.iter().any(|s| s.is_some_and(|it| it.object == o)));
}

/// Two human players of the same fief: one hands the object it holds to the other, each
/// order acting on its own character only.
#[test]
fn a_player_gives_to_another() {
    let Some((files, rules)) = load() else { return };
    let mut w = World::create(&rules, &files, &WorldSetup { seed: 3, players: vec![0, 0], ..Default::default() }).unwrap();
    let humans: Vec<usize> = (0..w.persos.len()).filter(|&i| w.persos[i].player).collect();
    let (a, b) = (humans[0], humans[1]);
    for p in &mut w.persos {
        p.ai = false;
    }
    for &i in &humans {
        w.persos[i].set_visible(true);
    }
    w.persos[b].x = w.persos[a].x + 40;
    w.persos[b].y = w.persos[a].y;
    let o = rules.buildings.iter().find(|b| b.factory).unwrap().objects[0];
    // In the bag or worn (a new object is put on when it can be).
    let count = |w: &World, i: usize| w.persos[i].inventory.iter().chain(&w.persos[i].equipped).flatten().filter(|it| it.object == o).count();
    w.persos[a].add_object(&rules, o);
    let before = count(&w, b);
    let slot = w.persos[a].inventory.iter().position(|s| s.is_some_and(|it| it.object == o)).unwrap();
    w.step(&rules, &[Order::HoldObject { perso: a, slot }]);
    assert_eq!(w.persos[a].selected_object, o as i64);
    assert_eq!(w.persos[b].selected_object, -1, "the other player holds nothing");
    w.step(&rules, &[Order::Give { perso: a, target: b }]);
    for _ in 0..100 {
        if count(&w, b) > before {
            break;
        }
        w.step(&rules, &[]);
    }
    assert_eq!(count(&w, b), before + 1, "the object reached the other player");
    assert_eq!(w.persos[a].selected_object, -1);
    assert_eq!(count(&w, a), 0);
}
