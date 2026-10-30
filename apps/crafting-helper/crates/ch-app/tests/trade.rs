//! The *Find a base on trade* panel through `App`, with a recording fake
//! shell and the shipped data: closed and opening nothing by default, the
//! selection's rules, the link built from the app's own state (the fixed
//! address, the league percent-encoded into one path segment, the query),
//! the length cap, the craft untouched underneath, and the `trade:` log
//! lines, which carry counts and kinds only.

mod common;

use ch_app::ch_data::{trade_query, TradePick, TradeSearch, MAX_TRADE_PICKS};
use ch_app::{
    percent_encode, trade_link, App, CraftMode, GuideStatusView, StatusKind, TradePickChoice,
    TradeSelection, TradeWeightSource, View, DEFAULT_TRADE_LEAGUE, MAX_TRADE_LINK_CHARS,
    TRADE_SEARCH_URL,
};
use common::*;
use serde_json::{json, Value};

const RINGS: &str = "Rings";
const SAPPHIRE_RING: &str = "Sapphire Ring";
const LIFE: &str = "prefix-to-maximum-life";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";

fn status(view: &View) -> (StatusKind, String) {
    let s = view.status.as_ref().expect("a status line");
    (s.kind, s.text.clone())
}

fn pick(id: &str, floor: Option<u32>) -> TradePickChoice {
    TradePickChoice {
        mod_id: id.to_string(),
        floor,
    }
}

/// Opens the panel and chooses `class`, then `base` at `item_level`, then
/// the picks, each as the page sends it: the whole selection.
fn choose(
    app: &App,
    class: &str,
    base: &str,
    item_level: Option<u32>,
    picks: &[TradePickChoice],
) -> View {
    let mut sel = app.get_view().trade.selection;
    sel.open = true;
    sel.class = Some(class.to_string());
    app.set_trade_search(&sel);
    sel.base = Some(base.to_string());
    sel.item_level = item_level;
    app.set_trade_search(&sel);
    sel.picks = picks.to_vec();
    app.set_trade_search(&sel)
}

/// The two picks of the ring search: life (prefix) and fire resistance
/// (suffix), any tier.
fn ring_picks() -> Vec<TradePickChoice> {
    vec![pick(LIFE, None), pick(FIRE, None)]
}

/// `s` with each `%XX` read back as its byte.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

/// The link's league segment and its query, decoded.
fn split_link(link: &str) -> (String, Value) {
    let rest = link
        .strip_prefix(TRADE_SEARCH_URL)
        .unwrap_or_else(|| panic!("{link} is not the trade search"));
    let (league, query) = rest.split_once("?q=").expect("a ?q= query");
    (
        league.to_string(),
        serde_json::from_str(&percent_decode(query)).unwrap(),
    )
}

fn trade_lines(dir: &std::path::Path) -> Vec<String> {
    diag_lines(dir)
        .into_iter()
        .filter(|l| l.starts_with("trade:"))
        .collect()
}

#[test]
fn trade_panel_is_closed_and_opens_nothing_by_default_baseline() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // Baseline: with no trade command sent, the panel is closed, carries the
    // default selection and no link, and nothing is opened, whatever else
    // happens.
    let view = app.get_view();
    assert_eq!(view.trade.selection, TradeSelection::default());
    assert!(!view.trade.selection.open);
    assert!(view.trade.selection.both_slots, "Both slots is the default");
    assert_eq!(view.trade.league, DEFAULT_TRADE_LEAGUE);
    assert!(view.trade.link.is_none() && !view.trade.can_search);
    assert!(view.trade.classes.is_empty() && view.trade.prefixes.is_empty());
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.update_settings(&json!({ "screen_watch": false }))
        .unwrap();
    app.open_settings().unwrap();
    assert!(shell.rec().opened.is_empty());
    assert!(!app.get_view().trade.selection.open);
    assert!(trade_lines(tmp.path()).is_empty());
    // Every view the shell was sent carried the closed panel.
    assert!(shell.rec().views.iter().all(|v| !v.trade.selection.open));

    // Positive control: the same instrument records an opened search.
    choose(&app, RINGS, SAPPHIRE_RING, Some(75), &ring_picks());
    app.open_trade_search();
    assert_eq!(shell.rec().opened.len(), 1);
    app.shutdown();
}

