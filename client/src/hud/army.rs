//! The army: soldiers selected with a rectangle or the keys (`FrmMoteur2D.Selectionner_Soldats`,
//! `ClsIntTouches.Presser_Touche`), then given orders (`ModDialogue.Ordonner_*`). The selection
//! only lives here; the orders carry the soldiers' numbers (see `Order::ArmyMove` and the like).

use super::Hud;
use macroquad::prelude::*;
use sim::perso::Perso;
use sim::world::{Order, World};

/// The least a rectangle must cover, in interface pixels, to select rather than click
/// (`RectangleSelectionMinX`, `RectangleSelectionMinY`).
pub const MIN_DRAG: f32 = 16.0;

/// The mouse button held since it went down on the map.
#[derive(Clone, Copy)]
struct Drag {
    from: (f32, f32),
    to: (f32, f32),
    right: bool,
    /// It covers enough for a selection (`SelectionArmee`).
    big: bool,
}

/// A drag whose button came up.
#[derive(Clone, Copy)]
pub struct Ended {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub right: bool,
    pub big: bool,
}

/// `SoldatsSelectionnes` and `ArmeeSelectionnee`.
#[derive(Default)]
pub struct Army {
    pub selected: Vec<usize>,
    /// Soldiers other than the player are selected: they take the orders, not the player.
    pub on: bool,
    drag: Option<Drag>,
}

/// What a selection with the keys keeps (`ToucheSoldats*`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    SameRace,
    Armed,
    Rested,
    Idle,
    Fast,
    Unassigned,
    Ground,
    Flying,
    Defensive,
    Offensive,
    Invert,
}

impl Army {
    /// A button went down on the map.
    pub fn begin(&mut self, at: (f32, f32), right: bool) {
        self.drag = Some(Drag { from: at, to: at, right, big: false });
    }

    /// Follows the drag; when its button is no longer down (or soldiers cannot be selected
    /// any more) it ends. `min` is the least a drag covers, in the mouse's pixels.
    pub fn update(&mut self, ready: bool, mouse: (f32, f32), min: f32, left_down: bool, right_down: bool) -> Option<Ended> {
        let d = self.drag.as_mut()?;
        if !ready {
            self.drag = None;
            return None;
        }
        d.to = mouse;
        d.big = (d.to.0 - d.from.0).abs() >= min || (d.to.1 - d.from.1).abs() >= min;
        if if d.right { right_down } else { left_down } {
            return None;
        }
        let d = self.drag.take()?;
        Some(Ended { from: d.from, to: d.to, right: d.right, big: d.big })
    }

    /// The rectangle being dragged, in the mouse's pixels, and whether with the right button.
    pub fn dragging(&self) -> Option<((f32, f32), (f32, f32), bool)> {
        self.drag.map(|d| (d.from, d.to, d.right))
    }

    /// The soldiers that take orders (`NumeroChef = player`, alive, `Incommandable` not).
    pub fn commanded(world: &World, player: usize) -> Vec<usize> {
        let ok = |p: &Perso| p.chef == player as i64 && p.alive() && !p.incommandable;
        (0..world.persos.len()).filter(|&i| i != player && ok(&world.persos[i])).collect()
    }

    /// The selection forgets its dead; `on` is told again.
    pub fn prune(&mut self, world: &World, player: usize) {
        self.selected.retain(|&i| world.persos.get(i).is_some_and(|p| p.alive()));
        self.refresh(player);
    }

    fn refresh(&mut self, player: usize) {
        self.on = match self.selected.as_slice() {
            [] => false,
            [only] => *only != player,
            _ => true,
        };
    }

    /// Selected soldiers that follow an AI file go manual, and go back to the AI when not.
    fn ai(&self, world: &World, player: usize, soldiers: &[usize], ai: bool, orders: &mut Vec<Order>) {
        let some: Vec<usize> = soldiers.iter().copied().filter(|&s| world.persos.get(s).is_some_and(|p| p.choose_ai && p.ai != ai)).collect();
        if !some.is_empty() {
            orders.push(Order::ArmyAi { perso: player, soldiers: some, ai });
        }
    }

    /// Nothing selected any more (`Ordonner_Activer_IA`).
    pub fn clear(&mut self, world: &World, player: usize, orders: &mut Vec<Order>) {
        let was = std::mem::take(&mut self.selected);
        self.ai(world, player, &was, true, orders);
        self.refresh(player);
    }

    /// `Selectionner_Soldats`: the soldiers overlapping `rect` (world pixels: x, y, width,
    /// height) join the selection with `add` (Shift), leave it with `remove` (Ctrl), else
    /// replace it. A click (`area` unset) takes only the first soldier it covers. With
    /// `left_release` the player joins too (or leaves), `right_release` leaves out those with
    /// an AI file. Returns whether anything was covered.
    #[allow(clippy::too_many_arguments)]
    pub fn select(
        &mut self,
        world: &World,
        player: usize,
        rect: (i32, i32, i32, i32),
        (add, remove): (bool, bool),
        area: bool,
        (left_release, right_release): (bool, bool),
        orders: &mut Vec<Order>,
    ) -> bool {
        let remove = remove && !add;
        let before = self.selected.clone();
        if !(add || remove) {
            self.selected.clear();
        }
        let mut found = false;
        let (mut manual, mut back) = (Vec::new(), Vec::new());
        for s in Self::commanded(world, player) {
            let p = &world.persos[s];
            if !world.terrain.overlaps((p.x as i32, p.y as i32, p.w, p.h), rect) {
                continue;
            }
            found = true;
            if remove {
                self.selected.retain(|&i| i != s);
                back.push(s);
            } else if !(right_release && p.choose_ai) {
                self.selected.push(s);
                manual.push(s);
            }
            if !area {
                break;
            }
        }
        let p = &world.persos[player];
        if left_release && world.terrain.overlaps((p.x as i32, p.y as i32, p.w, p.h), rect) {
            found = true;
            if remove {
                self.selected.retain(|&i| i != player);
            } else {
                self.selected.push(player);
            }
        }
        if add {
            let mut seen = Vec::new();
            self.selected.retain(|&i| {
                let new = !seen.contains(&i);
                seen.push(i);
                new
            });
        }
        self.refresh(player);
        let dropped: Vec<usize> = before.into_iter().filter(|i| !self.selected.contains(i)).collect();
        self.ai(world, player, &dropped, true, orders);
        self.ai(world, player, &manual, false, orders);
        self.ai(world, player, &back, true, orders);
        found
    }

