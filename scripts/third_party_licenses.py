"""Writes THIRD_PARTY_LICENSES.txt: every library built into the game (Rust crates and SDL2) with its licence
text, taken from the crates' own source. Run it from WSL after dependencies change:

    cargo metadata --format-version 1 > /tmp/meta.json && python3 scripts/third_party_licenses.py /tmp/meta.json
"""
import json
import pathlib
import sys

meta = json.load(open(sys.argv[1]))
root = meta["resolve"]["root"]
# Only what the game actually links (normal dependencies, not build tools or tests).
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
keep, todo = set(), [root]
while todo:
    cur = todo.pop()
    for d in nodes[cur]["deps"]:
        if any(k["kind"] in (None, "normal") for k in d["dep_kinds"]) and d["pkg"] not in keep:
            keep.add(d["pkg"])
            todo.append(d["pkg"])
pkgs = sorted((p for p in meta["packages"] if p["id"] in keep), key=lambda p: p["name"])

out = ["ASHEN SANCTUM - THIRD-PARTY LICENCES", "=" * 36, "",
       "The game is built with these libraries. Their licences require this notice to ship with it.", ""]
texts = {}
for p in pkgs:
    out.append(f"- {p['name']} {p['version']}: {p.get('license') or 'see below'}  {p.get('repository') or ''}".rstrip())
    folder = pathlib.Path(p["manifest_path"]).parent
    for f in sorted(folder.iterdir()):
        if f.name.upper().startswith(("LICENSE", "LICENCE", "COPYING")) and f.is_file():
            t = f.read_text(errors="replace").strip()
            texts.setdefault(t, []).append(p["name"])
sdl = next((p for p in pkgs if p["name"] == "sdl2-sys"), None)
if sdl:
    for f in pathlib.Path(sdl["manifest_path"]).parent.glob("SDL/LICENSE*"):
        t = f.read_text(errors="replace").strip()
        texts.setdefault(t, []).append("SDL2 (Simple DirectMedia Layer, built into the game)")
out.append("- SDL2 (Simple DirectMedia Layer): zlib licence  https://www.libsdl.org")
out += ["", ""]
for t, who in texts.items():
    out += ["-" * 78, "Used by: " + ", ".join(sorted(set(who))), "-" * 78, "", t, "", ""]
pathlib.Path("THIRD_PARTY_LICENSES.txt").write_text("\n".join(out) + "\n")
print(f"{len(pkgs)} crates, {len(texts)} licence texts")
