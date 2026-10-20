#!/usr/bin/env python3
"""Draws "Chargement en cours"-style text, in any language, onto the loading screens.

The original loading screens (`Images/Chargement/*.png`) have "Chargement en cours" baked into
the band under the painting: rounded letters shaded like stone tubes lit from the upper right,
with a dark outline, a speckled grey-blue face and a palette of a few dozen colours. This
redraws that effect for any text:

    python3 -I tools/loading_text.py draw OUT.png "Loading" [--font FONT] [--onto SCREEN.png]

The text is fitted into the band (x-height of the original, at most its width), centred like
the original. With `--onto`, the band of that screen is replaced; otherwise only the band is
written (1024×96, transparent background).

    python3 -I tools/loading_text.py build Donnees/Langues.ini Images/Chargement OUT_DIR

writes every screen in every language of `Langues.ini` but the original's French, as
`OUT_DIR/<screen>-<suffix>.png` (e.g. `Usine-EN.png`), with the text of `TEXTS`; only those
older than their screen or this script are redrawn.
"""

import argparse
import os
import re

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

# The band under the painting, and the original text's metrics in it (pixels of the screen).
BAND_TOP, BAND_H, WIDTH = 672, 96, 1024
BASELINE = 746 - BAND_TOP
X_HEIGHT = 50
MAX_WIDTH = 900
SCALE = 4  # supersampling

# The original's colours, darkest to lightest, sampled from its letters.
OUTLINE = (8, 4, 8)
RIM = (90, 69, 66)
RAMP = [(16, 20, 8), (24, 28, 8), (33, 28, 16), (57, 44, 40), (90, 85, 74), (115, 109, 99), (132, 125, 115), (156, 154, 148)]
HIGHLIGHT = (140, 150, 165)

LATIN = "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf"
CJK = "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc"
# "Chargement en cours" by language (`Langues.ini` `Suffixe`): text, font, face in the font.
# The CJK collection's faces: 0 Japanese, 1 Korean, 2 simplified Chinese, 3 traditional.
TEXTS = {
    "EN": ("Loading…", LATIN, 0),
    "ES": ("Cargando…", LATIN, 0),
    "DE": ("Wird geladen…", LATIN, 0),
    "IT": ("Caricamento…", LATIN, 0),
    "ZH": ("加载中…", CJK, 2),
    "JA": ("読み込み中…", CJK, 0),
}


def text_mask(text, font_path, index=0):
    """The text's coverage at SCALE, sized like the original and positioned in the band."""
    size = 100
    font = ImageFont.truetype(font_path, size * SCALE, index=index)
    # x-height from an "x" when the script has one; CJK glyphs sit on the em box instead
    xb = font.getbbox("x")
    latin = all(ord(c) < 0x2E80 for c in text)
    x_height = (xb[3] - xb[1]) if latin else font.getbbox("中")[3] - font.getbbox("中")[1]
    target = X_HEIGHT if latin else X_HEIGHT * 1.5
    k = target * SCALE / x_height
    font = ImageFont.truetype(font_path, int(size * SCALE * k), index=index)
    box = font.getbbox(text)
    if box[2] - box[0] > MAX_WIDTH * SCALE:
        k2 = MAX_WIDTH * SCALE / (box[2] - box[0])
        font = ImageFont.truetype(font_path, int(size * SCALE * k * k2), index=index)
        box = font.getbbox(text)
    w, h = WIDTH * SCALE, BAND_H * SCALE
    im = Image.new("L", (w, h), 0)
    x = (w - (box[2] - box[0])) / 2 - box[0]
    if latin:
        # baseline where the original's is
        y = BASELINE * SCALE - font.getmetrics()[0]
    else:
        # the glyphs' box centred where the original's x-height band is
        y = (BASELINE - X_HEIGHT / 2) * SCALE - (box[1] + box[3]) / 2
    ImageDraw.Draw(im).text((x, y), text, font=font, fill=255)
    return np.asarray(im).astype(np.float32) / 255


