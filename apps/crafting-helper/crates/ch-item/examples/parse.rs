//! Parses one item text file and prints the model, then every line the
//! parser did not understand, and last `unparsed lines: <n>`.
//!
//! ```text
//! cargo run --manifest-path apps/crafting-helper/crates/ch-item/Cargo.toml --example parse -- <file>
//! ```
//!
//! Exits non-zero only when the file cannot be read or is not item text.

use std::env;
use std::fs;

use ch_item::{Item, ModLine, Value};

fn main() -> Result<(), String> {
    let path = env::args().nth(1).ok_or("usage: parse <item text file>")?;
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let item = ch_item::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    print_item(&item);
    Ok(())
}

fn print_item(item: &Item) {
    println!("form: {:?}", item.form);
    println!("class: {}", item.class.as_str());
    println!("rarity: {:?}", item.rarity);
    if let Some(name) = &item.name {
        println!("name: {name}");
    }
    match &item.base_type {
        Some(base) => println!("base type: {base}"),
        None => println!("base type: (unknown)"),
    }
    if let Some(level) = &item.item_level {
        println!("item level: {}", level.value);
    }
    if let Some(q) = &item.quality {
        println!("quality: {}%{}", q.value, augmented(q.augmented));
    }
    if let Some(r) = &item.requirements {
        let mut parts = Vec::new();
        for (label, v) in [
            ("level", r.level),
            ("str", r.strength),
            ("dex", r.dexterity),
            ("int", r.intelligence),
        ] {
            if let Some(v) = v {
                parts.push(format!("{label} {v}"));
            }
        }
        parts.extend(r.unmet.iter().map(|k| format!("{k} unmet")));
        parts.extend(r.other.iter().cloned());
        println!("requirements: {}", parts.join(", "));
    }
    if let Some(s) = &item.sockets {
        println!("sockets: {} ({})", s.count, s.text);
    }
    for p in &item.properties {
        println!(
            "property: {} = {}{}",
            p.name,
            p.value,
            augmented(p.augmented)
        );
    }
    println!(
        "modifiers: {} ({} prefix, {} suffix)",
        item.modifiers.len(),
        item.prefix_count(),
        item.suffix_count()
    );
    for m in &item.modifiers {
        let mut head = format!("  {:?}", m.kind);
        if let Some(name) = &m.affix_name {
            head.push_str(&format!(" \"{name}\""));
        }
        if let Some(tier) = m.tier {
            head.push_str(&format!(" tier {tier}"));
        }
        for (set, word) in [
            (m.fractured, "fractured"),
            (m.desecrated, "desecrated"),
            (m.unrevealed, "unrevealed"),
            (m.crafted, "crafted"),
        ] {
            if set {
                head.push_str(&format!(" [{word}]"));
            }
        }
        if !m.tags.is_empty() {
            head.push_str(&format!(" tags: {}", m.tags.join(", ")));
        }
        println!("{head}");
        for l in &m.lines {
            println!("    {}", describe_line(l));
        }
    }
    for d in &item.description {
        println!("description: {}", d.text);
    }
    let flags: Vec<&str> = [
        (item.corrupted, "corrupted"),
        (item.mirrored, "mirrored"),
        (item.unidentified, "unidentified"),
    ]
    .iter()
    .filter(|(set, _)| *set)
    .map(|(_, w)| *w)
    .collect();
    if !flags.is_empty() {
        println!("flags: {}", flags.join(", "));
    }
    for p in &item.report.partial {
        println!(
            "partly understood (line {}, section {}): {:?} -- {:?}: {}",
            p.line + 1,
            p.section,
            p.text,
            p.part,
            p.reason
        );
    }
    for u in &item.report.unparsed {
        println!(
            "unparsed (line {}, section {}): {:?} -- {}",
            u.line + 1,
            u.section,
            u.text,
            u.reason
        );
    }
    println!("unparsed lines: {}", item.report.unparsed.len());
}

fn describe_line(l: &ModLine) -> String {
    let values: Vec<String> = l.values.iter().map(describe_value).collect();
    let mut out = format!("{} [{}]", l.template, values.join(", "));
    if l.unscalable {
        out.push_str(" unscalable");
    }
    for m in &l.markers {
        out.push_str(&format!(" ({})", m.as_str()));
    }
    out
}

fn describe_value(v: &Value) -> String {
    match v.range {
        Some(r) => format!("{} ({}..{})", v.value, r.min, r.max),
        None => v.value.to_string(),
    }
}

fn augmented(set: bool) -> &'static str {
    if set {
        " (augmented)"
    } else {
        ""
    }
}
