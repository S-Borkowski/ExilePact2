//! The blueprint format: the reader, the validator's errors (each pinned to
//! its `file:line:col`), the catalogue and the embedded list.
//!
//! The test blueprints are under `tests/fixtures/`: `valid/` ones the
//! validator takes, `invalid/` ones it refuses. Each is read as the path
//! `tests/fixtures/<valid|invalid>/<name>.toml`, the one its errors name.

use std::collections::HashSet;
use std::path::Path;

use ch_blueprint::catalogue::{self, data_backed, id_from_name, is_kebab, ItemGroup, Kind, OmenFits};
use ch_blueprint::{
    check, Binding, Checked, Currency, Diagnostic, Goal, Order, Rarity, Side, SideRule, Spanned, Transition, CATALOGUE,
};

/// Checks a fixture, read as the path its errors name.
fn fixture(rel: &str) -> (String, Result<Checked, Vec<Diagnostic>>) {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(rel);
    let text = std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()));
    // As the repository stores it: a CRLF checkout would end an unclosed
    // string at the `\r`, a second error the syntax case does not expect.
    let text = text.replace("\r\n", "\n");
    let shown = format!("tests/fixtures/{rel}");
    let result = check(&shown, &text);
    (shown, result)
}

fn errors(rel: &str) -> (String, Vec<String>) {
    let (shown, result) = fixture(rel);
    match result {
        Ok(_) => panic!("{shown} was taken; it should be refused"),
        Err(list) => (shown, list.iter().map(|d| d.to_string()).collect()),
    }
}

fn valid(rel: &str) -> Checked {
    match fixture(rel).1 {
        Ok(c) => c,
        Err(list) => panic!(
            "{rel} was refused:\n{}",
            list.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n")
        ),
    }
}

/// A blueprint text around one step, for the small cases.
fn one_step(step: &str) -> String {
    format!(
        "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\n\n[[start]]\nrarity = [\"rare\"]\n\n[[step]]\n{step}\n"
    )
}

fn step_errors(step: &str) -> Vec<String> {
    match check("case.toml", &one_step(step)) {
        Ok(_) => Vec::new(),
        Err(list) => list.iter().map(|d| d.to_string()).collect(),
    }
}

#[test]
fn minimal_blueprint_parses() {
    let c = valid("valid/minimal.toml");
    let b = &c.blueprint;
    assert_eq!(b.id.get_ref(), "minimal");
    assert_eq!(b.name.get_ref(), "Minimal");
    let classes: Vec<&str> = b.classes.get_ref().iter().map(|c| c.get_ref().as_str()).collect();
    assert_eq!(classes, ["Rings"]);
    assert_eq!(b.source.get_ref(), "roadmap");
    assert!(b.notes.is_none());
    assert_eq!(b.starts.len(), 1);
    let start = b.starts[0].get_ref();
    assert_eq!(start.rarity.as_ref().map(|r| r.get_ref().clone()), Some(vec![Rarity::Rare]));
    assert_eq!(b.steps.len(), 1);
    let step = b.steps[0].get_ref();
    assert_eq!(step.id.get_ref(), "slam");
    assert_eq!(*step.goal.get_ref(), Goal::Fill);
    assert_eq!(step.actions.len(), 1);
    assert_eq!(step.actions[0].get_ref().currency.get_ref(), "exalted-orb");
    assert!(step.actions[0].get_ref().omens.is_empty());
    // The defaults: hit goes on, miss repeats.
    assert_eq!(step.hit_transition(true), Transition::Next);
    assert_eq!(step.miss_transition(), Transition::Repeat);
    // The spans point at the values: `"exalted-orb"` is on line 17, column 12.
    let span = step.actions[0].get_ref().currency.span();
    assert_eq!(ch_blueprint::line_col(&c.text, span.start), (17, 12));
}

#[test]
fn every_key_of_the_format_parses() {
    let b = valid("valid/every-key.toml").blueprint;
    assert_eq!(b.starts.len(), 2);
    let rare = b.starts[1].get_ref();
    assert_eq!(rare.wanted_at_least.as_ref().map(|n| *n.get_ref()), Some(3));
    assert_eq!(rare.fractured_wanted.as_ref().map(|f| *f.get_ref()), Some(true));
    assert_eq!(rare.unwanted_at_most.as_ref().map(|n| *n.get_ref()), Some(2));
    let ids: Vec<&str> = b.steps.iter().map(|s| s.get_ref().id.get_ref().as_str()).collect();
    assert_eq!(
        ids,
        ["essence", "fill", "whittle", "desecrate", "reveal", "unseal", "flux", "alloy", "quality", "socket", "fracture"]
    );

    let fill = b.steps[1].get_ref();
    assert_eq!(fill.side.as_ref().map(|s| *s.get_ref()), Some(Side::Prefix));
    assert_eq!(fill.bind.as_ref().map(|s| *s.get_ref()), Some(Binding::AnyRemaining));
    let until = fill.until.as_ref().unwrap().get_ref();
    assert_eq!(until.wanted_remaining_at_most.as_ref().map(|n| *n.get_ref()), Some(2));
    // A hit repeats while `until` fails, unless `on_hit` says otherwise.
    assert_eq!(fill.hit_transition(false), Transition::Next);
    assert_eq!(fill.miss_transition(), Transition::Goto("whittle".into()));
    assert_eq!(fill.actions.len(), 2);
    let omens: Vec<&str> = fill.actions[0].get_ref().omens.iter().map(|o| o.get_ref().as_str()).collect();
    assert_eq!(omens, ["side-omen", "omen-of-greater-exaltation"]);

    let essence = b.steps[0].get_ref();
    assert!(essence.when.is_some());
    assert_eq!(essence.hit_transition(true), Transition::Next);
    let socket = b.steps[9].get_ref();
    assert_eq!(*socket.goal.get_ref(), Goal::Manual);
    assert!(socket.actions.is_empty());
    assert_eq!(socket.hit_transition(true), Transition::Restart);
}

