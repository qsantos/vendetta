//! Info-bubbles over the map (`FrmMoteur2D.Afficher_Infos_Bulles_Terrain`).

use super::Hud;
use sim::rules::Rules;
use sim::world::World;

impl Hud {
    /// The bubble for what the cursor is over (`TypeCurseur` `hover` and `IndiceCible` `hit`).
    pub(super) fn terrain_bubble(&self, world: &World, rules: &Rules, player: usize, hover: i32, hit: usize, m: (i32, i32)) {
        let s = &self.strings;
        let me = &world.persos[player];
        // Ctrl on a damaged allied house shows the repair cursor over the allied bubble.
        let hover = if hover == 4 && hit != player { 7 } else { hover };
        let text = match hover {
            // An object on the cursor: whether a click sells it or drops it.
            0 if me.selected_object >= 0 => {
                let inseparable = rules.objects[me.selected_object as usize].inseparable;
                let market = me.in_house && world.houses.get(me.house_target).is_some_and(|m| m.market);
                let labels = if me.in_own_castle || me.in_castle || market { &s.sell } else { &s.drop };
                if inseparable {
                    labels[1].clone()
                } else if me.alive() {
                    labels[0].clone()
                } else {
                    return;
                }
            }
            1 => {
                let r = rules.terrain_resources[world.resources[hit].kind].resource;
                if r < 0 {
                    return;
                }
                let r = r as usize;
                format!("{} : {} / {}", rules.resources[r].name, me.resource(r).floor(), me.max_resource(r))
            }
            4 | 5 => {
                let h = &world.houses[player];
                let b = &rules.buildings[h.kind];
                if b.product > 0 {
                    let r = b.product as usize;
                    format!("{} #{} : {} / {} ", b.name, rules.resources[r].name, me.resource(r).floor(), me.max_resource(r))
                } else if b.factory {
                    match b.objects.get(me.craft_slot.max(0) as usize) {
                        Some(&o) if me.craft_progress > 0.0 => {
                            let pct = (me.craft_progress / rules.objects[o].craft_time as f64 * 100.0).floor();
                            format!("{} #{} : {pct}%", b.name, rules.objects[o].name)
                        }
                        _ => format!("{} ", b.name),
                    }
                } else {
                    format!("{}  ", b.name)
                }
            }
            7 | 10 => {
                let h = &world.houses[hit];
                let b = &rules.buildings[h.kind];
                let female = b.female as usize;
                let owner = &world.persos[hit];
                let mut t = if hover == 10 {
                    format!("&{}  ", b.name)
                } else if let Some(chef) = usize::try_from(owner.chef).ok().and_then(|c| world.persos.get(c)) {
                    // A summon's house shows its chief.
                    format!("{} ({})", b.name, chef.name)
                } else {
                    format!("{} ", b.name)
                };
                if hover == 7 && (!h.built || h.stock() < 1.0 && !(b.shoot || b.market)) {
                    t += &format!("#&{}", s.closed[female]);
                }
                if !world.persos[hit].alive() {
                    t += &format!("#&{} ", s.abandoned[female]);
                }
                t
            }
            6 => {
                let f = &world.fiefs[me.fief];
                let mut t = String::new();
                if f.changing_epoch {
                    if let Some(next) = rules.epochs.get(f.epoch + 1) {
                        let d = next.duration as f64 * world.epoch_speed;
                        t += &format!("#{} : {:.2} {}", next.name, (d - f.epoch_timer as f64) / d * 100.0, s.percent);
                    }
                }
                if me.action == 400 {
                    t += &format!("#{} : {} {}", s.actions[18], (100.0 - me.resurrection_time).floor(), s.percent);
                }
                let castle = &rules.peoples[f.people].castle_name;
                if me.resurrections > -1 && !me.alive() {
                    let (red, label) = match me.resurrections {
                        0 => ("&", &s.lives_left[0]),
                        1 => ("", &s.lives_left[0]),
                        _ => ("", &s.lives_left[1]),
                    };
                    format!("{castle}  {t}#{red}{label} : {}", me.resurrections)
                } else if me.holding() {
                    s.sell[0].clone()
                } else {
                    format!("{castle}  {t}")
                }
            }
            8 | 11 => {
                let name = &rules.peoples[world.fiefs[hit].people].castle_name;
                if hover == 11 { format!("&{name}  ") } else { format!("{name}  ") }
            }
            9 => {
                // the name, then each temporary effect
                let q = &world.persos[hit];
                let mut t = format!("&{}  ", q.name);
                for e in &q.effects {
                    t += &format!("#&({})", rules.objects[e.object].effect_name[q.female as usize]);
                }
                t
            }
            13 => format!("{}  ", rules.treasures[world.treasures[hit].tier].name),
            16 => {
                let name = &world.decors[hit].name;
                if name.is_empty() {
                    return;
                }
                format!("{name}  ")
            }
            12 => format!("{} ", rules.buildings[world.houses[hit].kind].name),
            // 17 (soldiers guarding an ally) would show the same, but armies are not ported.
            14 | 17 => {
                let q = &world.persos[hit];
                let name = if q.name.is_empty() { &rules.races[q.race].name[q.female as usize] } else { &q.name };
                format!("{name}  ")
            }
            _ => return,
        };
        self.tooltip(m, &text);
    }
}
