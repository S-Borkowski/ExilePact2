//! Target: the generated search highlights exactly the intended items, as
//! the search model reads it; and the outcomes for an empty, an over-limit
//! and an unmatchable selection.

mod common;

use common::{all_items, check, fixture_builder, matchable_ids, subsets, synth_item, Item};
use wr_regex::model::highlights;
use wr_regex::{
    corruption_collisions, quote, rarity_term, tier_term, Builder, Corruption, ModData, Outcome,
    PartWhat, Rarity, Selection, WantMode, LIMIT,
};

const GLIMMER: &str = "monsters-deal-of-damage-as-extra-glimmer";
const MURK: &str = "monsters-deal-of-damage-as-extra-murk";
const HOPPING: &str = "monsters-have-increased-hopping-speed";
const DROWSY: &str = "players-are-cursed-with-drowsiness";
const CONTAINER: &str = "area-is-overrun-by-the-murkfolk";
const CONTAINED: &str = "area-contains-additional-murk-pools";

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn want(list: Vec<String>, mode: WantMode) -> Selection {
    Selection {
        want: list,
        want_mode: mode,
        ..Selection::default()
    }
}

fn avoid(list: Vec<String>) -> Selection {
    Selection {
        avoid: list,
        ..Selection::default()
    }
}

#[test]
fn fixture_items_match_their_index() {
    // The oracle trusts items.json; this keeps it honest about the text.
    let data = common::fixture_data();
    for item in common::fixture_items() {
        for id in &item.mods {
            let f = data
                .family(id)
                .unwrap_or_else(|| panic!("{}: unknown mod {id}", item.name));
            for line in f.own_lines() {
                // `regex::escape` escapes `#` too. A number may carry its
                // roll range, as in `+14(11-15)%` or `-4(-4--3)%`.
                let pattern = format!(
                    "(?m)^{}$",
                    regex::escape(line).replace(r"\#", r"-?\d+(?:\(-?\d+--?\d+\))?")
                );
                assert!(
                    regex::Regex::new(&pattern).unwrap().is_match(&item.text),
                    "{}: {line:?} missing",
                    item.name
                );
            }
        }
        assert!(
            item.text.contains(&format!("(Tier {})", item.tier)),
            "{}",
            item.name
        );
        assert_eq!(
            item.text.contains("\nCorrupted\n"),
            item.corrupted,
            "{}",
            item.name
        );
    }
}

