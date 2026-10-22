//! The message log at the top left (`ClsJeuMessages`): what happened to the player, found by
//! comparing its state once a second, and the tips of `Info_Conseils.txt`.

use super::{Hud, WHITE_};
use macroquad::prelude::*;
use sim::files::Files;
use sim::ini::Ini;
use sim::perso::Perso;
use sim::rules::Rules;
use sim::world::World;

/// `Messages.ini`: `NombreMaxMessages`, `Duree` (seconds per character before the oldest line
/// goes), `PositionX/Y`, `DecalageY`.
const MAX_MESSAGES: usize = 12;
const DURATION: f64 = 0.16;
const POSITION: (i32, i32) = (60, 30);
const LINE: i32 = 16;

/// The texts of `Messages.ini`, with the spaces the original adds around them.
struct Texts {
    prefix: String,
    prefix_xp: String,
    alive: String,
    dead: String,
    money: String,
    gain: String,
    loss: String,
    bestiary: String,
    /// `MessageVie`, `Energie`, `Magie`, `Moral`, `Attaque`, `Defense`.
    levels: [String; 6],
    soldier: String,
    soldiers: String,
    skill: String,
    /// `MessageEmpereurVieM/F`, `MessageEmpereurMortM/F`.
    emperor: [String; 2],
    emperor_dead: [String; 2],
    /// The first line of `Info_Conseils.txt`, then the tips.
    tip_prefix: String,
    tips: Vec<String>,
}

/// What the log compares the player with (`Actualiser_Messages`' copies).
struct Seen {
    alive: bool,
    money: f64,
    resources: Vec<f64>,
    bestiary: Vec<i64>,
    soldiers: i64,
    /// `MaxVie` … `MaxDefense`: the base maxima.
    max: [i64; 6],
    /// Levels of the special, resource, service and item skills.
    skills: [Vec<i64>; 4],
    objects: Vec<i64>,
    emperor: i64,
}

pub struct MessageLog {
    texts: Texts,
    lines: Vec<String>,
    /// `TempsDernierMessage`
    last: f64,
    seen: Option<Seen>,
}

fn levels(p: &Perso) -> [Vec<i64>; 4] {
    [&p.special, &p.resource_skills, &p.services, &p.objects].map(|s| s.iter().map(|s| s.xp.level).collect())
}

/// `Objet_NombreType`: worn objects and bag stacks of type `o`.
fn count(p: &Perso, o: usize) -> i64 {
    let worn = p.equipped.iter().flatten().filter(|it| it.object == o).count() as i64;
    worn + p.inventory.iter().flatten().filter(|it| it.object == o).map(|it| it.count).sum::<i64>()
}

impl MessageLog {
    /// The texts of `other` (another language's), keeping the messages shown.
    pub fn set_texts(&mut self, other: MessageLog) {
        self.texts = other.texts;
    }

    /// Starts comparing afresh, e.g. with another character.
    pub fn forget(&mut self) {
        self.seen = None;
    }

    pub fn load(files: &Files, lang: &str) -> MessageLog {
        let ini = Ini::parse(files.text(&format!("Donnees/{lang}/Messages.ini")).unwrap_or(""));
        let get = |k: &str| ini.find(k).unwrap_or("").to_owned();
        let pre = |k: &str| format!(" {}", get(k));
        let post = |k: &str| format!("{} ", get(k));
        // `Afficher_Conseil`: the first line is the prefix; a line starting with `-` starts a
        // tip, the others continue it; `#` lines are comments.
        let file = files.text(&format!("Donnees/{lang}/{}", get("FichierConseil"))).unwrap_or("");
        let mut lines = file.lines();
        let tip_prefix = lines.next().unwrap_or("").to_owned();
        let mut tips: Vec<String> = Vec::new();
        let start = get("PrefixeConseil");
        for line in lines.filter(|l| !l.is_empty() && !l.starts_with('#')) {
            if line.starts_with(start.as_str()) {
                tips.push(line.to_owned());
            } else if let Some(t) = tips.last_mut() {
                *t += &format!("#{line}");
            }
        }
        let texts = Texts {
            prefix: post("PrefixeMessage"),
            prefix_xp: post("PrefixeExperience"),
            alive: get("MessageVivant"),
            dead: get("MessageMort"),
            money: pre("MessageArgent"),
            gain: post("MessageGain"),
            loss: post("MessagePerte"),
            bestiary: post("MessageBestiaire"),
            levels: ["MessageVie", "MessageEnergie", "MessageMagie", "MessageMoral", "MessageAttaque", "MessageDefense"].map(pre),
            soldier: pre("MessageSoldat"),
            soldiers: pre("MessageSoldats"),
            skill: format!(" {} ", get("MessageCompetence")),
            emperor: [get("MessageEmpereurVieM"), get("MessageEmpereurVieF")],
            emperor_dead: [get("MessageEmpereurMortM"), get("MessageEmpereurMortF")],
            tip_prefix,
            tips,
        };
        MessageLog { texts, lines: Vec::new(), last: 0.0, seen: None }
    }