def render(text, font_path, index=0, seed=1, palette=True):
    """The band (RGBA, WIDTH×BAND_H) with `text` in the original's style."""
    rng = np.random.default_rng(seed)
    cover = text_mask(text, font_path, index)
    inside = cover > 0.5
    # distance to the letter's edge, in screen pixels
    d = ndimage.distance_transform_edt(inside) / SCALE
    # a rounded tube: steep at the edge, flat inside; as round as the strokes are thick (thin
    # strokes, as in Chinese or Japanese, would turn to mush)
    radius = min(4.5, 0.85 * np.percentile(d[inside], 97))
    t = np.clip(d / radius, 0, 1)
    height = np.sqrt(1 - (1 - t) ** 2)
    height = ndimage.gaussian_filter(height, SCALE * 0.6)
    gy, gx = np.gradient(height * radius * SCALE)
    normal = np.dstack([-gx, -gy, np.full_like(gx, 1.2)])
    normal /= np.linalg.norm(normal, axis=2, keepdims=True)
    light = np.array([0.55, -0.55, 0.62])  # from the upper right (y goes down)
    light /= np.linalg.norm(light)
    shade = np.clip((normal * light).sum(2), 0, 1)
    # stone: low-frequency mottling and a fine grain
    mottle = ndimage.gaussian_filter(rng.standard_normal(shade.shape), SCALE * 3)
    mottle /= mottle.std() + 1e-9
    grain = ndimage.gaussian_filter(rng.standard_normal(shade.shape), SCALE * 0.5)
    grain /= grain.std() + 1e-9
    # deep shadows on the side away from the light, a broad lit face
    value = shade**1.25 * 1.08 + 0.06 * mottle + 0.04 * grain
    ramp = np.array(RAMP, np.float32)
    pos = np.clip(value, 0, 1) * (len(ramp) - 1)
    lo = np.floor(pos).astype(int).clip(0, len(ramp) - 2)
    f = (pos - lo)[..., None]
    colour = ramp[lo] * (1 - f) + ramp[lo + 1] * f
    # grey-blue speckles where the face is lit
    speck = (rng.random(shade.shape) < 0.1) & (shade > 0.7)
    speck = ndimage.binary_dilation(speck, iterations=SCALE // 2)
    colour[speck] = colour[speck] * 0.4 + np.array(HIGHLIGHT) * 0.6
    # the rim just inside the edge, then the dark outline around it
    rim = inside & (d < 1.6)
    colour[rim] = colour[rim] * 0.35 + np.array(RIM) * 0.65
    outline = ndimage.distance_transform_edt(~inside) / SCALE
    ring = (~inside) & (outline < 1.2)
    colour[ring] = OUTLINE
    alpha = (inside | ring).astype(np.float32)
    # down to screen pixels
    rgba = np.dstack([colour * alpha[..., None], alpha * 255])
    im = Image.fromarray(rgba.clip(0, 255).astype(np.uint8), "RGBA").resize((WIDTH, BAND_H), Image.BOX)
    a = np.asarray(im).astype(np.float32)
    # un-premultiply, and hard edges like the original (its black became transparent)
    rgb = a[..., :3] / np.maximum(a[..., 3:], 1) * 255
    keep = a[..., 3] > 110
    rgb[~keep] = 0
    out = Image.fromarray(np.dstack([rgb, keep * 255]).clip(0, 255).astype(np.uint8), "RGBA")
    if palette:
        # a few dozen colours with dithering, like the original's GIF-era palette
        q = out.convert("RGB").quantize(72, dither=Image.Dither.FLOYDSTEINBERG).convert("RGB")
        out = Image.fromarray(np.dstack([np.asarray(q), keep * 255]).astype(np.uint8), "RGBA")
    return out


def onto(screen_path, band):
    """The screen with its band replaced by `band`."""
    screen = Image.open(screen_path).convert("RGBA")
    screen.paste((0, 0, 0, 0), (0, BAND_TOP, WIDTH, BAND_TOP + BAND_H))
    screen.alpha_composite(band, (0, BAND_TOP))
    return screen


def suffixes(langues_ini):
    """The languages' suffixes in `Langues.ini`."""
    with open(langues_ini, encoding="utf-8") as f:
        return [m.group(1).strip().upper() for m in re.finditer(r"(?im)^\s*Suffixe\s*=\s*(\S+)", f.read())]


def build(langues_ini, screens, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    pngs = sorted(f for f in os.listdir(screens) if f.lower().endswith(".png"))
    here = os.path.getmtime(__file__)
    for suffix in suffixes(langues_ini):
        if suffix == "FR":
            continue  # the original's
        if suffix not in TEXTS:
            raise SystemExit(f"no loading text for language {suffix}: add it to TEXTS in {__file__}")
        text, font, index = TEXTS[suffix]
        band = None
        for png in pngs:
            src = os.path.join(screens, png)
            dst = os.path.join(out_dir, f"{png[:-4]}-{suffix}.png")
            if os.path.exists(dst) and os.path.getmtime(dst) > max(os.path.getmtime(src), here):
                continue
            if band is None:
                band = render(text, font, index)
            onto(src, band).save(dst)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="command", required=True)
    d = sub.add_parser("draw", help="one text")
    d.add_argument("out")
    d.add_argument("text")
    d.add_argument("--font", default=LATIN)
    d.add_argument("--index", type=int, default=0, help="face in a font collection (.ttc)")
    d.add_argument("--onto", help="a loading screen whose band to replace")
    d.add_argument("--seed", type=int, default=1)
    d.add_argument("--no-palette", action="store_true")
    b = sub.add_parser("build", help="every screen in every language")
    b.add_argument("languages", help="Donnees/Langues.ini")
    b.add_argument("screens", help="Images/Chargement")
    b.add_argument("out")
    a = p.parse_args()
    if a.command == "build":
        build(a.languages, a.screens, a.out)
        return
    band = render(a.text, a.font, a.index, a.seed, not a.no_palette)
    (onto(a.onto, band) if a.onto else band).save(a.out)


if __name__ == "__main__":
    main()
