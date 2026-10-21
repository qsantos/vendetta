//! Loading game files and textures, from disk natively or over HTTP in the browser.

use crate::atlas::Shelf;
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

/// How long loading may work before letting the page breathe (draw, answer its menus): the
/// game loads behind the page's menus, on the same thread.
const SLICE: f64 = 0.008;

/// Lets the page have the rest of this frame: loading goes on at the next one (the loading
/// coroutine is polled once per frame).
pub async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn(|_| {
        if std::mem::replace(&mut yielded, true) { Poll::Ready(()) } else { Poll::Pending }
    })
    .await
}

/// Runs `jobs` with at most `PARALLEL` in flight, handing each result to `done` as it
/// completes. Every pending job is polled whenever this is: macroquad's downloads never
/// wake their waker, they rely on being polled each frame.
async fn run_parallel<T>(jobs: Vec<Pin<Box<dyn Future<Output = T> + Send>>>, mut done: impl FnMut(T)) {
    let mut queue = jobs.into_iter();
    let mut running: Vec<Pin<Box<dyn Future<Output = T> + Send>>> = Vec::new();
    std::future::poll_fn(|cx| {
        // results handled at most for a slice of time per frame (cached files all come at once)
        let start = crate::perf::now();
        while running.len() < PARALLEL {
            match queue.next() {
                Some(job) => running.push(job),
                None => break,
            }
        }
        let mut i = 0;
        while i < running.len() && crate::perf::now() - start < SLICE {
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
    /// Every file, those of the languages `only_language` leaves out included.
    all: Vec<(String, u64)>,
    /// By `key`: (size, URL). The URL carries the hash (`?v=`), so the server can let
    /// browsers cache it forever: a changed file gets a new URL.
    by_key: HashMap<String, (u64, String)>,
}

/// What loading an image counts for on the loading bar, by its file's size: decoding it and
/// packing it into the atlas takes about 2.5 times as long as taking as many bytes of data.
pub fn image_work(bytes: u64) -> u64 {
    bytes * 5 / 2
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
        Ok(Manifest { all: entries.clone(), entries, by_key })
    }

    /// Leaves out the files of the languages other than `lang` (their folders in `Donnees/` and
    /// `Donnees/ModsOfficiels/`, their font): the game never reads them, and the page does not
    /// download them (see bundle.py).
    pub fn only_language(&mut self, languages: &[sim::rules::Language], lang: &str) {
        let others: Vec<String> = languages.iter().filter(|l| l.folder != lang).flat_map(|l| language_prefixes(&l.folder)).collect();
        self.entries.retain(|(p, _)| {
            let p = p.to_lowercase();
            !others.iter().any(|o| p.starts_with(o))
        });
    }

    /// The data files of language `lang` (its folder), with their URLs, for a change of
    /// language during the game (see `load_language`).
    pub fn language_files(&self, lang: &str) -> Vec<(String, String)> {
        let prefixes = language_prefixes(lang);
        self.all
            .iter()
            .filter(|(p, _)| Self::is_data(p) && prefixes.iter().any(|o| p.to_lowercase().starts_with(o)))
            .map(|(p, _)| (p.clone(), self.url(p).unwrap_or_default().to_owned()))
            .collect()
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
        let wanted = lower.ends_with(".ini")
            || lower.ends_with(".txt")
            || lower.ends_with(".ia")
            || lower.starts_with("cartes/")
            || lower.starts_with("sons/");
        wanted && !lower.ends_with("apercu.png") && !lower.ends_with("carte.png")
    }

    pub fn data_bytes(&self) -> u64 {
        self.entries.iter().filter(|(p, _)| Self::is_data(p)).map(|(_, s)| s).sum()
    }

    /// Bytes of the images the game loads at start, before the world exists to say exactly:
    /// the folders it always uses. Keep in sync with the estimate in `index.html`.
    pub fn image_estimate(&self) -> u64 {
        const FOLDERS: [&str; 10] = [
            "Images/Batiments/",
            "Images/Persos/",
            "Images/Apparences/",
            "Images/Effets/",
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

/// The start of the paths of a language's files (lower case): its folders in `Donnees/` and
/// `Donnees/ModsOfficiels/`, its font.
fn language_prefixes(folder: &str) -> [String; 3] {
    [format!("Donnees/{folder}/"), format!("Donnees/ModsOfficiels/{folder}/"), format!("Polices/{folder}.")].map(|p| p.to_lowercase())
}

/// A language's data files (`Manifest::language_files`), downloaded during the game, and its
/// font, if it has one.
pub async fn load_language(wanted: Vec<(String, String)>, font: Option<String>) -> Result<(Vec<(String, Vec<u8>)>, Option<Vec<u8>>), String> {
    let jobs = wanted
        .into_iter()
        .map(|(path, url)| -> Pin<Box<dyn Future<Output = _> + Send>> { Box::pin(async move { (load_file(&url).await, path) }) })
        .collect();
    let mut loaded = Vec::new();
    let mut error = None;
    run_parallel(jobs, |(data, path)| match data {
        Ok(data) => loaded.push((path, data)),
        Err(e) => error = Some(format!("{path}: {e}")),
    })
    .await;
    if let Some(e) = error {
        return Err(e);
    }
    let font = match font {
        Some(url) => Some(load_file(&url).await.map_err(|e| format!("{url}: {e}"))?),
        None => None,
    };
    Ok((loaded, font))
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

/// Side of an atlas page. WebGL guarantees at least 4096; the images the game loads fit in two.
const PAGE: u16 = crate::sharp::PAGE as u16;

/// An image, wherever it was packed: a part of an atlas page.
#[derive(Clone)]
pub struct Sprite {
    page: Texture2D,
    rect: Rect,
}

impl Sprite {
    /// `draw_texture_ex` for this image: `params.source` is a part of the image (in whole
    /// pixels); `params.dest_size` is honoured, not rotations or flips. Sampled sharp
    /// bilinear (see `sharp`), never outside `source`.
    pub fn draw_ex(&self, x: f32, y: f32, colour: Color, params: DrawTextureParams) {
        let source = match params.source {
            Some(s) => Rect::new(self.rect.x + s.x, self.rect.y + s.y, s.w, s.h),
            None => self.rect,
        };
        if source.w <= 0.0 || source.h <= 0.0 {
            return;
        }
        let dest = params.dest_size.unwrap_or(vec2(source.w, source.h));
        let normal = crate::sharp::normal(source, dest);
        let page = PAGE as f32;
        let corner = |dx: f32, dy: f32, u: f32, v: f32| Vertex {
            position: vec3(x + dx, y + dy, 0.0),
            uv: vec2(u / page, v / page),
            color: colour.into(),
            normal,
        };
        let vertices = [
            corner(0.0, 0.0, source.x, source.y),
            corner(dest.x, 0.0, source.right(), source.y),
            corner(dest.x, dest.y, source.right(), source.bottom()),
            corner(0.0, dest.y, source.x, source.bottom()),
        ];
        let gl = unsafe { get_internal_gl() }.quad_gl;
        gl.texture(Some(&self.page));
        gl.draw_mode(DrawMode::Triangles);
        gl.geometry(&vertices, &[0, 1, 2, 0, 2, 3]);
    }

    /// `draw_texture` for this image.
    pub fn draw(&self, x: f32, y: f32, colour: Color) {
        self.draw_ex(x, y, colour, DrawTextureParams::default());
    }
}

/// Images keyed like `Files` (case-insensitive, `.bmp` → `.png`). They are packed at load time
/// into big textures (atlases), so that the map and the interface switch textures rarely; the
/// files themselves stay one per image, as mods expect.
#[derive(Default)]
pub struct Textures {
    /// Versioned URL of every image, by key.
    urls: HashMap<String, String>,
    /// The appearance sheets (`Images/Apparences/...`) by bare file name ("hut.bmp"), as
    /// objects name their overlays.
    appearances: HashMap<String, String>,
    loaded: HashMap<String, Sprite>,
    /// The pages, the last one being filled.
    pages: Vec<(Texture2D, Shelf)>,
    /// Pixels kept on the CPU: the terrain tileset, which the zoomed-out overview averages.
    pixels: HashMap<String, Image>,
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
        Textures { urls, appearances, ..Default::default() }
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

    /// Loads a list of images, several at once, counting their bytes into `progress`, then
    /// packs them into the atlas pages, tallest first.
    pub async fn load_all(&mut self, manifest: &Manifest, paths: &[String], progress: &Progress, message: &str) {
        let jobs = self
            .missing(paths)
            .into_iter()
            .map(|p| -> Pin<Box<dyn Future<Output = _> + Send>> {
                let url = self.urls[&key(&p)].clone();
                Box::pin(async move { (p, load_file(&url).await) })
            })
            .collect();
        let mut images = Vec::new();
        run_parallel(jobs, |(p, bytes)| {
            progress.add(image_work(manifest.size(&p)), message);
            match bytes.map_err(|e| e.to_string()).and_then(|b| {
                Image::from_file_with_format(&b, Some(ImageFormat::Png)).map_err(|e| e.to_string())
            }) {
                Ok(image) => images.push((key(&p), image)),
                Err(e) => {
                    warn!("{p}: {e}");
                }
            }
        })
        .await;
        images.sort_by_key(|(k, image)| (std::cmp::Reverse(image.height), k.clone()));
        let mut start = crate::perf::now();
        for (k, image) in images {
            self.pack(k, image);
            if crate::perf::now() - start > SLICE {
                yield_now().await;
                start = crate::perf::now();
            }
        }
    }

    fn pack(&mut self, k: String, image: Image) {
        let (w, h) = (image.width, image.height);
        let at = self.pages.last_mut().and_then(|(_, shelf)| shelf.place(w, h));
        let (x, y) = match at {
            Some(at) => at,
            None => {
                let page = Texture2D::from_rgba8(PAGE, PAGE, &vec![0; PAGE as usize * PAGE as usize * 4]);
                // sampled sharp bilinear (see `sharp`)
                page.set_filter(FilterMode::Linear);
                let mut shelf = Shelf::new(PAGE);
                let Some(at) = shelf.place(w, h) else {
                    warn!("{k}: {w}×{h} is bigger than an atlas page");
                    return;
                };
                self.pages.push((page, shelf));
                at
            }
        };
        let page = self.pages.last().unwrap().0.clone();
        page.update_part(&image, x as i32, y as i32, w as i32, h as i32);
        let rect = Rect::new(x as f32, y as f32, w as f32, h as f32);
        if k.starts_with("images/terrains/") {
            self.pixels.insert(k.clone(), image);
        }
        self.loaded.insert(k, Sprite { page, rect });
    }

    pub fn get(&self, path: &str) -> Option<&Sprite> {
        self.loaded.get(&key(path))
    }

    /// The pixels of an image kept on the CPU (only the terrain tileset).
    pub fn pixels(&self, path: &str) -> Option<&Image> {
        self.pixels.get(&key(path))
    }

    /// The path of the appearance sheet an object names (`FichierApparence`).
    pub fn appearance(&self, name: &str) -> Option<&str> {
        self.appearances.get(&name.to_lowercase()).map(String::as_str)
    }
}