    /// `Ajouter_Message`: a message of several lines is followed by as many empty lines as it
    /// has (one more than needed), so that the next message comes under it.
    fn add(&mut self, message: String) {
        if self.lines.is_empty() {
            self.last = get_time();
        }
        let n = message.matches('#').count() + 1;
        self.lines.push(message);
        if n > 1 {
            self.lines.extend(std::iter::repeat_n(String::new(), n));
        }
    }

    /// `Afficher_Conseil`: a tip, `Rnd * (n - 1)` rounded (the last one never comes up but
    /// rounding up). `pick` is the random number.
    pub fn add_tip(&mut self, pick: f64) {
        let n = self.texts.tips.len();
        if n == 0 {
            return;
        }
        let i = sim::vb::clng(pick * (n - 1) as f64).clamp(0, n as i64 - 1) as usize;
        let tip = format!("{} {}", self.texts.tip_prefix, self.texts.tips[i]);
        self.add(tip);
    }

    /// Any line of text (`&` for red).
    pub fn add_text(&mut self, text: &str) {
        self.add(text.to_owned());
    }

    /// `Actualiser_Messages`, once a second: compares the player with what was seen.
    fn update(&mut self, world: &World, rules: &Rules, player: usize) {
        let p = &world.persos[player];
        let fief = &world.fiefs[p.fief];
        let Some(seen) = &mut self.seen else {
            // The first time, only what to compare with. Quirk: objects are counted per slot,
            // not per stack, so a stack is reported as gained a second later.
            let mut objects = vec![0; rules.objects.len()];
            for it in p.equipped.iter().chain(&p.inventory).flatten() {
                objects[it.object] += 1;
            }
            self.seen = Some(Seen {
                alive: p.alive(),
                money: p.money(),
                resources: (0..rules.resources.len()).map(|r| p.resource(r)).collect(),
                bestiary: p.bestiary.clone(),
                soldiers: p.soldiers,
                max: p.max,
                skills: levels(p),
                objects,
                emperor: fief.emperor,
            });
            return;
        };
        let t = &self.texts;
        let mut out = Vec::new();
        let alive = p.alive();
        if alive != seen.alive {
            out.push(format!("{}{}", t.prefix, if alive { &t.alive } else { &t.dead }));
            seen.alive = alive;
        }
        for (r, race) in rules.races.iter().enumerate() {
            let n = p.bestiary[r] - seen.bestiary[r];
            if n >= 1 {
                let name = if n > 1 { &race.plural[0] } else { &race.name[0] };
                out.push(format!("{}{}{n} {}.", t.prefix, t.bestiary, name.to_lowercase()));
            }
            seen.bestiary[r] = p.bestiary[r];
        }
        let n = p.soldiers - seen.soldiers;
        let unit = |n: i64| if n == 1 { &t.soldier } else { &t.soldiers };
        if n >= 1 {
            out.push(format!("{}{}{n}{}", t.prefix, t.gain, unit(n)));
        } else if -n >= 1 {
            out.push(format!("{}{}{}{}", t.prefix, t.loss, -n, unit(-n)));
        }
        seen.soldiers = p.soldiers;
        // Item income does not count.
        let income = if alive { p.gain_money } else { 0.0 };
        let money = p.money();
        if money - seen.money - income >= 1.0 {
            out.push(format!("{}{}{}{}", t.prefix, t.gain, (money - seen.money - income).floor(), t.money));
        } else if seen.money + income - money >= 1.0 {
            out.push(format!("{}{}{}{}", t.prefix, t.loss, (seen.money - money + income).floor(), t.money));
        }
        seen.money = money;
        let income = |r: usize| if alive { p.gain_resources.get(r).copied().unwrap_or(0.0) } else { 0.0 };
        // Gains, but not of the resource being worked.
        let mut list = Vec::new();
        for (r, res) in rules.resources.iter().enumerate() {
            let n = p.resource(r) - seen.resources[r] - income(r);
            if n >= 1.0 && r as i64 != p.current_resource {
                list.push(format!("{} {}", n.floor(), res.name.to_lowercase()));
            }
        }
        if !list.is_empty() {
            out.push(format!("{}{}{}.", t.prefix, t.gain, list.join(", ")));
        }
        // Losses, but not of the raw material of the resource being worked. Quirk: the amount
        // adds the income instead of taking it off.
        let raw = usize::try_from(p.current_resource).ok().map(|c| rules.resources[c].raw);
        let mut list = Vec::new();
        for (r, res) in rules.resources.iter().enumerate() {
            let n = seen.resources[r] - p.resource(r) - income(r);
            if n >= 1.0 && raw != Some(r as i64) {
                let n = seen.resources[r] - p.resource(r) + income(r);
                list.push(format!("{} {}", n.floor(), res.name.to_lowercase()));
            }
            seen.resources[r] = p.resource(r);
        }
        if !list.is_empty() {
            out.push(format!("{}{}{}.", t.prefix, t.loss, list.join(", ")));
        }
        // Objects, the held one included.
        let held = |o: usize| (p.selected_object == o as i64) as i64;
        let name = |o: usize, n: i64| if n >= 2 { &rules.objects[o].plural } else { &rules.objects[o].name };
        let mut list = Vec::new();
        for o in 0..rules.objects.len() {
            let n = count(p, o) - seen.objects[o] + held(o);
            if n >= 1 {
                list.push(format!("{n} {}", name(o, n)));
            }
        }
        if !list.is_empty() {
            out.push(format!("{}{}{}.", t.prefix, t.gain, list.join(", ")));
        }
        let mut list = Vec::new();
        for o in 0..rules.objects.len() {
            let c = count(p, o);
            if seen.objects[o] - c - held(o) >= 1 {
                // Quirk: the number shown leaves out the held object.
                let n = seen.objects[o] - c;
                list.push(format!("{n} {}", name(o, n)));
            }
            seen.objects[o] = c + held(o);
        }
        if !list.is_empty() {
            out.push(format!("{}{}{}.", t.prefix, t.loss, list.join(", ")));
        }
        // Experience: the bars' maxima, then the skills' levels.
        for k in 0..6 {
            let n = p.max[k] - seen.max[k];
            if n >= 1 {
                out.push(format!("{}{}{n}{}", t.prefix_xp, t.gain, t.levels[k]));
            }
            seen.max[k] = p.max[k];
        }
        let now = levels(p);
        for (kind, (now, before)) in now.iter().zip(seen.skills.iter_mut()).enumerate() {
            for (i, (&level, old)) in now.iter().zip(before.iter_mut()).enumerate() {
                if level - *old >= 1 {
                    let skill = match kind {
                        0 => rules.special_skills.get(i),
                        1 => rules.resources.get(i).map(|r| &r.skill_name),
                        2 => rules.services.get(i + 1).map(|s| &s.skill_name),
                        _ => rules.item_skills.get(i),
                    };
                    let skill = skill.map_or(String::new(), |s| s.to_lowercase());
                    out.push(format!("{}{}{}{}{skill}.", t.prefix_xp, t.gain, level - *old, t.skill));
                }
                *old = level;
            }
        }
        if seen.emperor != fief.emperor {
            let title = |e: &Perso| &rules.epochs[fief.epoch].emperor_title[e.female as usize];
            if let Ok(e) = usize::try_from(fief.emperor) {
                let e = &world.persos[e];
                let m = format!("{}{}{}", e.shown_name(rules), t.emperor[e.female as usize], title(e));
                out.push(format!("{}{}", t.prefix, m.to_uppercase()));
            } else if let Some(e) = usize::try_from(seen.emperor).ok().and_then(|e| world.persos.get(e)) {
                let m = format!("{} {}{}", title(e), e.shown_name(rules), t.emperor_dead[e.female as usize]);
                out.push(format!("{}{}", t.prefix, m.to_uppercase()));
            }
            seen.emperor = fief.emperor;
        }
        for m in out {
            self.add(m);
        }
    }

