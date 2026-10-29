//! Embeds every `apps/crafting-helper/blueprints/*.toml`, sorted by file
//! name, as `(file name, text)` pairs in `$OUT_DIR/shipped.rs`. An empty or
//! missing folder gives an empty list. Cargo reruns this when the folder
//! (any file in it) changes, and on every build while it is missing.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let folder = manifest.join("..").join("..").join("blueprints");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", folder.display());

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(&folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_toml = path.extension().is_some_and(|e| e == "toml");
            if is_toml && path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files.push((name.to_string(), path.clone()));
                }
            }
        }
    }
    files.sort();

    let mut out = String::from("&[\n");
    for (name, path) in &files {
        out.push_str(&format!(
            "    Shipped {{ file: {name:?}, text: include_str!({:?}) }},\n",
            path.display().to_string()
        ));
    }
    out.push(']');

    let dest = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("shipped.rs");
    fs::write(dest, out).expect("write shipped.rs");
}
