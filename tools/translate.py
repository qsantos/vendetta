#!/usr/bin/env python3
"""The translations beyond the original's French and English, kept in `translations/<suffix>.json`
and added to the converted assets.

    python3 -I tools/translate.py extract ASSETS OUT.json
        every text of the game, by place: its French (the original) and English (the
        original's translation, rough), to translate from

    python3 -I tools/translate.py apply ASSETS [TRANSLATIONS]
        writes each translation of `translations/` into ASSETS: `Donnees/<folder>/` (the French
        files with their text replaced), `Donnees/ModsOfficiels/<folder>/` (the names and
        descriptions of the parameter files, as the English ones are), its entry in
        `Langues.ini`, and for Chinese and Japanese a font with the characters they use
        (`Polices/<folder>.otf`), all listed in `manifest.txt`. Only what changed is written.

A translation file: {"language": {"name", "folder", "suffix", "font"?}, "texts": {place: text}},
where a place is `<file>|<section>|<key>` (`Jeu.ini|[Jeu]|EtiquetteNiveau`); a text missing
keeps the French.
"""

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TRANSLATIONS = os.path.join(HERE, "..", "translations")

LINE = re.compile(r"^(\s*)([^=;\[\s][^=]*?)(\s*=\s*)(.*?)(\s*)$")
# Values that are not text: file names, colours, key codes…
PATH = re.compile(r"\.(bmp|png|jpg|gif|wav|mid|midi|ogg|mp3|ini|txt|ttf|ico|avi)\b|[\\/]", re.I)
LETTERS = re.compile(r"[^\W\d_]{2}")
# Parameter keys that name things for lookups, never translated.
IDENTIFIERS = {"id"}


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path, text):
    """Writes only a change (keeps dates, for the builds that compare them)."""
    if os.path.exists(path) and read(path) == text:
        return
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def entries(text):
    """(line index, place's section and key, value as written, inner text) of each key."""
    section, seen = "", {}
    for n, line in enumerate(text.split("\n")):
        s = line.strip()
        if s.startswith("["):
            section, seen = s[: s.index("]") + 1] if "]" in s else s, {}
            continue
        m = LINE.match(line)
        if not m or s.startswith((";", "#")):
            continue
        key = m.group(2)
        k = seen.get(key.lower(), 0)
        seen[key.lower()] = k + 1
        place_key = key if k == 0 else f"{key}#{k}"
        value = m.group(4)
        quoted = len(value) >= 2 and value[0] == '"' and value[-1] == '"'
        yield n, section, place_key, value, value[1:-1] if quoted else value


def is_text(value):
    return bool(LETTERS.search(value)) and not PATH.search(value)


def folder_texts(donnees, folder):
    """{place: text} of the texts of `Donnees/<folder>/`."""
    out = {}
    d = os.path.join(donnees, folder)
    for name in sorted(os.listdir(d)):
        if not name.lower().endswith(".ini"):
            continue
        for _, section, key, _, inner in entries(read(os.path.join(d, name))):
            if is_text(inner):
                out[f"{name}|{section}|{key}"] = inner
    return out


def base(key):
    """The key without its language suffix and index: `NomEN(2)` → `Nom`."""
    k = key.split("(")[0].split("#")[0].strip()
    return k[:-2] if k.upper().endswith("EN") and len(k) > 2 else k


def params_texts(donnees):
    """{place: (French, English)} of the parameter files' texts: the keys the English translates,
    in place (`NomEN`) or in its mod folder."""
    english_mods = os.path.join(donnees, "ModsOfficiels", "English")
    overlay = {}
    text_keys = set()
    for name in os.listdir(english_mods):
        if name.lower().endswith(".ini"):
            for _, section, key, _, inner in entries(read(os.path.join(english_mods, name))):
                overlay[f"{name}|{section}|{key}"] = inner
                text_keys.add(base(key).lower())
    files = sorted(n for n in os.listdir(donnees) if n.startswith("Parametres_") and n.endswith(".ini"))
    for name in files:
        for _, _, key, _, _ in entries(read(os.path.join(donnees, name))):
            k = key.split("(")[0].split("#")[0].strip()
            if k.upper().endswith("EN") and len(k) > 2:
                text_keys.add(k[:-2].lower())
    text_keys -= IDENTIFIERS
    out = {}
    for name in files:
        records = {}
        for _, section, key, _, inner in entries(read(os.path.join(donnees, name))):
            records.setdefault(section, []).append((key, inner))
        for section, keys in records.items():
            en_inplace = {}
            for key, inner in keys:
                k = key.split("#")[0]
                head = k.split("(")[0].strip()
                if head.upper().endswith("EN") and head[:-2].lower() in text_keys:
                    en_inplace[head[:-2] + k[len(head):]] = inner
            for key, inner in keys:
                k = key.split("#")[0]
                if base(k).lower() not in text_keys or base(k) != k.split("(")[0].strip() or not is_text(inner):
                    continue
                place = f"{name}|{section}|{key}"
                english = en_inplace.get(k, overlay.get(place))
                out[place] = (inner, english)
    return out


def extract(assets, out):
    donnees = os.path.join(assets, "Donnees")
    french, english = folder_texts(donnees, "Francais"), folder_texts(donnees, "English")
    items = [{"place": p, "fr": t, "en": english.get(p)} for p, t in french.items()]
    items += [{"place": p, "fr": fr, "en": en} for p, (fr, en) in params_texts(donnees).items()]
    with open(out, "w", encoding="utf-8") as f:
        json.dump(items, f, ensure_ascii=False, indent=0)
    print(f"{len(items)} texts, {len({i['fr'] for i in items})} different")


