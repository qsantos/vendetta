//! Loading game files and textures, from disk natively or over HTTP in the browser.

use crate::loading::Progress;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::prelude::*;
use sim::files::{Files, key};
use std::collections::HashMap;

pub const ROOT: &str = "assets";

/// Downloads in flight at once. Each request costs a round trip; overlapping them hides
/// most of that latency (browsers queue beyond their own per-server limit).
const PARALLEL: usize = 16;

/// Runs `jobs` with at most `PARALLEL` in flight, handing each result to `done` as it
/// completes. Every pending job is polled whenever this is: macroquad's downloads never
/// wake their waker, they rely on being polled each frame.
async fn run_parallel<T>(jobs: Vec<Pin<Box<dyn Future<Output = T> + Send>>>, mut done: impl FnMut(T)) {
    let mut queue = jobs.into_iter();
    let mut running: Vec<Pin<Box<dyn Future<Output = T> + Send>>> = Vec::new();
    std::future::poll_fn(|cx| {
        while running.len() < PARALLEL {
            match queue.next() {
                Some(job) => running.push(job),
                None => break,
            }
        }
        let mut i = 0;
        while i < running.len() {
            if let Poll::Ready(v) = running[i].as_mut().poll(cx) {
                drop(running.swap_remove(i));
                done(v);
                if let Some(job) = queue.next() {
                    running.push(job);
                }
            } else {
                i += 1;
            }
        }
        if running.is_empty() { Poll::Ready(()) } else { Poll::Pending }
    })
    .await
}

/// `manifest.txt`: every converted file with its size in bytes and content hash
/// (`path<TAB>size<TAB>hash`).
pub struct Manifest {
    pub entries: Vec<(String, u64)>,
    /// By `key`: (size, URL). The URL carries the hash (`?v=`), so the server can let
    /// browsers cache it forever: a changed file gets a new URL.
    by_key: HashMap<String, (u64, String)>,
}

impl Manifest {
    pub async fn load() -> Result<Manifest, String> {
        let text = load_string(&format!("{ROOT}/manifest.txt")).await.map_err(|e| format!("manifest: {e}"))?;
        let mut entries = Vec::new();
        let mut by_key = HashMap::new();
        for line in text.lines().filter(|l| !l.is_empty()) {
            let mut fields = line.split('\t');
            let path = fields.next().unwrap_or_default().to_owned();
            let size = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            // Natively the files are read from disk, where a query would break the path.
            let url = match fields.next() {
                Some(hash) if cfg!(target_arch = "wasm32") => format!("{ROOT}/{path}?v={hash}"),
                _ => format!("{ROOT}/{path}"),
            };
            by_key.insert(key(&path), (size, url));
            entries.push((path, size));
        }
        Ok(Manifest { entries, by_key })
    }

    pub fn size(&self, path: &str) -> u64 {
        self.by_key.get(&key(path)).map_or(0, |e| e.0)
    }

