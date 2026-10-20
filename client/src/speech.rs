//! `Afficher_Commentaires`: what characters are saying. The simulation tells who says which
//! line ([`Event::Say`]); the lines, in the player's language, and how long they show are kept
//! here.

use sim::event::{Event, Line};
use sim::rules::Rules;
use sim::world::World;

/// Each perso's line ("" for none) and the tick it was said.
#[derive(Default)]
pub struct Speech {
    lines: Vec<(String, u64)>,
}

impl Speech {
    /// After a tick: the lines said during it, then those that have been shown long enough go.
    pub fn tick(&mut self, world: &World, rules: &Rules) {
        self.lines.resize(world.persos.len(), Default::default());
        for event in &world.events {
            match event {
                Event::Say { perso, line } => {
                    let text = match line {
                        Line::None => String::new(),
                        Line::Comment { kind, index } => rules.comments.get(*kind).and_then(|l| l.get(*index)).cloned().unwrap_or_default(),
                        Line::Text(t) => t.clone(),
                    };
                    // `Definir_Commentaires`: a new line only replaces an empty one.
                    let (current, said) = &mut self.lines[*perso];
                    if current.is_empty() || text.is_empty() {
                        if !text.is_empty() {
                            *said = world.tick;
                        }
                        *current = text;
                    }
                }
                Event::Silence { perso } => self.lines[*perso].0.clear(),
                _ => {}
            }
        }
        // A line stays `Duree` seconds per character.
        let duration = rules.comment_duration * world.speed as f64;
        for (text, said) in &mut self.lines {
            if !text.is_empty() && (world.tick - *said) as f64 > crate::text::columns(text) as f64 * duration {
                text.clear();
            }
        }
    }

    /// What perso `i` is saying ("" for nothing).
    pub fn line(&self, i: usize) -> &str {
        self.lines.get(i).map_or("", |l| l.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::files::Files;
    use sim::world::WorldSetup;
    use std::path::Path;

    /// A line shows until `Duree` per character has passed, and another line does not replace
    /// it meanwhile; dying stops it.
    #[test]
    fn lines_show_then_expire() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        if !root.exists() {
            eprintln!("no assets, skipping");
            return;
        }
        let files = Files::from_dir(&root).unwrap();
        let rules = Rules::load(&files, "Francais").unwrap();
        let mut w = World::create(&rules, &files, &WorldSetup { seed: 1, ..Default::default() }).unwrap();
        for p in &mut w.persos {
            p.ai = false;
        }
        let i = (0..w.persos.len()).find(|&i| w.persos[i].speaks && w.persos[i].special_comments <= 0).unwrap();
        let mut speech = Speech::default();
        let say = |w: &mut World, line: Line| w.events = vec![Event::Say { perso: i, line }];
        say(&mut w, Line::Comment { kind: 16, index: 0 });
        speech.tick(&w, &rules);
        let line = rules.comments[16][0].clone();
        assert_eq!(speech.line(i), line);
        say(&mut w, Line::Comment { kind: 0, index: 0 });
        speech.tick(&w, &rules);
        assert_eq!(speech.line(i), line, "a new line does not replace the current one");
        let ticks = (crate::text::columns(&line) as f64 * rules.comment_duration * w.speed as f64) as usize + 2;
        for _ in 0..ticks {
            w.step(&rules, &[]);
            w.events.retain(|e| !matches!(e, Event::Say { .. } | Event::Silence { .. }));
            speech.tick(&w, &rules);
        }
        assert_eq!(speech.line(i), "");
        say(&mut w, Line::Text("Bonjour".to_owned()));
        speech.tick(&w, &rules);
        assert_eq!(speech.line(i), "Bonjour");
        w.events = vec![Event::Silence { perso: i }];
        speech.tick(&w, &rules);
        assert_eq!(speech.line(i), "");
    }
}
