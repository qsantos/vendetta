//! Needs the converted assets in `rust/assets` (see `assetconv`); skipped otherwise.

use sim::event::{Event, NumberKind};
use sim::files::Files;
use sim::rules::Rules;
use sim::vb::{Rnd, clng};
use sim::world::{FiefSetup, Order, World, WorldSetup};
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

/// Two human fiefs of two characters, nobody driven by the AI.
fn world(files: &Files, rules: &Rules) -> World {
    let setup = WorldSetup {
        seed: 7,
        width: 64,
        height: 64,
        fiefs: (0..2).map(|i| FiefSetup { people: 0, team: i + 1, epoch: 0, citizens: 2 }).collect(),
        resources: 5,
        ..Default::default()
    };
    let mut w = World::create(rules, files, &setup).unwrap();
    for p in &mut w.persos {
        p.ai = false;
    }
    w
}

/// A human of fief `f`, standing at (x, y) outside, unarmed.
fn fighter(w: &mut World, rules: &Rules, f: usize, x: i64, y: i64) -> usize {
    let i = w.persos.iter().position(|p| p.fief == f).unwrap();
    let p = &mut w.persos[i];
    p.equipped.iter_mut().for_each(|s| *s = None);
    p.inventory.iter_mut().for_each(|s| *s = None);
    p.effects.clear();
    p.update_bonus(rules);
    p.set_visible(true);
    (p.x, p.y) = (x, y);
    p.dest = (x, y);
    i
}

/// A generator whose next draws are `want` (true: above `threshold`).
fn seed_for(threshold: f64, want: &[bool]) -> Rnd {
    (1..).map(Rnd::new).find(|r| {
        let mut r = r.clone();
        want.iter().all(|&w| (r.next() > threshold) == w)
    }).unwrap()
}

/// `(Attaque - Defense) * CoefDegatPerso * Resistance`; the attacker spends what the kill
/// needed, `Vie / CoefDegatPerso + Defense - 1`, capped by its bar.
#[test]
fn a_blow_does_damage_and_costs_attack() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let d = fighter(&mut w, &rules, 1, 110, 100);
    w.persos[a].attack = 20.0;
    w.persos[d].defense = 8.0;
    w.persos[d].bars[0] = 15.0;
    // Neither a critical failure nor a critical hit.
    w.rng = seed_for(rules.combat.failure_chance, &[true, true]);
    w.hit_perso(&rules, a, d, true);
    let res = w.persos[d].resistance(w.persos[a].bonus_attack_resistance);
    let damage = (20.0 - 8.0) * rules.combat.damage_coef * res;
    assert_eq!(w.persos[d].bars[0], 15.0 - damage);
    assert_eq!(w.persos[d].defense, w.persos[d].bonus_armour as f64, "the defence is broken");
    let needed: f64 = 15.0 / rules.combat.damage_coef + 8.0 - 1.0;
    assert_eq!(w.persos[a].attack, (20.0 - needed).max(0.0));
    let (x, y) = (clng(w.persos[d].x as f64 + w.persos[d].w as f64 / 2.0), w.persos[d].y);
    assert!(w.events.contains(&Event::Number { x, y, value: clng(damage), kind: NumberKind::Damage, critical: false }));
    // The idle, human-controlled defender fights back.
    assert_eq!((w.persos[d].action, w.persos[d].perso_target), (300, a));
}

/// A critical hit adds the critical skill ratio to the attack; a critical failure costs 20.
#[test]
fn criticals() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let d = fighter(&mut w, &rules, 1, 110, 100);
    w.persos[a].attack = 20.0;
    w.persos[d].defense = 8.0;
    w.persos[d].bars[0] = 15.0;
    w.rng = seed_for(rules.combat.critical_chance, &[true, false]);
    w.hit_perso(&rules, a, d, false);
    let level = w.persos[a].special[2].bonus as f64 / rules.start_special as f64;
    let res = w.persos[d].resistance(w.persos[a].bonus_attack_resistance);
    let damage = ((20.0 * (level + rules.combat.critical_coef) - 8.0) * rules.combat.damage_coef * res).min(15.0);
    assert_eq!(w.persos[d].bars[0].max(0.0), 15.0 - damage);

    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let d = fighter(&mut w, &rules, 1, 110, 100);
    w.persos[a].attack = 30.0;
    let life = w.persos[d].bars[0];
    w.rng = seed_for(rules.combat.failure_chance, &[false]);
    w.hit_perso(&rules, a, d, true);
    assert_eq!(w.persos[d].bars[0], life, "missed");
    assert_eq!(w.persos[a].attack, 30.0 - rules.combat.failure_loss);
}

