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
    let kind = rules.buildings.iter().position(|b| b.owner_race < 0 && !b.emperor).unwrap();
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

/// An area attack hits every enemy within its radius of the attacker, each with the whole
/// attack bar, which ends empty.
#[test]
fn area_attacks_hit_everyone_near() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let near = fighter(&mut w, &rules, 1, 130, 100);
    let far = w.persos.iter().rposition(|p| p.fief == 1).unwrap();
    assert_ne!(near, far);
    let q = &mut w.persos[far];
    (q.x, q.y) = (600, 600);
    q.set_visible(true);
    w.persos[a].perso_target = a;
    w.persos[a].attack = 30.0;
    let (life_near, life_far) = (w.persos[near].bars[0], w.persos[far].bars[0]);
    w.rng = seed_for(rules.combat.failure_chance, &[true, true]);
    w.hit_area(&rules, a, 64, -1);
    assert!(w.persos[near].bars[0] < life_near);
    assert_eq!(w.persos[far].bars[0], life_far);
    assert_eq!(w.persos[a].attack, 0.0);
}

/// Trampling hits the enemies under the perso with its trampling strength.
#[test]
fn trampling_crushes_weaker_enemies() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 100);
    let d = fighter(&mut w, &rules, 1, 104, 104);
    w.persos[a].trample = 15;
    w.persos[a].attack = 0.0;
    w.persos[d].defense = 0.0;
    let life = w.persos[d].bars[0];
    w.rng = seed_for(rules.combat.failure_chance, &[true, true]);
    w.trample(&rules, a);
    let res = w.persos[d].resistance(w.persos[a].bonus_attack_resistance);
    assert_eq!(w.persos[d].bars[0], life - 15.0 * rules.combat.damage_coef * res);
}

const GOLEM: usize = 43; // Golem de pierre: summons a race-30 golem

/// Using a summoning object creates a soldier next to its chef, which counts it, follows its
/// orders and drops out of the count when it dies.
#[test]
fn summoning_a_soldier() {
    use sim::items::Equip;
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = fighter(&mut w, &rules, 0, 300, 300);
    let o = &rules.objects[GOLEM];
    let race = o.invocation as usize;
    let n = w.persos.len();
    w.persos[i].add_object(&rules, GOLEM);
    let slot = w.persos[i].inventory.iter().position(|s| s.is_some_and(|it| it.object == GOLEM)).unwrap();
    assert_eq!(w.equip_object_at(&rules, i, Some(slot), false), Equip::Used);
    w.step(&rules, &[]);
    assert_eq!(w.persos.len(), n + 1, "a new slot");
    let g = n;
    let s = &w.persos[g];
    assert_eq!((s.race, s.chef, s.fief, s.team), (race, i as i64, w.persos[i].fief, w.persos[i].team));
    assert!(s.alive() && s.ai && s.invocation());
    assert_eq!(w.persos[i].soldiers, 1);
    assert_eq!(w.houses.len(), w.persos.len());
    // Placed on the right of its chef, then lined up in formation 0, above it.
    assert_eq!((s.x, s.y), (w.persos[i].x + w.persos[i].w as i64, w.persos[i].y));
    assert_eq!(s.chef_offset, (0, -24));
    // The chef's attack order sends it too.
    let enemy = w.persos.iter().position(|p| p.fief == 1).unwrap();
    w.attack_perso(&rules, i, enemy);
    assert_eq!((w.persos[g].action, w.persos[g].perso_target, w.persos[g].ai_order), (300, enemy, 1));
    w.kill(&rules, g);
    assert_eq!(w.persos[i].soldiers, 0);
    // A new summon takes the dead one's slot.
    w.persos[i].add_object(&rules, GOLEM);
    let slot = w.persos[i].inventory.iter().position(|s| s.is_some_and(|it| it.object == GOLEM)).unwrap();
    w.persos[i].recharges.clear();
    w.persos[i].bars[2] = w.persos[i].bonus_max[2] as f64;
    assert_eq!(w.equip_object_at(&rules, i, Some(slot), false), Equip::Used);
    w.step(&rules, &[]);
    assert_eq!(w.persos.len(), n + 1);
    assert!(w.persos[g].alive());
    assert_eq!(w.persos[i].soldiers, 1);
}

