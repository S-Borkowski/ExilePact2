//! The data built into the executable (`data/waystone-mods.json`): it loads,
//! and each family's token, searched alone, lights that family's own item
//! and no other waystone, with and without the reward lines, through the
//! search model.

mod common;

use common::*;
use wr_app::EMBEDDED_DATA;
use wr_regex::model::Search;
use wr_regex::{quote, Builder, ModData};

#[test]
fn shipped_data_every_family_token_matches_only_its_own_item() {
    let data = ModData::parse(EMBEDDED_DATA).expect("the shipped data loads");
    assert!(
        data.families.len() >= 40,
        "only {} families: is the shipped data the real page's?",
        data.families.len()
    );
    let builder = Builder::new(data.clone());

    // One rare item per family with every line, one magic corrupted item
    // per family without its reward lines, and two waystones with no mods.
    let mut items: Vec<(Option<usize>, String)> = Vec::new();
    for (i, f) in data.families.iter().enumerate() {
        let own: Vec<String> = f.own_lines().map(str::to_string).collect();
        items.push((
            Some(i),
            waystone(&f.lines, "Rare", 1 + (i % 16) as u8, false),
        ));
        items.push((Some(i), waystone(&own, "Magic", 16 - (i % 16) as u8, true)));
    }
    items.push((None, waystone(&[], "Normal", 7, false)));
    items.push((None, waystone(&[], "Rare", 12, true)));

    let mut tokened = 0;
    for (i, (f, t)) in data.families.iter().zip(builder.tokens().all()).enumerate() {
        assert_eq!(f.id, t.id);
        let Some(token) = &t.token else {
            let why = t.why_not.as_deref().unwrap_or("");
            assert!(!why.is_empty(), "{} has neither a token nor a reason", f.id);
            continue;
        };
        tokened += 1;
        let search = Search::parse(&quote(token)).unwrap();
        for (owner, text) in &items {
            let lit = search.highlights(text);
            let own = *owner == Some(i);
            assert_eq!(
                lit,
                own,
                "token {token:?} of {} {} an item of {:?}:\n{text}",
                f.id,
                if lit { "lights" } else { "misses" },
                owner.map(|o| data.families[o].id.as_str())
            );
        }
    }
    // Most families are told apart; the few that are not have a reason.
    assert!(
        tokened * 10 >= data.families.len() * 8,
        "only {tokened} of {} families have a token",
        data.families.len()
    );
}