/// An attack order: the attacker walks to its target and hits it once in range, then waits
/// for its attack bar to fill again.
#[test]
fn attacking_walks_then_hits() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let d = fighter(&mut w, &rules, 1, 200, 100);
    let life = w.persos[d].bars[0];
    w.step(&rules, &[Order::AttackPerso { perso: a, target: d }]);
    assert_eq!(w.persos[a].action, 300);
    let mut hit = None;
    for t in 0..200 {
        w.step(&rules, &[]);
        if w.persos[d].bars[0] < life {
            hit = Some(t);
            break;
        }
    }
    assert!(hit.is_some(), "never hit");
    let (p, q) = (&w.persos[a], &w.persos[d]);
    assert!((p.x - q.x).abs() <= p.w as i64 + 1, "in range");
    assert!(!p.force_attack);
}

/// A killing blow: the killer, touching its victim, takes a quarter of its money and
/// resources; the kill is counted and a tomb is left. Out of reach, the loot is a treasure.
#[test]
fn a_kill_is_looted() {
    let Some((files, rules)) = load() else { return };
    for touching in [true, false] {
        let mut w = world(&files, &rules);
        let a = fighter(&mut w, &rules, 0, 100, 100);
        let d = fighter(&mut w, &rules, 1, if touching { 110 } else { 300 }, 100);
        w.persos[a].set_money(0.0);
        w.persos[d].set_money(100.0);
        w.persos[a].attack = 200.0;
        w.persos[d].bars[0] = 1.0;
        let (treasures, decors) = (w.treasures.len(), w.decors.len());
        w.rng = seed_for(rules.combat.failure_chance, &[true, true]);
        w.hit_perso(&rules, a, d, true);
        assert!(!w.persos[d].alive());
        assert_eq!(w.persos[d].money(), 75.0);
        assert_eq!(w.persos[d].deaths, 1);
        assert_eq!(w.persos[a].frags[0], 1);
        assert_eq!(w.persos[a].bestiary[w.persos[d].race], 1);
        if touching {
            assert_eq!(w.persos[a].money(), 25.0);
        } else {
            assert_eq!(w.persos[a].money(), 0.0);
            assert_eq!(w.treasures.len(), treasures + 1);
            assert!(w.treasures.last().unwrap().money() >= 25.0);
        }
        let tomb = w.decors.last().unwrap();
        assert_eq!(w.decors.len(), decors + 1);
        assert!(tomb.name.starts_with(&format!("&{} (", w.persos[d].name)), "{}", tomb.name);
        assert!(tomb.name.ends_with(&w.persos[a].name));
    }
}

/// A ghost returning to its castle comes back to life there; dying put a hostile character
/// back in its fief's team.
#[test]
fn ghosts_resurrect_in_their_castle() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = fighter(&mut w, &rules, 0, 100, 100);
    let team = w.persos[i].team;
    w.attack_allies(&rules, i, true);
    assert_eq!(w.persos[i].team, -(i as i64));
    w.kill(&rules, i);
    assert!(!w.persos[i].alive());
    assert_eq!(w.persos[i].team, team);
    assert_eq!(w.fiefs[0].dead, 1);
    w.step(&rules, &[Order::GoOwnCastle { perso: i }]);
    for _ in 0..2000 {
        w.step(&rules, &[]);
        if w.persos[i].alive() {
            break;
        }
    }
    let p = &w.persos[i];
    assert!(p.alive() && p.in_own_castle);
    assert_eq!(p.bars[0], p.bonus_max[0] as f64);
    assert_eq!(w.fiefs[0].dead, 0);
}

/// A blow on a house spends at most 20 attack points, scaled by the demolition skill; a
/// destroyed house leaves half its till in a treasure.
#[test]
fn houses_are_hit_and_destroyed() {
    use sim::combat::Target;
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let o = fighter(&mut w, &rules, 1, 400, 400);
    let kind = rules.buildings.iter().position(|b| b.service == 0 && b.product < 0 && !b.factory).unwrap();
    w.create_house(&rules, o, 120, 100, kind, o);
    let max = w.houses[o].max_life as f64;
    w.houses[o].set_life(max);
    w.houses[o].money = 40.0;
    w.persos[a].attack = 50.0;
    w.hit_building(&rules, a, Target::House(o), o, -1);
    let skill = w.persos[a].special[1].bonus as f64 / rules.start_special as f64;
    let spent = rules.combat.max_building_damage;
    assert_eq!(w.houses[o].life, max - spent * rules.combat.building_damage_coef * skill);
    assert_eq!(w.persos[a].attack, 50.0 - spent);

    let treasures = w.treasures.len();
    w.houses[o].life = 1.0;
    w.persos[a].attack = 50.0;
    w.hit_building(&rules, a, Target::House(o), o, -1);
    assert!(!w.houses[o].visible());
    assert_eq!(w.persos[a].attack, 50.0 - 1.0);
    assert_eq!(w.persos[a].frags[1], 1);
    assert_eq!(w.treasures.len(), treasures + 1);
    assert_eq!(w.treasures.last().unwrap().money(), 20.0);
}