    /// The Ctrl and letter keys: select the soldiers that are `filter`; with `Invert`, those
    /// that were not; `SameRace` adds to what is selected. Returns whether anything was done.
    pub fn select_by(&mut self, world: &World, player: usize, filter: Filter, orders: &mut Vec<Order>) {
        let me = &world.persos[player];
        let ok = |p: &Perso| p.chef == player as i64 && p.alive() && !p.incommandable;
        let soldier = |s: usize| s != player && world.persos.get(s).is_some_and(|p| ok(p));
        let free = |s: usize| soldier(s) && !world.persos[s].choose_ai;
        let all: Vec<usize> = (0..world.persos.len()).filter(|&s| free(s)).collect();
        let before = std::mem::take(&mut self.selected);
        let chosen: Vec<usize> = match filter {
            Filter::All => all,
            Filter::SameRace if self.on_before(&before, player) => {
                let races: Vec<usize> = before.iter().filter_map(|&s| world.persos.get(s).map(|p| p.race)).collect();
                let mut chosen = before.clone();
                chosen.extend((0..world.persos.len()).filter(|&s| soldier(s) && !before.contains(&s) && races.contains(&world.persos[s].race)));
                chosen
            }
            Filter::SameRace => before.clone(),
            Filter::Invert => all.into_iter().filter(|s| !before.contains(s)).collect(),
            f => all
                .into_iter()
                .filter(|&s| {
                    let p = &world.persos[s];
                    match f {
                        Filter::Armed => p.equipped.first().is_some_and(|e| e.is_some()) || p.equipped.get(1).is_some_and(|e| e.is_some()),
                        Filter::Rested => p.bars[0] == p.bonus_max[0] as f64,
                        Filter::Idle => p.idle(),
                        Filter::Fast => p.step >= me.step,
                        Filter::Unassigned => p.attitude == 0 && p.ai_order != 4,
                        Filter::Ground => !p.bonus_flying,
                        Filter::Flying => p.bonus_flying,
                        Filter::Defensive => p.attack < p.defense,
                        Filter::Offensive => p.attack >= p.defense,
                        _ => true,
                    }
                })
                .collect(),
        };
        self.selected = chosen;
        let dropped: Vec<usize> = before.iter().copied().filter(|i| !self.selected.contains(i)).collect();
        let fresh: Vec<usize> = self.selected.iter().copied().filter(|i| !before.contains(i)).collect();
        self.ai(world, player, &dropped, true, orders);
        self.ai(world, player, &fresh, false, orders);
        self.refresh(player);
        self.on = !self.selected.is_empty();
    }

    fn on_before(&self, before: &[usize], player: usize) -> bool {
        match before {
            [] => false,
            [only] => *only != player,
            _ => true,
        }
    }

    /// `Ctrl+Suppr`: the selected soldiers die, but for the player.
    pub fn kill(&mut self, world: &World, player: usize, orders: &mut Vec<Order>) {
        if self.on {
            orders.push(Order::ArmyKill { perso: player, soldiers: self.selected.clone() });
            self.clear(world, player, orders);
        }
    }
}

impl Hud {
    /// The player can select soldiers: alive, with soldiers.
    pub fn army_ready(world: &World, player: usize) -> bool {
        let p = &world.persos[player];
        p.alive() && p.soldiers > 0
    }

    /// `Afficher_CadreSelection_Soldats`: the rectangle being dragged, white for the left
    /// button, red for the right.
    pub fn draw_army_drag(&self, ui: f32) {
        let Some((from, to, right)) = self.army.dragging() else { return };
        if (to.0 - from.0).abs() < 1.0 && (to.1 - from.1).abs() < 1.0 {
            return;
        }
        let c = if right { Color::from_rgba(255, 0, 0, 255) } else { WHITE };
        let (x0, x1) = (from.0.min(to.0) / ui, from.0.max(to.0) / ui);
        let (y0, y1) = (from.1.min(to.1) / ui, from.1.max(to.1) / ui);
        super::outline(x0 as i32, y0 as i32, x1 as i32, y1 as i32, c);
    }

    /// The soldiers' orders: the attitude buttons and keys give `attitude` to the selected
    /// soldiers, which are then not selected any more (`Bouton_ChangerAttitude`).
    pub(super) fn attitude(&mut self, world: &World, player: usize, attitude: i64, orders: &mut Vec<Order>) {
        if self.army.selected.is_empty() {
            return;
        }
        orders.push(Order::ArmyAttitude { perso: player, soldiers: self.army.selected.clone(), attitude });
        self.army.clear(world, player, orders);
    }
}
