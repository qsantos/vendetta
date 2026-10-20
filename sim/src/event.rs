//! What a tick produced that only the client shows: floating numbers (`AffDegat`), visual
//! effects with their sounds (`Activer_Effet`) and what characters say (`Commentaires`). They
//! have no effect on the game and are not saved; what is said is in no language, so that the
//! game is the same whatever each player reads.

use crate::vb::clng;
use crate::world::World;

/// The colour of a floating number (`Ajouter_Degat`'s `TypeDegat`, `Combat.ini [Degats]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    /// 0, `CouleurDegat` (red): life lost.
    Damage,
    /// 1, `CouleurGueri` (blue): life gained.
    Heal,
    /// 2, `CouleurVente` (yellow): money paid or earned.
    Money,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `AffDegat.Ajouter_Degat`: `value` floats up from (x, y), the middle of the text (the
    /// client applies `Combat.ini [Degats]`: the offsets, the font size, bigger when
    /// `critical`). Values that round to 0 are not emitted, like the original.
    Number { x: i64, y: i64, value: i64, kind: NumberKind, critical: bool },
    /// `AffDegat.Ajouter_Texte`: a line in the colour of `kind`, e.g. the name of an effect
    /// caught on a hit (damage); (x, y) as for `Number`, the original also shifts it 4 px right
    /// and 10 px up.
    Text { x: i64, y: i64, text: String, kind: NumberKind },
    /// `Activer_Effet`: animation `effect` (0-based, `Parametres_Effets.ini`) centred on the
    /// box (x, y, w, h). The original plays sound `sound` (the same index, `Effet_Son`) when
    /// the animation shows on screen.
    Effect { effect: usize, sound: usize, x: i64, y: i64, w: i64, h: i64 },
    /// `Definir_Commentaires`: perso `perso` says `line` (only characters that speak). Like the
    /// original, the client shows it unless the perso is already saying something, which an
    /// empty line ([`Line::None`]) stops instead; a line stays `Commentaires.ini [Commentaires]
    /// Duree` per character.
    Say { perso: usize, line: Line },
    /// Dying: whatever perso `perso` was saying stops.
    Silence { perso: usize },
}

/// What a character says.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// Nothing: stops the current line.
    None,
    /// Line `index` of comment type `kind` (`Commentaires.ini [Commentaires_kind] Message(index+1)`).
    Comment { kind: usize, index: usize },
    /// A text of its own.
    Text(String),
}

impl World {
    /// `AffDegat.Ajouter_Degat`: positions are Longs and the value is `CLng`-rounded.
    pub(crate) fn number(&mut self, x: f64, y: f64, value: f64, kind: NumberKind, critical: bool) {
        let value = clng(value);
        if value != 0 {
            self.events.push(Event::Number { x: clng(x), y: clng(y), value, kind, critical });
        }
    }

    /// `AffDegat.Ajouter_Texte`
    pub(crate) fn text(&mut self, x: f64, y: f64, text: String, kind: NumberKind) {
        self.events.push(Event::Text { x: clng(x), y: clng(y), text, kind });
    }

    /// `Activer_Effet`: nothing for a negative effect.
    pub(crate) fn effect(&mut self, effect: i64, x: f64, y: f64, w: f64, h: f64) {
        if let Ok(effect) = usize::try_from(effect) {
            let (x, y, w, h) = (clng(x), clng(y), clng(w), clng(h));
            self.events.push(Event::Effect { effect, sound: effect, x, y, w, h });
        }
    }

    /// Perso `i` says `line`, if it speaks.
    pub(crate) fn speak(&mut self, i: usize, line: Line) {
        if self.persos[i].speaks {
            self.events.push(Event::Say { perso: i, line });
        }
    }

    /// `Activer_Effet` on perso `i`'s box.
    pub(crate) fn effect_on(&mut self, effect: i64, i: usize) {
        let p = &self.persos[i];
        let (x, y, w, h) = (p.x as f64, p.y as f64, p.w as f64, p.h as f64);
        self.effect(effect, x, y, w, h);
    }
}
