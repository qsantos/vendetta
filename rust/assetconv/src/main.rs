//! Converts an installed copy of the original game into web-friendly assets.
//!
//! usage: assetconv GAME_DIR OUT_DIR
//!
//! - `Images/**/*.bmp` become PNGs quantised to RGB565 like the original's 16-bit display,
//!   where black (the DirectDraw colour key) is transparent. Checkerboard dithers, which the original used to fake translucency,
//!   are replaced with real alpha.
//! - Other BMPs (map data) become opaque PNGs, pixel for pixel.
//! - Text files (ini, txt, ia) are re-encoded from Latin-1 to UTF-8.
//! - Everything else useful (wav, mid, gif) is copied as is.
//! - `manifest.txt` lists every output file, since the browser cannot list directories.

use image::{Rgba, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} GAME_DIR OUT_DIR", args[0]);
        std::process::exit(2);
    }
    let src = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let mut files = Vec::new();
    walk(&src, &mut files);
    files.sort();

    let mut manifest = Vec::new();
    for path in &files {
        let rel = path.strip_prefix(&src).unwrap();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let ext = rel
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let dst_rel = match ext.as_str() {
            "bmp" => rel.with_extension("png"),
            "ini" | "txt" | "ia" | "wav" | "mid" | "gif" => rel.to_path_buf(),
            _ => continue,
        };
        if fs::metadata(path).unwrap().len() == 0 {
            // e.g. Images/Terrains/Carte1.bmp, a cache the original regenerates
            continue;
        }
        let dst = out.join(&dst_rel);
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        match ext.as_str() {
            "bmp" => {
                let img = match image::open(path) {
                    Ok(img) => img.to_rgba8(),
                    Err(e) => {
                        eprintln!("skipping {rel_str}: {e}");
                        continue;
                    }
                };
                let img = if rel_str.starts_with("Images/") {
                    convert_sprite(&rel_str, img)
                } else {
                    img
                };
                img.save(&dst).unwrap();
            }
            "ini" | "txt" | "ia" => {
                let bytes = fs::read(path).unwrap();
                let (text, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(&bytes);
                fs::write(&dst, text.as_bytes()).unwrap();
            }
            _ => {
                fs::copy(path, &dst).unwrap();
            }
        }
        manifest.push(dst_rel.to_string_lossy().replace('\\', "/"));
    }
    manifest.sort();
    fs::write(out.join("manifest.txt"), manifest.join("\n") + "\n").unwrap();
    eprintln!("wrote {} files", manifest.len());
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// The original runs in 16-bit colour (RGB565) with a zero colour key, so every colour is
/// quantised and anything that quantises to black is transparent.
fn convert_sprite(rel: &str, mut img: RgbaImage) -> RgbaImage {
    for p in img.pixels_mut() {
        let (r, g, b) = (p[0] & 0xF8, p[1] & 0xFC, p[2] & 0xF8);
        *p = if r == 0 && g == 0 && b == 0 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([r | r >> 5, g | g >> 6, b | b >> 5, 255])
        };
    }
    if rel.starts_with("Images/Temps/") {
        flatten_dither(&mut img);
    } else {
        smooth_checkerboard(&mut img);
    }
    img
}

/// The day/night overlays are a uniform ordered dither of a single colour; turn them into
/// that colour at the equivalent opacity.
fn flatten_dither(img: &mut RgbaImage) {
    let (mut n, mut r, mut g, mut b) = (0u64, 0u64, 0u64, 0u64);
    for p in img.pixels() {
        if p[3] != 0 {
            n += 1;
            r += p[0] as u64;
            g += p[1] as u64;
            b += p[2] as u64;
        }
    }
    if n == 0 {
        return;
    }
    let total = img.width() as u64 * img.height() as u64;
    let colour = Rgba([
        (r / n) as u8,
        (g / n) as u8,
        (b / n) as u8,
        ((255 * n + total / 2) / total) as u8,
    ]);
    for p in img.pixels_mut() {
        *p = colour;
    }
}

/// Replace 50% checkerboard areas (each pixel's 4 neighbours have the opposite opacity and
/// its 4 diagonal neighbours the same one) with the average colour at half opacity.
fn smooth_checkerboard(img: &mut RgbaImage) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let opaque = |img: &RgbaImage, x: i64, y: i64| img.get_pixel(x as u32, y as u32)[3] != 0;
    let mut checker = vec![false; (w * h) as usize];
    let mut any = false;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let o = opaque(img, x, y);
            let straight = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|&(dx, dy)| opaque(img, x + dx, y + dy) != o);
            let diagonal = [(1, 1), (-1, 1), (1, -1), (-1, -1)]
                .iter()
                .all(|&(dx, dy)| opaque(img, x + dx, y + dy) == o);
            if straight && diagonal {
                checker[(y * w + x) as usize] = true;
                any = true;
            }
        }
    }
    if !any {
        return;
    }
    let src = img.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if !checker[(y * w + x) as usize] {
                continue;
            }
            let (mut n, mut r, mut g, mut b) = (0u32, 0u32, 0u32, 0u32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = src.get_pixel((x + dx) as u32, (y + dy) as u32);
                    if p[3] != 0 {
                        n += 1;
                        r += p[0] as u32;
                        g += p[1] as u32;
                        b += p[2] as u32;
                    }
                }
            }
            if n > 0 {
                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgba([(r / n) as u8, (g / n) as u8, (b / n) as u8, 128]),
                );
            }
        }
    }
}
