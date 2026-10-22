#!/usr/bin/env python3
"""Draws the victory and defeat banner (`Images/Fenetres/VictoireDefaite*.png`) in any language.

The original banner is 320×400: "Victoire" over "Appuyez sur Echap pour quitter" in the top
half, "Défaite" over the same line in the bottom half. Its title letters are bevelled, lit from
the upper left, light green with darker veins, in a beige glow fading to black; the line under
them is smaller and plainer. The whole of it is dithered to a checkerboard, to be seen through
(the asset converter turns that into half opacity). This redraws that for any text:

    python3 -I tools/banner_text.py draw OUT.png VICTORY DEFEAT LINE [--suffix XX]

writes the banner as the game loads it (RGBA, half opacity, colours of a 16-bit display), and

    python3 -I tools/banner_text.py build ASSETS

those of every language of `ASSETS/Donnees/Langues.ini`, as
`ASSETS/Images/Fenetres/Banniere<suffix>.png`, listed in `ASSETS/manifest.txt`; only those
older than this script are redrawn.
"""

import argparse
import os
import re

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

W, H = 320, 400
SCALE = 4  # supersampling
# The original's metrics in each half (pixels): the title's baseline and cap height, the line's
# baseline and cap height, both centred on CENTRE, at most MAX_WIDTH wide.
TITLE_BASELINE, TITLE_CAP = 116, 60
LINE_BASELINE, LINE_CAP = 145, 12
CENTRE = 158
MAX_WIDTH = 300

# The original's colours, sampled from it: the title's face from its shadowed side to its lit
# one, its veins, its glow; the line's face, top to bottom.
FACE = [(41, 57, 33), (66, 82, 41), (109, 133, 72), (156, 189, 99), (176, 209, 127), (189, 220, 144), (198, 222, 156), (214, 232, 178)]
VEIN = (60, 80, 40)
GLOW = (150, 150, 115)
LINE = [(200, 226, 150), (156, 189, 99), (90, 115, 55)]

LATIN = "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf"
# the line under the title, thinner like the original's
LATIN_LINE = "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf"
CJK = "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc"
# By language (`Langues.ini` `Suffixe`): victory, defeat, the line under them, font, face in
# the font. The CJK collection's faces: 0 Japanese, 1 Korean, 2 simplified Chinese.
TEXTS = {
    "FR": ("Victoire", "Défaite", "Appuyez sur Echap pour quitter", LATIN, 0),
    "EN": ("Victory", "Defeat", "Press Escape to quit", LATIN, 0),
    "ES": ("Victoria", "Derrota", "Pulsa Escape para salir", LATIN, 0),
    "DE": ("Sieg", "Niederlage", "Escape zum Beenden", LATIN, 0),
    "IT": ("Vittoria", "Sconfitta", "Premi Esc per uscire", LATIN, 0),
    "JA": ("勝利", "敗北", "Escキーで終了", CJK, 0),
    "ZH": ("胜利", "失败", "按 Esc 键退出", CJK, 2),
}


def cjk(text):
    return any(ord(c) >= 0x2E80 for c in text)


def text_mask(text, font_path, index, baseline, cap, max_width):
    """The text's coverage at SCALE in a W×200 half, sized and placed like the original's."""
    size = 100
    font = ImageFont.truetype(font_path, size * SCALE, index=index)
    if cjk(text):
        # CJK glyphs sit on the em box: as tall as the original's capitals and a bit more
        b = font.getbbox("中")
        target = cap * 1.25
    else:
        b = font.getbbox("H")
        target = cap
    k = target * SCALE / (b[3] - b[1])
    font = ImageFont.truetype(font_path, max(1, int(size * SCALE * k)), index=index)
    box = font.getbbox(text)
    if box[2] - box[0] > max_width * SCALE:
        k *= max_width * SCALE / (box[2] - box[0])
        font = ImageFont.truetype(font_path, max(1, int(size * SCALE * k)), index=index)
        box = font.getbbox(text)
    im = Image.new("L", (W * SCALE, 200 * SCALE), 0)
    x = CENTRE * SCALE - (box[0] + box[2]) / 2
    if cjk(text):
        # the glyphs' box on the baseline, a little below as the original's descenders
        y = baseline * SCALE - box[3] + cap * SCALE * 0.08
    else:
        y = baseline * SCALE - font.getmetrics()[0]
    ImageDraw.Draw(im).text((x, y), text, font=font, fill=255)
    return np.asarray(im).astype(np.float32) / 255