#[test]
fn until_repeats_a_hit_by_default() {
    let text = one_step(
        "id = \"fill\"\nsay = \"Exalt\"\ngoal = \"fill\"\nuntil = { wanted_remaining_at_most = 2 }\n[[step.action]]\ncurrency = \"exalted-orb\"",
    );
    let c = check("case.toml", &text).unwrap();
    let step = c.blueprint.steps[0].get_ref();
    assert_eq!(step.hit_transition(false), Transition::Repeat);
    assert_eq!(step.hit_transition(true), Transition::Next);
}

#[test]
fn syntax_error_names_file_line_and_column() {
    let (shown, list) = errors("invalid/syntax.toml");
    assert_eq!(list.len(), 1, "{list:?}");
    // The string opened on line 5 runs to the end of the line, column 44.
    let at = format!("{shown}:5:44: ");
    assert!(list[0].starts_with(&at), "{list:?}");
    assert!(list[0].len() > at.len(), "the error says what is wrong: {list:?}");
}

#[test]
fn unknown_key_names_its_line() {
    let (shown, list) = errors("invalid/unknown-key.toml");
    assert_eq!(
        list,
        [
            format!(
                "{shown}:13:1: unknown key `colour` in [[step]] (known: id, say, goal, side, bind, requires, when, until, action, on_hit, on_miss, guard)"
            ),
            format!("{shown}:17:1: unknown key `omen` in [[step.action]] (known: currency, omens)"),
        ]
    );
}

#[test]
fn unknown_currency_names_its_line() {
    let (shown, list) = errors("invalid/unknown-currency.toml");
    assert_eq!(list, [format!("{shown}:15:12: step `slam`: unknown currency `exalted-orbb`")]);
}

#[test]
fn omen_must_fit_its_currency() {
    // Whittling on an exalt is refused; on a chaos orb, below it, it is not.
    let (shown, list) = errors("invalid/omen-must-fit.toml");
    assert_eq!(
        list,
        [format!(
            "{shown}:17:10: step `slam`: `omen-of-whittling` does not modify `exalted-orb` (it modifies chaos-orb, greater-chaos-orb, perfect-chaos-orb)"
        )]
    );
    // Crystallisation fits a Perfect essence and an alloy, not a Lesser one.
    let fits = |currency: &str| {
        step_errors(&format!(
            "id = \"e\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nbind = \"essence\"\non_miss = \"next\"\n[[step.action]]\ncurrency = \"{currency}\"\nomens = [\"omen-of-sinistral-crystallisation\"]"
        ))
    };
    assert_eq!(fits("perfect-essence-of-the-body"), Vec::<String>::new());
    assert_eq!(fits("runic-alloy"), Vec::<String>::new());
    assert_eq!(fits("essence-of-hysteria"), Vec::<String>::new());
    assert_eq!(fits("lesser-essence-of-the-body").len(), 1);
    // Abyssal Echoes fits the reveal, the lords fit the bones.
    assert_eq!(
        step_errors("id = \"r\"\nsay = \"Reveal\"\ngoal = \"reveal\"\nbind = \"hardest\"\n[[step.action]]\ncurrency = \"reveal\"\nomens = [\"omen-of-abyssal-echoes\"]"),
        Vec::<String>::new()
    );
    assert_eq!(
        step_errors("id = \"d\"\nsay = \"Desecrate\"\ngoal = \"desecrate\"\n[[step.action]]\ncurrency = \"preserved-collarbone\"\nomens = [\"omen-of-the-liege\", \"omen-of-dextral-necromancy\"]"),
        Vec::<String>::new()
    );
    assert_eq!(
        step_errors("id = \"d\"\nsay = \"Desecrate\"\ngoal = \"desecrate\"\n[[step.action]]\ncurrency = \"desecrate\"\nomens = [\"omen-of-abyssal-echoes\"]")
            .len(),
        1
    );
}

#[test]
fn goto_to_missing_step_is_refused() {
    let (shown, list) = errors("invalid/goto-missing.toml");
    assert_eq!(
        list,
        [format!("{shown}:13:11: step `slam`: `goto:whittel` names no step of this blueprint")]
    );
}

#[test]
fn every_error_in_a_file_is_reported() {
    let (shown, list) = errors("invalid/many-errors.toml");
    let expected = [
        "1:6: id `not-the-stem` is not the file name `many-errors`",
        "3:21: unknown class `Ringz`",
        "16:12: step `slam`: `orb-of-annulment` (removes only) cannot meet the `add-wanted` goal",
        "16:12: step `slam`: both a Sinistral and a Dextral omen on one action",
        "20:6: duplicate step id `slam` (first on line 10)",
        "26:10: step `slam`: `side-omen` needs the step's `bind` or `side` to pick a side",
        "26:10: step `slam`: `side-omen` on `divine-orb`, which has no Sinistral or Dextral omen",
        "31:8: step `empty` has no [[step.action]] alternatives",
    ];
    assert_eq!(list.len(), expected.len(), "{list:#?}");
    for (got, want) in list.iter().zip(expected) {
        assert!(got.starts_with(&format!("{shown}:{want}")), "{got}\n  wanted {want}");
    }
}

