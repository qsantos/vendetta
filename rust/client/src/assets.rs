//! Loading game files and textures, from disk natively or over HTTP in the browser.

use crate::loading::Progress;
use macroquad::prelude::*;
use sim::files::{Files, key};
use std::collections::HashMap;

pub const ROOT: &str = "assets";

/// Loads every data file the simulation needs (ini, txt and map images) into memory. Sprite
/// images are loaded on demand as textures instead.
pub async fn load_files(manifest: &str, progress: &Progress, pct: (f64, f64)) -> Result<Files, String> {
    let label = progress.label(1);
    let total = manifest.lines().count().max(1);
    let mut files = Files::new();
    for (i, path) in manifest.lines().enumerate() {
        progress.set(pct.0 + (pct.1 - pct.0) * i as f64 / total as f64, &label);
        let lower = path.to_lowercase();
        files.add_listing(path);
        let wanted = lower.ends_with(".ini") || lower.ends_with(".txt") || lower.ends_with(".ia") || lower.starts_with("cartes/");
        if !wanted || lower.ends_with("apercu.png") || lower.ends_with("carte.png") {
            continue;
        }
        let data = load_file(&format!("{ROOT}/{path}"))
            .await
            .map_err(|e| format!("{path}: {e}"))?;
        files.insert(path, data);
    }
    Ok(files)
}

/// Texture cache keyed like `Files` (case-insensitive, `.bmp` → `.png`).
#[derive(Default)]
pub struct Textures {
    real_paths: HashMap<String, String>,
    loaded: HashMap<String, Texture2D>,
}

pub async fn load_manifest() -> Result<String, String> {
    load_string(&format!("{ROOT}/manifest.txt")).await.map_err(|e| format!("manifest: {e}"))
}

impl Textures {
    pub fn new(manifest: &str) -> Textures {
        let real_paths = manifest
            .lines()
            .filter(|p| p.ends_with(".png"))
            .map(|p| (key(p), p.to_owned()))
            .collect();
        Textures { real_paths, loaded: HashMap::new() }
    }

    /// Loads (once) the texture for an original image path such as `Images\Decors\Pont.bmp`.
    pub async fn load(&mut self, path: &str) -> Option<Texture2D> {
        let k = key(path);
        if let Some(t) = self.loaded.get(&k) {
            return Some(t.clone());
        }
        let real = self.real_paths.get(&k)?;
        let t = load_texture(&format!("{ROOT}/{real}")).await.ok()?;
        t.set_filter(FilterMode::Nearest);
        self.loaded.insert(k, t.clone());
        Some(t)
    }

    /// Loads a list of images, showing progress from `pct.0` to `pct.1`.
    pub async fn load_all(&mut self, paths: &[String], progress: &Progress, pct: (f64, f64), message: &str) {
        for (i, p) in paths.iter().enumerate() {
            progress.set(pct.0 + (pct.1 - pct.0) * i as f64 / paths.len().max(1) as f64, message);
            self.load(p).await;
        }
    }

    pub fn get(&self, path: &str) -> Option<&Texture2D> {
        self.loaded.get(&key(path))
    }
}
