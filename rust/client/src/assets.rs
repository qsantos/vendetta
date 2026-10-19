//! Loading game files and textures, from disk natively or over HTTP in the browser.

use crate::loading::Progress;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
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
    for (path, _) in &manifest.entries {
        files.add_listing(path);
    }
    // Owned values: the borrowing closures would not be `Send` for the coroutine.
    let wanted: Vec<(String, u64)> = manifest.entries.iter().filter(|(p, _)| Manifest::is_data(p)).cloned().collect();
    let jobs = wanted
        .into_iter()
        .map(|(path, size)| -> Pin<Box<dyn Future<Output = _> + Send>> {
            Box::pin(async move {
                let data = load_file(&format!("{ROOT}/{path}")).await;
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

    /// Loads a list of images, several at once, counting their bytes into `progress`.
    pub async fn load_all(&mut self, manifest: &Manifest, paths: &[String], progress: &Progress, message: &str) {
        let jobs = self
            .missing(paths)
            .into_iter()
            .map(|p| -> Pin<Box<dyn Future<Output = _> + Send>> {
                let url = format!("{ROOT}/{}", self.real_paths[&key(&p)]);
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
}