#[test]
fn every_shape_error_is_reported() {
    let (shown, list) = errors("invalid/bad-shape.toml");
    let expected = [
        "1:1: missing key `name` in the top level",
        "2:11: `classes` must be a list",
        "6:19: `rarity` is `unique`, not one of normal, magic, rare",
        "7:1: `free_prefixes_at_least` is a step condition (`requires`, `until`, `guard`), not a start one",
        "9:1: missing key `say` in [[step]]",
        "11:8: `goal` is `slam`, not one of add-wanted,",
        "12:10: `on_hit`: `onward` is not one of next, repeat, restart, abandon, done, or `goto:<step id>`",
        "16:10: `omens` must hold strings only",
    ];
    assert_eq!(list.len(), expected.len(), "{list:#?}");
    for (got, want) in list.iter().zip(expected) {
        assert!(got.starts_with(&format!("{shown}:{want}")), "{got}\n  wanted {want}");
    }
}

#[test]
fn a_blueprint_needs_a_start_and_a_step() {
    let text = "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\n";
    let list: Vec<String> = check("case.toml", text).unwrap_err().iter().map(|d| d.to_string()).collect();
    assert_eq!(
        list,
        [
            "case.toml:1:1: no [[start]]: a blueprint needs at least one start alternative",
            "case.toml:1:1: no [[step]]: a blueprint needs at least one step",
        ]
    );
    let empty = text.replace("[\"Rings\"]", "[]");
    let list: Vec<String> = check("case.toml", &empty).unwrap_err().iter().map(|d| d.to_string()).collect();
    assert!(list.contains(&"case.toml:3:11: `classes` names no class".to_string()), "{list:?}");
}

#[test]
fn a_goal_its_action_cannot_produce_is_refused() {
    let case = |goal: &str, currency: &str| {
        step_errors(&format!(
            "id = \"s\"\nsay = \"Do it\"\ngoal = \"{goal}\"\nbind = \"hardest\"\non_miss = \"next\"\n[[step.action]]\ncurrency = \"{currency}\""
        ))
    };
    // A removal-only currency cannot add; an adding one cannot remove.
    assert_eq!(case("add-wanted", "orb-of-annulment").len(), 1);
    assert_eq!(case("remove-unwanted", "exalted-orb").len(), 1);
    // Catalysts and quality currency only serve a manual step.
    assert_eq!(case("fill", "flesh-catalyst").len(), 1);
    assert_eq!(case("manual", "flesh-catalyst"), Vec::<String>::new());
    // A bone desecrates; a flux converts.
    assert_eq!(case("desecrate", "ancient-rib"), Vec::<String>::new());
    assert_eq!(case("desecrate", "exalted-orb").len(), 1);
    assert_eq!(case("convert", "void-flux"), Vec::<String>::new());
    assert_eq!(case("convert", "chaos-orb").len(), 1);
    // A chaos orb swaps: it can add, and remove.
    assert_eq!(case("add-wanted", "chaos-orb"), Vec::<String>::new());
    assert_eq!(case("remove-unwanted", "chaos-orb"), Vec::<String>::new());
    // Essences upgrade (Lesser to Greater) or swap (Perfect): both add.
    assert_eq!(case("add-wanted", "greater-essence-of-ice"), Vec::<String>::new());
    assert_eq!(case("add-wanted", "perfect-essence-of-ice"), Vec::<String>::new());
    // An omen in the currency's place says where it belongs.
    let list = case("fill", "omen-of-light");
    assert_eq!(list.len(), 1);
    assert!(list[0].contains("is an omen, not a currency"), "{list:?}");
}

#[test]
fn side_omens_are_checked() {
    let case = |extra: &str, omens: &str| {
        step_errors(&format!(
            "id = \"s\"\nsay = \"Exalt\"\ngoal = \"add-wanted\"\n{extra}\n[[step.action]]\ncurrency = \"exalted-orb\"\nomens = [{omens}]"
        ))
    };
    assert_eq!(case("bind = \"hardest\"", "\"side-omen\""), Vec::<String>::new());
    assert_eq!(case("side = \"suffix\"", "\"side-omen\""), Vec::<String>::new());
    assert_eq!(case("", "\"side-omen\"").len(), 1);
    assert_eq!(
        case("", "\"omen-of-sinistral-exaltation\", \"omen-of-dextral-exaltation\"").len(),
        1
    );
    assert_eq!(case("bind = \"hardest\"", "\"side-omen\", \"omen-of-dextral-exaltation\"").len(), 1);
    assert_eq!(case("", "\"omen-of-greater-exaltation\", \"omen-of-greater-exaltation\"").len(), 1);
    assert_eq!(case("", "\"omen-of-nothing\"").len(), 1);

    // `side-omen` resolves to the currency's own pair.
    let exalt = Currency::of("exalted-orb").unwrap();
    assert_eq!(exalt.side_omen(Side::Prefix).unwrap().id, "omen-of-sinistral-exaltation");
    assert_eq!(exalt.side_omen(Side::Suffix).unwrap().id, "omen-of-dextral-exaltation");
    let annul = Currency::of("orb-of-annulment").unwrap();
    assert_eq!(annul.side_omen(Side::Suffix).unwrap().id, "omen-of-dextral-annulment");
    let bones = Currency::of("desecrate").unwrap();
    assert_eq!(bones.side_omen(Side::Prefix).unwrap().id, "omen-of-sinistral-necromancy");
    assert!(Currency::of("divine-orb").unwrap().side_omen(Side::Prefix).is_none());
}

