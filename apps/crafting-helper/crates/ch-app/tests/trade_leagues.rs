//! The Settings window's league list through `App`, with a fake shell that
//! counts its league list asks and answers what the test sets (the trade
//! site's 2026-10-30 shape, a failure, an answer over the cap, other realms,
//! a private league, names that are not names, a slow answer the test
//! releases): nothing is asked for unless `refresh_trade_leagues` is sent,
//! a refresh replaces the offered list for this run only, a refused one
//! keeps the list and the league and says why, a second refresh while one
//! runs asks nothing, the saved league is always offered, and the log
//! carries counts only. No test reaches the network; each waits on the
//! Settings view the app sends, never a fixed sleep.

mod common;

use ch_app::{
    Answer, SettingsView, StatusKind, TradeSelection, DEFAULT_TRADE_LEAGUE, MAX_TRADE_LEAGUES,
    MAX_TRADE_LEAGUES_BODY, SHIPPED_TRADE_LEAGUES,
};
use common::*;
use serde_json::json;

fn strings(v: &[&str]) -> Vec<String> {
    lines(v)
}

/// [`SITE_LEAGUES`], then `saved` when they lack it.
fn site_and(saved: &str) -> Vec<String> {
    let mut v = strings(&SITE_LEAGUES);
    if !v.iter().any(|l| l == saved) {
        v.push(saved.to_string());
    }
    v
}

fn note(view: &SettingsView) -> (StatusKind, String) {
    let n = view.trade_league_note.as_ref().expect("a note");
    (n.kind, n.text.clone())
}

fn refused(why: &str) -> (StatusKind, String) {
    (
        StatusKind::Error,
        format!("Not refreshed: {why}. The list and your league are unchanged."),
    )
}

fn body(text: &str) -> Answer {
    Answer {
        status: 200,
        body: text.as_bytes().to_vec(),
    }
}

fn league_lines(dir: &std::path::Path) -> Vec<String> {
    diag_lines(dir)
        .into_iter()
        .filter(|l| l.starts_with("trade:"))
        .collect()
}

#[test]
fn trade_leagues_are_not_requested_at_startup_or_by_default_baseline() {
    let shell = FakeShell::default();
    shell.answer_leagues(Ok(site_leagues()));
    let h = harness(Setup {
        shell: Some(shell.clone()),
        saved: Some(json!({ "schema": 1, "hotkey": "Ctrl+Alt+X", "check_updates": true })),
        ..Setup::default()
    });
    // Start (with automatic update checks on), open Settings, read both
    // views, every trade command, a settings patch, an update check and two
    // days of ticks: no league request.
    h.app.start();
    h.wait_ended();
    h.app.open_settings().unwrap();
    h.app.get_app_settings();
    h.app.get_view();
    let mut sel = TradeSelection {
        open: true,
        class: Some("Rings".to_string()),
        ..TradeSelection::default()
    };
    h.app.set_trade_search(&sel);
    sel.base = Some("Sapphire Ring".to_string());
    h.app.set_trade_search(&sel);
    sel.picks = vec![ch_app::TradePickChoice {
        mod_id: "prefix-to-maximum-life".to_string(),
        floor: None,
    }];
    h.app.set_trade_search(&sel);
    h.app.open_trade_search();
    sel.open = false;
    h.app.set_trade_search(&sel);
    h.app
        .update_settings(&json!({ "trade_league": "Standard", "screen_watch": false }))
        .unwrap();
    let _ = h.app.update_settings(&json!({ "trade_league": "" }));
    h.manual(list(&[RUNNING], true));
    h.ticks(0, 48 * HOUR);
    h.wait_ended();
    assert_eq!(shell.league_calls(), 0, "a league request nobody asked for");
    let view = h.app.get_app_settings();
    assert_eq!(view.trade_leagues, SHIPPED_TRADE_LEAGUES);
    assert!(!view.trade_leagues_refreshing);
    assert!(shell
        .rec()
        .app_settings
        .iter()
        .all(|v| !v.trade_leagues_refreshing));

    // Positive control: the same instrument counts the request once asked.
    h.app.refresh_trade_leagues();
    let view = wait_refreshed(&h.app, &shell);
    assert_eq!(shell.league_calls(), 1);
    assert_eq!(view.trade_leagues, site_and("Standard"));
    h.app.shutdown();

    // A new start offers the shipped list again and asks nothing.
    let again = start(&h.store, &shell);
    assert_eq!(
        again.get_app_settings().trade_leagues,
        SHIPPED_TRADE_LEAGUES
    );
    again.shutdown();
    assert_eq!(shell.league_calls(), 1);
}

