"""Copy sprite thumbnails from the art repo into tools/level-editor/art/ for the level editor.

Usage (Windows, from anywhere):  python tools/level-editor/make_thumbs.py [art_dir]
Run it again after generating new art.
"""
import sys
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
ART = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(r"D:\projects\AshenSanctum-art")
OUT = HERE / "art"
CHARS = ["mage", "zombie", "skeleton", "wolf", "goblin", "archer", "boss_bone", "boss_plague", "boss_hex", "boss_ashking",
         "npc_elder", "npc_merchant", "npc_healer", "npc_guard", "npc_villager",
         "frost_wolf", "raider", "yeti", "ice_troll", "ice_wraith", "boss_giant", "boss_yeti", "boss_witch", "boss_dragon",
         "npc_captain", "npc_trader", "npc_seer", "npc_fisher"]


def save(src, name):
    if not src.exists():
        print("missing", src)
        return
    im = Image.open(src).convert("RGBA")
    box = im.getbbox()
    if box:
        im = im.crop(box)
    im.save(OUT / f"{name}.png")


def main():
    OUT.mkdir(exist_ok=True)
    sheets = ART / "sheets"
    for p in sheets.glob("prop_*.png"):
        save(p, p.stem[5:])
    for n in ["floor_stone1", "grass1", "grass2", "dirt1", "road1", "food_apple", "food_bread", "food_roast", "seal"]:
        save(sheets / f"{n}.png", n)
    save(sheets / "wall_stack.png", "wall")
    save(sheets / "palisade_stack.png", "palisade")
    for c in CHARS:
        save(ART / "generated" / c / "rotation_urls_south.png", c)
    print("thumbnails in", OUT)


if __name__ == "__main__":
    main()
