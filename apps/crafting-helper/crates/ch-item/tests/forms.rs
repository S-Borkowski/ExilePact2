//! The two copy forms: advanced (`Ctrl+Alt+C`, headers and roll ranges) and
//! short (`Ctrl+C`), and a text that shows neither.

mod common;

use ch_item::{Form, Item, ModKind};
use common::{constructed, parse_ok, summary};

#[test]
fn advanced_form_is_detected() {
    let item = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    assert_eq!(item.form, Form::Advanced);
    let implicit_only = parse_ok(&constructed("staves-normal.txt"));
    assert_eq!(implicit_only.form, Form::Advanced);
}

#[test]
fn short_form_is_detected() {
    let item = parse_ok(&constructed("gloves-rare-short-form.txt"));
    assert_eq!(item.form, Form::Short);
    assert!(item.report.unparsed.is_empty());
}

#[test]
fn form_is_undetermined_without_a_modifier_line() {
    let normal = parse_ok(&constructed("helmets-normal.txt"));
    assert_eq!(normal.form, Form::Undetermined);
    assert!(normal.modifiers.is_empty());
    let unidentified = parse_ok(&constructed("foci-rare-unidentified.txt"));
    assert_eq!(unidentified.form, Form::Undetermined);
}

/// Each modifier line with the flags of the modifier it belongs to.
fn flat(item: &Item) -> Vec<(String, Vec<f64>, bool, bool, bool)> {
    item.modifiers
        .iter()
        .flat_map(|m| {
            m.lines.iter().map(move |l| {
                (
                    l.template.clone(),
                    l.values.iter().map(|v| v.value).collect(),
                    m.fractured,
                    m.desecrated,
                    m.unrevealed,
                )
            })
        })
        .collect()
}

#[test]
fn short_and_advanced_forms_agree() {
    let adv = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    let short = parse_ok(&constructed("gloves-rare-short-form.txt"));
    assert!(adv.report.unparsed.is_empty() && short.report.unparsed.is_empty());
    // Same templates, values and flags, in the same order.
    assert_eq!(flat(&adv), flat(&short));
    // Everything outside the modifiers is identical.
    let head = |s: String| -> String {
        s.lines()
            .filter(|l| {
                !l.starts_with("mod ") && !l.starts_with("  line ") && !l.starts_with("form ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(head(summary(&adv)), head(summary(&short)));
    // The short form has no prefix/suffix, affix name, tier or roll range.
    for m in &short.modifiers {
        assert!(m.affix_name.is_none() && m.tier.is_none() && m.header_line.is_none());
        assert!(!matches!(m.kind, ModKind::Prefix | ModKind::Suffix));
        assert!(m
            .lines
            .iter()
            .flat_map(|l| &l.values)
            .all(|v| v.range.is_none()));
    }
    assert!(adv.modifiers.iter().any(|m| m.tier.is_some()));
    // Implicit and rune kinds come from markers, so both forms have them.
    let kinds = |i: &Item| -> Vec<ModKind> {
        i.modifiers
            .iter()
            .map(|m| m.kind)
            .filter(|k| matches!(k, ModKind::Implicit | ModKind::Rune))
            .collect()
    };
    assert_eq!(kinds(&adv), kinds(&short));
    assert_eq!(kinds(&adv), vec![ModKind::Implicit, ModKind::Rune]);
}
