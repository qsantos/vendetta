//! Music (`ClsPerMusique`): a random track of the mood's folder, a new one every 256 s.
//! Tracks are the original MIDI rendered to Ogg (see `assetconv`), fetched only when needed.

use crate::assets::Manifest;
use crate::storage::{self, Sounds};
use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound, set_sound_volume, stop_sound};
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::prelude::*;

/// `Musiques.ini` folders, by category number.
const FOLDERS: [&str; 9] = ["Menu", "Debut", "Ville", "Combat", "Mort", "Victoire", "Defaite", "Boss", "Empereur"];
pub const DEBUT: usize = 1;
pub const VILLE: usize = 2;
pub const COMBAT: usize = 3;
pub const MORT: usize = 4;
pub const VICTOIRE: usize = 5;
pub const DEFAITE: usize = 6;
pub const BOSS: usize = 7;
pub const EMPEREUR: usize = 8;
/// `IntervalMusiques`: seconds between two tracks.
const INTERVAL: f64 = 256.0;
const VOLUME: f32 = 0.6;

pub struct Music {
    /// URLs of the tracks of each category.
    tracks: Vec<Vec<String>>,
    category: Option<usize>,
    started: f64,
    playing: Option<Sound>,
    loading: Option<Coroutine<Option<Sound>>>,
    /// The player's volume the track plays at (see `storage::volume`).
    volume: f32,
}

impl Music {
    pub fn new(manifest: &Manifest) -> Music {
        let tracks = FOLDERS
            .iter()
            .map(|folder| {
                let prefix = format!("Musiques/{folder}/");
                manifest
                    .entries
                    .iter()
                    .filter(|(p, _)| p.starts_with(&prefix) && p.ends_with(".ogg"))
                    .filter_map(|(p, _)| manifest.url(p).map(str::to_owned))
                    .collect()
            })
            .collect();
        Music { tracks, category: None, started: 0.0, playing: None, loading: None, volume: 1.0 }
    }

    /// `Jouer_Musique`: a new random track of `category` if the current one has played its
    /// 256 s, or right away when the mood changes and the change is not optional. Combat
    /// does not interrupt a boss track.
    pub fn play(&mut self, category: usize, optional: bool) {
        if crate::options::test_mode() {
            return;
        }
        let optional = optional || (category == COMBAT && self.category == Some(BOSS));
        let due = get_time() > self.started + INTERVAL;
        if self.tracks[category].is_empty() || !(due || (self.category != Some(category) && !optional)) {
            return;
        }
        self.category = Some(category);
        self.started = get_time();
        self.stop();
        let url = self.tracks[category][rand::gen_range(0, self.tracks[category].len())].clone();
        self.loading = Some(start_coroutine(async move { load_sound(&url).await.ok() }));
    }

    // No music key: the original's `ToucheMusique` is unbound by default, and Ctrl+M mutes
    // the tab in browsers anyway.
    fn stop(&mut self) {
        if let Some(s) = self.playing.take() {
            stop_sound(&s);
        }
        self.loading = None;
    }

    /// Call every frame: starts a track once downloaded, follows the player's volume.
    pub fn update(&mut self) {
        let volume = storage::volume(Sounds::Music);
        if volume != self.volume {
            self.volume = volume;
            if let Some(s) = &self.playing {
                set_sound_volume(s, VOLUME * volume);
            }
        }
        if let Some(task) = &self.loading {
            if task.is_done() {
                if let Some(Some(sound)) = self.loading.take().and_then(|t| t.retrieve()) {
                    play_sound(&sound, PlaySoundParams { looped: false, volume: VOLUME * self.volume });
                    self.playing = Some(sound);
                }
            }
        }
    }
}
