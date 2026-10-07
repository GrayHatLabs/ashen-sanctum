//! Embeds every `levels/*.json` (made with the level editor) into the binary, so the
//! game, including the handheld build, ships with them. A `levels/` folder next to the
//! running game still overrides them at runtime.
use std::fmt::Write;

fn main() {
    println!("cargo:rerun-if-changed=levels");
    // The Android crate (android/rust) builds the same source from two folders down.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = [root.join("levels"), root.join("../../levels")].into_iter().find(|d| d.is_dir()).unwrap_or_else(|| root.join("levels"));
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "json")).collect())
        .unwrap_or_default();
    files.sort();
    let mut out = String::from("/// Level files embedded at build time: (file name without .json, contents).\npub static EMBEDDED: &[(&str, &str)] = &[\n");
    for p in &files {
        println!("cargo:rerun-if-changed={}", p.display());
        let stem = p.file_stem().unwrap().to_string_lossy();
        writeln!(out, "    ({:?}, include_str!({:?})),", stem, p.display().to_string()).unwrap();
    }
    out.push_str("];\n");
    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("levels_gen.rs");
    std::fs::write(dest, out).unwrap();
}