def replaced(text, file, texts):
    """`text` (an ini file) with the values of its places in `texts` replaced."""
    lines = text.split("\n")
    for n, section, key, value, inner in entries(text):
        new = texts.get(f"{file}|{section}|{key}")
        if new is None or new == inner:
            continue
        m = LINE.match(lines[n])
        quoted = value != inner
        lines[n] = m.group(1) + m.group(2) + m.group(3) + (f'"{new}"' if quoted else new) + m.group(5)
    return "\n".join(lines)


def params_overlay(donnees, file, texts):
    """The mod file setting the translated texts of parameter file `file`."""
    out = []
    section = None
    for _, sec, key, _, _ in entries(read(os.path.join(donnees, file))):
        t = texts.get(f"{file}|{sec}|{key}")
        if t is None:
            continue
        if sec != section:
            out.append(f"\n{sec}")
            section = sec
        out.append(f"\t{key.split('#')[0]} = {t}")
    if not out:
        return None
    # the section header first, as the original's mods have (`[Objets]`)
    kind = {"Parametres_" + k + ".ini": k for k in ["Objets", "Batiments", "Races", "Peuples", "Ressources", "Decors", "Effets", "Resistances", "CompetencesObjets"]}.get(file)
    head = f"[{kind}]\n" if kind else ""
    return head + "\n".join(out).lstrip("\n") + "\n"


def languages_ini(text, langs):
    """`Langues.ini` with `langs` added (by folder) if missing."""
    have = re.findall(r"(?im)^\s*Dossier\s*=\s*(\S+?)\\?\s*$", text)
    n = int(re.search(r"(?im)^\s*NombreLangues\s*=\s*(\d+)", text).group(1))
    for lang in langs:
        if lang["folder"] in have:
            continue
        n += 1
        text = text.rstrip("\r\n") + f"\n\n[Langue_{n}]\nNom={lang['name']}\nDossier={lang['folder']}\\\nSuffixe={lang['suffix']}\n"
    return re.sub(r"(?im)^(\s*NombreLangues\s*=\s*)\d+", lambda m: m.group(1) + str(n), text)


def subset_font(source, index, chars, out):
    from fontTools import subset
    from fontTools.ttLib import TTFont

    font = TTFont(source, fontNumber=index)
    s = subset.Subsetter(subset.Options(layout_features=[], name_IDs=["*"], notdef_outline=True, hinting=False))
    s.populate(text="".join(sorted(chars)))
    s.subset(font)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    tmp = out + ".tmp"
    font.save(tmp)
    if os.path.exists(out) and read_bytes(out) == read_bytes(tmp):
        os.remove(tmp)
    else:
        os.replace(tmp, out)


def fnv1a(data):
    """assetconv's content hash."""
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def update_manifest(assets, paths):
    """`manifest.txt` with the lines of `paths` (relative to ASSETS) added or replaced."""
    path = os.path.join(assets, "manifest.txt")
    lines = {l.split("\t")[0]: l for l in read(path).split("\n") if l.strip()}
    for rel in paths:
        data = read_bytes(os.path.join(assets, rel))
        lines[rel] = f"{rel}\t{len(data)}\t{fnv1a(data):016x}"
    write(path, "\n".join(sorted(lines.values())) + "\n")


def read_bytes(path):
    with open(path, "rb") as f:
        return f.read()


def apply(assets, translations=TRANSLATIONS):
    donnees = os.path.join(assets, "Donnees")
    langs, written = [], ["Donnees/Langues.ini"]
    for name in sorted(os.listdir(translations)):
        if not name.endswith(".json"):
            continue
        with open(os.path.join(translations, name), encoding="utf-8") as f:
            data = json.load(f)
        lang, texts = data["language"], data["texts"]
        langs.append(lang)
        src = os.path.join(donnees, "Francais")
        for file in sorted(os.listdir(src)):
            text = read(os.path.join(src, file))
            write(os.path.join(donnees, lang["folder"], file), replaced(text, file, texts) if file.lower().endswith(".ini") else text)
            written.append(f"Donnees/{lang['folder']}/{file}")
        mods = os.path.join(donnees, "ModsOfficiels", lang["folder"])
        for file in sorted(os.listdir(donnees)):
            if file.startswith("Parametres_") and file.endswith(".ini"):
                overlay = params_overlay(donnees, file, texts)
                if overlay:
                    write(os.path.join(mods, file), overlay)
                    written.append(f"Donnees/ModsOfficiels/{lang['folder']}/{file}")
        if "font" in lang:
            chars = {c for t in texts.values() for c in t if ord(c) > 0x2E7F}
            font = lang["font"]
            subset_font(font["file"], font.get("index", 0), chars, os.path.join(assets, "Polices", lang["folder"] + ".otf"))
            written.append(f"Polices/{lang['folder']}.otf")
    path = os.path.join(donnees, "Langues.ini")
    write(path, languages_ini(read(path), langs))
    update_manifest(assets, written)


if __name__ == "__main__":
    if sys.argv[1:2] == ["extract"] and len(sys.argv) == 4:
        extract(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ["apply"] and len(sys.argv) in (3, 4):
        apply(*sys.argv[2:])
    else:
        sys.exit(__doc__)