#[test]
fn trade_leagues_refresh_fills_the_list_from_the_answer() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    shell.answer_leagues(Ok(site_leagues()));
    let app = start(tmp.path(), &shell);
    assert_eq!(app.get_app_settings().trade_leagues, SHIPPED_TRADE_LEAGUES);

    // At once: refreshing, the list as it was; the window hears it.
    shell.hold_leagues(true);
    let view = app.refresh_trade_leagues();
    assert!(view.trade_leagues_refreshing);
    assert_eq!(view.trade_leagues, SHIPPED_TRADE_LEAGUES);
    assert_eq!(view.trade_league_note, None);
    assert_eq!(shell.rec().app_settings.last(), Some(&view));
    wait_until("the request", || shell.league_calls() == 1);
    shell.hold_leagues(false);

    // Then the site's poe2 leagues in its order, the saved league (the
    // default, which the answer lacks) last, and the league unchanged.
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, site_and(DEFAULT_TRADE_LEAGUE));
    assert_eq!(view.trade_league, DEFAULT_TRADE_LEAGUE);
    assert_eq!(
        note(&view),
        (
            StatusKind::Info,
            "Refreshed: 3 leagues from the trade site.".to_string()
        )
    );
    assert_eq!(shell.rec().app_settings.last(), Some(&view));
    // A league from the new list is taken and saved; the list stays.
    let view = app
        .update_settings(&json!({ "trade_league": "Dawn of the Hunt" }))
        .unwrap();
    assert_eq!(view.trade_leagues, strings(&SITE_LEAGUES));
    assert_eq!(app.get_view().trade.league, "Dawn of the Hunt");
    app.shutdown();

    // The list is never saved: the next start offers the shipped list and
    // the saved league, and asks nothing.
    let saved = saved_settings(tmp.path());
    assert_eq!(saved["trade_league"], "Dawn of the Hunt");
    assert!(saved.get("trade_leagues").is_none(), "{saved}");
    assert!(!saved.to_string().contains("HC Dawn"), "{saved}");
    let again = start(tmp.path(), &shell);
    let view = again.get_app_settings();
    let mut want = strings(&SHIPPED_TRADE_LEAGUES);
    want.push("Dawn of the Hunt".to_string());
    assert_eq!(view.trade_leagues, want);
    assert_eq!(view.trade_league_note, None);
    again.shutdown();
    assert_eq!(shell.league_calls(), 1);
}

#[test]
fn trade_leagues_refresh_keeps_the_list_and_saved_league_on_a_failed_fetch() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.update_settings(&json!({ "trade_league": "Runes of Aldur" }))
        .unwrap();

    // No answer: the transport's words, the shipped list and the league kept.
    shell.answer_leagues(Err("the server could not be reached".to_string()));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(note(&view), refused("the server could not be reached"));
    assert_eq!(view.trade_leagues, SHIPPED_TRADE_LEAGUES);
    assert_eq!(view.trade_league, "Runes of Aldur");

    // A refreshed list survives a later failure, a status other than 200
    // and a redirect alike.
    shell.answer_leagues(Ok(site_leagues()));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, site_and("Runes of Aldur"));
    for (code, why) in [
        (503, "the trade site answered 503"),
        (302, "the trade site answered 302"),
        (404, "the trade site answered 404"),
    ] {
        let mut answer = site_leagues();
        answer.status = code;
        shell.answer_leagues(Ok(answer));
        app.refresh_trade_leagues();
        let view = wait_refreshed(&app, &shell);
        assert_eq!(note(&view), refused(why));
        assert_eq!(view.trade_leagues, site_and("Runes of Aldur"), "{code}");
        assert_eq!(view.trade_league, "Runes of Aldur");
    }
    // A shell with nothing to answer is a failure too.
    shell.rec().league_answer = None;
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(note(&view), refused(NO_LEAGUE_ANSWER));
    assert_eq!(view.trade_league, "Runes of Aldur");
    assert_eq!(shell.league_calls(), 6);
    app.shutdown();
    assert_eq!(saved_settings(tmp.path())["trade_league"], "Runes of Aldur");
}

