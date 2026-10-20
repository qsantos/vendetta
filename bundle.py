"""Packs the asset files the game loads as it starts into a few bundles, for the browser: a
download each instead of one per file (about 1,250 of them), which a slow or unreliable
connection makes slow, and stalled now and then. The page downloads them all at once as it
opens and hands the game its files out of them (see web/index.html); any other file (music,
artwork) is still downloaded alone.

usage: python3 bundle.py ASSETS_DIR OUT_DIR

Writes OUT_DIR/bundles/<name>.bin, the files of each bundle end to end, and
OUT_DIR/bundles.json: {"bundles": [{"url", "size", "files": [[path, offset, size], ...]}]},
each url versioned by its content (`?v=<hash>`, cached for good).
"""
import hashlib
import json
import os
import re
import sys

assets, out = sys.argv[1], sys.argv[2]

# By what the game loads in turn: its data first, then its images (see `load_game` in
# client/src/main.rs, `Manifest::is_data` and `Manifest::image_estimate` in assets.rs).
BUNDLES = [
    ("sounds", lambda p: p.lower().startswith("sons/")),
    ("data", lambda p: is_data(p)),
    ("buildings", lambda p: p.startswith("Images/Batiments/")),
    ("characters", lambda p: p.startswith("Images/Persos/")),
    ("appearances", lambda p: p.startswith(("Images/Apparences/", "Images/Objets/", "Images/Effets/"))),
    ("terrain", lambda p: p.startswith(("Images/Terrains/", "Images/Decors/"))),
    ("interface", lambda p: p.startswith(("Images/Boutons/", "Images/Fenetres/Fenetre-", "Images/Ressources/", "Images/Curseurs/"))),
]


def is_data(path):
    lower = path.lower()
    return bool(re.search(r"\.(ini|txt|ia)$", lower) or lower.startswith(("cartes/", "sons/"))) and not re.search(
        r"(apercu|carte)\.png$", lower
    )


paths = []
with open(os.path.join(assets, "manifest.txt"), encoding="utf-8") as f:
    for line in f:
        if line.strip():
            paths.append(line.rstrip("\n").split("\t")[0])

os.makedirs(os.path.join(out, "bundles"), exist_ok=True)
index = []
taken = set()
for name, wanted in BUNDLES:
    files, content, offset = [], bytearray(), 0
    for path in paths:
        if path in taken or not wanted(path):
            continue
        taken.add(path)
        with open(os.path.join(assets, path), "rb") as f:
            data = f.read()
        files.append([path, offset, len(data)])
        content += data
        offset += len(data)
    with open(os.path.join(out, "bundles", name + ".bin"), "wb") as f:
        f.write(content)
    digest = hashlib.sha256(content).hexdigest()[:16]
    index.append({"url": f"bundles/{name}.bin?v={digest}", "size": len(content), "files": files})
    print(f"{name}: {len(files)} files, {len(content) / 1e6:.2f} MB", file=sys.stderr)
with open(os.path.join(out, "bundles.json"), "w", encoding="utf-8") as f:
    json.dump({"bundles": index}, f, ensure_ascii=False, separators=(",", ":"))
