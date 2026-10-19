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