#[test]
fn trade_leagues_refresh_refuses_a_malformed_or_oversized_answer() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // One byte over the cap, though it is a good list.
    let pad = "p".repeat(MAX_TRADE_LEAGUES_BODY);
    let over = format!(r#"{{"result":[{{"id":"Standard","realm":"poe2","text":"{pad}"}}]}}"#);
    assert!(over.len() > MAX_TRADE_LEAGUES_BODY);
    let many: Vec<(String, &str)> = (0..=MAX_TRADE_LEAGUES)
        .map(|i| (format!("League {i}"), "poe2"))
        .collect();
    let many: Vec<(&str, &str)> = many.iter().map(|(l, r)| (l.as_str(), *r)).collect();
    let cases: Vec<(Answer, &str)> = vec![
        (body(&over), "the answer is larger than 64 KiB"),
        (body("not json"), "the answer is not a league list"),
        (body(r#"{"result":{}}"#), "the answer is not a league list"),
        (body(r#"{"leagues":[]}"#), "the answer is not a league list"),
        (
            body(r#"[{"id":"Standard","realm":"poe2"}]"#),
            "the answer is not a league list",
        ),
        (
            body(r#"{"result":[{"realm":"poe2"}]}"#),
            "the answer is not a league list",
        ),
        (
            body(r#"{"result":[{"id":7,"realm":"poe2"}]}"#),
            "the answer is not a league list",
        ),
        (
            body(r#"{"result":[{"id":"Standard"}]}"#),
            "the answer is not a league list",
        ),
        (
            leagues_answer(&many),
            "the answer lists more than 50 leagues",
        ),
        (
            body(r#"{"result":[]}"#),
            "the answer holds no Path of Exile 2 league",
        ),
        (
            leagues_answer(&[("Standard", "pc"), ("Hardcore", "xbox")]),
            "the answer holds no Path of Exile 2 league",
        ),
    ];
    for (answer, why) in cases {
        shell.answer_leagues(Ok(answer));
        app.refresh_trade_leagues();
        let view = wait_refreshed(&app, &shell);
        assert_eq!(note(&view), refused(why));
        assert_eq!(view.trade_leagues, SHIPPED_TRADE_LEAGUES, "{why}");
        assert_eq!(view.trade_league, DEFAULT_TRADE_LEAGUE);
    }

    // Controls: exactly 50 leagues, and an answer of exactly the cap, are
    // read.
    let fifty: Vec<(String, &str)> = (0..MAX_TRADE_LEAGUES)
        .map(|i| (format!("League {i}"), "poe2"))
        .collect();
    let fifty: Vec<(&str, &str)> = fifty.iter().map(|(l, r)| (l.as_str(), *r)).collect();
    shell.answer_leagues(Ok(leagues_answer(&fifty)));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues.len(), MAX_TRADE_LEAGUES + 1);
    let head = r#"{"result":[{"id":"Standard","realm":"poe2","text":""#;
    let tail = r#""}]}"#;
    let at_cap = format!(
        "{head}{}{tail}",
        "p".repeat(MAX_TRADE_LEAGUES_BODY - head.len() - tail.len())
    );
    assert_eq!(at_cap.len(), MAX_TRADE_LEAGUES_BODY);
    shell.answer_leagues(Ok(body(&at_cap)));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, ["Standard", DEFAULT_TRADE_LEAGUE]);
    app.shutdown();
}

#[test]
fn trade_leagues_refresh_skips_other_realms_private_and_invalid_names() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let long = "x".repeat(65);
    let answer = json!({ "result": [
        { "id": "Standard", "realm": "poe2", "text": "Standard" },
        { "id": "Settlers", "realm": "pc", "text": "Settlers" },
        { "id": "Private League (PL123)", "realm": "poe2", "text": "Private", "private": true },
        { "id": "", "realm": "poe2", "text": "" },
        { "id": " Padded ", "realm": "poe2", "text": "Padded" },
        { "id": long, "realm": "poe2", "text": "long" },
        { "id": "Tab\tName", "realm": "poe2", "text": "tab" },
        { "id": "Standard", "realm": "poe2", "text": "again" },
        { "id": "Hardcore", "realm": "poe2", "text": "Hardcore", "private": false },
    ]});
    shell.answer_leagues(Ok(body(&answer.to_string())));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    // Kept in the site's order, each once; the saved league last.
    assert_eq!(
        view.trade_leagues,
        ["Standard", "Hardcore", DEFAULT_TRADE_LEAGUE]
    );
    assert_eq!(
        note(&view),
        (
            StatusKind::Info,
            "Refreshed: 2 leagues from the trade site; 7 skipped.".to_string()
        )
    );
    app.shutdown();
    assert!(league_lines(tmp.path())
        .contains(&"trade: leagues refreshed, 2 leagues, 7 skipped".to_string()));
}

#[test]
fn trade_leagues_second_refresh_while_running_sends_no_request() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    shell.answer_leagues(Ok(site_leagues()));
    let app = start(tmp.path(), &shell);

    shell.hold_leagues(true);
    let first = app.refresh_trade_leagues();
    assert!(first.trade_leagues_refreshing);
    wait_until("the request in flight", || shell.league_calls() == 1);
    // Clicks while it runs start nothing and return the view as it is.
    for _ in 0..3 {
        let view = app.refresh_trade_leagues();
        assert!(view.trade_leagues_refreshing);
        assert_eq!(view.trade_leagues, SHIPPED_TRADE_LEAGUES);
    }
    assert_eq!(shell.league_calls(), 1);
    // The app answers other commands meanwhile: no lock is held across the
    // request.
    app.update_settings(&json!({ "trade_league": "Standard" }))
        .unwrap();
    assert!(app.get_app_settings().trade_leagues_refreshing);
    shell.hold_leagues(false);
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, strings(&SITE_LEAGUES));
    assert_eq!(shell.league_calls(), 1);
    let refreshed = |dir| {
        league_lines(dir)
            .iter()
            .filter(|l| l.starts_with("trade: leagues refreshed"))
            .count()
    };
    assert_eq!(refreshed(tmp.path()), 1);

    // Control: once it ended, a refresh asks again.
    app.refresh_trade_leagues();
    wait_refreshed(&app, &shell);
    assert_eq!(shell.league_calls(), 2);
    app.shutdown();
    assert_eq!(refreshed(tmp.path()), 2);
}

#[test]
fn trade_leagues_offer_the_saved_league_when_the_list_lacks_it() {
    let tmp = tempfile::tempdir().unwrap();
    save_settings(
        tmp.path(),
        &json!({ "schema": 1, "trade_league": "Settlers of Old" }),
    );
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let view = app.get_app_settings();
    assert_eq!(view.trade_league, "Settlers of Old");
    let mut want = strings(&SHIPPED_TRADE_LEAGUES);
    want.push("Settlers of Old".to_string());
    assert_eq!(view.trade_leagues, want);
    // A refreshed list lacking it still offers it, last; the league stays.
    shell.answer_leagues(Ok(site_leagues()));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, site_and("Settlers of Old"));
    assert_eq!(view.trade_league, "Settlers of Old");
    assert_eq!(app.get_view().trade.league, "Settlers of Old");
    // A refused refresh keeps offering it too.
    shell.answer_leagues(Err("the request timed out".to_string()));
    app.refresh_trade_leagues();
    let view = wait_refreshed(&app, &shell);
    assert_eq!(view.trade_leagues, site_and("Settlers of Old"));
    app.shutdown();

    // The next start: the shipped list and the saved league, no request.
    let again = start(tmp.path(), &shell);
    assert_eq!(again.get_app_settings().trade_leagues, want);
    again.shutdown();
    assert_eq!(shell.league_calls(), 2);
}

#[test]
fn trade_leagues_log_lines_carry_counts_only() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.update_settings(&json!({ "trade_league": "HC Forbidden Rites" }))
        .unwrap();
    shell.answer_leagues(Ok(site_leagues()));
    app.refresh_trade_leagues();
    wait_refreshed(&app, &shell);
    let mut answer = site_leagues();
    answer.status = 503;
    shell.answer_leagues(Ok(answer));
    app.refresh_trade_leagues();
    wait_refreshed(&app, &shell);
    shell.answer_leagues(Err("the request timed out".to_string()));
    app.refresh_trade_leagues();
    wait_refreshed(&app, &shell);
    shell.answer_leagues(Ok(leagues_answer(&[
        ("Standard", "poe2"),
        ("Settlers", "pc"),
    ])));
    app.refresh_trade_leagues();
    wait_refreshed(&app, &shell);
    app.shutdown();

    assert_eq!(
        league_lines(tmp.path()),
        [
            "trade: leagues refreshed, 3 leagues, 0 skipped",
            "trade: league refresh refused: the trade site answered 503",
            "trade: league refresh refused: the request timed out",
            "trade: leagues refreshed, 1 league, 1 skipped",
        ]
    );
    let all = diag_lines(tmp.path()).join("\n");
    for name in
        SITE_LEAGUES
            .iter()
            .chain(&SHIPPED_TRADE_LEAGUES)
            .chain(&["Settlers", "Forbidden", "Hunt"])
    {
        assert!(!all.contains(name), "{name} reached the log:\n{all}");
    }
}
