//! Needs the converted assets in `rust/assets` (see `assetconv`); skipped otherwise.

use sim::files::Files;
use sim::items::{Equip, bonus_sup};
use sim::perso::Perso;
use sim::rules::Rules;
use sim::vb::{Rnd, clng};
use sim::world::{Order, World, WorldSetup};
use std::path::Path;

const SWORD: usize = 13; // Epée: one hand, BonusAttaque 10
const SHIELD: usize = 21; // Bouclier: one hand, BonusDefense 12
const HALBERD: usize = 14; // Hallebarde: two hands
const RING_A: usize = 68;
const RING_B: usize = 69;
const HORSE: usize = 169; // Cheval de selle: mount
const WATER: usize = 158; // Eau: the water effect

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

/// A fresh human with an empty bag.
fn human(rules: &Rules) -> Perso {
    let mut p = Perso::new(rules, &mut Rnd::new(1), 0, 1, false);
    p.update_bonus(rules);
    p
}

fn bag_slot(p: &Perso, o: usize) -> usize {
    p.inventory.iter().position(|s| s.is_some_and(|it| it.object == o)).unwrap()
}

fn worn(p: &Perso, s: usize) -> Option<usize> {
    p.equipped[s].map(|it| it.object)
}

#[test]
fn object_rules() {
    let Some((_, rules)) = load() else { return };
    let o = &rules.objects;
    assert_eq!((o[SWORD].category, o[SWORD].bonus[4]), (0, 10));
    assert_eq!(o[HALBERD].category, 1);
    assert_eq!((o[RING_A].category, o[RING_B].category), (6, 6));
    assert!(o[HORSE].mount && o[HORSE].category == 10);
    assert_eq!(o[HORSE].bonus_move, 2.0);
    // Eau: resistance bonuses (Feu, Froid, Electricité), attack resistance Eau, 20 s.
    assert_eq!(o[WATER].attack_resistance, 6);
    assert_eq!(o[WATER].duration, 20);
    assert_eq!(o[WATER].bonus_resistances[2], 0.35f32 as f64);
    // Races: the human rides, statuses become resistances (Mort vivant: Mort 0 → 0.2).
    assert!(rules.races[0].mount);
    assert_eq!(rules.races[22].resistances[5], 0.2f32 as f64);
    assert_eq!(rules.races[0].objects.len(), 3 + 14);
}

/// The attack bonus of a sword scales with its item skill: `Int(10 × level / 20)`.
#[test]
fn weapon_bonus_scales_with_skill() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    let base = p.bonus_max[4];
    p.add_object(&rules, SWORD);
    assert_eq!(p.equip(&rules, Some(bag_slot(&p, SWORD)), false), Equip::Equipped);
    assert_eq!(worn(&p, 0), Some(SWORD));
    assert!(p.inventory.iter().all(Option::is_none));
    let coef = rules.races[0].coef[4] as f32 as f64;
    assert_eq!(p.bonus_max[4], clng((p.max[4] + 10) as f64 * coef));
    assert!(p.bonus_max[4] > base);
    let skill = rules.objects[SWORD].skill;
    p.objects[skill].carac = 33;
    p.update_bonus(&rules);
    let level = p.objects[skill].bonus;
    assert_eq!(bonus_sup(&rules, level, 10.0), 10 * level / 20);
    assert_eq!(p.bonus_max[4], clng((p.max[4] + 10 * level / 20) as f64 * coef));
    // Unequipping removes the bonus.
    p.store(&rules, 0);
    assert_eq!(worn(&p, 0), None);
    assert_eq!(p.bonus_max[4], base);
    assert_eq!(p.object_count(SWORD), 1);
}

