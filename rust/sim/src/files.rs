//! Read-only access to the converted game data (see `assetconv`).
//!
//! Paths use '/' and are matched case-insensitively, like on Windows where the original
//! ran. The original's `.bmp` references resolve to the converted `.png` files.

use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct Files {
    by_key: HashMap<String, Vec<u8>>,
    /// Files known to exist (e.g. images, whose content only the renderer loads).
    listed: BTreeSet<String>,
}

impl Files {
    pub fn new() -> Files {
        Files::default()
    }

    /// Loads every file under `root` (native builds only; the browser fetches the manifest).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_dir(root: &std::path::Path) -> std::io::Result<Files> {
        fn walk(files: &mut Files, root: &std::path::Path, dir: &std::path::Path) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    walk(files, root, &path)?;
                } else {
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                    files.insert(&rel, std::fs::read(&path)?);
                }
            }
            Ok(())
        }
        let mut files = Files::new();
        walk(&mut files, root, root)?;
        Ok(files)
    }

    pub fn insert(&mut self, path: &str, data: Vec<u8>) {
        self.listed.insert(key(path));
        self.by_key.insert(key(path), data);
    }

    /// Records that a file exists without loading its content.
    pub fn add_listing(&mut self, path: &str) {
        self.listed.insert(key(path));
    }

    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.by_key.get(&key(path)).map(Vec::as_slice)
    }

    pub fn text(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn exists(&self, path: &str) -> bool {
        self.listed.contains(&key(path))
    }

    /// Paths (normalised keys) of every file directly or indirectly under `dir`, sorted
    /// case-insensitively like the original's `Dir` listing on NTFS.
    pub fn list(&self, dir: &str) -> Vec<String> {
        let prefix = key(dir).trim_end_matches('/').to_owned() + "/";
        self.listed.range(prefix.clone()..).take_while(|k| k.starts_with(&prefix)).cloned().collect()
    }

    /// Files directly in `dir` (not in sub-folders).
    pub fn list_files(&self, dir: &str) -> Vec<String> {
        let prefix = key(dir).trim_end_matches('/').to_owned() + "/";
        self.list(dir).into_iter().filter(|k| !k[prefix.len()..].contains('/')).collect()
    }
}

pub fn key(path: &str) -> String {
    let p = path.replace('\\', "/").to_lowercase();
    let p = p.trim_start_matches('/');
    match p.strip_suffix(".bmp") {
        Some(stem) => format!("{stem}.png"),
        None => p.to_owned(),
    }
}
