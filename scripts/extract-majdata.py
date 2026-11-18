#!/usr/bin/env python3
"""Copy MajdataPlay's playfield art and slide layouts into engine/assets/majdata.

    python3 scripts/extract-majdata.py --src <MajdataPlay checkout>

Run by hand against a checkout of https://github.com/TeamMajdata/MajdataPlay at
PIN (a sparse checkout of the three folders in SOURCES is enough). It refuses
any other commit, so a re-run always reproduces the committed files.

Writes, under the output folder (visualizer-majdata 12):
  skin/…, effects/…  the PNGs the engine draws, copied unchanged
  slides.txt          every slide prefab's arrows, in draw order:
                        prefab <Name>
                        bar <child name> <x> <y> <angle in degrees>
                      x, y are the arrow's local position in MajdataPlay world
                      units (the ring is 4.8 from the centre), laid out for a
                      slide that starts at key 1
  sprites.txt         <path> <width> <height> for every copied PNG
  SOURCE.md           where it all came from

MajdataPlay is GPL-3.0; so is everything this script writes.
Standard library only.
"""

import argparse
import math
import os
import re
import shutil
import struct
import subprocess
import sys

PIN = "158c78a1bfd2793d9adba33d476c3b37d64599fc"

SLIDES = "Assets/Prefabs/Game/Slides"
SKIN = "Assets/StreamingAssets/Skins/default"
EFFECTS = "Assets/Sprites/Game/EffectSprites"
SOURCES = (SLIDES, SKIN, EFFECTS)

# Note art only: no judgement text, SlideOK, each-line or background art —
# the visualizer has no gameplay overlay.
SKIN_FOLDERS = (
    "TapSkins",
    "StarSkins",
    "HoldSkins",
    "SlideSkins",
    "WifiSkins",
    "TouchSkins",
    "TouchHoldSkins",
)
SKIN_FILES = ("outline.png",)
# The ring behind each approaching note, and the dot at a hold's tail.
GUIDE_FILES = (
    "Normal.png",
    "Each.png",
    "Break.png",
    "Hold_End.png",
    "Hold_Each_End.png",
    "Hold_Break_End.png",
)
EFFECT_FILES = (
    "Hex.png",
    "StarWhite.png",
    "Star.png",
    "TouchEff.png",
    "TouchEffparts_01.png",
    "TouchEffparts_02.png",
)

# The arrows of a slide; the other children are its judgement (SlideOK).
BAR_NAME = re.compile(r"^(Slide_|wifi_)")

_DOC = re.compile(r"^--- !u!(\d+) &(-?\d+)(.*)$", re.M)
_FIELD = r"^\s*{}:\s*(.*)$"


def _field(body, name):
    match = re.search(_FIELD.format(re.escape(name)), body, re.M)
    return match.group(1).strip() if match else None


def _vector(text):
    return {k: float(v) for k, v in re.findall(r"(\w+):\s*(-?[\d.eE+-]+)", text)}


def _file_id(text):
    match = re.search(r"fileID:\s*(-?\d+)", text or "")
    return match.group(1) if match else None


def parse_prefab(text):
    """(prefab name, [(child name, x, y, angle_deg), …]) for one slide prefab.

    Bars come in the root transform's m_Children order — the order the game
    draws and the star walks — not the order of the documents in the file.
    """
    docs = {}
    heads = list(_DOC.finditer(text))
    for i, head in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
        kind, ident, rest = head.group(1), head.group(2), head.group(3)
        docs[ident] = (kind, "stripped" in rest, text[head.end() : end])

    names = {i: _field(b, "m_Name") for i, (k, _, b) in docs.items() if k == "1"}
    transforms = {i: b for i, (k, stripped, b) in docs.items() if k == "4" and not stripped}

    root = next(
        b for b in transforms.values() if _file_id(_field(b, "m_Father")) == "0"
    )
    prefab = names[_file_id(_field(root, "m_GameObject"))]
    children = re.findall(r"^\s*-\s*\{fileID:\s*(-?\d+)\}", root.split("m_Children:", 1)[1].split("m_Father:", 1)[0], re.M)

    bars = []
    for child in children:
        body = transforms.get(child)
        if body is None:  # a nested prefab instance
            continue
        name = names.get(_file_id(_field(body, "m_GameObject")), "")
        if not BAR_NAME.match(name):
            continue
        pos = _vector(_field(body, "m_LocalPosition"))
        rot = _vector(_field(body, "m_LocalRotation"))
        angle = math.degrees(2 * math.atan2(rot["z"], rot["w"]))
        bars.append((name, pos["x"], pos["y"], angle))
    return prefab, bars


