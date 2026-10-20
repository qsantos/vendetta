//! Converts an installed copy of the original game into web-friendly assets.
//!
//! usage: assetconv GAME_DIR OUT_DIR
//!
//! - `Images/**/*.bmp` become PNGs quantised to RGB565 like the original's 16-bit display,
//!   where black (the DirectDraw colour key) is transparent. Checkerboard dithers, which the original used to fake translucency,
//!   are replaced with real alpha.
//! - Other BMPs (map data) become opaque PNGs, pixel for pixel.
//! - Text files (ini, txt, ia) are re-encoded from Latin-1 to UTF-8.
//! - Everything else useful (wav, mid, gif) is copied as is. MIDI music is also rendered to
//!   Ogg Vorbis (browsers cannot play MIDI) with a General MIDI soundfont and `ffmpeg`.
//! - `manifest.txt` lists every output file with its size and a content hash
//!   (`path<TAB>bytes<TAB>hash`): the browser cannot list directories, shows loading progress
//!   in bytes, and requests `path?v=hash` so that files can be cached forever.

use image::{Rgba, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("usage: {} GAME_DIR OUT_DIR [SOUNDFONT.sf2]", args[0]);
        std::process::exit(2);
    }
    let src = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let soundfont_path = args.get(3).cloned().unwrap_or_else(|| DEFAULT_SOUNDFONT.to_owned());
    let soundfont = match fs::File::open(&soundfont_path).map_err(|e| e.to_string()).and_then(|mut f| {
        rustysynth::SoundFont::new(&mut f).map_err(|e| format!("{e:?}"))
    }) {
        Ok(sf) => Some(std::sync::Arc::new(sf)),
        Err(e) => {
            eprintln!("no music rendering ({soundfont_path}: {e})");
            None
        }
    };
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
                save_png(&img, &dst);
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
        record(&mut manifest, &dst_rel, &dst);
        if let (Some(sf), "mid") = (&soundfont, ext.as_str()) {
            let ogg_rel = dst_rel.with_extension("ogg");
            let ogg = out.join(&ogg_rel);
            match render_midi(sf, &dst, &ogg) {
                Ok(()) => record(&mut manifest, &ogg_rel, &ogg),
                Err(e) => eprintln!("skipping {}: {e}", ogg_rel.display()),
            }
        }
    }
    manifest.sort();
    fs::write(out.join("manifest.txt"), manifest.join("\n") + "\n").unwrap();
    eprintln!("wrote {} files", manifest.len());
}

/// Writes an image as a PNG made as small as it losslessly can be (palettes, fewer channels,
/// best filters: about a fifth of the size of a plain encoding, for the browser to download).
fn save_png(img: &RgbaImage, dst: &Path) {
    let raw = oxipng::RawImage::new(img.width(), img.height(), oxipng::ColorType::RGBA, oxipng::BitDepth::Eight, img.as_raw().clone()).unwrap();
    let options = oxipng::Options { optimize_alpha: false, ..oxipng::Options::from_preset(4) };
    fs::write(dst, raw.create_optimized_png(&options).unwrap()).unwrap();
}

/// The manifest line of an output file: `path<TAB>size<TAB>hash`.
fn record(manifest: &mut Vec<String>, rel: &Path, path: &Path) {
    let content = fs::read(path).unwrap();
    let (size, hash) = (content.len(), fnv1a(&content));
    manifest.push(format!("{}\t{size}\t{hash:016x}", rel.to_string_lossy().replace('\\', "/")));
}

/// Debian's General MIDI soundfont (TimGM6mb), close in spirit to the Windows synthesizer the
/// original played its MIDI music through.
const DEFAULT_SOUNDFONT: &str = "/usr/share/sounds/sf2/default-GM.sf2";
const SAMPLE_RATE: i32 = 44100;

/// Renders a MIDI file to Ogg Vorbis: synthesise it, normalise it to -1 dBFS, and encode it
/// with `ffmpeg`.
fn render_midi(soundfont: &std::sync::Arc<rustysynth::SoundFont>, mid: &Path, ogg: &Path) -> Result<(), String> {
    use rustysynth::{MidiFile, MidiFileSequencer, Synthesizer, SynthesizerSettings};
    use std::io::Write;
    let settings = SynthesizerSettings::new(SAMPLE_RATE);
    let synth = Synthesizer::new(soundfont, &settings).map_err(|e| format!("{e:?}"))?;
    let midi = MidiFile::new(&mut fs::File::open(mid).map_err(|e| e.to_string())?).map_err(|e| format!("{e:?}"))?;
    let midi = std::sync::Arc::new(midi);
    let mut sequencer = MidiFileSequencer::new(synth);
    sequencer.play(&midi, false);
    // the whole piece plus two seconds for the last notes to fade
    let len = (SAMPLE_RATE as f64 * (midi.get_length() + 2.0)) as usize;
    let (mut left, mut right) = (vec![0f32; len], vec![0f32; len]);
    sequencer.render(&mut left, &mut right);
    let peak = left.iter().chain(&right).fold(0f32, |m, v| m.max(v.abs()));
    let gain = if peak > 0.0 { 0.89 / peak } else { 1.0 };
    let pcm: Vec<u8> = left.iter().zip(&right).flat_map(|(l, r)| [l * gain, r * gain]).flat_map(f32::to_le_bytes).collect();
    let mut ffmpeg = std::process::Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-f", "f32le", "-ar", &SAMPLE_RATE.to_string(), "-ac", "2", "-i", "-"])
        .args(["-c:a", "libvorbis", "-q:a", "2"])
        .arg(ogg)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    ffmpeg.stdin.take().unwrap().write_all(&pcm).map_err(|e| e.to_string())?;
    match ffmpeg.wait() {
        Ok(status) if status.success() => Ok(()),
        _ => Err("ffmpeg failed".into()),
    }
}

/// 64-bit FNV-1a: a stable content hash, only used to version URLs.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
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
    // The test above misses each area's edge pixels, whose outer neighbours break the
    // pattern, leaving a dotted outline. Add the edge pixels that continue the pattern: a
    // transparent pixel enclosed by opaque ones, or an opaque one of the same colour as a
    // dithered pixel diagonal to it. (A transparent pixel outside a sprite has transparent
    // neighbours, so no halo grows around dithered sprite edges.)
    for _ in 0..2 {
        let mut grown = checker.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let k = (y * w + x) as usize;
                let in_checker = |dx: i64, dy: i64| checker[((y + dy) * w + x + dx) as usize];
                if checker[k] || ![(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| in_checker(dx, dy)) {
                    continue;
                }
                let p = *img.get_pixel(x as u32, y as u32);
                grown[k] = if p[3] == 0 {
                    [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().filter(|&&(dx, dy)| opaque(img, x + dx, y + dy)).count() >= 3
                } else {
                    [(1, 1), (-1, 1), (1, -1), (-1, -1)].iter().any(|&(dx, dy)| {
                        let q = img.get_pixel((x + dx) as u32, (y + dy) as u32);
                        in_checker(dx, dy) && q[3] != 0 && q.0[..3] == p.0[..3]
                    })
                };
            }
        }
        checker = grown;
    }
    let src = img.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if !checker[(y * w + x) as usize] {
                continue;
            }
            // Average the dithered pixels around (not the frame next to the area).
            let (mut n, mut r, mut g, mut b) = (0u32, 0u32, 0u32, 0u32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = src.get_pixel((x + dx) as u32, (y + dy) as u32);
                    if p[3] != 0 && checker[((y + dy) * w + x + dx) as usize] {
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
