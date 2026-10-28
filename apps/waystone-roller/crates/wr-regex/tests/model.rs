//! Baseline: the search model on hand-written strings, i.e. how the game's
//! search box behaves without the app. The rules the generator relies on
//! were checked in the game on 2026-10-28 (Live 1; research note `## Search
//! syntax`); `.` not crossing a line and refusing lookaround are the
//! model's own choices, not measured.

use wr_regex::model::{highlights, ModelError, Search};

const RARE_T15: &str = "Item Class: Waystones\n\
Rarity: Rare\n\
Dusk Hollow\n\
Waystone (Tier 15)\n\
--------\n\
Monsters deal 25% of Damage as Extra Glimmer\n\
Players are cursed with Drowsiness\n\
--------\n\
Corrupted\n";

const MAGIC_T5: &str = "Item Class: Waystones\n\
Rarity: Magic\n\
Waystone (Tier 5)\n\
--------\n\
Monsters have 30% increased Hopping Speed\n";

fn hl(search: &str, item: &str) -> bool {
    highlights(search, item).unwrap_or_else(|e| panic!("{search:?} refused: {e}"))
}

#[test]
fn search_model_terms_and_baseline() {
    // Every term must match: both present highlights...
    assert!(hl("glimmer drowsi", RARE_T15));
    // ...one missing does not (negative control on the same item).
    assert!(!hl("glimmer hopping", RARE_T15));
    assert!(!hl("glimmer hopping", MAGIC_T5));
    // Repeated spaces separate terms like one.
    assert!(hl("glimmer   drowsi", RARE_T15));
    // An unquoted space splits: "extra glimmer" unquoted is two terms that
    // both match, while the quoted form needs the words adjacent.
    assert!(hl("deal glimmer", RARE_T15));
    assert!(!hl("\"deal glimmer\"", RARE_T15));
    assert!(hl("\"extra glimmer\"", RARE_T15));
    // An empty search highlights everything.
    assert!(hl("", MAGIC_T5));
}

#[test]
fn search_model_quoted_alternation_any_baseline() {
    assert!(hl("\"hopping|drowsi\"", RARE_T15));
    assert!(hl("\"hopping|drowsi\"", MAGIC_T5));
    assert!(!hl("\"bleat|murk\"", RARE_T15));
    // An alternative with a space inside the quotes.
    assert!(hl("\"cursed with|nothing here\"", RARE_T15));
    assert!(!hl("\"cursed with|nothing here\"", MAGIC_T5));
    // An unclosed quote is refused rather than guessed at.
    assert_eq!(
        Search::parse("\"abc").unwrap_err(),
        ModelError::UnclosedQuote
    );
}

#[test]
fn search_model_negated_alternation_excludes_all_baseline() {
    // "!a|b" hides an item matching either alternative.
    assert!(!hl("\"!glimmer|hopping\"", RARE_T15));
    assert!(!hl("\"!glimmer|hopping\"", MAGIC_T5));
    // ...and shows one matching neither.
    assert!(hl("\"!bleat|murk\"", RARE_T15));
    assert!(hl("!bleat", MAGIC_T5));
    // Negation combines with a positive term.
    assert!(hl("waystone !corr", MAGIC_T5));
    assert!(!hl("waystone !corr", RARE_T15));
    assert!(hl("corr", RARE_T15));
    assert!(!hl("corr", MAGIC_T5));
    assert_eq!(Search::parse("!").unwrap_err(), ModelError::EmptyNegation);
}

#[test]
fn search_model_is_case_insensitive_baseline() {
    assert!(hl("GLIMMER", RARE_T15));
    assert!(hl("glimmer", RARE_T15));
    assert!(hl("\"y: r\"", RARE_T15));
    assert!(!hl("\"y: m\"", RARE_T15));
    assert!(hl("\"y: m\"", MAGIC_T5));
    assert!(hl("\"y: [mr]\"", MAGIC_T5));
    assert!(hl("\"y: [mr]\"", RARE_T15));
    assert!(!hl("\"y: n\"", RARE_T15));
}

#[test]
fn search_model_escaped_paren_baseline() {
    assert!(hl("\"r 1[4-6]\\)\"", RARE_T15));
    assert!(!hl("\"r 1[4-6]\\)\"", MAGIC_T5));
    assert!(hl("\"r [5-9]\\)|r 1[0-2]\\)\"", MAGIC_T5));
    assert!(!hl("\"r [5-9]\\)|r 1[0-2]\\)\"", RARE_T15));
    // "r 1\)" must not take tier 15 for tier 1.
    assert!(!hl("\"r 1\\)\"", RARE_T15));
    // Measured in Live 1: the box searches roll ranges, so a bare "15\)"
    // lights a tier-2 waystone through "+14(11-15)%"; the "r " form does not.
    let ranged_t2 = "Item Class: Waystones\nRarity: Rare\nWaystone (Tier 2)\n--------\n\
                     Monsters have +14(11-15)% Critical Damage Bonus\n";
    assert!(hl("\"15\\)\"", ranged_t2));
    assert!(!hl("\"r 15\\)\"", ranged_t2));
    assert!(hl("\"r 2\\)\"", ranged_t2));
    // An unescaped ")" is not a valid pattern.
    assert!(matches!(
        Search::parse("1[4-6])"),
        Err(ModelError::BadPattern { .. })
    ));
    // "." does not cross a line: "glimmer" ends one line, "players" starts
    // the next.
    assert!(!hl("\"glimmer.players\"", RARE_T15));
    assert!(hl("\"glimmer\\nplayers\"", RARE_T15));
    // Lookaround is refused, not modelled.
    assert!(matches!(
        Search::parse("\"(?!glimmer)\""),
        Err(ModelError::Lookaround { .. })
    ));
    assert!(matches!(
        Search::parse("(?=a)"),
        Err(ModelError::Lookaround { .. })
    ));
}