#[test]
fn a_reveal_or_convert_step_needs_a_binding() {
    let list = step_errors("id = \"r\"\nsay = \"Reveal\"\ngoal = \"reveal\"\n[[step.action]]\ncurrency = \"reveal\"");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(list[0].contains("needs `bind`"), "{list:?}");
    let list = step_errors("id = \"e\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nwhen = \"bound\"\non_miss = \"next\"\n[[step.action]]\ncurrency = \"essence-of-ice\"");
    assert_eq!(list.len(), 1, "{list:?}");
}

#[test]
fn catalogue_ids_are_unique_kebab_case() {
    let mut seen = HashSet::new();
    for e in CATALOGUE {
        assert!(seen.insert(e.id), "{} twice", e.id);
        assert!(is_kebab(e.id), "{} is not kebab case", e.id);
        assert_eq!(e.id, id_from_name(e.name), "{} is not the id of {:?}", e.id, e.name);
        // The settings' blocked list keeps ids up to 64 characters.
        assert!(e.id.chars().count() <= 64, "{}", e.id);
        assert_eq!(e.kind == Kind::Omen, e.omen.is_some(), "{}", e.id);
        assert!(data_backed(e.id).is_none(), "{} looks data-backed", e.id);
    }
    // The placeholders name no crafting item.
    for placeholder in [catalogue::DESECRATE, catalogue::REVEAL, catalogue::SIDE_OMEN] {
        assert!(!seen.contains(placeholder), "{placeholder}");
        assert!(is_kebab(placeholder));
    }
    // Every currency an omen names is a catalogue currency.
    for e in CATALOGUE {
        if let Some(OmenFits::Currencies(ids)) = e.omen.map(|o| o.fits) {
            for id in ids {
                let target = catalogue::entry(id).unwrap_or_else(|| panic!("{} names {id}", e.id));
                assert_eq!(target.kind, Kind::Currency, "{} names {id}", e.id);
            }
        }
    }
    // The ids the doc spells out.
    assert_eq!(id_from_name("Orb of Annulment"), "orb-of-annulment");
    assert_eq!(id_from_name("Hinekora's Lock"), "hinekoras-lock");
    assert_eq!(id_from_name("Perfect Exalted Orb"), "perfect-exalted-orb");
    assert_eq!(id_from_name("The Runebinder's Alloy"), "the-runebinders-alloy");
    assert_eq!(id_from_name("Ancient Potent Liquid Contempt"), "ancient-potent-liquid-contempt");
    assert!(catalogue::entry("hinekoras-lock").is_some());
    assert_eq!(catalogue::entry("perfect-exalted-orb").unwrap().min_modifier_level, Some(50));
}

#[test]
fn data_backed_ids_are_known_by_shape() {
    use catalogue::EssenceTier::*;
    let tier = |id: &str| data_backed(id).and_then(|d| d.tier);
    assert_eq!(tier("lesser-essence-of-the-body"), Some(Lesser));
    assert_eq!(tier("essence-of-the-body"), Some(Plain));
    assert_eq!(tier("greater-essence-of-the-body"), Some(Greater));
    assert_eq!(tier("perfect-essence-of-the-body"), Some(Perfect));
    assert_eq!(tier("essence-of-the-abyss"), Some(Special));
    assert_eq!(data_backed("the-runefathers-alloy").map(|d| d.kind), Some(Kind::Alloy));
    assert_eq!(data_backed("liquid-despair").map(|d| d.kind), Some(Kind::Liquid));
    assert_eq!(data_backed("ancient-diluted-liquid-ire").map(|d| d.kind), Some(Kind::Liquid));
    assert_eq!(data_backed("exalted-orb"), None);
    assert_eq!(data_backed("Essence-of-Ice"), None);
}

#[test]
fn bones_resolve_by_group_and_level() {
    let ids = |group, level| -> Vec<&str> {
        catalogue::bones_for(group, level).iter().map(|e| e.id).collect()
    };
    assert_eq!(
        ids(ItemGroup::Jewellery, 60),
        ["preserved-collarbone", "ancient-collarbone", "gnawed-collarbone"]
    );
    assert_eq!(ids(ItemGroup::Jewellery, 65), ["preserved-collarbone", "ancient-collarbone"]);
    assert_eq!(ids(ItemGroup::Jewels, 80), ["preserved-cranium"]);
    assert_eq!(ItemGroup::of_class("Rings"), Some(ItemGroup::Jewellery));
    assert_eq!(ItemGroup::of_class("Quivers"), Some(ItemGroup::WeaponsAndQuivers));
    // Every data class is in exactly one group.
    for class in catalogue::CLASSES {
        let groups = [ItemGroup::WeaponsAndQuivers, ItemGroup::Armour, ItemGroup::Jewellery, ItemGroup::Jewels]
            .into_iter()
            .filter(|g| g.classes().contains(class))
            .count();
        assert_eq!(groups, 1, "{class}");
    }
}