#[test]
fn open_trade_search_opens_only_the_trade_address_built_from_the_state() {
    assert_eq!(
        TRADE_SEARCH_URL,
        "https://www.pathofexile.com/trade2/search/poe2/"
    );
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    let view = choose(&app, RINGS, SAPPHIRE_RING, Some(75), &ring_picks());
    let trade = &view.trade;
    assert!(trade.selection.open);
    assert_eq!(trade.selection.picks, ring_picks());
    assert!(
        trade.classes.iter().any(|c| c == RINGS) && trade.classes.iter().any(|c| c == "Jewels")
    );
    assert!(trade.bases.iter().any(|b| b == SAPPHIRE_RING));
    // Each base type once, though the data lists some more than once.
    let mut bases = trade.bases.clone();
    bases.dedup();
    assert_eq!(bases.len(), trade.bases.len());
    assert_eq!(
        trade.count_line.as_deref(),
        Some("The search asks for two: one prefix and one suffix.")
    );
    let link = trade.link.clone().expect("a link");
    assert!(trade.can_search && trade.why_not.is_none());

    // The link: the fixed prefix, the league, and the query ch-data builds
    // for the same selection.
    let (league, query) = split_link(&link);
    assert_eq!(league, "Forbidden%20Rites");
    let want = trade_query(&TradeSearch {
        class: RINGS.to_string(),
        base: SAPPHIRE_RING.to_string(),
        item_level: Some(75),
        picks: vec![
            TradePick {
                id: LIFE.to_string(),
                floor: None,
            },
            TradePick {
                id: FIRE.to_string(),
                floor: None,
            },
        ],
        both_slots: true,
    })
    .unwrap();
    assert_eq!(query, want.query);
    assert_eq!(query["query"]["type"], SAPPHIRE_RING);
    assert_eq!(
        query["query"]["filters"]["type_filters"]["filters"]["rarity"]["option"],
        "magic"
    );
    assert_eq!(query["query"]["stats"][0]["value"]["min"], 2);

    // Every row shows its weight; the picked rows are marked.
    for row in trade.prefixes.iter().chain(&trade.suffixes) {
        assert!(row.weight.parse::<f64>().unwrap() > 0.0, "{row:?}");
        assert_eq!(row.weight.split('.').nth(1).map(str::len), Some(1));
        assert_eq!(row.picked, [LIFE, FIRE].contains(&row.mod_id.as_str()));
        assert_eq!(row.floors[0].label, "Any tier");
        assert_eq!(row.floors[0].floor, None);
        assert!(row.floors[1..]
            .iter()
            .all(|f| f.label.starts_with("At least T")));
    }
    assert!(trade
        .prefixes
        .iter()
        .all(|r| r.weight_source == TradeWeightSource::Spawn));

    // Nothing opened yet; the button opens exactly that link.
    assert!(shell.rec().opened.is_empty());
    let view = app.open_trade_search();
    assert_eq!(shell.rec().opened, [link.clone()]);
    assert_eq!(
        status(&view),
        (
            StatusKind::Info,
            "Opened the trade search in your browser.".to_string()
        )
    );

    // A stricter floor raises the pick's weight and sets a value floor.
    let before = view
        .trade
        .prefixes
        .iter()
        .find(|r| r.mod_id == LIFE)
        .unwrap()
        .weight
        .parse::<f64>()
        .unwrap();
    let mut sel = view.trade.selection.clone();
    let best = view
        .trade
        .prefixes
        .iter()
        .find(|r| r.mod_id == LIFE)
        .unwrap()
        .floors[1]
        .floor
        .unwrap();
    sel.picks[0].floor = Some(best);
    let view = app.set_trade_search(&sel);
    let life = view
        .trade
        .prefixes
        .iter()
        .find(|r| r.mod_id == LIFE)
        .unwrap();
    assert_eq!(life.floor, Some(best));
    assert!(life.weight.parse::<f64>().unwrap() > before);
    let (_, query) = split_link(view.trade.link.as_deref().unwrap());
    assert!(query["query"]["stats"][0]["filters"][0]["value"]["min"].is_number());

    // A failure to open says why, in the status line and the log.
    shell.rec().open_fails = Some("no default browser".to_string());
    let view = app.open_trade_search();
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Couldn't open the browser: no default browser.".to_string()
        )
    );
    assert_eq!(shell.rec().opened.len(), 2);
    // Every address opened is the trade search's.
    assert!(shell
        .rec()
        .opened
        .iter()
        .all(|a| a.starts_with(TRADE_SEARCH_URL)));
    app.shutdown();
    let log = trade_lines(tmp.path());
    assert!(
        log.contains(&"trade: opened, 2 modifiers, at least 2".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"trade: open failed: no default browser".to_string()),
        "{log:?}"
    );

    // A shell that cannot open a browser refuses by default.
    struct NoBrowser;
    impl ch_app::Shell for NoBrowser {
        fn register_hotkey(&self, _: &str) -> Result<(), ch_app::HotkeyError> {
            Ok(())
        }
        fn read_clipboard(&self) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn beep(&self) -> Vec<ch_app::BeepAttempt> {
            Vec::new()
        }
        fn emit_view(&self, _: &View) {}
        fn set_overlay_visible(&self, _: bool) {}
    }
    let tmp = tempfile::tempdir().unwrap();
    let app = App::new(ch_app::AppConfig {
        store: ch_app::Store::open(tmp.path()),
        shell: Box::new(NoBrowser),
        screen: None,
    });
    choose(&app, RINGS, SAPPHIRE_RING, None, &ring_picks());
    let view = app.open_trade_search();
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Couldn't open the browser: this shell cannot open a web browser.".to_string()
        )
    );
    app.shutdown();
}

