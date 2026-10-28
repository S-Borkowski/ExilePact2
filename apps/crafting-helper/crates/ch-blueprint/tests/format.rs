//! The blueprint format: the reader, the validator's errors (each pinned to
//! its `file:line:col`), the catalogue and the embedded list.
//!
//! The test blueprints are under `tests/fixtures/`: `valid/` ones the
//! validator takes, `invalid/` ones it refuses. Each is read as the path
//! `tests/fixtures/<valid|invalid>/<name>.toml`, the one its errors name.

use std::collections::HashSet;
use std::path::Path;

use ch_blueprint::catalogue::{self, data_backed, id_from_name, is_kebab, ItemGroup, Kind, OmenFits};
use ch_blueprint::{check, Binding, Checked, Currency, Diagnostic, Goal, Rarity, Side, Transition, CATALOGUE};

/// Checks a fixture, read as the path its errors name.
fn fixture(rel: &str) -> (String, Result<Checked, Vec<Diagnostic>>) {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(rel);
    let text = std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()));
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
        ["essence", "fill", "whittle", "desecrate", "reveal", "unseal", "flux", "alloy", "quality", "socket"]
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
                "{shown}:13:1: unknown key `colour` in [[step]] (known: id, say, goal, side, bind, requires, when, until, action, on_hit, on_miss)"
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
            "id = \"e\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nbind = \"essence\"\n[[step.action]]\ncurrency = \"{currency}\"\nomens = [\"omen-of-sinistral-crystallisation\"]"
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
        "7:1: `free_prefixes_at_least` is a step condition (`requires`, `until`), not a start one",
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
            "id = \"s\"\nsay = \"Do it\"\ngoal = \"{goal}\"\nbind = \"hardest\"\n[[step.action]]\ncurrency = \"{currency}\""
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
    let list = step_errors("id = \"e\"\nsay = \"Essence\"\ngoal = \"add-wanted\"\nwhen = \"bound\"\n[[step.action]]\ncurrency = \"essence-of-ice\"");
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