#[test]
fn shipped_list_matches_the_folder() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../blueprints");
    let mut on_disk: Vec<(String, String)> = match std::fs::read_dir(&folder) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "toml"))
            .map(|p| {
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                let text = std::fs::read_to_string(&p).unwrap();
                (name, text)
            })
            .collect(),
        // No folder yet: the embedded list is empty too.
        Err(_) => Vec::new(),
    };
    on_disk.sort();
    let shipped: Vec<(String, String)> = ch_blueprint::shipped()
        .iter()
        .map(|s| (s.file.to_string(), s.text.to_string()))
        .collect();
    let names = |list: &[(String, String)]| list.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    assert_eq!(names(&shipped), names(&on_disk), "rebuild after adding or removing a blueprint");
    assert_eq!(shipped, on_disk, "an embedded text differs from its file");
    assert_eq!(ch_blueprint::check_shipped().len(), shipped.len());
}

// ---------------------------------------------------------------------------
// State order (docs/models/crafting-slots-spec.md, "Route rules").

#[test]
fn a_state_ordered_blueprint_with_every_new_key_is_valid() {
    let c = valid("valid/state-every-key.toml");
    let b = &c.blueprint;
    assert_eq!(b.order(), Order::State);
    let relies: Vec<&str> = b
        .relies_on
        .as_ref()
        .unwrap()
        .get_ref()
        .iter()
        .map(|f| f.get_ref().as_str())
        .collect();
    assert_eq!(relies, ["greater-with-side-omen", "greater-needs-two-free", "whittling-mod-level"]);

    // The four new condition keys, in a start and in guards.
    let flag = |f: &Option<Spanned<bool>>| f.as_ref().map(|v| *v.get_ref());
    let start = b.starts[0].get_ref();
    assert_eq!(flag(&start.blocked_side), Some(false));
    assert_eq!(flag(&start.closable_side), Some(true));
    assert_eq!(flag(&start.one_side_open), Some(false));
    assert_eq!(flag(&start.desecrated_unwanted), Some(false));

    let ids: Vec<&str> = b.steps.iter().map(|s| s.get_ref().id.get_ref().as_str()).collect();
    assert_eq!(ids, ["unseal", "clean", "essence", "close", "slam", "last"]);
    let step = |i: usize| b.steps[i].get_ref();
    let guard = |i: usize| step(i).guard.as_ref().unwrap().get_ref().clone();
    assert_eq!(flag(&guard(0).desecrated_unwanted), Some(true));
    assert_eq!(flag(&guard(1).blocked_side), Some(true));
    assert_eq!(flag(&guard(3).closable_side), Some(true));
    assert_eq!(flag(&guard(3).one_side_open), Some(false));
    assert_eq!(guard(4).free_prefixes_at_least.as_ref().map(|n| *n.get_ref()), Some(0));
    assert_eq!(flag(&guard(5).one_side_open), Some(true));

    // `side` takes a side or a rule word; a step has at most one of the two.
    let sides: Vec<(Option<Side>, Option<SideRule>)> = b
        .steps
        .iter()
        .map(|s| {
            let s = s.get_ref();
            (s.side.as_ref().map(|v| *v.get_ref()), s.side_rule.as_ref().map(|v| *v.get_ref()))
        })
        .collect();
    assert_eq!(
        sides,
        [
            (None, None),
            (None, Some(SideRule::Blocked)),
            (None, None),
            (None, Some(SideRule::Close)),
            (None, Some(SideRule::Target)),
            (Some(Side::Prefix), None),
        ]
    );
    // A rule word keeps its span, so the engine's errors can point at it.
    let close = step(3).side_rule.as_ref().unwrap().span();
    assert_eq!(ch_blueprint::line_col(&c.text, close.start), (64, 8));

    // `greater-omen` beside `side-omen`, in either order.
    let omens: Vec<&str> = step(4).actions[1].get_ref().omens.iter().map(|o| o.get_ref().as_str()).collect();
    assert_eq!(omens, ["greater-omen", "side-omen"]);

    // A file without `order` is listed, as every blueprint was before.
    let minimal = valid("valid/minimal.toml").blueprint;
    assert_eq!(minimal.order(), Order::Listed);
    assert!(minimal.relies_on.is_none());
    assert!(minimal.steps[0].get_ref().guard.is_none());
    assert_eq!(valid("valid/every-key.toml").blueprint.order(), Order::Listed);
}

#[test]
fn transitions_are_refused_in_a_state_ordered_blueprint() {
    let (shown, list) = errors("invalid/state-transitions.toml");
    let state = "has no place in a state-ordered blueprint, whose next step is the first one whose `guard` holds";
    assert_eq!(
        list,
        [
            format!("{shown}:15:12: step `slam`: `requires` {state}"),
            format!("{shown}:16:9: step `slam`: `until` {state}"),
            format!("{shown}:17:10: step `slam`: `on_hit` {state}"),
            format!("{shown}:18:11: step `slam`: `on_miss` {state}"),
            format!("{shown}:28:10: step `clean`: `on_hit` {state}"),
        ]
    );
}