#[test]
fn trade_league_is_percent_encoded_into_one_path_segment() {
    // Only A-Z a-z 0-9 - . _ ~ stay literal.
    assert_eq!(percent_encode("Forbidden Rites"), "Forbidden%20Rites");
    assert_eq!(percent_encode("../x?y#z"), "..%2Fx%3Fy%23z");
    assert_eq!(percent_encode("a/b\\c&d=e+f%"), "a%2Fb%5Cc%26d%3De%2Bf%25");
    assert_eq!(percent_encode("Liga é"), "Liga%20%C3%A9");

    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    choose(&app, RINGS, SAPPHIRE_RING, Some(75), &ring_picks());
    for (league, segment) in [
        ("HC Forbidden Rites", "HC%20Forbidden%20Rites"),
        ("../x?y#z", "..%2Fx%3Fy%23z"),
        ("a/b/../../c", "a%2Fb%2F..%2F..%2Fc"),
        ("50% off & more", "50%25%20off%20%26%20more"),
    ] {
        app.update_settings(&json!({ "trade_league": league }))
            .unwrap();
        let view = app.get_view();
        assert_eq!(view.trade.league, league);
        let link = view.trade.link.clone().expect("a link");
        let (got, _) = split_link(&link);
        assert_eq!(got, segment, "{league}");
        // One path segment: nothing in it ends the path or starts a query
        // or a fragment, and it decodes back to the league.
        assert!(!got.contains(['/', '?', '#', '\\']), "{got}");
        assert_eq!(percent_decode(&got), league);
        // The query holds no raw delimiter either.
        let q = link.split_once("?q=").unwrap().1;
        assert!(!q.contains(['/', '?', '#', '&', '=', ' ', '"']), "{q}");
        app.open_trade_search();
        assert_eq!(shell.rec().opened.last(), Some(&link));
    }

    // A league that would be a dot segment of its own makes no link.
    for league in [".", ".."] {
        app.update_settings(&json!({ "trade_league": league }))
            .unwrap();
        let view = app.get_view();
        assert!(view.trade.link.is_none(), "{league}");
        assert_eq!(view.trade.why_not.as_ref().unwrap().kind, StatusKind::Error);
        let opened = shell.rec().opened.len();
        app.open_trade_search();
        assert_eq!(shell.rec().opened.len(), opened, "{league}");
    }
    app.shutdown();
}

