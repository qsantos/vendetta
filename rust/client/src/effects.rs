//! What the simulation's events show: visual effects (`ClsJeuEffet`, `ClsAffEffet`) with their
//! sounds (`ClsPerSon`), and floating numbers (`ClsAffDegats`). Images and sounds are fetched
//! the first time an effect needs them.

use crate::assets::{Manifest, Textures};
use crate::text::TextRenderer;
use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound};
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::prelude::*;
use sim::event::{Event, NumberKind};
use sim::files::Files;
use sim::params::{Kind, Params};
use sim::vb::clng;
use std::collections::HashMap;

/// `Parametres_Effets.ini [Effet_n]`.
struct EffectType {
    image: String,
    w: i32,
    h: i32,
    /// `NombreEtapes`
    steps: f64,
    /// `DureeEtape`: ticks per frame.
    step_duration: f64,
    /// URL of `Sons\<Son>`.
    sound: Option<String>,
}

struct Active {
    kind: usize,
    x: i32,
    y: i32,
    /// `Temps`: the frame, advanced by `1 / DureeEtape` per tick.
    time: f64,
    sounded: bool,
}

/// `ClsAffDegats`: a number or text rising from where it appeared.
struct Floating {
    x: i32,
    y: i32,
    text: String,
    colour: Color,
    critical: bool,
    /// `Duree`, in ticks.
    age: i32,
}

enum SoundState {
    Loading(Coroutine<Option<Sound>>),
    Ready(Sound),
    Failed,
}

/// `Combat.ini [Degats]`: `DureeMax`, `DecalageY`, `TailleNormale`, `TailleCritique`.
const NUMBER_TICKS: i32 = 15;
const NUMBER_OFFSET_Y: i64 = -6;
const NUMBER_SIZE: i64 = 8;
const CRITICAL_SIZE: i64 = 10;
const SOUND_VOLUME: f32 = 0.8;

pub struct Effects {
    types: Vec<EffectType>,
    active: Vec<Active>,
    floating: Vec<Floating>,
    sounds: HashMap<usize, SoundState>,
    /// When each effect type's sound last started: DirectSound had one buffer per type, so a
    /// sound does not restart while it plays (approximated as 0.5 s).
    last_sound: HashMap<usize, f64>,
    font: TextRenderer,
    critical_font: TextRenderer,
}

