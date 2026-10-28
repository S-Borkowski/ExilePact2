//! Never drop a line: every non-blank, non-separator input line is consumed
//! into the model or reported as unparsed, exactly once, and a line the
//! parser does not understand changes nothing else.

mod common;

use common::{
    assert_accounted, constructed, constructed_dir, insert_after, insert_before, line_index,
    parse_ok, read, real_dir, summary, txt_files, waystone_dir,
};

const ODD: &str = "Something odd happens here";

#[test]
fn every_line_is_accounted_for() {
    let mut files = txt_files(&constructed_dir());
    assert!(files.len() >= 23, "constructed fixtures: {}", files.len());
    files.extend(txt_files(&real_dir()));
    files.extend(txt_files(&waystone_dir()));
    for f in files {
        let text = read(&f);
        let item = parse_ok(&text);
        assert_accounted(&text, &item, &f.display().to_string());
    }
}

/// Parses `injected`, which is `base` plus the line `odd`, and checks that
/// exactly that line is reported and nothing else changed.
fn assert_reported_once(base: &str, injected: &str, odd: &str) {
    let before = parse_ok(base);
    let after = parse_ok(injected);
    assert_eq!(after.report.unparsed.len(), 1, "{:?}", after.report);
    let u = &after.report.unparsed[0];
    assert_eq!(u.text, odd);
    assert_eq!(u.line, line_index(injected, odd));
    assert!(!u.reason.is_empty());
    assert_eq!(after.report.partial, before.report.partial);
    assert_eq!(summary(&after), summary(&before));
    assert_accounted(injected, &after, "injected text");
}

#[test]
fn unknown_line_in_the_name_block_is_reported_once() {
    let base = constructed("gloves-rare-fractured-desecrated.txt");
    let injected = insert_after(&base, "Plated Gauntlets", ODD);
    assert_reported_once(&base, &injected, ODD);
    assert_eq!(parse_ok(&injected).report.unparsed[0].section, 0);
    let before_rarity = insert_before(&base, "Rarity: Rare", ODD);
    assert_reported_once(&base, &before_rarity, ODD);
}

#[test]
fn unknown_line_in_a_property_section_is_reported_once() {
    let base = constructed("gloves-rare-fractured-desecrated.txt");
    let injected = insert_after(&base, "Armour: 152 (augmented)", ODD);
    assert_reported_once(&base, &injected, ODD);
    assert_eq!(parse_ok(&injected).report.unparsed[0].section, 1);
}

#[test]
fn unknown_line_in_a_modifier_section_is_reported_once() {
    let base = constructed("gloves-rare-fractured-desecrated.txt");
    let anchor = "{ Fractured Prefix Modifier \"Hale\" (Tier: 4) — Life }";
    let injected = insert_before(&base, anchor, ODD);
    assert_reported_once(&base, &injected, ODD);
}

#[test]
fn unknown_line_after_the_last_section_is_reported_once() {
    let base = constructed("shields-rare-corrupted.txt");
    let same_section = insert_after(&base, "Corrupted", ODD);
    assert_reported_once(&base, &same_section, ODD);
    let own_section = format!("{}--------\n{ODD}\n", base.replace("\r\n", "\n"));
    assert_reported_once(&base, &own_section, ODD);
    let waystone = read(&waystone_dir().join("rare-t5.txt"));
    let after_description = format!("{}--------\n{ODD}\n", waystone.replace("\r\n", "\n"));
    assert_reported_once(&waystone, &after_description, ODD);
}

#[test]
fn unknown_header_word_is_reported_once() {
    let base = constructed("gloves-rare-fractured-desecrated.txt");
    let header = "{ Fractured Prefix Modifier \"Hale\" (Tier: 4) — Life }";
    let odd_header = "{ Glimmering Fractured Prefix Modifier \"Hale\" (Tier: 4) — Life }";
    let injected = base.replace(header, odd_header);
    let before = parse_ok(&base);
    let after = parse_ok(&injected);
    assert!(after.report.unparsed.is_empty());
    assert_eq!(after.report.partial.len(), 1, "{:?}", after.report);
    let p = &after.report.partial[0];
    assert_eq!(p.text, odd_header);
    assert_eq!(p.part, "Glimmering");
    assert_eq!(p.line, line_index(&injected, odd_header));
    assert_eq!(summary(&after), summary(&before));
    assert_accounted(&injected, &after, "injected header");
}
