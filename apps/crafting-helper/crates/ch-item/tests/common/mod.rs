//! Helpers shared by the integration tests: fixture paths, an index-free
//! summary of an item (so an injected line can be checked not to change
//! anything else) and the line-accounting check.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ch_item::{parse, split_lines, Item};

pub const SEPARATOR: &str = "--------";

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn constructed_dir() -> PathBuf {
    crate_dir().join("fixtures").join("constructed")
}

pub fn real_dir() -> PathBuf {
    crate_dir().join("fixtures").join("real")
}

/// The waystone roller's fixtures, read in place (never copied): invented
/// mods in the item-text layout measured in its Live 1.
pub fn waystone_dir() -> PathBuf {
    crate_dir()
        .join("..")
        .join("..")
        .join("..")
        .join("waystone-roller")
        .join("fixtures")
        .join("items")
}

/// Every `.txt` file in `dir`, sorted by name; empty when there is none.
pub fn txt_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub fn constructed(name: &str) -> String {
    read(&constructed_dir().join(name))
}

pub fn parse_ok(text: &str) -> Item {
    parse(text).unwrap_or_else(|e| panic!("not parsed: {e}\n{text}"))
}

/// Builds item text from lines joined with LF.
pub fn text(lines: &[&str]) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Inserts `new_line` after the first line equal to `anchor`.
pub fn insert_after(text: &str, anchor: &str, new_line: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut done = false;
    for line in split_lines(text) {
        out.push(line);
        if !done && line.trim() == anchor {
            out.push(new_line);
            done = true;
        }
    }
    assert!(done, "anchor {anchor:?} not found");
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// Inserts `new_line` before the first line equal to `anchor`.
pub fn insert_before(text: &str, anchor: &str, new_line: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut done = false;
    for line in split_lines(text) {
        if !done && line.trim() == anchor {
            out.push(new_line);
            done = true;
        }
        out.push(line);
    }
    assert!(done, "anchor {anchor:?} not found");
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// The index of the first line of `text` equal to `wanted`.
pub fn line_index(text: &str, wanted: &str) -> usize {
    split_lines(text)
        .iter()
        .position(|l| l.trim() == wanted)
        .unwrap_or_else(|| panic!("line {wanted:?} not found"))
}

/// Everything the parser understood, without input line indices and without
/// the report: two texts that differ only by a reported line give the same
/// summary.
pub fn summary(item: &Item) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "form {:?}", item.form);
    let _ = writeln!(s, "class {:?} rarity {:?}", item.class, item.rarity);
    let _ = writeln!(s, "name {:?} base {:?}", item.name, item.base_type);
    for l in &item.name_lines {
        let _ = writeln!(s, "name line {:?}", l.text);
    }
    let _ = writeln!(s, "ilvl {:?}", item.item_level.as_ref().map(|l| l.value));
    let _ = writeln!(
        s,
        "quality {:?}",
        item.quality.as_ref().map(|q| (q.value, q.augmented))
    );
    if let Some(r) = &item.requirements {
        let _ = writeln!(
            s,
            "requires {:?} {:?} {:?} {:?} {:?}",
            r.level, r.strength, r.dexterity, r.intelligence, r.other
        );
    }
    if let Some(k) = &item.sockets {
        let _ = writeln!(s, "sockets {:?} {}", k.text, k.count);
    }
    for p in &item.properties {
        let _ = writeln!(s, "property {:?} {:?} {}", p.name, p.value, p.augmented);
    }
    for d in &item.description {
        let _ = writeln!(s, "description {:?}", d.text);
    }
    for m in &item.modifiers {
        let _ = writeln!(
            s,
            "mod {:?} {:?} {:?} {:?} f{} d{} u{} c{}",
            m.kind,
            m.affix_name,
            m.tier,
            m.tags,
            m.fractured,
            m.desecrated,
            m.unrevealed,
            m.crafted
        );
        for l in &m.lines {
            let _ = writeln!(
                s,
                "  line {:?} {:?} {:?} {} {:?}",
                l.text, l.template, l.values, l.unscalable, l.markers
            );
        }
    }
    let _ = writeln!(
        s,
        "flags corrupted {} mirrored {} unidentified {}",
        item.corrupted, item.mirrored, item.unidentified
    );
    s
}

/// Every non-blank, non-separator input line is consumed into the model or
/// listed as unparsed, exactly once.
pub fn assert_accounted(text: &str, item: &Item, what: &str) {
    let expected: Vec<usize> = split_lines(text)
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            !t.is_empty() && t != SEPARATOR
        })
        .map(|(i, _)| i)
        .collect();
    let mut got = item.consumed_lines();
    got.extend(item.report.unparsed.iter().map(|u| u.line));
    got.sort_unstable();
    assert_eq!(got, expected, "line accounting of {what}");
}