    /// `Afficher_Messages`, before drawing: drops the lines beyond 12, then the oldest one
    /// once it has been shown 0.16 s per character.
    fn expire(&mut self) {
        let now = get_time();
        if self.lines.len() > MAX_MESSAGES {
            self.lines.drain(..self.lines.len() - MAX_MESSAGES);
            self.last = now;
        }
        if let Some(first) = self.lines.first()
            && (now - self.last).abs() > crate::text::columns(first) as f64 * DURATION
        {
            self.lines.remove(0);
            self.last = now;
        }
    }
}

impl Hud {
    /// The message log's once-a-second update (`ActualiserLent`), after a tick.
    pub fn update_messages(&mut self, world: &World, rules: &Rules, player: usize) {
        if world.tick % 10 == 0 {
            self.messages.update(world, rules, player);
        }
    }

    /// Adds a tip to the message log; `pick` is a random number in [0, 1).
    pub fn add_tip(&mut self, pick: f64) {
        self.messages.add_tip(pick);
    }

    /// A line of the game's own (not the original's) in the message log, `&` for red.
    pub fn add_notice(&mut self, text: &str) {
        self.messages.add_text(text);
    }

    /// `Afficher_Messages`: the message log, under the rest of the interface.
    pub(super) fn draw_messages(&mut self) {
        self.messages.expire();
        // (the touch interface has no trays on the left to keep clear of)
        let x = if crate::layout::touch() { 8 } else { POSITION.0 };
        for (i, line) in self.messages.lines.iter().enumerate() {
            self.text(x, POSITION.1 + LINE * i as i32, line, WHITE_);
        }
    }
}