#[test]
fn trade_search_refuses_an_unknown_base_and_keeps_the_state() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let good = choose(&app, RINGS, SAPPHIRE_RING, Some(75), &ring_picks());
    let link = good.trade.link.clone().unwrap();
    let kept = good.trade.selection.clone();

    let refused = |change: &dyn Fn(&mut TradeSelection), want: &str| {
        let mut sel = kept.clone();
        change(&mut sel);
        let view = app.set_trade_search(&sel);
        assert_eq!(
            status(&view),
            (
                StatusKind::Error,
                format!("Trade search not changed: {want}.")
            ),
            "{sel:?}"
        );
        assert_eq!(view.trade.selection, kept, "{want}");
        assert_eq!(view.trade.link.as_deref(), Some(link.as_str()), "{want}");
    };
    refused(
        &|s| s.base = Some("Iron Mitts".to_string()),
        "Rings has no base type Iron Mitts",
    );
    refused(
        &|s| s.class = Some("Shoes".to_string()),
        "Shoes is not an item class the helper knows",
    );
    for level in [0, 101] {
        refused(
            &|s| s.item_level = Some(level),
            "the minimum item level is a whole number from 1 to 100",
        );
    }
    refused(
        &|s| s.picks.push(pick("prefix-increased-armour", None)),
        "prefix-increased-armour is not a modifier this base offers at this item level",
    );
    refused(
        &|s| s.picks.push(pick(LIFE, Some(1))),
        "a modifier is picked twice",
    );
    refused(
        &|s| s.picks[0].floor = Some(99),
        "T99 is not a tier of +# to maximum Life this base reaches at this item level",
    );
    refused(
        &|s| {
            s.picks = (0..=MAX_TRADE_PICKS)
                .map(|i| pick(&format!("m{i}"), None))
                .collect()
        },
        "at most 8 modifiers in one search",
    );
    // A modifier the site cannot search is refused too.
    let hybrid = good.trade.not_searchable[0].clone();
    refused(
        &|s| s.picks.push(pick(&hybrid.mod_id, None)),
        &format!(
            "{} cannot be searched on the trade site: {}",
            hybrid.text, hybrid.reason
        ),
    );

    // A change Rust takes clears the refusal's line.
    let mut sel = kept.clone();
    sel.both_slots = false;
    let view = app.set_trade_search(&sel);
    assert!(view.status.is_none(), "{:?}", view.status);
    assert_eq!(
        view.trade.count_line.as_deref(),
        Some("The search asks for at least one of your picks.")
    );
    assert_eq!(view.trade.selection.picks, kept.picks);

    // A new base keeps what it offers; a new class starts over and says how
    // many picks went.
    sel.base = Some("Ruby Ring".to_string());
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.selection.picks, kept.picks);
    sel.class = Some("Amulets".to_string());
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.selection.base, None);
    assert!(view.trade.selection.picks.is_empty());
    assert_eq!(
        status(&view),
        (
            StatusKind::Info,
            "Item class changed: 2 picks cleared.".to_string()
        )
    );
    assert!(shell.rec().opened.is_empty());
    app.shutdown();
    let log = trade_lines(tmp.path());
    for line in [
        "trade: refused: unknown base type",
        "trade: refused: unknown item class",
        "trade: refused: item level out of range",
        "trade: refused: a modifier the base does not offer",
        "trade: refused: a modifier picked twice",
        "trade: refused: a floor the base does not reach",
        "trade: refused: too many picks",
        "trade: refused: a modifier the trade site cannot search",
    ] {
        assert!(log.contains(&line.to_string()), "{line}: {log:?}");
    }
}

