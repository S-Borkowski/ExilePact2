//! The parity test: the engine against the slot model, through the case file
//! they share, `tests/items/slots/cases.json` (written by the model's item,
//! `tools/crafting_helper/slot_model.py`; never edited here). Each case is
//! an abstract item, its ordered targets and what each shipped v2 blueprint
//! must preview on it (docs/models/crafting-slots-spec.md, "Representative
//! crafts"). This file keeps the other half: per case, a constructed item
//! text in `tests/items/slots/` (see its README) and the real target ids and
//! floors behind each alias. A difference is settled by the spec: the
//! engine wrong, fix the engine; the case wrong, the case file's owner
//! fixes it.

mod common;

use ch_engine::ch_blueprint::Side;
use ch_engine::ch_data::Section;
use ch_engine::{Engine, Odds, Plan, Rejection, Route, StepView, Target};
use common::{crate_dir, engine, item, items_path, shipped};
use serde_json::Value;

/// The four state-ordered blueprints the cases expect, all shipped.
const V2: [&str; 4] = [
    "close-a-side",
    "essence-desecrate-double-slam",
    "magic-augment-essence",
    "rare-clean-then-slam",
];

/// The listed `fill-desecrate-exalt`, kept as a test blueprint for the
/// baseline.
const LISTED: &str = "gear/fill-desecrate-exalt/fill-desecrate-exalt.toml";

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const ACC: &str = "prefix-to-accuracy-rating";
const EVASION: &str = "prefix-to-evasion-rating";
const DEFENCES: &str = "desecrated-prefix-increased-global-armour-evasion-and-energy-shield";
const LOW_LIFE_ATTACK: &str = "desecrated-prefix-increased-attack-damage-while-on-low-life";
const ARMOUR_PCT: &str = "prefix-increased-armour";
const SPIRIT: &str = "prefix-to-spirit";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";

/// One case's half of the table: its item text under `tests/items/slots/`,
/// and each target alias's data modifier id and floor.
struct Fixture {
    item: &'static str,
    targets: &'static [(&'static str, &'static str, Option<u32>)],
}