/// A guard tower is built and owned by a guard summoned for it (`CreerProprietaire`).
#[test]
fn owner_invocations() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = fighter(&mut w, &rules, 0, 300, 300);
    let kind = rules.buildings.iter().position(|b| b.owner_race >= 0).unwrap();
    let n = w.persos.len();
    w.create_house(&rules, i, 320, 300, kind, i);
    assert_eq!(w.persos.len(), n + 1);
    let g = &w.persos[n];
    assert_eq!((g.race as i64, g.chef), (rules.buildings[kind].owner_race, i as i64));
    assert!(w.houses[n].visible() && w.houses[n].kind == kind);
    assert!(!w.houses[i].visible(), "the builder keeps no house");
}

const BOW: usize = 16; // Arc court: shoots race-84 arrows

/// A bow shoots an arrow, a flying perso that hits the first enemy on its way and vanishes.
#[test]
fn arrows_fly_and_hit() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let a = fighter(&mut w, &rules, 0, 100, 300);
    let d = fighter(&mut w, &rules, 1, 160, 300);
    w.persos[d].defense = 0.0;
    let p = &mut w.persos[a];
    p.add_object(&rules, BOW);
    p.equip(&rules, Some(0), false);
    assert_eq!(p.launcher, rules.objects[BOW].launcher);
    if let Ok(r) = usize::try_from(p.ammo) {
        p.set_resource(r, p.max_resource(r));
    }
    assert!(p.bonus_range > 0.0);
    let life = w.persos[d].bars[0];
    let n = w.persos.len();
    w.step(&rules, &[Order::AttackPerso { perso: a, target: d }]);
    let mut shot = false;
    for _ in 0..100 {
        w.step(&rules, &[]);
        if w.persos.len() > n {
            let arrow = &w.persos[n];
            assert!(arrow.projectile && arrow.chef == a as i64 && !arrow.ai);
            assert_eq!(arrow.race as i64, rules.objects[BOW].launcher);
            shot = true;
        }
        if w.persos[d].bars[0] < life {
            break;
        }
    }
    assert!(shot, "no arrow");
    assert!(w.persos[d].bars[0] < life, "the arrow hit");
    assert!(!w.persos[n].alive(), "and vanished");
    assert_eq!(w.persos[a].projectiles, 0);
}

const HEAL_OTHERS: usize = 41; // Guérison d'autrui: +10 Vie to the nearest wounded ally
const TAX: usize = 199; // Impôt: a tenth of the money of the fief's citizens

/// Uses `object` from perso `i`'s bag; it takes effect at the end of the next tick.
fn use_object(w: &mut World, rules: &Rules, i: usize, object: usize) {
    let p = &mut w.persos[i];
    p.add_object(rules, object);
    let slot = p.inventory.iter().position(|s| s.is_some_and(|it| it.object == object)).unwrap();
    for b in 0..4 {
        p.bars[b] = p.bonus_max[b] as f64;
    }
    assert_eq!(w.equip_object_at(rules, i, Some(slot), false), sim::items::Equip::Used);
    w.step(rules, &[]);
}

#[test]
fn healing_and_taxes() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = fighter(&mut w, &rules, 0, 300, 300);
    let j = w.persos.iter().rposition(|p| p.fief == 0).unwrap();
    let q = &mut w.persos[j];
    (q.x, q.y) = (350, 300);
    q.set_visible(true);
    q.bars[0] = 5.0;
    use_object(&mut w, &rules, i, HEAL_OTHERS);
    let coef = w.persos[i].bonus_max[0] as f64 / w.persos[i].max[0] as f64;
    assert!(w.persos[j].recovery[0] >= 10.0 * coef - 1.0, "{}", w.persos[j].recovery[0]);
    assert!(w.events.iter().any(|e| matches!(e, Event::Number { kind: NumberKind::Heal, .. })));

    w.persos[i].set_money(0.0);
    w.persos[j].set_money(100.0);
    use_object(&mut w, &rules, i, TAX);
    assert_eq!(w.persos[j].money(), 90.0);
    assert_eq!(w.persos[i].money(), 10.0);
}

/// Soldiers follow their chef's moves, keeping their place in the formation.
#[test]
fn soldiers_follow_their_chef() {
    let Some((files, rules)) = load() else { return };
    let mut w = world(&files, &rules);
    let i = fighter(&mut w, &rules, 0, 300, 300);
    let n = w.persos.len();
    use_object(&mut w, &rules, i, GOLEM);
    let g = n;
    assert_eq!(w.persos[g].chef, i as i64);
    w.persos[g].ai = false;
    w.step(&rules, &[Order::Move { perso: i, x: 500, y: 300 }]);
    let off = w.persos[g].chef_offset;
    assert_eq!(w.persos[g].dest, (500 + off.0, 300 + off.1));
    assert_eq!(w.persos[g].action, 1);
}