/// A second one-hand weapon goes to slot 1; a two-hander empties both hands into the bag.
#[test]
fn hands_and_rings() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    for o in [SWORD, SHIELD, HALBERD, RING_A, RING_B] {
        p.add_object(&rules, o);
    }
    p.equip(&rules, Some(bag_slot(&p, SWORD)), false);
    p.equip(&rules, Some(bag_slot(&p, SHIELD)), false);
    assert_eq!((worn(&p, 0), worn(&p, 1)), (Some(SWORD), Some(SHIELD)));
    p.equip(&rules, Some(bag_slot(&p, HALBERD)), false);
    assert_eq!((worn(&p, 0), worn(&p, 1)), (Some(HALBERD), None));
    assert!(p.two_handed(&rules));
    assert_eq!((p.object_count(SWORD), p.object_count(SHIELD)), (1, 1));
    bag_slot(&p, SWORD);
    bag_slot(&p, SHIELD);
    // A one-hand weapon replaces the two-hander.
    p.equip(&rules, Some(bag_slot(&p, SWORD)), false);
    assert_eq!((worn(&p, 0), worn(&p, 1)), (Some(SWORD), None));
    bag_slot(&p, HALBERD);
    // Rings fill both ring slots.
    p.equip(&rules, Some(bag_slot(&p, RING_A)), false);
    p.equip(&rules, Some(bag_slot(&p, RING_B)), false);
    assert_eq!((worn(&p, 6), worn(&p, 7)), (Some(RING_A), Some(RING_B)));
    // With both slots taken, `right` swaps the second ring.
    p.store(&rules, 7);
    p.equip(&rules, Some(bag_slot(&p, RING_B)), true);
    assert_eq!((worn(&p, 6), worn(&p, 7)), (Some(RING_A), Some(RING_B)));
}

/// Mounts need a race that rides; a mount replaces the movement bonus.
#[test]
fn mounts() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    p.add_object(&rules, HORSE);
    assert_eq!(p.equip(&rules, Some(bag_slot(&p, HORSE)), false), Equip::Equipped);
    let level = p.object_skill_level(&rules, HORSE);
    assert_eq!(p.bonus_move, p.speed + bonus_sup(&rules, level, 2.0));
    assert_eq!((p.overlays.clone(), p.mount), (vec![HORSE], 1));
    let rider = rules.races.iter().position(|r| !r.mount).unwrap();
    let mut q = Perso::new(&rules, &mut Rnd::new(1), 0, 1, false);
    q.change_race(&rules, &mut Rnd::new(1), rider, true);
    q.update_bonus(&rules);
    q.add_object(&rules, HORSE);
    assert_eq!(q.equip(&rules, Some(bag_slot(&q, HORSE)), false), Equip::Refused);
}

/// A race's starting equipment is worn when the race equips automatically.
#[test]
fn race_starting_equipment() {
    let Some((_, rules)) = load() else { return };
    let (race, o) = rules
        .races
        .iter()
        .enumerate()
        .find_map(|(i, r)| {
            let o = *r.equipment.first()?;
            let t = &rules.objects[o];
            (r.ai_equip > 0 && !t.usable && !t.mount && t.category >= 0).then_some((i, o))
        })
        .unwrap();
    let mut p = Perso::new(&rules, &mut Rnd::new(1), 0, 1, false);
    // The first race change sets `IA_Equiper`, the equipment of the second one is worn.
    p.change_race(&rules, &mut Rnd::new(1), race, true);
    p.change_race(&rules, &mut Rnd::new(1), race, false);
    p.update_bonus(&rules);
    assert!(p.equipped.iter().flatten().any(|it| it.object == o), "race {race} object {o}");
}