#[test]
fn open_trade_search_without_a_searchable_pick_opens_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    let mut sel = TradeSelection {
        open: true,
        ..TradeSelection::default()
    };
    let expect = |view: &View, why: &str, status_text: &str| {
        assert!(view.trade.link.is_none() && !view.trade.can_search);
        let line = view.trade.why_not.as_ref().expect("why there is no link");
        assert_eq!((line.kind, line.text.as_str()), (StatusKind::Info, why));
        let opened = app.open_trade_search();
        assert_eq!(
            status(&opened),
            (StatusKind::Error, status_text.to_string())
        );
    };
    let view = app.set_trade_search(&sel);
    assert!(view.trade.selection.open);
    assert!(view.trade.count_line.is_none());
    expect(
        &view,
        "Choose an item class.",
        "Trade search not opened: choose an item class.",
    );
    sel.class = Some(RINGS.to_string());
    let view = app.set_trade_search(&sel);
    assert!(view.trade.prefixes.is_empty() && !view.trade.bases.is_empty());
    expect(
        &view,
        "Choose a base type.",
        "Trade search not opened: choose a base type.",
    );
    sel.base = Some(SAPPHIRE_RING.to_string());
    let view = app.set_trade_search(&sel);
    assert!(!view.trade.prefixes.is_empty() && !view.trade.not_searchable.is_empty());
    expect(
        &view,
        "Pick at least one modifier.",
        "Trade search not opened: pick at least one modifier.",
    );

    // Only modifiers the site cannot search: refused, nothing picked.
    let hybrid = view.trade.not_searchable[0].clone();
    assert_eq!(
        hybrid.reason,
        "more than one line; the trade site lists each line as its own stat"
    );
    sel.picks = vec![pick(&hybrid.mod_id, None)];
    let view = app.set_trade_search(&sel);
    assert!(view.trade.selection.picks.is_empty());
    expect(
        &view,
        "Pick at least one modifier.",
        "Trade search not opened: pick at least one modifier.",
    );
    assert!(shell.rec().opened.is_empty());
    app.shutdown();
    let log = trade_lines(tmp.path());
    for line in [
        "trade: refused: no item class",
        "trade: refused: no base type",
        "trade: refused: no modifier picked",
    ] {
        assert!(log.contains(&line.to_string()), "{line}: {log:?}");
    }
    assert!(
        !log.iter().any(|l| l.starts_with("trade: opened")),
        "{log:?}"
    );
}

#[test]
fn trade_panel_leaves_the_craft_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let app = start_with_screen(tmp.path(), &shell, &screen, true);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.accept_craft();
    app.tick_watch();
    let before = app.get_view();
    assert_eq!(before.craft.mode, CraftMode::Guiding);
    assert!(before.overlay.visible);
    let shows = shell.rec().overlay.len();
    let reads = screen.calls();

    // Open the panel, pick, open the search, close it: the craft, its guide
    // and the overlay are as they were.
    let view = choose(&app, RINGS, SAPPHIRE_RING, Some(75), &ring_picks());
    assert_eq!(view.craft, before.craft);
    assert_eq!(view.overlay, before.overlay);
    let view = app.open_trade_search();
    assert_eq!(view.craft, before.craft);
    assert_eq!(
        shell.rec().overlay.len(),
        shows,
        "the overlay was not touched"
    );
    assert_eq!(screen.calls(), reads, "no watch call was caused");

    // A grab goes to the craft while the panel is open; the panel stays.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let guiding = view.craft.guiding.as_ref().unwrap();
    assert_eq!(guiding.history.len(), 1);
    assert_eq!(guiding.status, GuideStatusView::Active);
    assert!(view.trade.selection.open);
    assert_eq!(view.trade.selection.picks, ring_picks());
    let craft = view.craft.clone();

    let mut sel = view.trade.selection.clone();
    sel.open = false;
    let view = app.set_trade_search(&sel);
    assert_eq!(view.craft, craft);
    assert!(!view.trade.selection.open);
    // Closed, the selection is kept, and reopening shows it as it was.
    sel.open = true;
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.selection.picks, ring_picks());
    assert_eq!(view.trade.selection.base.as_deref(), Some(SAPPHIRE_RING));
    assert_eq!(view.craft, craft);
    // Never saved, though the settings are.
    app.update_settings(&json!({ "screen_watch": false }))
        .unwrap();
    app.shutdown();
    let saved = saved_settings(tmp.path()).to_string();
    assert!(
        !saved.contains(SAPPHIRE_RING) && !saved.contains(LIFE),
        "{saved}"
    );
}

