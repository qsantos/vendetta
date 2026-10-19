//! Speech lines, orders to soldiers and calls for help (`ClsJeuCommentaires`, `ModDialogue`).

use crate::rules::Rules;
use crate::vb::clng;
use crate::world::World;

/// `ChanceRepondreAide`, `ChanceRepondreOrdre`.
const HELP_CHANCE: f64 = 3.0;
const ORDER_REPLY_CHANCE: f64 = 0.1;
/// `IntervalSoldatsX`, `IntervalSoldatsY`: room between soldiers in formation.
const SOLDIER_SPACING: i64 = 24;
/// `NombreFormations`
const FORMATIONS: i64 = 5;

impl World {
    /// `Commentaires.Message(n, perso)`: a random line of type `n` (the race's own type if it
    /// has one); one `Rnd` unless `n < 0`.
    pub fn comment_line(&mut self, rules: &Rules, i: usize, n: i64) -> String {
        if n < 0 {
            return String::new();
        }
        let s = self.persos[i].special_comments;
        let t = if s > 0 { s } else { n };
        let lines = usize::try_from(t).ok().and_then(|t| rules.comments.get(t));
        let k = self.rng.below(lines.map_or(0, |l| l.len()) as i64);
        lines.and_then(|l| l.get(k as usize)).cloned().unwrap_or_default()
    }

    /// `Definir_Commentaires = Commentaires.Message(n, perso)`
    pub fn say(&mut self, rules: &Rules, i: usize, n: i64) {
        let text = self.comment_line(rules, i, n);
        let tick = self.tick;
        self.persos[i].set_comment(text, tick);
    }

    /// `Definir_Commentaires = ""` then the line: says it even over a current line.
    pub(crate) fn say_now(&mut self, rules: &Rules, i: usize, n: i64) {
        let tick = self.tick;
        self.persos[i].set_comment(String::new(), tick);
        self.say(rules, i, n);
    }

    /// `Ordonner_Chargez`: chef `c` sends its free soldiers at a target (`kind` 0 perso,
    /// 1 house, 2 castle; -1 for the chef's own target, else back into formation). Shift sends
    /// at most 5 (10 with Ctrl), Ctrl alone 1; `silent` is `SansCommentaires`.
    #[allow(clippy::too_many_arguments)]
    pub fn order_charge(
        &mut self,
        rules: &Rules,
        c: usize,
        kind: i64,
        target: i64,
        ctrl: bool,
        shift: bool,
        silent: bool,
    ) {
        let (mut kind, mut target) = (kind, target);
        if kind == -1 {
            let p = &self.persos[c];
            let (k, t) = match p.action {
                300 => (0, p.perso_target),
                301 => (1, p.house_target),
                302 => (2, p.castle_target),
                _ => (-1, 0),
            };
            if k >= 0 {
                kind = k;
                if target == -1 {
                    target = t as i64;
                }
            }
        }
        let max = match (shift, ctrl) {
            (true, true) => 10,
            (true, false) => 5,
            (false, true) => 1,
            (false, false) => self.persos[c].soldiers,
        };
        let mut sent = 0;
        for i in 0..self.persos.len() {
            let s = &self.persos[i];
            if !(s.chef == c as i64
                && s.alive()
                && !s.fighting()
                && !s.incommandable
                && s.attitude == 0
                && s.ai_order == 0
                && !s.choose_ai
                && s.action != 2
                && s.action != 3)
            {
                continue;
            }
            let t = target.max(0) as usize;
            match kind {
                0 => self.attack_perso(rules, i, t),
                1 => self.attack_house(rules, i, t),
                2 => self.attack_castle(rules, i, t),
                _ => {
                    let (s, chef) = (&self.persos[i], &self.persos[c]);
                    let (x, y) = (chef.dest.0 + s.chef_offset.0, chef.dest.1 + s.chef_offset.1);
                    let s = &mut self.persos[i];
                    s.go_to(x, y);
                    s.chef_post = (s.x, s.y);
                }
            }
            if (0..=2).contains(&kind) {
                let s = &mut self.persos[i];
                s.ai_order = kind + 1;
                s.ai_target = t;
            }
            if self.rng.next() < ORDER_REPLY_CHANCE && !silent {
                self.say(rules, i, 31);
            }
            sent += 1;
            if sent >= max {
                break;
            }
        }
        if sent > 0 && !silent {
            let n = match kind {
                0 => 25,
                1 => 27,
                2 => 29,
                _ => 23,
            };
            let n = if (0..=2).contains(&kind) && sent > 1 { n + 1 } else { n };
            self.say_now(rules, c, n);
        }
    }