    /// The versioned URL of a file (looked up case-insensitively, `.bmp` → `.png`).
    pub fn url(&self, path: &str) -> Option<&str> {
        self.by_key.get(&key(path)).map(|e| e.1.as_str())
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
    for (path, _) in &manifest.entries {
        files.add_listing(path);
    }
    // Owned values: the borrowing closures would not be `Send` for the coroutine.
    let wanted: Vec<(String, u64)> = manifest.entries.iter().filter(|(p, _)| Manifest::is_data(p)).cloned().collect();
    let jobs = wanted
        .into_iter()
        .map(|(path, size)| -> Pin<Box<dyn Future<Output = _> + Send>> {
            let url = manifest.url(&path).unwrap_or_default().to_owned();
            Box::pin(async move {
                let data = load_file(&url).await;
                (path, size, data)
            })
        })
        .collect();
    let mut error = None;
    run_parallel(jobs, |(path, size, data)| match data {
        Ok(data) => {
            files.insert(&path, data);
            progress.add(size, &label);
        }
        Err(e) => error = Some(format!("{path}: {e}")),
    })
    .await;
    match error {
        Some(e) => Err(e),
        None => Ok(files),
    }
}

/// Texture cache keyed like `Files` (case-insensitive, `.bmp` → `.png`).
#[derive(Default)]
pub struct Textures {
    /// Versioned URL of every image, by key.
    urls: HashMap<String, String>,
    loaded: HashMap<String, Texture2D>,
    /// The appearance sheets (`Images/Apparences/...`) by bare file name ("hut.bmp"), as
    /// objects name their overlays.
    appearances: HashMap<String, String>,
    /// Images requested during the game (summons, projectiles, effects), by key.
    pending: HashMap<String, Coroutine<Option<Texture2D>>>,
}

impl Textures {
    pub fn new(manifest: &Manifest) -> Textures {
        let urls = manifest
            .entries
            .iter()
            .filter(|(p, _)| p.ends_with(".png"))
            .map(|(p, _)| (key(p), manifest.url(p).unwrap_or_default().to_owned()))
            .collect();
        let appearances = manifest
            .entries
            .iter()
            .filter(|(p, _)| p.starts_with("Images/Apparences/") && p.ends_with(".png"))
            .map(|(p, _)| (p.rsplit('/').next().unwrap_or(p).to_lowercase().replace(".png", ".bmp"), p.clone()))
            .collect();
        Textures { urls, appearances, loaded: HashMap::new(), pending: HashMap::new() }
    }

    /// Images among `paths` not loaded yet, without duplicates.
    pub fn missing(&self, paths: &[String]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        paths
            .iter()
            .filter(|p| {
                let k = key(p);
                !self.loaded.contains_key(&k) && self.urls.contains_key(&k) && seen.insert(k)
            })
            .cloned()
            .collect()
    }

    /// Loads a list of images, several at once, counting their bytes into `progress`.
    pub async fn load_all(&mut self, manifest: &Manifest, paths: &[String], progress: &Progress, message: &str) {
        let jobs = self
            .missing(paths)
            .into_iter()
            .map(|p| -> Pin<Box<dyn Future<Output = _> + Send>> {
                let url = self.urls[&key(&p)].clone();
                Box::pin(async move { (p, load_texture(&url).await) })
            })
            .collect();
        let loaded = &mut self.loaded;
        run_parallel(jobs, |(p, texture)| {
            if let Ok(t) = texture {
                t.set_filter(FilterMode::Nearest);
                loaded.insert(key(&p), t);
            }
            progress.add(manifest.size(&p), message);
        })
        .await;
    }

    pub fn get(&self, path: &str) -> Option<&Texture2D> {
        self.loaded.get(&key(path))
    }

    /// The path of the appearance sheet an object names (`FichierApparence`).
    pub fn appearance(&self, name: &str) -> Option<&str> {
        self.appearances.get(&name.to_lowercase()).map(String::as_str)
    }

    /// Starts downloading an image needed during the game, if it is not loaded or loading.
    pub fn request(&mut self, path: &str) {
        let k = key(path);
        if self.loaded.contains_key(&k) || self.pending.contains_key(&k) {
            return;
        }
        if let Some(url) = self.urls.get(&k).cloned() {
            let task = start_coroutine(async move { load_texture(&url).await.ok() });
            self.pending.insert(k, task);
        }
    }

    /// Call every frame: keeps the requested images that finished downloading.
    pub fn poll(&mut self) {
        let done: Vec<String> = self.pending.iter().filter(|(_, t)| t.is_done()).map(|(k, _)| k.clone()).collect();
        for k in done {
            if let Some(Some(t)) = self.pending.remove(&k).and_then(|t| t.retrieve()) {
                t.set_filter(FilterMode::Nearest);
                self.loaded.insert(k, t);
            }
        }
    }
}