#[test]
fn guards_and_side_rules_are_refused_in_a_listed_blueprint() {
    let (shown, list) = errors("invalid/listed-guard.toml");
    let listed = "needs `order = \"state\"`";
    assert_eq!(
        list,
        [
            format!("{shown}:14:8: step `slam`: `side = \"target\"` {listed}"),
            format!("{shown}:15:9: step `slam`: `guard` {listed}"),
            format!("{shown}:24:8: step `clean`: `side = \"blocked\"` {listed}"),
            format!("{shown}:34:8: step `close`: `side = \"close\"` {listed}"),
        ]
    );
    // The new condition keys are not state order's alone: the start's
    // `one_side_open` and the `requires` with `blocked_side` above drew no
    // error. A word that is neither a side nor a rule is a shape error, in
    // either order.
    let list = step_errors("id = \"s\"\nsay = \"Exalt\"\ngoal = \"fill\"\nside = \"open\"\n[[step.action]]\ncurrency = \"exalted-orb\"");
    assert_eq!(
        list,
        ["case.toml:13:8: `side` is `open`, not one of prefix, suffix, target, close, blocked"]
    );
}

#[test]
fn relies_on_names_only_known_facts() {
    let (shown, list) = errors("invalid/relies-on.toml");
    assert_eq!(list.len(), 3, "{list:#?}");
    // A documented fact needs no naming; only the not-established ones are
    // known.
    let known = "is not a not-established fact (known: greater-with-side-omen, ";
    assert!(list[0].starts_with(&format!("{shown}:6:40: `relies_on`: `magic-slots` {known}")), "{list:#?}");
    assert!(list[1].starts_with(&format!("{shown}:6:55: `relies_on`: `no-such-fact` {known}")), "{list:#?}");
    assert_eq!(list[2], format!("{shown}:6:71: `relies_on`: `greater-with-side-omen` is named twice"));

    // Required, possibly empty, in a state-ordered blueprint.
    let state = |relies: &str| {
        let text = format!(
            "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\norder = \"state\"\n{relies}\n[[start]]\nrarity = [\"rare\"]\n\n[[step]]\nid = \"s\"\nsay = \"Exalt\"\ngoal = \"add-wanted\"\nside = \"target\"\n[[step.action]]\ncurrency = \"exalted-orb\"\n"
        );
        match check("case.toml", &text) {
            Ok(_) => Vec::new(),
            Err(list) => list.iter().map(|d| d.to_string()).collect(),
        }
    };
    assert_eq!(state("relies_on = []"), Vec::<String>::new());
    assert_eq!(state("relies_on = [\"open-side-landing\"]"), Vec::<String>::new());
    assert_eq!(
        state(""),
        ["case.toml:5:9: `order = \"state\"` needs `relies_on`: the not-established facts the route relies on, `[]` for none"]
    );
    // Allowed, and checked, in a listed one.
    let listed = |relies: &str| {
        let text = format!(
            "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\n{relies}\n[[start]]\nrarity = [\"rare\"]\n\n[[step]]\nid = \"s\"\nsay = \"Exalt\"\ngoal = \"fill\"\n[[step.action]]\ncurrency = \"exalted-orb\"\n"
        );
        check("case.toml", &text).map(|_| ()).map_err(|l| l.len())
    };
    assert_eq!(listed("relies_on = [\"fractured-not-removed\"]"), Ok(()));
    assert_eq!(listed("relies_on = [\"rare-slots\"]"), Err(1));
    // Every id the crate knows is kebab case and listed once.
    let mut seen = HashSet::new();
    for id in catalogue::NOT_ESTABLISHED {
        assert!(is_kebab(id), "{id}");
        assert!(seen.insert(id), "{id} twice");
    }
}

#[test]
fn greater_omen_needs_an_exalted_orb() {
    let (shown, list) = errors("invalid/greater-omen.toml");
    let exalts = "it is for an Exalted Orb (exalted-orb, greater-exalted-orb, perfect-exalted-orb)";
    assert_eq!(
        list,
        [
            format!("{shown}:18:10: step `chaos`: `greater-omen` on `chaos-orb`; {exalts}"),
            format!("{shown}:27:10: step `slam`: `greater-omen` on `regal-orb`; {exalts}"),
            format!("{shown}:30:12: step `slam`: `greater-omen` beside `omen-of-greater-exaltation` on one action"),
        ]
    );
    // Every grade takes it, alone or beside `side-omen`.
    let case = |currency: &str, omens: &str| {
        step_errors(&format!(
            "id = \"s\"\nsay = \"Exalt\"\ngoal = \"add-wanted\"\nbind = \"hardest\"\n[[step.action]]\ncurrency = \"{currency}\"\nomens = [{omens}]"
        ))
    };
    for grade in ["exalted-orb", "greater-exalted-orb", "perfect-exalted-orb"] {
        assert_eq!(case(grade, "\"greater-omen\""), Vec::<String>::new(), "{grade}");
        assert_eq!(case(grade, "\"side-omen\", \"greater-omen\""), Vec::<String>::new(), "{grade}");
    }
    assert_eq!(case("exalted-orb", "\"greater-omen\", \"greater-omen\"").len(), 1);
    // An annulment can neither add nor take the omen: two errors.
    assert_eq!(case("orb-of-annulment", "\"greater-omen\"").len(), 2);
    // A placeholder, not a catalogue omen: there is nothing to block by
    // this id.
    assert!(catalogue::entry(catalogue::GREATER_OMEN).is_none());
    assert!(is_kebab(catalogue::GREATER_OMEN));
}

