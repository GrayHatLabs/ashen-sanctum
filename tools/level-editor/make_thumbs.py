"""Copy sprite thumbnails from the art repo into tools/level-editor/art/ for the level editor.

Usage (Windows, from anywhere):  python tools/level-editor/make_thumbs.py [art_dir]
Run it again after generating new art. Which pictures it needs comes from catalog.js (written by the
game with `--export-catalog`): every prop, monster and person in it gets a thumbnail.
"""
import json
import re
import sys
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
ART = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(r"D:\projects\AshenSanctum-art")
OUT = HERE / "art"
# Tiles, ground and items the editor shows besides the catalog.
EXTRA = ["floor_stone1", "grass1", "grass2", "dirt1", "road1", "food_apple", "food_bread", "food_roast", "seal", "stairs_up", "stairs_down"]


def save(src, name):
    im = Image.open(src).convert("RGBA")
    box = im.getbbox()
    if box:
        im = im.crop(box)
    im.save(OUT / f"{name}.png")


def find(name):
    """The best picture of `name` in the art repo: a character's south-facing frame, a prop's sheet, or its image."""
    g, sheets = ART / "generated" / name, ART / "sheets"
    for p in [g / "rotation_urls_south.png", g / "image.png", sheets / f"prop_{name}.png", sheets / f"{name}.png",
              ART / "generated" / f"brk_{name}_0" / "image.png", ART / "generated" / f"brk_{name}_0" / "rotation_urls_south.png"]:
        if p.exists():
            return p
    hits = sorted(g.glob("*.png")) if g.is_dir() else []
    return hits[0] if hits else None


def main():
    OUT.mkdir(exist_ok=True)
    text = (HERE / "catalog.js").read_text(encoding="utf8")
    cat = json.loads(re.search(r"window\.CATALOG = (\{.*\});", text, re.S).group(1))
    names = EXTRA + [p["kind"] for p in cat["props"]] + [m["kind"] for m in cat["monsters"]] + [n["img"] for n in cat["npcs"]]
    names = list(dict.fromkeys(names))
    missing = []
    for n in names:
        src = find(n)
        if src:
            save(src, n)
        else:
            missing.append(n)
    sheets = ART / "sheets"
    if (sheets / "wall_stack.png").exists():
        save(sheets / "wall_stack.png", "wall")
    print(f"thumbnails in {OUT} ({len(names) - len(missing)} of {len(names)} found)")
    if missing:
        print("no picture for:", ", ".join(missing))


if __name__ == "__main__":
    main()