def ramp(colours, v):
    r = np.array(colours, np.float32)
    pos = np.clip(v, 0, 1) * (len(r) - 1)
    lo = np.floor(pos).astype(int).clip(0, len(r) - 2)
    f = (pos - lo)[..., None]
    return r[lo] * (1 - f) + r[lo + 1] * f


def shade(inside, radius):
    """How lit a bevel of `radius` pixels is, lit from the upper left (0 to 1)."""
    d = ndimage.distance_transform_edt(inside) / SCALE
    t = np.clip(d / radius, 0, 1)
    height = ndimage.gaussian_filter(np.sqrt(1 - (1 - t) ** 2), SCALE * 0.5)
    gy, gx = np.gradient(height * radius * SCALE)
    normal = np.dstack([-gx, -gy, np.full_like(gx, 1.0)])
    normal /= np.linalg.norm(normal, axis=2, keepdims=True)
    light = np.array([-0.6, -0.6, 0.55])
    light /= np.linalg.norm(light)
    return (normal * light).sum(2), d


# How lit a flat face is.
FLAT = 0.55 / np.linalg.norm([-0.6, -0.6, 0.55])


def glow(inside, reach, strength):
    """The glow around the letters, a pixel's outline fading out: the colour's weight."""
    out = ndimage.distance_transform_edt(~inside) / SCALE
    return np.where(inside, 0, strength * np.where(out < 1, 1, np.exp(-(out - 1) / reach)) * (out < 1 + reach * 3.5))