#[test]
fn trade_link_over_the_length_cap_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let view = choose(&app, RINGS, SAPPHIRE_RING, None, &[]);
    // Eight picks, each at its best tier (a value floor apiece), in a
    // league of 64 characters that each encode to six.
    let picks: Vec<TradePickChoice> = view
        .trade
        .prefixes
        .iter()
        .take(4)
        .chain(view.trade.suffixes.iter().take(4))
        .map(|r| pick(&r.mod_id, r.floors[1].floor))
        .collect();
    assert_eq!(picks.len(), MAX_TRADE_PICKS);
    let league = "é".repeat(64);
    app.update_settings(&json!({ "trade_league": league }))
        .unwrap();
    let mut sel = view.trade.selection.clone();
    sel.picks = picks.clone();
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.selection.picks, picks);
    assert_eq!(
        view.trade.pick_limit.as_deref(),
        Some("At most 8 modifiers in one search.")
    );
    assert!(!view.trade.can_pick);
    assert!(view.trade.link.is_none() && !view.trade.can_search);
    let why = view.trade.why_not.clone().unwrap();
    assert_eq!(
        (why.kind, why.text.as_str()),
        (
            StatusKind::Error,
            "Too many modifiers for one link: remove one."
        )
    );
    // The count line still says what the search would ask for.
    assert!(view.trade.count_line.is_some());
    let view = app.open_trade_search();
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Trade search not opened: too many modifiers for one link: remove one.".to_string()
        )
    );
    assert!(shell.rec().opened.is_empty());

    // Control: fewer picks make a link under the cap, which opens.
    sel.picks.truncate(2);
    let view = app.set_trade_search(&sel);
    let link = view.trade.link.clone().expect("a link under the cap");
    assert!(link.len() <= MAX_TRADE_LINK_CHARS);
    app.open_trade_search();
    assert_eq!(shell.rec().opened, [link]);

    // The builder itself: the cap is on the whole link.
    let small = json!({ "q": "x" });
    assert!(trade_link("Standard", &small).is_ok());
    let big = json!({ "q": "x".repeat(MAX_TRADE_LINK_CHARS) });
    assert_eq!(
        trade_link("Standard", &big).unwrap_err().log,
        "the link is too long"
    );
    app.shutdown();
    assert!(trade_lines(tmp.path()).contains(&"trade: refused: the link is too long".to_string()));
}

#[test]
fn trade_log_lines_carry_counts_only() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let league = "Secret League";
    app.update_settings(&json!({ "trade_league": league }))
        .unwrap();
    let view = choose(&app, RINGS, SAPPHIRE_RING, Some(75), &[pick(COLD, None)]);
    let link = view.trade.link.clone().unwrap();
    app.open_trade_search();
    // A refusal naming a base, and one naming a modifier.
    let mut sel = view.trade.selection.clone();
    sel.base = Some("Iron Mitts".to_string());
    app.set_trade_search(&sel);
    let mut sel = view.trade.selection.clone();
    sel.picks.push(pick("prefix-increased-armour", None));
    app.set_trade_search(&sel);
    // A failure whose error quotes the link.
    shell.rec().open_fails = Some(format!("cannot open {link}"));
    let view = app.open_trade_search();
    assert_eq!(
        status(&view).1,
        "Couldn't open the browser: cannot open the trade search."
    );
    app.shutdown();

    let log = trade_lines(tmp.path());
    assert_eq!(
        log,
        [
            "trade: opened, 1 modifier, at least 1",
            "trade: refused: unknown base type",
            "trade: refused: a modifier the base does not offer",
            "trade: open failed: cannot open the trade search",
        ]
    );
    let all = diag_lines(tmp.path()).join("\n");
    for secret in [
        league,
        "Secret",
        RINGS,
        SAPPHIRE_RING,
        "Iron Mitts",
        COLD,
        "Cold Resistance",
        "prefix-increased-armour",
        "pathofexile",
        "?q=",
        "explicit.stat",
    ] {
        assert!(!all.contains(secret), "{secret} reached the log:\n{all}");
    }
}

// The item-level suggestion ------------------------------------------------