    /// `Ordonner_Formation(c, ChangerFormation, SansCommentaires)`: chef `c`'s free soldiers
    /// line up in rows around it (formation 0 a block above, 1 and 2 rows above and below, 3
    /// and 4 columns left and right), walking there unless the chef is fighting. `change` first
    /// switches to the next formation.
    pub fn order_formation(&mut self, rules: &Rules, c: usize, change: bool, silent: bool) {
        let p = &mut self.persos[c];
        if change {
            p.formation = (p.formation + 1) % FORMATIONS;
        }
        let s = SOLDIER_SPACING;
        // `Rangee` (a Long): soldiers per row.
        let mut row = clng((p.soldiers as f64).sqrt().floor() * 1.5);
        if row == 0 {
            row = 1;
        }
        let half = |row: i64| clng(s as f64 * row as f64 / 2.0 - s as f64 / 2.0);
        let (mut x, mut y) = match p.formation {
            0 => {
                row = clng(row as f64 / 1.5);
                (half(row), clng(s as f64 * row as f64 / 2.0 + s as f64 / 2.0))
            }
            1 | 2 => (half(row), 0),
            3 | 4 => (0, half(row)),
            _ => (0, 0),
        };
        let formation = p.formation;
        let mut j = 0;
        let mut i = 0;
        while i < self.persos.len() {
            let q = &self.persos[i];
            if q.chef == c as i64 && q.alive() && q.ai_order == 0 && q.attitude == 0 && !q.incommandable && !q.choose_ai {
                let wrap = j % row == 0;
                match formation {
                    0 | 1 | 2 if wrap => {
                        y += if formation == 2 { s } else { -s };
                        x -= s * (row - 1);
                    }
                    0 | 1 | 2 => x += s,
                    3 | 4 if wrap => {
                        x += if formation == 4 { s } else { -s };
                        y -= s * (row - 1);
                    }
                    3 | 4 => y += s,
                    _ => {}
                }
                j += 1;
                // Not on the chef: the same soldier takes the next place.
                if x == 0 && y == 0 {
                    continue;
                }
                let chef = &self.persos[c];
                let (fighting, (dx, dy)) = (chef.fighting(), chef.dest);
                let q = &mut self.persos[i];
                q.chef_offset = (x, y);
                if !fighting {
                    q.go_to(dx + x, dy + y);
                    q.chef_post = q.dest;
                }
            }
            i += 1;
        }
        if !silent {
            let line = if self.persos[c].moving { 23 } else { 22 };
            self.say(rules, c, line);
        }
    }

    /// `Appeler_De_lAide(v)`: perso `v`, in trouble, calls its team. Protector chefs send their
    /// soldiers; other free villagers may come themselves.
    pub fn call_for_help(&mut self, rules: &Rules, v: usize) {
        let team = self.persos[v].team;
        for i in 0..self.persos.len() {
            let q = &self.persos[i];
            if q.team != team || !q.ai {
                continue;
            }
            if q.ai_protector && q.soldiers > 4 {
                let p = &self.persos[v];
                match p.action {
                    300..=302 => {
                        let (kind, line, t) = match p.action {
                            300 => (0, 6, p.perso_target),
                            301 => (1, 7, p.house_target),
                            _ => (2, 8, p.castle_target),
                        };
                        if p.ai {
                            self.say_now(rules, v, line);
                        }
                        self.order_charge(rules, i, kind, t as i64, false, true, false);
                        self.say(rules, i, 10);
                    }
                    _ => {
                        if let Some(j) = self.find_attacker(v) {
                            if self.persos[v].ai {
                                self.say_now(rules, v, 9);
                            }
                            self.order_charge(rules, i, 0, j as i64, false, true, false);
                            self.say(rules, i, 10);
                        }
                    }
                }
            } else if !q.invocation() && !q.ai_pacific && !q.ai_selfish && q.action < 102 && q.vitality == 1.0 {
                let citizens = self.fiefs[self.persos[v].fief].citizens as f64;
                if self.rng.next() < HELP_CHANCE / (citizens + 1.0) {
                    let p = &self.persos[v];
                    match p.action {
                        300..=302 => {
                            let (line, t) = match p.action {
                                300 => (6, p.perso_target),
                                301 => (7, p.house_target),
                                _ => (8, p.castle_target),
                            };
                            let action = p.action;
                            if p.ai {
                                self.say_now(rules, v, line);
                            }
                            match action {
                                300 => self.attack_perso(rules, i, t),
                                301 => self.attack_house(rules, i, t),
                                _ => self.attack_castle(rules, i, t),
                            }
                            self.say(rules, i, 10);
                        }
                        _ => {
                            if let Some(j) = self.find_attacker(v) {
                                if self.persos[v].ai {
                                    self.say_now(rules, v, 9);
                                }
                                self.attack_perso(rules, i, j);
                                self.say(rules, i, 10);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The first perso attacking `v`, or (half the time, one `Rnd` per perso examined)
    /// attacking its house.
    fn find_attacker(&mut self, v: usize) -> Option<usize> {
        for j in 0..self.persos.len() {
            let q = &self.persos[j];
            let (a, b) = (q.action == 300 && q.perso_target == v, q.action == 301 && q.house_target == v);
            let r = self.rng.next() < 0.5;
            if a || b && r {
                return Some(j);
            }
        }
        None
    }
}