#[test]
fn a_say_naming_an_omen_its_actions_lack_is_refused() {
    let (shown, list) = errors("invalid/say-omen.toml");
    assert_eq!(
        list,
        [
            format!("{shown}:13:7: step `fill`: `say` mentions an omen, but no action of the step carries one"),
            format!("{shown}:23:7: step `unseal`: `say` names Omen of Light, which no action of the step carries"),
            format!(
                "{shown}:31:7: step `slam`: `say` names Omen of Sinistral Exaltation, which no action of the step carries"
            ),
        ]
    );
    let case = |say: &str, currency: &str, omens: &str| {
        step_errors(&format!(
            "id = \"s\"\nsay = \"{say}\"\ngoal = \"add-wanted\"\nbind = \"hardest\"\n[[step.action]]\ncurrency = \"{currency}\"\nomens = [{omens}]\n[[step.action]]\ncurrency = \"{currency}\""
        ))
    };
    let none = Vec::<String>::new();
    // `side-omen` counts as both of its currency's side omens, and one
    // action carrying the omen is enough.
    assert_eq!(case("Exalt (Sinistral or Dextral Exaltation)", "exalted-orb", "\"side-omen\""), none);
    assert_eq!(case("Exalt with its side's Exaltation omen", "exalted-orb", "\"side-omen\""), none);
    // ... but not as another currency's.
    assert_eq!(case("Exalt (Sinistral Annulment)", "exalted-orb", "\"side-omen\"").len(), 1);
    // `greater-omen` counts as Omen of Greater Exaltation.
    assert_eq!(
        case("Exalt, Omen of Greater Exaltation when two are free", "perfect-exalted-orb", "\"greater-omen\""),
        none
    );
    assert_eq!(case("Exalt, Greater Exaltation when two are free", "perfect-exalted-orb", "\"side-omen\"").len(), 1);
    // A name is matched whatever its case, as a whole phrase.
    assert_eq!(case("omen of greater exaltation", "exalted-orb", "").len(), 1);
    assert_eq!(case("An OMEN helps", "exalted-orb", "").len(), 1);
    assert_eq!(case("Exalt the open side", "exalted-orb", ""), none);
    assert_eq!(case("Exalt like the Homogenising guides", "exalted-orb", ""), none);
}

#[test]
fn facts_match_the_spec_table() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../docs/models/crafting-slots-spec.md");
    let text = std::fs::read_to_string(&spec).unwrap_or_else(|e| panic!("{}: {e}", spec.display()));
    let text = text.replace("\r\n", "\n");
    let start = text.find("\n## Facts\n").expect("the spec has a `## Facts` section") + 1;
    let section = &text[start..];
    let section = &section[..section[1..].find("\n#").map_or(section.len(), |i| i + 1)];
    let mut rows = section
        .lines()
        .filter(|l| l.starts_with('|'))
        .map(|l| l.trim().trim_matches('|').split('|').map(str::trim).collect::<Vec<&str>>());
    assert_eq!(rows.next().expect("a header row"), ["id", "label", "fact", "source"]);
    let separator = rows.next().expect("a separator row");
    assert!(separator.iter().all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')));

    let mut not_established = Vec::new();
    let mut all = HashSet::new();
    for row in rows {
        assert_eq!(row.len(), 4, "{row:?}");
        let id = row[0]
            .strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .unwrap_or_else(|| panic!("the id is not backticked: {row:?}"));
        assert!(is_kebab(id), "{id}");
        assert!(all.insert(id), "{id} twice in the table");
        let label = row[1];
        assert!(["measured", "documented", "not established"].contains(&label), "{id}: label `{label}`");
        // Every source is a URL with the date it was read: fetched, or, for
        // a page only an issue records, reviewed.
        let source = row[3];
        assert!(
            source.contains("https://") && (source.contains("fetched 2026-") || source.contains("reviewed 2026-")),
            "{id}: source `{source}`"
        );
        if label == "not established" {
            not_established.push(id);
        }
    }
    assert!(!not_established.is_empty(), "the table has no not-established row");
    assert_eq!(
        catalogue::NOT_ESTABLISHED,
        not_established.as_slice(),
        "the crate's fact ids differ from the spec's not-established rows"
    );
}

// ---------------------------------------------------------------------------
// The guides' additions (issue #35): the fracture goal, the modifier count
// conditions, the one-crafted rules and the jewel rule.

#[test]
fn modifier_count_conditions_parse() {
    let b = valid("valid/every-key.toml").blueprint;
    let count = |v: &Option<Spanned<u32>>| v.as_ref().map(|n| *n.get_ref());
    let rare = b.starts[1].get_ref();
    assert_eq!((count(&rare.modifiers_at_least), count(&rare.modifiers_at_most)), (Some(4), Some(6)));
    let fracture = b.steps[10].get_ref();
    assert_eq!(*fracture.goal.get_ref(), Goal::Fracture);
    let requires = fracture.requires.as_ref().unwrap().get_ref();
    assert_eq!((count(&requires.modifiers_at_least), count(&requires.modifiers_at_most)), (Some(4), Some(6)));
    assert_eq!(fracture.actions[0].get_ref().currency.get_ref(), "fracturing-orb");

    // Start keys, so a step's conditions take them too, in either order.
    assert!(ch_blueprint::Condition::START_KEYS.contains(&"modifiers_at_least"));
    assert!(ch_blueprint::Condition::START_KEYS.contains(&"modifiers_at_most"));
    let guard = "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\norder = \"state\"\nrelies_on = []\n\n[[start]]\nrarity = [\"rare\"]\n\n[[step]]\nid = \"s\"\nsay = \"Exalt\"\ngoal = \"add-wanted\"\nside = \"target\"\nguard = { modifiers_at_most = 5 }\n[[step.action]]\ncurrency = \"exalted-orb\"\n";
    let c = check("case.toml", guard).unwrap_or_else(|l| panic!("{l:?}"));
    let guard = c.blueprint.steps[0].get_ref().guard.as_ref().unwrap().get_ref().clone();
    assert_eq!(count(&guard.modifiers_at_most), Some(5));

    // A count is a whole number, 0 or more.
    let list = step_errors(
        "id = \"s\"\nsay = \"Exalt\"\ngoal = \"fill\"\nrequires = { modifiers_at_least = -1 }\n[[step.action]]\ncurrency = \"exalted-orb\"",
    );
    assert_eq!(list, ["case.toml:13:35: `modifiers_at_least` must be a whole number, 0 or more"]);
}