impl Effects {
    pub fn new(files: &Files, manifest: &Manifest) -> Effects {
        let types = Params::load(files, "Francais")
            .map(|params| {
                params
                    .list(Kind::Effets)
                    .iter()
                    .map(|r| {
                        let num = |k: &str| r.get(k).and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
                        let sound = r.get("Son").map(str::trim).filter(|s| !s.is_empty());
                        EffectType {
                            image: format!("Images/Effets/{}.bmp", r.get("Fichier").unwrap_or("").trim()),
                            w: num("Largeur") as i32,
                            h: num("Hauteur") as i32,
                            steps: num("NombreEtapes"),
                            step_duration: num("DureeEtape").max(1.0),
                            sound: sound.and_then(|s| manifest.url(&format!("Sons/{s}")).map(str::to_owned)),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ttf = include_bytes!("../fonts/LiberationSans-Bold.ttf");
        Effects {
            types,
            active: Vec::new(),
            floating: Vec::new(),
            sounds: HashMap::new(),
            last_sound: HashMap::new(),
            font: TextRenderer::new(ttf, NUMBER_SIZE as f32 * 96.0 / 72.0),
            critical_font: TextRenderer::new(ttf, CRITICAL_SIZE as f32 * 96.0 / 72.0),
        }
    }

    /// After each tick: advances what is shown, then adds what the tick produced.
    pub fn tick(&mut self, events: &[Event], textures: &mut Textures) {
        let types = &self.types;
        self.active.retain_mut(|a| {
            let t = &types[a.kind];
            a.time += 1.0 / t.step_duration;
            a.time < t.steps
        });
        self.floating.retain_mut(|f| {
            f.y -= 1;
            f.age += 1;
            f.age <= NUMBER_TICKS
        });
        for e in events {
            match e {
                Event::Number { x, y, value, kind, critical } => {
                    let text = value.to_string();
                    let size = if *critical { CRITICAL_SIZE } else { NUMBER_SIZE };
                    let colour = match kind {
                        NumberKind::Damage => Color::from_rgba(255, 0, 0, 255),
                        NumberKind::Heal => Color::from_rgba(0, 0, 255, 255),
                        NumberKind::Money => Color::from_rgba(255, 255, 0, 255),
                    };
                    let x = x - text.len() as i64 * size / 2;
                    let y = y + NUMBER_OFFSET_Y;
                    self.floating.push(Floating { x: x as i32, y: y as i32, text, colour, critical: *critical, age: 0 });
                }
                Event::Text { x, y, text } => {
                    let x = x + 4 - text.chars().count() as i64 * NUMBER_SIZE / 2;
                    let colour = Color::from_rgba(255, 0, 0, 255);
                    self.floating.push(Floating { x: x as i32, y: (y - 16) as i32, text: text.clone(), colour, critical: false, age: 0 });
                }
                Event::Effect { effect, sound: _, x, y, w, h } => {
                    let Some(t) = self.types.get(*effect) else { continue };
                    textures.request(&t.image);
                    if let (Some(url), false) = (&t.sound, self.sounds.contains_key(effect)) {
                        let url = url.clone();
                        let task = start_coroutine(async move { load_sound(&url).await.ok() });
                        self.sounds.insert(*effect, SoundState::Loading(task));
                    }
                    // Centred on the target box (`ClsJeuEffet.cls:40-55`).
                    let ex = x + clng((*w - t.w as i64) as f64 / 2.0);
                    let ey = y + clng((*h - t.h as i64) as f64 / 2.0);
                    self.active.push(Active { kind: *effect, x: ex as i32, y: ey as i32, time: 0.0, sounded: false });
                }
            }
        }
    }

    /// Draws effects and numbers over the map (world pixels; `zoom` screen pixels each), and
    /// plays an effect's sound when it shows its frame 1 on screen.
    pub fn draw(&mut self, textures: &Textures, cam: (i32, i32), view: (i32, i32), zoom: f32) {
        self.poll_sounds();
        let on_screen = |x: i32, y: i32, w: i32, h: i32| x + w > cam.0 && y + h > cam.1 && x < cam.0 + view.0 && y < cam.1 + view.1;
        for a in &mut self.active {
            let t = &self.types[a.kind];
            // `CInt(Temps)`: VB rounds half to even.
            let frame = (clng(a.time) as f64).min(t.steps - 1.0).max(0.0);
            if let Some(tex) = textures.get(&t.image) {
                let src = Rect::new(frame as f32 * t.w as f32, 0.0, t.w as f32, t.h as f32);
                let params = DrawTextureParams { source: Some(src), ..Default::default() };
                draw_texture_ex(tex, (a.x - cam.0) as f32, (a.y - cam.1) as f32, WHITE, params);
            }
            if !a.sounded && frame >= 1.0 && on_screen(a.x, a.y, t.w, t.h) {
                a.sounded = true;
                let now = get_time();
                let recent = self.last_sound.get(&a.kind).is_some_and(|&s| now - s < 0.5);
                if let (Some(SoundState::Ready(s)), false) = (self.sounds.get(&a.kind), recent) {
                    play_sound(s, PlaySoundParams { looped: false, volume: SOUND_VOLUME });
                    self.last_sound.insert(a.kind, now);
                }
            }
        }
        self.font.set_scale(zoom);
        self.critical_font.set_scale(zoom);
        for f in &self.floating {
            let font = if f.critical { &self.critical_font } else { &self.font };
            let size = if f.critical { CRITICAL_SIZE } else { NUMBER_SIZE } as i32;
            if !on_screen(f.x, f.y, 1, size) {
                continue;
            }
            // Text drawn at (x, y) has its baseline about 1.1 em lower (spec 05 §0.3).
            font.draw(&f.text, f.x - cam.0, f.y - cam.1 + size * 11 / 10, f.colour);
        }
    }

    fn poll_sounds(&mut self) {
        for state in self.sounds.values_mut() {
            if let SoundState::Loading(task) = state {
                if task.is_done() {
                    *state = match task.retrieve() {
                        Some(Some(s)) => SoundState::Ready(s),
                        _ => SoundState::Failed,
                    };
                }
            }
        }
    }
}