const BOOTS: &str = "Boots";
const BANGLED_SANDALS: &str = "Bangled Sandals";
const SPEED: &str = "prefix-increased-movement-speed";

/// The query ch-data builds for one pick on the sandals at `item_level`.
fn sandals_query(item_level: Option<u32>, floor: Option<u32>) -> Value {
    trade_query(&TradeSearch {
        class: BOOTS.to_string(),
        base: BANGLED_SANDALS.to_string(),
        item_level,
        picks: vec![TradePick {
            id: SPEED.to_string(),
            floor,
        }],
        both_slots: true,
    })
    .unwrap()
    .query
}

fn ilvl_filter(query: &Value) -> &Value {
    &query["query"]["filters"]["type_filters"]["filters"]["ilvl"]
}

#[test]
fn trade_search_with_a_blank_item_level_uses_the_suggestion() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // Baseline: no floor picked, nothing suggested, and the query is the one
    // a blank field always built (no item-level filter).
    let view = choose(&app, BOOTS, BANGLED_SANDALS, None, &[pick(SPEED, None)]);
    assert_eq!(view.trade.suggested_item_level, None);
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(query, sandals_query(None, None));
    assert!(ilvl_filter(&query).is_null(), "{query}");

    // Movement speed at least T2 needs item level 65: the blank field
    // searches with it, as if it had been typed.
    let mut sel = view.trade.selection.clone();
    sel.picks = vec![pick(SPEED, Some(2))];
    let view = app.set_trade_search(&sel);
    assert_eq!(
        view.trade.selection.item_level, None,
        "the field stays blank"
    );
    assert_eq!(view.trade.suggested_item_level, Some(65));
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(query, sandals_query(Some(65), Some(2)));
    assert_eq!(ilvl_filter(&query)["min"], 65);

    // At least T1 needs 82.
    sel.picks = vec![pick(SPEED, Some(1))];
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.suggested_item_level, Some(82));
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(ilvl_filter(&query)["min"], 82);

    // The link opened is the one the view shows.
    let shown = view.trade.link.clone().unwrap();
    app.open_trade_search();
    assert_eq!(shell.rec().opened, vec![shown]);

    // A ring: fire resistance at least T1 and life at any tier gives 82.
    let view = choose(
        &app,
        RINGS,
        SAPPHIRE_RING,
        None,
        &[pick(FIRE, Some(1)), pick(LIFE, None)],
    );
    assert_eq!(view.trade.suggested_item_level, Some(82));
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(ilvl_filter(&query)["min"], 82);
    app.shutdown();
}

#[test]
fn trade_search_keeps_a_typed_item_level() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // A typed 70 with movement speed at least T2: the suggestion (65) is
    // still shown, and the query searches with the typed level.
    let view = choose(
        &app,
        BOOTS,
        BANGLED_SANDALS,
        Some(70),
        &[pick(SPEED, Some(2))],
    );
    assert_eq!(view.trade.selection.item_level, Some(70));
    assert_eq!(view.trade.suggested_item_level, Some(65));
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(query, sandals_query(Some(70), Some(2)));
    assert_eq!(ilvl_filter(&query)["min"], 70);

    // A typed level above the suggestion on a ring wins too.
    let view = choose(
        &app,
        RINGS,
        SAPPHIRE_RING,
        Some(90),
        &[pick(FIRE, Some(1)), pick(LIFE, None)],
    );
    assert_eq!(view.trade.suggested_item_level, Some(82));
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(ilvl_filter(&query)["min"], 90);

    // Clearing the field goes back to the suggestion.
    let mut sel = view.trade.selection.clone();
    sel.item_level = None;
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.selection.picks.len(), 2, "both picks kept");
    let (_, query) = split_link(view.trade.link.as_deref().expect("a link"));
    assert_eq!(ilvl_filter(&query)["min"], 82);

    // The panel closed carries no suggestion, as it carries no link.
    sel.open = false;
    let view = app.set_trade_search(&sel);
    assert_eq!(view.trade.suggested_item_level, None);
    assert!(view.trade.link.is_none());
    app.shutdown();
}
