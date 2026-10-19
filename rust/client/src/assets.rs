//! Loading game files and textures, from disk natively or over HTTP in the browser.

use crate::loading::Progress;
use macroquad::prelude::*;
use sim::files::{Files, key};
use std::collections::HashMap;

pub const ROOT: &str = "assets";

/// `manifest.txt`: every converted file with its size in bytes (`path<TAB>size`).
pub struct Manifest {
    pub entries: Vec<(String, u64)>,
    sizes: HashMap<String, u64>,
}

impl Manifest {
    pub async fn load() -> Result<Manifest, String> {
        let text = load_string(&format!("{ROOT}/manifest.txt")).await.map_err(|e| format!("manifest: {e}"))?;
        let entries: Vec<(String, u64)> = text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| match l.split_once('\t') {
                Some((p, s)) => (p.to_owned(), s.parse().unwrap_or(0)),
                None => (l.to_owned(), 0),
            })
            .collect();
        let sizes = entries.iter().map(|(p, s)| (key(p), *s)).collect();
        Ok(Manifest { entries, sizes })
    }

    pub fn size(&self, path: &str) -> u64 {
        self.sizes.get(&key(path)).copied().unwrap_or(0)
    }

    /// Data files the simulation reads at start (ini, txt, AI files and maps).
    fn is_data(path: &str) -> bool {
        let lower = path.to_lowercase();
        let wanted = lower.ends_with(".ini") || lower.ends_with(".txt") || lower.ends_with(".ia") || lower.starts_with("cartes/");
        wanted && !lower.ends_with("apercu.png") && !lower.ends_with("carte.png")
    }

    pub fn data_bytes(&self) -> u64 {
        self.entries.iter().filter(|(p, _)| Self::is_data(p)).map(|(_, s)| s).sum()
    }

    /// Bytes of the images the game loads at start, before the world exists to say exactly:
    /// the folders it always uses. Keep in sync with the estimate in `index.html`.
    pub fn image_estimate(&self) -> u64 {
        const FOLDERS: [&str; 7] = [
            "Images/Batiments/",
            "Images/Boutons/",
            "Images/Decors/",
            "Images/Terrains/",
            "Images/Fenetres/Fenetre-",
            "Images/Ressources/",
            "Images/Curseurs/",
        ];
        self.entries.iter().filter(|(p, _)| FOLDERS.iter().any(|f| p.starts_with(f))).map(|(_, s)| s).sum()
    }
}

/// Loads every data file the simulation needs into memory, counting bytes into `progress`.
/// Sprite images are loaded as textures instead.
pub async fn load_files(manifest: &Manifest, progress: &Progress) -> Result<Files, String> {
    let label = progress.label(1);
    let mut files = Files::new();
    for (path, size) in &manifest.entries {
        files.add_listing(path);
        if !Manifest::is_data(path) {
            continue;
        }
        let data = load_file(&format!("{ROOT}/{path}")).await.map_err(|e| format!("{path}: {e}"))?;
        files.insert(path, data);
        progress.add(*size, &label);
    }
    Ok(files)
}

/// Texture cache keyed like `Files` (case-insensitive, `.bmp` → `.png`).
#[derive(Default)]
pub struct Textures {
    real_paths: HashMap<String, String>,
    loaded: HashMap<String, Texture2D>,
}

impl Textures {
    pub fn new(manifest: &Manifest) -> Textures {
        let real_paths = manifest
            .entries
            .iter()
            .filter(|(p, _)| p.ends_with(".png"))
            .map(|(p, _)| (key(p), p.clone()))
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

    /// Images among `paths` not loaded yet, without duplicates.
    pub fn missing(&self, paths: &[String]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        paths
            .iter()
            .filter(|p| {
                let k = key(p);
                !self.loaded.contains_key(&k) && self.real_paths.contains_key(&k) && seen.insert(k)
            })
            .cloned()
            .collect()
    }

    /// Loads a list of images, counting their bytes into `progress`.
    pub async fn load_all(&mut self, manifest: &Manifest, paths: &[String], progress: &Progress, message: &str) {
        for p in self.missing(paths) {
            self.load(&p).await;
            progress.add(manifest.size(&p), message);
        }
    }

    pub fn get(&self, path: &str) -> Option<&Texture2D> {
        self.loaded.get(&key(path))
    }
}
