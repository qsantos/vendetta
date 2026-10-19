//! Info-bubbles over the map (`FrmMoteur2D.Afficher_Infos_Bulles_Terrain`).

use super::Hud;
use sim::rules::Rules;
use sim::world::World;

impl Hud {
    /// The bubble for what the cursor is over (`hover`, `hit` from `World::obstacle_at`).
    pub(super) fn terrain_bubble(&self, world: &World, rules: &Rules, player: usize, hover: i32, hit: usize, m: (i32, i32)) {
        let s = &self.strings;
        let me = &world.persos[player];
        let text = match hover {
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
                let mut t = if hover == 10 { format!("&{}  ", b.name) } else { format!("{} ", b.name) };
                // TODO: invocation-owned houses show the chef's name
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
                } else {
                    // TODO: "Vendre" label while carrying an object
                    format!("{castle}  {t}")
                }
            }
            8 | 11 => {
                let name = &rules.peoples[world.fiefs[hit].people].castle_name;
                if hover == 11 { format!("&{name}  ") } else { format!("{name}  ") }
            }
            9 => {
                // TODO: active temporary effects
                format!("&{}  ", world.persos[hit].name)
            }
            13 => format!("{}  ", rules.treasures[world.treasures[hit].tier].name),
            16 => {
                let name = &world.decors[hit].name;
                if name.is_empty() {
                    return;
                }
                format!("{name}  ")
            }
            // TODO: 0 with an object on the cursor, 12 temple, 14/17 allies
            _ => return,
        };
        self.tooltip(m, &text);
    }
}