/// Orders: equip from the bag, store back, take onto the cursor, place, swap and stack.
#[test]
fn equipment_orders() {
    let Some((files, rules)) = load() else { return };
    let mut w = World::create(&rules, &files, &WorldSetup { seed: 5, ..Default::default() }).unwrap();
    let i = w.player.unwrap();
    for p in &mut w.persos {
        p.ai = false;
    }
    let p = &mut w.persos[i];
    p.inventory.iter_mut().for_each(|s| *s = None);
    p.equipped.iter_mut().for_each(|s| *s = None);
    p.add_object(&rules, SWORD);
    p.add_object(&rules, SHIELD);
    let slot = bag_slot(p, SWORD);
    w.step(&rules, &[Order::EquipObject { perso: i, slot, right: false }]);
    assert_eq!(worn(&w.persos[i], 0), Some(SWORD));
    w.step(&rules, &[Order::StoreEquipped { perso: i, slot: 0 }]);
    assert_eq!(worn(&w.persos[i], 0), None);
    // Take the sword onto the cursor, then drop it on the shield's slot: they swap.
    let s = bag_slot(&w.persos[i], SWORD);
    w.step(&rules, &[Order::HoldObject { perso: i, slot: s }]);
    let shield = bag_slot(&w.persos[i], SHIELD);
    w.step(&rules, &[Order::PlaceHeld { perso: i, slot: shield }]);
    assert_eq!(w.persos[i].inventory[shield].map(|it| it.object), Some(SWORD));
    assert_eq!(w.persos[i].selected_object, SHIELD as i64);
    // Equip the held shield, then take it back from the equipment onto the cursor.
    w.step(&rules, &[Order::EquipHeld { perso: i, right: false }]);
    assert_eq!(worn(&w.persos[i], 0), Some(SHIELD));
    assert_eq!(w.persos[i].selected_object, -1);
    w.step(&rules, &[Order::HoldEquipped { perso: i, slot: 0 }]);
    assert_eq!((worn(&w.persos[i], 0), w.persos[i].selected_object), (None, SHIELD as i64));
    // Into an empty slot.
    let empty = w.persos[i].inventory.iter().position(Option::is_none).unwrap();
    w.step(&rules, &[Order::PlaceHeld { perso: i, slot: empty }]);
    assert_eq!(w.persos[i].inventory[empty].map(|it| it.object), Some(SHIELD));
    // Stacking.
    let (o, stack) = rules.objects.iter().enumerate().find(|(_, o)| o.stack > 1).map(|(k, o)| (k, o.stack)).unwrap();
    let p = &mut w.persos[i];
    p.add_object(&rules, o);
    p.selected_object = o as i64;
    let s = bag_slot(p, o);
    w.step(&rules, &[Order::PlaceHeld { perso: i, slot: s }]);
    assert_eq!(w.persos[i].inventory[s].map(|it| it.count), Some(2.min(stack)));
    assert_eq!(w.persos[i].selected_object, -1);
}

/// Temporary effects: bonuses while active, longer when renewed, gone when they expire.
#[test]
fn temporary_effects() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    let fire = p.bonus_resistances[2];
    p.add_effect(&rules, WATER, 20, 20, None);
    assert_eq!(p.bonus_resistances[2], ((fire - 0.35f32 as f64) as f32 as f64).clamp(rules.resistance_min, 3.0));
    p.add_effect(&rules, WATER, 20, 20, None);
    assert_eq!((p.effects.len(), p.effects[0].duration), (1, 20));
    // A second effect expiring the same tick waits one more tick (quirk).
    p.add_effect(&rules, SWORD, 20, -1, None);
    for _ in 0..20 {
        p.tick_effects(&rules);
    }
    assert_eq!((p.effects.len(), p.effects[0].object, p.effects[0].duration), (1, SWORD, 1));
    p.tick_effects(&rules);
    assert!(p.effects.is_empty());
    assert_eq!(p.bonus_resistances[2], fire);
}

/// Characters with `IA_Equiper` wear a new object when its slot is free; a usable one stays in
/// the bag.
#[test]
fn new_objects_are_worn() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    assert!(p.ai_equip > 0);
    p.add_new_object(&rules, SWORD);
    assert_eq!(worn(&p, 0), Some(SWORD));
    p.add_new_object(&rules, HALBERD);
    assert_eq!(worn(&p, 0), Some(SWORD));
    let potion = rules.objects.iter().position(|o| o.usable && o.category == 1).unwrap();
    p.add_new_object(&rules, potion);
    bag_slot(&p, potion);
}

/// Losing Attaque trains the item skill of a weapon with an attack bonus (×0.1).
#[test]
fn weapons_gain_experience() {
    let Some((_, rules)) = load() else { return };
    let mut p = human(&rules);
    p.add_object(&rules, SWORD);
    p.equip(&rules, Some(0), false);
    let skill = rules.objects[SWORD].skill;
    let before = p.objects[skill].xp.exp;
    p.attack = p.bonus_max[4] as f64;
    let (max, bonus) = (p.max[4] as f64, p.bonus_max[4] as f64);
    p.set_attack(&rules, p.attack - 10.0);
    assert!((p.objects[skill].xp.exp - before - 10.0 * max / bonus * 0.1).abs() < 1e-9);
}