def png_size(path):
    """(width, height) from a PNG's IHDR chunk."""
    with open(path, "rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n" or head[12:16] != b"IHDR":
        raise ValueError(f"not a PNG: {path}")
    return struct.unpack(">II", head[16:24])


def _head(src):
    try:
        out = subprocess.run(
            ["git", "-C", src, "rev-parse", "HEAD"], capture_output=True, text=True, check=True
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return out.stdout.strip()


def _copy(src_file, out, rel, copied):
    dest = os.path.join(out, rel)
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    shutil.copyfile(src_file, dest)
    copied.append(rel)


def main(argv=None):
    here = os.path.dirname(os.path.abspath(__file__))
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--src", required=True, help="MajdataPlay checkout at PIN")
    parser.add_argument(
        "--out", default=os.path.join(here, "..", "engine", "assets", "majdata")
    )
    args = parser.parse_args(argv)

    head = _head(args.src)
    if head != PIN:
        print(f"MajdataPlay checkout is at {head}, expected {PIN}", file=sys.stderr)
        return 2
    missing = [s for s in SOURCES if not os.path.isdir(os.path.join(args.src, s))]
    if missing:
        print(f"missing from the checkout: {', '.join(missing)}", file=sys.stderr)
        return 2

    out = os.path.abspath(args.out)
    for owned in ("skin", "effects"):
        shutil.rmtree(os.path.join(out, owned), ignore_errors=True)
    os.makedirs(out, exist_ok=True)

    copied = []
    skin = os.path.join(args.src, SKIN)
    for folder in SKIN_FOLDERS:
        for name in sorted(os.listdir(os.path.join(skin, folder))):
            if name.endswith(".png"):
                _copy(os.path.join(skin, folder, name), out, f"skin/{folder}/{name}", copied)
    for name in SKIN_FILES:
        _copy(os.path.join(skin, name), out, f"skin/{name}", copied)
    for name in GUIDE_FILES:
        _copy(os.path.join(skin, "NoteGuideSkins", name), out, f"skin/NoteGuideSkins/{name}", copied)
    for name in EFFECT_FILES:
        _copy(os.path.join(args.src, EFFECTS, name), out, f"effects/{name}", copied)

    lines = [f"# MajdataPlay {PIN}, {SLIDES} — GPL-3.0. Written by scripts/extract-majdata.py."]
    slides = os.path.join(args.src, SLIDES)
    count = 0
    for file in sorted(os.listdir(slides)):
        if not file.endswith(".prefab"):
            continue
        with open(os.path.join(slides, file), encoding="utf-8") as f:
            prefab, bars = parse_prefab(f.read())
        lines.append(f"prefab {prefab}")
        lines.extend(f"bar {n} {x:.6f} {y:.6f} {a:.4f}" for n, x, y, a in bars)
        count += 1
    with open(os.path.join(out, "slides.txt"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    with open(os.path.join(out, "sprites.txt"), "w", encoding="utf-8") as f:
        f.write(f"# Pixel sizes of the copied PNGs. Written by scripts/extract-majdata.py.\n")
        for rel in sorted(copied):
            w, h = png_size(os.path.join(out, rel))
            f.write(f"{rel} {w} {h}\n")

    with open(os.path.join(out, "SOURCE.md"), "w", encoding="utf-8") as f:
        f.write(
            "# MajdataPlay art and slide layouts\n\n"
            "Everything in this folder comes from "
            "[MajdataPlay](https://github.com/TeamMajdata/MajdataPlay) at commit\n"
            f"`{PIN}`, and is licensed under the GPL-3.0 (see `LICENSE` and\n"
            "`LICENSE-THIRD-PARTY` at the repository root). The skin is likely derived\n"
            "from SEGA's maimai art; see the SEGA notice in `LICENSE-THIRD-PARTY`.\n\n"
            "- `skin/`: from `Assets/StreamingAssets/Skins/default`, copied unchanged\n"
            "- `effects/`: from `Assets/Sprites/Game/EffectSprites`, copied unchanged\n"
            "- `slides.txt`: each prefab in `Assets/Prefabs/Game/Slides`, its arrows'\n"
            "  positions and angles in draw order\n"
            "- `sprites.txt`: each PNG's pixel size\n\n"
            "Do not edit these files. To regenerate them, check out that commit and run\n"
            "`python3 scripts/extract-majdata.py --src <checkout>`.\n"
        )

    print(f"{len(copied)} images, {count} slide prefabs -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