/// Case id to its item text and its targets. Every floor is the tier the
/// case's currency reaches: a Perfect Exalted Orb (modifier level 50 and
/// up) and a Perfect Orb of Augmentation (70 and up) have a tier at or
/// above each floor they are expected to place, and Lesser Essence of the
/// Body adds maximum Life T8 on an amulet, the floor of the essence target.
fn fixture(case: &str) -> Option<Fixture> {
    Some(match case {
        // Mana T5 and Life T4 (prefixes), Fire Resistance T5 (suffix);
        // Accuracy Rating T3 missing.
        "rare-2p1s-prefix-missing" => Fixture {
            item: "slots/rare-2p1s-prefix-missing.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("p3", ACC, Some(3)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // The same ring; the case blocks Omen of Sinistral Necromancy.
        "rare-2p1s-necromancy-blocked" => Fixture {
            item: "slots/rare-2p1s-necromancy-blocked.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("p3", ACC, Some(3)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // A str body armour, item level 82: Life T5 and increased Armour T2
        // (prefixes), Fire Resistance T3 (suffix); Spirit T3 missing. The
        // class has desecrated suffixes only.
        "body-armour-prefix-no-pool" => Fixture {
            item: "slots/body-armour-prefix-no-pool.txt",
            targets: &[
                ("p1", LIFE, Some(5)),
                ("p2", ARMOUR_PCT, Some(2)),
                ("p3", SPIRIT, Some(3)),
                ("s1", FIRE, Some(3)),
            ],
        },
        // The 2P/1S ring with a revealed desecrated Attack Damage prefix
        // (its one tier) in Mana's place, a target; Accuracy Rating T3
        // missing.
        "rare-desecrated-target" => Fixture {
            item: "slots/rare-desecrated-target.txt",
            targets: &[
                ("p1", LOW_LIFE_ATTACK, Some(1)),
                ("p2", LIFE, Some(4)),
                ("p3", ACC, Some(3)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // The 2P/1S ring with a revealed desecrated Remnant Effect prefix,
        // not a target (a reveal that offered none); Accuracy Rating T3
        // missing.
        "rare-wrong-reveal" => Fixture {
            item: "slots/rare-wrong-reveal.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("p3", ACC, Some(3)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // Magic amulet: Mana T6 and Cold Resistance T5. Life T8 is the
        // Lesser Essence of the Body's; the desecrated Global Defences
        // prefix has its one tier.
        "magic-two-good" => Fixture {
            item: "slots/magic-two-good.txt",
            targets: &[
                ("p1", MANA, Some(6)),
                ("p2", LIFE, Some(8)),
                ("p3", DEFENCES, Some(1)),
                ("s1", COLD, Some(5)),
                ("s2", FIRE, Some(3)),
                ("s3", LIGHTNING, Some(3)),
            ],
        },
        // A fractured Mana T4, Accuracy Rating T3 and Cold Damage T3
        // (prefixes); Fire Resistance T6 (suffix), the lowest modifier
        // level; Life T4 missing.
        "rare-fractured" => Fixture {
            item: "slots/rare-fractured.txt",
            targets: &[
                ("p1", MANA, Some(4)),
                ("p2", LIFE, Some(4)),
                ("s1", FIRE, Some(6)),
            ],
        },
        // Issue #28: Mana T5 and Accuracy Rating T4 (prefixes), Cold
        // Resistance T5 and Dexterity T7 (suffixes, Dexterity the lowest
        // level). Chaos Resistance (spawn weight 250) is the hardest.
        "rare-one-target" => Fixture {
            item: "slots/rare-one-target.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("p3", EVASION, Some(3)),
                ("s1", CHAOS, Some(3)),
            ],
        },
        // Issue #28's belt: Charm Effect Duration T3 and Flask Mana
        // Recovery T5 (prefixes, the latter the lowest level), Stun
        // Threshold T5 and Life Regeneration T5 (suffixes). Life T1, one
        // tier, is the hardest.
        "rare-no-target" => Fixture {
            item: "slots/rare-no-target.txt",
            targets: &[
                ("p1", LIFE, Some(1)),
                ("p2", MANA, Some(2)),
                ("s1", COLD, Some(3)),
                ("s2", FIRE, Some(3)),
            ],
        },
        // Magic amulet: Mana T6 and an open suffix.
        "magic-one-good" => Fixture {
            item: "slots/magic-one-good.txt",
            targets: &[
                ("p1", MANA, Some(6)),
                ("p2", LIFE, Some(8)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // Magic amulet: Mana T6 and Intelligence T5, not a target.
        "magic-good-and-bad" => Fixture {
            item: "slots/magic-good-and-bad.txt",
            targets: &[
                ("p1", MANA, Some(6)),
                ("p2", LIFE, Some(8)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // Mana T5, Rarity of Items T3 (the lone lowest level) and Accuracy
        // Rating T3 (prefixes), Fire Resistance T5 (suffix); Life missing.
        "rare-whittle-certain" => Fixture {
            item: "slots/rare-whittle-certain.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("s1", FIRE, Some(5)),
            ],
        },
        // The same item, with Omen of Whittling blocked by the case.
        "rare-whittle-blocked" => Fixture {
            item: "slots/rare-whittle-blocked.txt",
            targets: &[
                ("p1", MANA, Some(5)),
                ("p2", LIFE, Some(4)),
                ("s1", FIRE, Some(5)),
            ],
        },
        _ => return None,
    })
}

fn cases() -> Vec<Value> {
    let path = items_path("slots/cases.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let file: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    file["cases"].as_array().expect("`cases` is a list").clone()
}

fn text(v: &Value) -> &str {
    v.as_str()
        .unwrap_or_else(|| panic!("expected a string, got {v}"))
}

/// A step as the case file writes it: goal, side or `null`, currency as
/// written, omens resolved, and for a removal whether it is certain.
fn written(step: &StepView) -> String {
    let action = step.action.as_ref();
    let side = step.side.map_or("null", Side::name);
    let currency = action.map_or("", |a| a.currency.as_str());
    let omens = action.map(|a| a.omens.join(", ")).unwrap_or_default();
    format!("{} {side} {currency} [{omens}]", step.goal)
}

fn expected(step: &Value) -> String {
    let side = step["side"].as_str().unwrap_or("null");
    let omens: Vec<&str> = step["omens"]
        .as_array()
        .expect("`omens` is a list")
        .iter()
        .map(text)
        .collect();
    format!(
        "{} {side} {} [{}]",
        text(&step["goal"]),
        text(&step["currency"]),
        omens.join(", ")
    )
}

/// Whether the step's odds say it is certain (100%).
fn certain(step: &StepView) -> bool {
    step.odds.as_ref().and_then(Odds::percent) == Some(100.0)
}

fn kind(rejection: &Rejection) -> String {
    serde_json::to_value(rejection).unwrap()["kind"]
        .as_str()
        .unwrap()
        .to_string()
}

fn route<'a>(plan: &'a Plan, blueprint: &str) -> Result<&'a Route, String> {
    if let Some(r) = plan.rejected.iter().find(|r| r.blueprint == blueprint) {
        return Err(format!("rejected `{}`: {}", kind(&r.reason), r.reason));
    }
    plan.routes
        .iter()
        .find(|r| r.blueprint == blueprint)
        .ok_or_else(|| "neither routed nor rejected".to_string())
}

/// Every difference between one blueprint's route on the item and what
/// the case expects of it.
fn compare(case: &str, blueprint: &str, plan: &Plan, expect: &Value, out: &mut Vec<String>) {
    let at = format!("{case} / {blueprint}");
    if let Some(rejected) = expect.get("rejected") {
        let want = text(&rejected["kind"]);
        match plan.rejected.iter().find(|r| r.blueprint == blueprint) {
            Some(r) if kind(&r.reason) == want => {}
            Some(r) => out.push(format!(
                "{at}: rejected `{}` ({}), the case says `{want}`",
                kind(&r.reason),
                r.reason
            )),
            None => out.push(format!("{at}: not rejected, the case says `{want}`")),
        }
        return;
    }
    let route = match route(plan, blueprint) {
        Ok(r) => r,
        Err(why) => {
            out.push(format!("{at}: {why}, the case expects a route"));
            return;
        }
    };
    let want: Vec<&Value> = expect["steps"]
        .as_array()
        .expect("`steps` is a list")
        .iter()
        .collect();
    let complete = expect["complete"].as_bool().unwrap_or(false);
    let got: Vec<String> = route.steps.iter().map(written).collect();
    let wanted: Vec<String> = want.iter().map(|s| expected(s)).collect();
    let compared = if complete {
        got.len()
    } else {
        wanted.len().min(got.len())
    };
    if got[..compared] != wanted[..] {
        out.push(format!(
            "{at}: the route is\n    {}\n  the case says ({})\n    {}",
            got.join("\n    "),
            if complete { "complete" } else { "first steps" },
            wanted.join("\n    ")
        ));
        return;
    }
    for (i, (step, w)) in route.steps.iter().zip(&want).enumerate() {
        if let Some(c) = w.get("certain").and_then(Value::as_bool) {
            if certain(step) != c {
                out.push(format!(
                    "{at}: step {} ({}) has odds {:?}, the case says certain = {c}",
                    i + 1,
                    step.id,
                    step.odds
                ));
            }
        }
    }
}

/// The listed baseline: what the listed `fill-desecrate-exalt` offers
/// first, or its rejection.
fn compare_listed(case: &str, plan: &Plan, expect: &Value, out: &mut Vec<String>) {
    let at = format!("{case} / fill-desecrate-exalt (listed)");
    if let Some(rejected) = expect.get("rejected") {
        let want = text(&rejected["kind"]);
        match plan
            .rejected
            .iter()
            .find(|r| r.blueprint == "fill-desecrate-exalt")
        {
            Some(r) if kind(&r.reason) == want => {}
            Some(r) => out.push(format!(
                "{at}: rejected `{}`, the case says `{want}`",
                kind(&r.reason)
            )),
            None => out.push(format!("{at}: not rejected, the case says `{want}`")),
        }
        return;
    }
    match route(plan, "fill-desecrate-exalt") {
        Ok(r) => {
            let first = &r.steps[0];
            let action = first.action.as_ref().expect("an action");
            let omens: Vec<&str> = expect["omens"]
                .as_array()
                .unwrap()
                .iter()
                .map(text)
                .collect();
            if first.id != text(&expect["step"])
                || action.currency != text(&expect["currency"])
                || action.omens != omens
            {
                out.push(format!(
                    "{at}: first step `{}` with {} {:?}, the case says {expect}",
                    first.id, action.currency, action.omens
                ));
            }
        }
        Err(why) => out.push(format!("{at}: {why}")),
    }
}

/// The case's targets in its order, through the table; a table that does
/// not name an alias, or names it on the wrong side or pool, is reported.
fn targets_of(
    case: &Value,
    fx: &Fixture,
    plan_targets: Option<&Plan>,
    out: &mut Vec<String>,
) -> Vec<Target> {
    let id = text(&case["id"]);
    let mut list = Vec::new();
    for (i, t) in case["targets"].as_array().unwrap().iter().enumerate() {
        let alias = text(&t["alias"]);
        let Some(&(_, mod_id, floor)) = fx.targets.iter().find(|(a, _, _)| *a == alias) else {
            out.push(format!("{id}: the table has no target `{alias}`"));
            continue;
        };
        list.push(Target::new(mod_id, floor));
        if let Some(plan) = plan_targets {
            let info = &plan.targets[i];
            let side = if text(&t["side"]) == "prefix" {
                "Prefix"
            } else {
                "Suffix"
            };
            let pool = if text(&t["pool"]) == "desecrated" {
                Section::Desecrated
            } else {
                Section::Normal
            };
            if format!("{:?}", info.side) != side || info.section != pool {
                out.push(format!(
                    "{id}: `{alias}` ({mod_id}) is a {:?} {:?} modifier, the case says {} {}",
                    info.section,
                    info.side,
                    text(&t["side"]),
                    text(&t["pool"])
                ));
            }
        }
    }
    list
}

/// The item text matches the case's abstract item: rarity, and per side
/// the number of modifiers and which are fractured, in order.
fn item_shape(case: &Value, it: &ch_engine::ch_item::Item, out: &mut Vec<String>) {
    use ch_engine::ch_item::ModKind;
    let id = text(&case["id"]);
    let rarity = format!("{:?}", it.rarity).to_lowercase();
    if rarity != text(&case["rarity"]) {
        out.push(format!(
            "{id}: the item is {rarity}, the case says {}",
            text(&case["rarity"])
        ));
    }
    if format!("{:?}", it.class)
        .to_lowercase()
        .replace(['_', ' '], "")
        != text(&case["class"]).to_lowercase().replace(['_', ' '], "")
    {
        out.push(format!(
            "{id}: the item is {:?}, the case says {}",
            it.class,
            text(&case["class"])
        ));
    }
    for (key, kind) in [("prefixes", ModKind::Prefix), ("suffixes", ModKind::Suffix)] {
        let on_item: Vec<bool> = it
            .modifiers
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.fractured)
            .collect();
        let in_case: Vec<bool> = case[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["fractured"].as_bool().unwrap())
            .collect();
        if on_item != in_case {
            out.push(format!(
                "{id}: {key} fractured {on_item:?} on the item, {in_case:?} in the case"
            ));
        }
    }
}

#[test]
fn every_case_projects_as_the_model_records() {
    let v2 = shipped();
    let listed: Engine = engine(&[LISTED]);
    let mut out: Vec<String> = Vec::new();
    let all = cases();
    assert!(!all.is_empty(), "the case file has no case");

    for case in &all {
        let id = text(&case["id"]);
        let Some(fx) = fixture(id) else {
            out.push(format!(
                "{id}: no item text or targets in this file's table"
            ));
            continue;
        };
        let it = item(fx.item);
        item_shape(case, &it, &mut out);
        let list = targets_of(case, &fx, None, &mut out);
        let blocked: Vec<String> = case["blocked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| text(b).to_string())
            .collect();

        let plan = match v2.plan(&it, &list, &blocked) {
            Ok(p) => p,
            Err(e) => {
                out.push(format!("{id}: the engine refuses the item: {e}"));
                continue;
            }
        };
        // The table's ids are on the side and in the pool the case says.
        let mut sides = Vec::new();
        targets_of(case, &fx, Some(&plan), &mut sides);
        out.extend(sides);

        let expect = case["expect"].as_object().expect("`expect` is a table");
        for bp in V2 {
            match expect.get(bp) {
                Some(e) => compare(id, bp, &plan, e, &mut out),
                None => out.push(format!("{id}: the case has no expectation for {bp}")),
            }
        }
        if let Some(e) = case.get("listed") {
            let plan = listed.plan(&it, &list, &blocked).unwrap();
            compare_listed(id, &plan, e, &mut out);
        }
    }
    assert!(
        out.is_empty(),
        "{} difference(s) between the engine and {}:\n{}",
        out.len(),
        crate_dir().join("tests/items/slots/cases.json").display(),
        out.join("\n")
    );
}

/// The negative control: the same comparison reports an expectation the
/// engine does not meet (an omen dropped, a certainty flipped, a rejection
/// that is not the engine's), so the parity test's silence means agreement.
#[test]
fn the_parity_comparison_reports_a_changed_expectation() {
    let v2 = shipped();
    let all = cases();
    let case = all
        .iter()
        .find(|c| c["id"] == "rare-fractured")
        .expect("the rare-fractured case");
    let fx = fixture("rare-fractured").unwrap();
    let list = targets_of(case, &fx, None, &mut Vec::new());
    let plan = v2.plan(&item(fx.item), &list, &[]).unwrap();

    let mut expect = case["expect"]["rare-clean-then-slam"].clone();
    let mut out = Vec::new();
    compare(
        "rare-fractured",
        "rare-clean-then-slam",
        &plan,
        &expect,
        &mut out,
    );
    assert!(out.is_empty(), "{out:?}");

    expect["steps"][1]["omens"] = serde_json::json!([]);
    compare(
        "rare-fractured",
        "rare-clean-then-slam",
        &plan,
        &expect,
        &mut out,
    );
    assert_eq!(out.len(), 1, "a dropped omen is not reported");

    let mut expect = case["expect"]["rare-clean-then-slam"].clone();
    expect["steps"][0]["certain"] = Value::Bool(false);
    out.clear();
    compare(
        "rare-fractured",
        "rare-clean-then-slam",
        &plan,
        &expect,
        &mut out,
    );
    assert_eq!(out.len(), 1, "a flipped certainty is not reported");

    let expect = serde_json::json!({"rejected": {"kind": "no-start-fits"}});
    out.clear();
    compare(
        "rare-fractured",
        "rare-clean-then-slam",
        &plan,
        &expect,
        &mut out,
    );
    assert_eq!(
        out.len(),
        1,
        "a rejection the engine does not make is not reported"
    );

    // An item text that is not the case's item (a fractured flag lost).
    let mut shape = Vec::new();
    item_shape(case, &item("slots/rare-one-target.txt"), &mut shape);
    assert!(!shape.is_empty(), "a different item is not reported");
}