#[test]
fn a_fracture_step_needs_a_fracturing_orb() {
    let (shown, list) = errors("invalid/fracture-currency.toml");
    assert_eq!(
        list,
        [
            format!("{shown}:16:12: step `fracture`: `exalted-orb` (adds a modifier) cannot meet the `fracture` goal"),
            format!("{shown}:28:12: step `slam`: `fracturing-orb` (fractures) cannot meet the `add-wanted` goal"),
        ]
    );
    // The orb's second alternative on the fracture step drew no error.
    let fracture = |currency: &str| {
        step_errors(&format!(
            "id = \"f\"\nsay = \"Fracture\"\ngoal = \"fracture\"\non_miss = \"abandon\"\n[[step.action]]\ncurrency = \"{currency}\""
        ))
    };
    assert_eq!(fracture("fracturing-orb"), Vec::<String>::new());
    assert_eq!(fracture("chaos-orb").len(), 1);
    assert_eq!(fracture("perfect-essence-of-the-body").len(), 1);
}

#[test]
fn two_crafted_steps_without_a_removal_between_are_refused() {
    let (shown, list) = errors("invalid/crafted-twice.toml");
    assert_eq!(
        list,
        [format!(
            "{shown}:35:12: step `alloy`: `swift-alloy` places a second crafted modifier after step `essence`, with no annulment or Chaos Orb between them; an item holds one crafted modifier"
        )]
    );
    // The liquid after the annulment drew none. Neither does a Chaos Orb of
    // any grade between two crafted steps, nor alternatives of one step.
    let two = |between: &str| {
        let text = format!(
            "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\n\n[[start]]\nrarity = [\"magic\"]\n\n[[step]]\nid = \"a\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nbind = \"essence\"\non_miss = \"next\"\n[[step.action]]\ncurrency = \"greater-essence-of-the-body\"\n[[step.action]]\ncurrency = \"greater-essence-of-the-mind\"\n\n[[step]]\nid = \"between\"\nsay = \"Remove\"\ngoal = \"remove-unwanted\"\n[[step.action]]\ncurrency = \"{between}\"\n\n[[step]]\nid = \"b\"\nsay = \"Essence\"\ngoal = \"manual\"\n[[step.action]]\ncurrency = \"essence-of-the-breach\"\n"
        );
        check("case.toml", &text).map(|_| ()).map_err(|l| l.len())
    };
    assert_eq!(two("orb-of-annulment"), Ok(()));
    assert_eq!(two("perfect-chaos-orb"), Ok(()));
    // An exalt removes nothing (it is refused on a removal goal too).
    assert_eq!(two("exalted-orb"), Err(2));
}

#[test]
fn a_crafted_step_that_repeats_on_a_miss_is_refused() {
    let (shown, list) = errors("invalid/crafted-repeat.toml");
    let holds = "a step that places a crafted modifier; an item holds one";
    assert_eq!(
        list,
        [
            format!("{shown}:32:12: step `essence`: a miss repeats by default {holds}, so say where a miss goes (`on_miss`)"),
            format!("{shown}:47:11: step `hysteria`: a miss repeats {holds}, so say where a miss goes (`on_miss`)"),
            format!("{shown}:64:9: step `alloy`: `until` repeats {holds}"),
        ]
    );
    // The manual Breach step repeats by default and drew none. Nor does a
    // state-ordered essence step, which has no transitions.
    let state = "id = \"case\"\nname = \"Case\"\nclasses = [\"Rings\"]\nsource = \"roadmap\"\norder = \"state\"\nrelies_on = []\n\n[[start]]\nrarity = [\"magic\"]\n\n[[step]]\nid = \"essence\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nbind = \"essence\"\n[[step.action]]\ncurrency = \"greater-essence-of-the-body\"\n";
    assert!(check("case.toml", state).is_ok());
}

#[test]
fn greater_and_perfect_currencies_are_refused_on_a_jewel_blueprint() {
    let (shown, list) = errors("invalid/jewel-grades.toml");
    let costs = "on a jewel blueprint; a jewel's modifiers have one level each, so a Minimum Modifier Level only adds cost (use the plain orb)";
    assert_eq!(
        list,
        [
            format!("{shown}:15:12: step `slam`: `perfect-exalted-orb` {costs}"),
            format!("{shown}:18:12: step `slam`: `greater-exalted-orb` {costs}"),
            format!("{shown}:29:12: step `chaos`: `greater-chaos-orb` {costs}"),
        ]
    );
    // A blueprint for jewels and another class may name them: the other
    // class has levels to choose from.
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/invalid/jewel-grades.toml"))
        .unwrap()
        .replace("[\"Jewels\"]", "[\"Jewels\", \"Rings\"]");
    assert!(check("jewel-grades.toml", &text).is_ok());
}