def title(text, font_path, index, rng):
    cover = text_mask(text, font_path, index, TITLE_BASELINE, TITLE_CAP, MAX_WIDTH)
    inside = cover > 0.5
    # as round as the strokes are thick (thin strokes, as in Chinese or Japanese, would turn
    # to mush)
    d = ndimage.distance_transform_edt(inside) / SCALE
    lit, d = shade(inside, min(5.0, 0.85 * np.percentile(d[inside], 97)))
    # the flat face light green, the edges toward the light near white, those away dark green
    value = 0.72 + 2.2 * (lit - FLAT)
    mottle = ndimage.gaussian_filter(rng.standard_normal(inside.shape), SCALE * 4)
    value += 0.05 * mottle / (mottle.std() + 1e-9)
    colour = ramp(FACE, value)
    # veins: thin lines where a smooth noise crosses zero
    n = ndimage.gaussian_filter(rng.standard_normal(inside.shape), SCALE * 5)
    n /= n.std() + 1e-9
    vein = (np.abs(n) < 0.05) & (d > 2.5)
    vein = ndimage.binary_dilation(vein, iterations=SCALE // 2)
    colour[vein] = colour[vein] * 0.3 + np.array(VEIN) * 0.7
    weight = glow(inside, 0.9, 1.0)
    colour = np.where(inside[..., None], colour, np.array(GLOW, np.float32) * weight[..., None])
    return colour


def line(text, font_path, index, rng):
    cover = text_mask(text, font_path, index, LINE_BASELINE, LINE_CAP, MAX_WIDTH)
    inside = cover > 0.5
    lit, d = shade(inside, 1.2)
    ys = np.nonzero(inside.any(1))[0]
    top, bottom = (ys.min(), ys.max()) if len(ys) else (0, 1)
    v = (np.arange(inside.shape[0])[:, None] - top) / max(1, bottom - top)
    colour = ramp(LINE, np.broadcast_to(v, inside.shape) * 0.8 + 0.25 * (0.6 - lit))
    grain = ndimage.gaussian_filter(rng.standard_normal(inside.shape), SCALE * 0.7)
    colour *= (1 + 0.08 * grain / (grain.std() + 1e-9))[..., None]
    weight = glow(inside, 0.6, 0.35)
    return np.where(inside[..., None], colour, np.array(GLOW, np.float32) * weight[..., None])


def half(victory_or_defeat, under, font_path, index, seed):
    rng = np.random.default_rng(seed)
    c = title(victory_or_defeat, font_path, index, rng)
    l = line(under, font_path if cjk(under) else LATIN_LINE, index, rng)
    c = np.maximum(c, l)
    # down to screen pixels
    return np.asarray(Image.fromarray(c.clip(0, 255).astype(np.uint8), "RGB").resize((W, 200), Image.BOX)).astype(np.float32)


def solid(victory, defeat, under, font_path=LATIN, index=0, seed=1):
    """The banner before the checkerboard (RGB, black around the letters)."""
    return np.concatenate([half(victory, under, font_path, index, seed), half(defeat, under, font_path, index, seed + 1)])


def as_loaded(rgb):
    """As the game loads the original after the asset converter: a 16-bit display's colours,
    half opaque where the checkerboard was, transparent where black."""
    q = rgb.clip(0, 255).astype(np.uint8)
    q = np.dstack([q[..., 0] & 0xF8, q[..., 1] & 0xFC, q[..., 2] & 0xF8])
    keep = q.max(2) >= 16
    return Image.fromarray(np.dstack([q * keep[..., None], keep * 128]).astype(np.uint8), "RGBA")


def checkerboard(rgb):
    """As the original file: every other pixel black (for comparisons)."""
    yy, xx = np.mgrid[:H, :W]
    q = rgb.clip(0, 255).astype(np.uint8).copy()
    q[(xx + yy) % 2 == 1] = 0
    return Image.fromarray(q, "RGB")


def suffixes(langues_ini):
    with open(langues_ini, encoding="utf-8") as f:
        return [m.group(1).strip().upper() for m in re.finditer(r"(?im)^\s*Suffixe\s*=\s*(\S+)", f.read())]


def fnv1a(data):
    """assetconv's content hash."""
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def build(assets):
    here = os.path.getmtime(__file__)
    written = []
    for suffix in suffixes(os.path.join(assets, "Donnees", "Langues.ini")):
        if suffix not in TEXTS:
            raise SystemExit(f"no banner text for language {suffix}: add it to TEXTS in {__file__}")
        rel = f"Images/Fenetres/Banniere{suffix}.png"
        dst = os.path.join(assets, rel)
        written.append(rel)
        if os.path.exists(dst) and os.path.getmtime(dst) > here:
            continue
        victory, defeat, under, font, index = TEXTS[suffix]
        as_loaded(solid(victory, defeat, under, font, index)).save(dst, optimize=True)
    manifest = os.path.join(assets, "manifest.txt")
    with open(manifest, encoding="utf-8") as f:
        text = f.read()
    lines = {l.split("\t")[0]: l for l in text.split("\n") if l.strip()}
    for rel in written:
        with open(os.path.join(assets, rel), "rb") as f:
            data = f.read()
        lines[rel] = f"{rel}\t{len(data)}\t{fnv1a(data):016x}"
    new = "\n".join(sorted(lines.values())) + "\n"
    if new != text:
        with open(manifest, "w", encoding="utf-8") as f:
            f.write(new)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="command", required=True)
    d = sub.add_parser("draw", help="one banner")
    d.add_argument("out")
    d.add_argument("--suffix", help="the texts of a language of TEXTS")
    d.add_argument("--texts", nargs=3, metavar=("VICTORY", "DEFEAT", "LINE"))
    d.add_argument("--font", default=LATIN)
    d.add_argument("--index", type=int, default=0)
    d.add_argument("--seed", type=int, default=1)
    d.add_argument("--form", choices=["loaded", "solid", "checkerboard"], default="loaded")
    b = sub.add_parser("build", help="every language")
    b.add_argument("assets")
    a = p.parse_args()
    if a.command == "build":
        build(a.assets)
        return
    victory, defeat, under, font, index = TEXTS[a.suffix] if a.suffix else (*a.texts, a.font, a.index)
    rgb = solid(victory, defeat, under, font, index, a.seed)
    {"loaded": as_loaded, "solid": lambda c: Image.fromarray(c.clip(0, 255).astype(np.uint8)), "checkerboard": checkerboard}[a.form](rgb).save(a.out)


if __name__ == "__main__":
    main()