#[test]
fn build_want_any_highlights_items_with_any_target() {
    let b = fixture_builder();
    let items = all_items(b.data());
    let matchable = matchable_ids(&b);
    for set in subsets(&matchable, 1, 3) {
        check(&b, &want(set, WantMode::Any), &items);
    }
    // A family with no token of its own is matchable beside its container.
    let s = check(
        &b,
        &want(ids(&[CONTAINER, CONTAINED]), WantMode::Any),
        &items,
    );
    assert!(!s.is_empty());
    // ...but not alone, nor beside an unrelated family.
    for set in [ids(&[CONTAINED]), ids(&[CONTAINED, GLIMMER])] {
        assert_eq!(
            b.build(&want(set, WantMode::Any)),
            Outcome::Unmatchable {
                ids: ids(&[CONTAINED])
            }
        );
    }
    // One family: its own token, unquoted when it has no space.
    let token = b.tokens().get(GLIMMER).unwrap().token.clone().unwrap();
    match b.build(&want(ids(&[GLIMMER]), WantMode::Any)) {
        Outcome::Ok { regex, .. } => assert_eq!(regex, quote(&token)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn build_want_all_highlights_items_with_all_target() {
    let b = fixture_builder();
    let mut items = all_items(b.data());
    // Items with two or three of the families, so "all" has positives.
    let fams: Vec<_> = b.data().families.iter().collect();
    for w in fams.windows(3) {
        items.push(synth_item(w, "rare", 9, false, true));
        items.push(synth_item(&w[..2], "magic", 14, false, false));
    }
    let matchable = matchable_ids(&b);
    for set in subsets(&matchable, 1, 3) {
        let s = check(&b, &want(set.clone(), WantMode::All), &items);
        // One term per family.
        let terms = wr_regex::model::Search::parse(&s).unwrap().terms.len();
        assert_eq!(terms, set.len(), "{s:?}");
    }
}

#[test]
fn build_avoid_hides_items_with_any_avoided_target() {
    let b = fixture_builder();
    let items = all_items(b.data());
    let all: Vec<String> = b.data().families.iter().map(|f| f.id.clone()).collect();
    for set in subsets(&all, 1, 3) {
        match b.build(&avoid(set.clone())) {
            Outcome::Ok { .. } => {
                check(&b, &avoid(set), &items);
            }
            Outcome::Unmatchable { ids: bad } => {
                // Only the contained family, without its container, fails.
                assert_eq!(bad, ids(&[CONTAINED]), "{set:?}");
                assert!(set.contains(&CONTAINED.to_string()));
                assert!(!set.contains(&CONTAINER.to_string()), "{set:?}");
            }
            other => panic!("{set:?}: {other:?}"),
        }
    }
    // Every family at once.
    let s = check(&b, &avoid(all), &items);
    assert!(s.starts_with("\"!") || s.starts_with('!'), "{s:?}");
}

#[test]
fn build_avoid_cover_is_no_longer_than_single_tokens_target() {
    let b = fixture_builder();
    let items = all_items(b.data());
    let matchable = matchable_ids(&b);
    let mut shorter = 0;
    for set in subsets(&matchable, 1, 5) {
        let own: Vec<String> = b
            .data()
            .families
            .iter()
            .filter(|f| set.contains(&f.id))
            .map(|f| b.tokens().get(&f.id).unwrap().token.clone().unwrap())
            .collect();
        let own_term = quote(&format!("!{}", own.join("|")));
        let Outcome::Ok { parts, .. } = b.build(&avoid(set.clone())) else {
            panic!("{set:?} not Ok");
        };
        let part = parts.iter().find(|p| p.what == PartWhat::Avoid).unwrap();
        assert!(
            part.length <= own_term.len(),
            "{set:?}: cover {:?} longer than {own_term:?}",
            part.text
        );
        if part.length < own_term.len() {
            shorter += 1;
        }
        if set.len() <= 3 {
            check(&b, &avoid(set), &items);
        }
    }
    // The two "Monsters deal ... as Extra" families share a shorter cover.
    assert!(shorter > 0);
    let Outcome::Ok { parts, .. } = b.build(&avoid(ids(&[GLIMMER, MURK]))) else {
        panic!()
    };
    let own = format!(
        "!{}|{}",
        b.tokens().get(GLIMMER).unwrap().token.as_deref().unwrap(),
        b.tokens().get(MURK).unwrap().token.as_deref().unwrap()
    );
    assert!(
        parts[0].length < quote(&own).len(),
        "{:?} vs {own:?}",
        parts[0].text
    );
}

#[test]
fn build_tier_ranges_exhaustive_target() {
    let b = fixture_builder();
    let mut items: Vec<Item> = common::fixture_items();
    for t in 1..=16u8 {
        items.push(synth_item(&[], "normal", t, false, false));
        let f = &b.data().families[t as usize % b.data().families.len()];
        items.push(synth_item(&[f], "rare", t, t % 2 == 0, true));
    }
    let mut ranges = 0;
    for lo in 1..=16u8 {
        for hi in lo..=16u8 {
            let sel = Selection {
                tier_min: lo,
                tier_max: hi,
                ..Selection::default()
            };
            if (lo, hi) == (1, 16) {
                assert_eq!(tier_term(lo, hi), None);
                assert_eq!(b.build(&sel), Outcome::Empty);
                continue;
            }
            check(&b, &sel, &items);
            ranges += 1;
        }
    }
    assert_eq!(ranges, 135);
    // Negative control: the items carry roll ranges, so the bare two-digit
    // form refuted in Live 1 lights an item of another tier here too.
    let lights_another_tier = |s: &str, lo: u8, hi: u8| {
        items
            .iter()
            .any(|i| !(lo..=hi).contains(&i.tier) && highlights(s, &i.text).unwrap())
    };
    assert!(lights_another_tier("\"15\\)\"", 15, 15));
    assert!(lights_another_tier("\"1[4-6]\\)\"", 14, 16));
    assert!(lights_another_tier("\"1[0-2]\\)\"", 10, 12));
    assert!(!lights_another_tier("\"r 15\\)\"", 15, 15));
    // Every alternative carries the `r ` of "Tier": the box searches roll
    // ranges too, so a bare `15\)` also matches `+14(11-15)%` (Live 1).
    assert_eq!(tier_term(14, 16).as_deref(), Some("\"r 1[4-6]\\)\""));
    assert_eq!(
        tier_term(5, 12).as_deref(),
        Some("\"r [5-9]\\)|r 1[0-2]\\)\"")
    );
    assert_eq!(tier_term(7, 7).as_deref(), Some("\"r 7\\)\""));
    assert_eq!(tier_term(16, 16).as_deref(), Some("\"r 16\\)\""));
    // Out-of-range and reversed bounds are clamped and ordered.
    assert_eq!(tier_term(16, 14), tier_term(14, 16));
    assert_eq!(tier_term(0, 40), None);
}

#[test]
fn build_rarity_and_corruption_target() {
    let b = fixture_builder();
    let items = all_items(b.data());
    for mask in 0..8u8 {
        let rarity = Rarity {
            normal: mask & 1 != 0,
            magic: mask & 2 != 0,
            rare: mask & 4 != 0,
        };
        for corruption in [
            Corruption::Any,
            Corruption::Corrupted,
            Corruption::Uncorrupted,
        ] {
            let sel = Selection {
                rarity: rarity.clone(),
                corruption,
                ..Selection::default()
            };
            let none_or_all = mask == 0 || mask == 7;
            if none_or_all && corruption == Corruption::Any {
                assert_eq!(b.build(&sel), Outcome::Empty);
                continue;
            }
            check(&b, &sel, &items);
        }
    }
    let r = |n, m, ra| {
        rarity_term(&Rarity {
            normal: n,
            magic: m,
            rare: ra,
        })
    };
    assert_eq!(r(false, true, false).as_deref(), Some("\"y: m\""));
    assert_eq!(r(false, true, true).as_deref(), Some("\"y: [mr]\""));
    assert_eq!(r(true, false, true).as_deref(), Some("\"y: [nr]\""));
    assert_eq!(r(true, true, true), None);
    assert_eq!(r(false, false, false), None);
    let sel = |c| Selection {
        corruption: c,
        ..Selection::default()
    };
    assert!(
        matches!(b.build(&sel(Corruption::Corrupted)), Outcome::Ok { regex, .. } if regex == "corr")
    );
    assert!(
        matches!(b.build(&sel(Corruption::Uncorrupted)), Outcome::Ok { regex, .. } if regex == "!corr")
    );
    // `corr` is in no mod line of the fixture; a line holding it is caught.
    assert!(corruption_collisions(b.data()).is_empty());
    let mut doc: serde_json::Value = serde_json::from_str(&common::fixture_json()).unwrap();
    doc["families"][10]["lines"][0] = "Area has patches of Corrupted Ground".into();
    let bad = ModData::parse(&doc.to_string()).unwrap();
    assert_eq!(corruption_collisions(&bad), ids(&[DROWSY]));
}

#[test]
fn build_combined_selection_target() {
    let b = fixture_builder();
    let items = all_items(b.data());
    let sel = Selection {
        want: ids(&[GLIMMER, HOPPING]),
        want_mode: WantMode::Any,
        avoid: ids(&[DROWSY]),
        tier_min: 10,
        tier_max: 16,
        rarity: Rarity {
            normal: false,
            magic: true,
            rare: true,
        },
        corruption: Corruption::Uncorrupted,
    };
    let s = check(&b, &sel, &items);
    let Outcome::Ok { parts, .. } = b.build(&sel) else {
        panic!()
    };
    let what: Vec<PartWhat> = parts.iter().map(|p| p.what).collect();
    assert_eq!(
        what,
        [
            PartWhat::Rarity,
            PartWhat::Tier,
            PartWhat::Want,
            PartWhat::Avoid,
            PartWhat::Corruption
        ]
    );
    assert!(s.starts_with("\"y: [mr]\" \"r 1[0-6]\\)\" "), "{s:?}");
    assert!(s.ends_with(" !corr"), "{s:?}");
    // Some fixture item is highlighted and some is not (both outcomes seen).
    let hits = items
        .iter()
        .filter(|i| highlights(&s, &i.text).unwrap())
        .count();
    assert!(hits > 0 && hits < items.len(), "{hits}");
    // Want-all in the same combination.
    let all = Selection {
        want_mode: WantMode::All,
        ..sel
    };
    check(&b, &all, &items);
}

fn rarity_magic() -> Rarity {
    Rarity {
        normal: false,
        magic: true,
        rare: false,
    }
}

#[test]
fn build_over_limit_fails_with_part_lengths_target() {
    let (data, words) = common::three_letter_data(70);
    let b = Builder::new(data);
    let sel = Selection {
        want: words[..62].iter().map(|w| format!("fam-{w}")).collect(),
        want_mode: WantMode::All,
        rarity: rarity_magic(),
        ..Selection::default()
    };
    match b.build(&sel) {
        Outcome::TooLong { length, parts } => {
            // "y: m" (6) + space + 62 words of 3 joined by spaces (247).
            assert_eq!(length, 254);
            assert_eq!(parts.len(), 2);
            assert_eq!((parts[0].what, parts[0].length), (PartWhat::Rarity, 6));
            assert_eq!((parts[1].what, parts[1].length), (PartWhat::Want, 247));
            let total: usize = parts.iter().map(|p| p.length).sum::<usize>() + parts.len() - 1;
            assert_eq!(total, length);
            // Nothing dropped: every selected word is in the parts.
            for w in &words[..62] {
                assert!(parts[1].text.split(' ').any(|t| t == w), "{w} missing");
            }
            assert!(length > LIMIT);
        }
        other => panic!("{other:?}"),
    }
    // Avoiding all 70 does not fit either: `"!` + 70 words joined by `|` + `"`.
    match b.build(&avoid(words.iter().map(|w| format!("fam-{w}")).collect())) {
        Outcome::TooLong { length, parts } => {
            assert_eq!(length, 2 + 70 * 4 - 1 + 1);
            assert_eq!(parts[0].what, PartWhat::Avoid);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn build_exactly_250_is_accepted_target() {
    let (data, words) = common::three_letter_data(70);
    let b = Builder::new(data);
    let chosen: Vec<String> = words[..61].iter().map(|w| format!("fam-{w}")).collect();
    let sel = Selection {
        want: chosen.clone(),
        want_mode: WantMode::All,
        rarity: rarity_magic(),
        ..Selection::default()
    };
    let regex = match b.build(&sel) {
        Outcome::Ok { regex, .. } => regex,
        other => panic!("{other:?}"),
    };
    assert_eq!(regex.len(), 250);
    assert_eq!(LIMIT, 250);
    // It works: an item with all 61 is highlighted; one missing a mod, or
    // the wrong rarity, is not.
    let fams: Vec<_> = b
        .data()
        .families
        .iter()
        .filter(|f| chosen.contains(&f.id))
        .collect();
    let full = synth_item(&fams, "magic", 8, false, false);
    let short = synth_item(&fams[1..], "magic", 8, false, false);
    let rare = synth_item(&fams, "rare", 8, false, false);
    assert!(highlights(&regex, &full.text).unwrap());
    assert!(!highlights(&regex, &short.text).unwrap());
    assert!(!highlights(&regex, &rare.text).unwrap());
}

#[test]
fn build_empty_selection_is_empty() {
    let b = fixture_builder();
    assert_eq!(b.build(&Selection::default()), Outcome::Empty);
    let none = Selection {
        rarity: Rarity {
            normal: false,
            magic: false,
            rare: false,
        },
        ..Selection::default()
    };
    assert_eq!(b.build(&none), Outcome::Empty);
    let full_range = Selection {
        tier_min: 1,
        tier_max: 16,
        ..Selection::default()
    };
    assert_eq!(b.build(&full_range), Outcome::Empty);
    // Positive control: one term makes it non-empty.
    let one = Selection {
        corruption: Corruption::Corrupted,
        ..Selection::default()
    };
    assert!(matches!(b.build(&one), Outcome::Ok { .. }));
}

#[test]
fn build_unmatchable_want_all_fails_loudly() {
    let b = fixture_builder();
    let sel = want(ids(&[GLIMMER, CONTAINED]), WantMode::All);
    assert_eq!(
        b.build(&sel),
        Outcome::Unmatchable {
            ids: ids(&[CONTAINED])
        }
    );
    // Even with its container selected too: "all" needs each one alone.
    let sel = want(ids(&[CONTAINER, CONTAINED]), WantMode::All);
    assert_eq!(
        b.build(&sel),
        Outcome::Unmatchable {
            ids: ids(&[CONTAINED])
        }
    );
    // An id the data does not have is unmatchable, not ignored.
    let sel = want(ids(&[GLIMMER, "no-such-mod"]), WantMode::Any);
    assert_eq!(
        b.build(&sel),
        Outcome::Unmatchable {
            ids: ids(&["no-such-mod"])
        }
    );
    // Positive control: the same "all" without the contained family builds.
    let sel = want(ids(&[GLIMMER, CONTAINER]), WantMode::All);
    assert!(matches!(b.build(&sel), Outcome::Ok { .. }));
}
