//! The structural validator: reads a blueprint's text into the model and
//! refuses what is wrong with it, every error as one
//! `<path>:<line>:<col>: <message>` line.
//!
//! It runs in three passes, each reporting everything it finds before the
//! next one starts:
//!
//! 1. TOML syntax, from the parser's spans.
//! 2. The shape: unknown and missing keys, values of the wrong type, a word
//!    that is not one of a key's values. This pass walks the parsed
//!    document against [`TOP`] rather than leaving it to serde, which stops
//!    at the first problem.
//! 3. The meaning, on the model: the id against the file stem, classes,
//!    currency and omen ids, omens against their currency, goals against
//!    what the action can produce, step ids and go-tos, empty lists, the
//!    keys only one order takes, `relies_on` against the known facts, and a
//!    step's `say` against the omens its actions carry.
//!
//! What needs the modifier data (an essence, alloy or liquid that exists
//! for none of the blueprint's classes) is `ch-engine`'s check.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;
use toml::de::{DeTable, DeValue};

use crate::catalogue::{
    self, Currency, Kind, CLASSES, EXALTED_ORBS, GREATER_EXALTATION, GREATER_OMEN, NOT_ESTABLISHED, SIDE_OMEN,
};
use crate::model::{Binding, Blueprint, Goal, Order, Rarity, Side, Span, Spanned, Step, Transition, When};

/// One validation error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Diagnostic {
    /// The path the file was read as.
    pub path: String,
    /// 1-based.
    pub line: usize,
    /// 1-based, in characters.
    pub col: usize,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}: {}", self.path, self.line, self.col, self.message)
    }
}

/// The 1-based line and column (in characters) of a byte offset.
pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let col = before[line_start..].chars().count() + 1;
    (line, col)
}

/// A blueprint that passed the structural validator, with the text its
/// spans point into.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub path: String,
    pub text: String,
    pub blueprint: Blueprint,
}

impl Checked {
    /// An error at `span` of this blueprint's text (`ch-engine`'s
    /// data-backed errors use it).
    pub fn at(&self, span: Span, message: impl Into<String>) -> Diagnostic {
        diagnostic(&self.path, &self.text, span, message)
    }
}

fn diagnostic(path: &str, text: &str, span: Span, message: impl Into<String>) -> Diagnostic {
    let (line, col) = line_col(text, span.start);
    Diagnostic {
        path: path.to_string(),
        line,
        col,
        message: message.into(),
    }
}

/// Reads and validates the blueprint file at `path`.
pub fn check_file(path: &Path) -> Result<Checked, Vec<Diagnostic>> {
    let shown = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(text) => check(&shown, &text),
        Err(e) => Err(vec![Diagnostic {
            path: shown,
            line: 1,
            col: 1,
            message: format!("cannot read the file: {e}"),
        }]),
    }
}

/// Validates a blueprint's text, read from `path` (whose file stem the `id`
/// must equal). Returns every error found, in file order.
pub fn check(path: &str, text: &str) -> Result<Checked, Vec<Diagnostic>> {
    let mut errors = Errors {
        path,
        text,
        list: Vec::new(),
    };

    let (root, syntax) = DeTable::parse_recoverable(text);
    for e in &syntax {
        errors.push(e.span().unwrap_or(0..0), e.message().trim().to_string());
    }
    if !errors.list.is_empty() {
        return Err(errors.finish());
    }

    walk(&mut errors, root.get_ref(), root.span(), TOP, "the top level");
    if !errors.list.is_empty() {
        return Err(errors.finish());
    }

    let blueprint = match Blueprint::deserialize(toml::de::Deserializer::from(root)) {
        Ok(b) => b,
        Err(e) => {
            // The shape pass should have caught it; report it all the same.
            errors.push(e.span().unwrap_or(0..0), e.message().trim().to_string());
            return Err(errors.finish());
        }
    };

    meaning(&mut errors, &blueprint);
    if !errors.list.is_empty() {
        return Err(errors.finish());
    }
    Ok(Checked {
        path: path.to_string(),
        text: text.to_string(),
        blueprint,
    })
}

struct Errors<'a> {
    path: &'a str,
    text: &'a str,
    list: Vec<Diagnostic>,
}

impl Errors<'_> {
    fn push(&mut self, span: Span, message: impl Into<String>) {
        self.list.push(diagnostic(self.path, self.text, span, message));
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        self.list.sort();
        self.list.dedup();
        self.list
    }
}

// ---------------------------------------------------------------------------
// Pass 2: the shape.

/// The type a key's value must have.
#[derive(Clone, Copy)]
enum Ty {
    Str,
    Uint,
    Bool,
    /// One of these words.
    Word(&'static [&'static str]),
    /// A list of these words.
    Words(&'static [&'static str]),
    StrList,
    Transition,
    /// An inline or standard table.
    Table(&'static [Field], &'static str),
    /// An array of tables (`[[name]]`, or an inline array of tables).
    Tables(&'static [Field], &'static str),
}

#[derive(Clone, Copy)]
struct Field {
    key: &'static str,
    ty: Ty,
    required: bool,
}

const fn req(key: &'static str, ty: Ty) -> Field {
    Field { key, ty, required: true }
}

const fn opt(key: &'static str, ty: Ty) -> Field {
    Field { key, ty, required: false }
}

/// The keys of a start alternative (`[[start]]`).
const START: &[Field] = &[
    opt("rarity", Ty::Words(Rarity::NAMES)),
    opt("wanted_at_least", Ty::Uint),
    opt("wanted_prefixes_at_least", Ty::Uint),
    opt("wanted_suffixes_at_least", Ty::Uint),
    opt("fractured_wanted", Ty::Bool),
    opt("unwanted_at_most", Ty::Uint),
    opt("essence_binds", Ty::Bool),
    opt("blocked_side", Ty::Bool),
    opt("closable_side", Ty::Bool),
    opt("one_side_open", Ty::Bool),
    opt("desecrated_unwanted", Ty::Bool),
];

/// The keys of a step's `requires`, `until` and `guard`: a start's and more.
const STEP_CONDITION: &[Field] = &[
    opt("rarity", Ty::Words(Rarity::NAMES)),
    opt("wanted_at_least", Ty::Uint),
    opt("wanted_prefixes_at_least", Ty::Uint),
    opt("wanted_suffixes_at_least", Ty::Uint),
    opt("fractured_wanted", Ty::Bool),
    opt("unwanted_at_most", Ty::Uint),
    opt("essence_binds", Ty::Bool),
    opt("blocked_side", Ty::Bool),
    opt("closable_side", Ty::Bool),
    opt("one_side_open", Ty::Bool),
    opt("desecrated_unwanted", Ty::Bool),
    opt("free_prefixes_at_least", Ty::Uint),
    opt("free_suffixes_at_least", Ty::Uint),
    opt("no_desecrated", Ty::Bool),
    opt("unrevealed", Ty::Bool),
    opt("wanted_remaining_at_most", Ty::Uint),
    opt("wanted_remaining_exactly", Ty::Uint),
];

/// The keys of an action (`[[step.action]]`).
const ACTION: &[Field] = &[req("currency", Ty::Str), opt("omens", Ty::StrList)];

/// The keys of a step (`[[step]]`).
const STEP: &[Field] = &[
    req("id", Ty::Str),
    req("say", Ty::Str),
    req("goal", Ty::Word(Goal::NAMES)),
    opt("side", Ty::Word(Step::SIDE_WORDS)),
    opt("bind", Ty::Word(Binding::NAMES)),
    opt("requires", Ty::Table(STEP_CONDITION, "a step's `requires`")),
    opt("when", Ty::Word(When::NAMES)),
    opt("until", Ty::Table(STEP_CONDITION, "a step's `until`")),
    opt("action", Ty::Tables(ACTION, "[[step.action]]")),
    opt("on_hit", Ty::Transition),
    opt("on_miss", Ty::Transition),
    opt("guard", Ty::Table(STEP_CONDITION, "a step's `guard`")),
];

/// The keys of a blueprint file.
const TOP: &[Field] = &[
    req("id", Ty::Str),
    req("name", Ty::Str),
    req("classes", Ty::StrList),
    req("source", Ty::Str),
    opt("notes", Ty::Str),
    opt("order", Ty::Word(Order::NAMES)),
    opt("relies_on", Ty::StrList),
    opt("start", Ty::Tables(START, "[[start]]")),
    opt("step", Ty::Tables(STEP, "[[step]]")),
];

fn walk(errors: &mut Errors<'_>, table: &DeTable<'_>, span: Span, fields: &[Field], place: &str) {
    for (key, value) in table.iter() {
        let name: &str = key.get_ref();
        match fields.iter().find(|f| f.key == name) {
            Some(field) => check_value(errors, field, value),
            None if std::ptr::eq(fields, START) && STEP_CONDITION.iter().any(|f| f.key == name) => {
                errors.push(
                    key.span(),
                    format!("`{name}` is a step condition (`requires`, `until`, `guard`), not a start one"),
                );
            }
            None => {
                let known: Vec<&str> = fields.iter().map(|f| f.key).collect();
                errors.push(
                    key.span(),
                    format!("unknown key `{name}` in {place} (known: {})", known.join(", ")),
                );
            }
        }
    }
    for field in fields.iter().filter(|f| f.required) {
        if !table.iter().any(|(k, _)| k.get_ref().as_ref() == field.key) {
            errors.push(span.clone(), format!("missing key `{}` in {place}", field.key));
        }
    }
}

fn check_value(errors: &mut Errors<'_>, field: &Field, value: &Spanned<DeValue<'_>>) {
    let key = field.key;
    let span = value.span();
    match (field.ty, value.get_ref()) {
        (Ty::Str, DeValue::String(_)) => {}
        (Ty::Str, _) => errors.push(span, format!("`{key}` must be a string")),
        (Ty::Bool, DeValue::Boolean(_)) => {}
        (Ty::Bool, _) => errors.push(span, format!("`{key}` must be true or false")),
        (Ty::Uint, DeValue::Integer(n)) if uint(n).is_some() => {}
        (Ty::Uint, _) => errors.push(span, format!("`{key}` must be a whole number, 0 or more")),
        (Ty::Word(words), DeValue::String(s)) => check_word(errors, key, words, s, span),
        (Ty::Word(words), _) => errors.push(span, format!("`{key}` must be one of {}", words.join(", "))),
        (Ty::Transition, DeValue::String(s)) => {
            if let Err(why) = Transition::parse(s) {
                errors.push(span, format!("`{key}`: {why}"));
            }
        }
        (Ty::Transition, _) => errors.push(span, format!("`{key}` must be a string")),
        (Ty::StrList, DeValue::Array(items)) => {
            for item in items.iter() {
                if !matches!(item.get_ref(), DeValue::String(_)) {
                    errors.push(item.span(), format!("`{key}` must hold strings only"));
                }
            }
        }
        (Ty::Words(words), DeValue::Array(items)) => {
            for item in items.iter() {
                match item.get_ref() {
                    DeValue::String(s) => check_word(errors, key, words, s, item.span()),
                    _ => errors.push(item.span(), format!("`{key}` must hold strings only")),
                }
            }
        }
        (Ty::StrList | Ty::Words(_), _) => errors.push(span, format!("`{key}` must be a list")),
        (Ty::Table(fields, place), DeValue::Table(t)) => walk(errors, t, span, fields, place),
        (Ty::Table(..), _) => errors.push(span, format!("`{key}` must be a table")),
        (Ty::Tables(fields, place), DeValue::Array(items)) => {
            for item in items.iter() {
                match item.get_ref() {
                    DeValue::Table(t) => walk(errors, t, item.span(), fields, place),
                    _ => errors.push(item.span(), format!("`{key}` must hold tables only")),
                }
            }
        }
        (Ty::Tables(_, place), _) => {
            errors.push(span, format!("`{key}` must be written as {place} tables"));
        }
    }
}

fn check_word(errors: &mut Errors<'_>, key: &str, words: &[&str], value: &str, span: Span) {
    if !words.contains(&value) {
        errors.push(span, format!("`{key}` is `{value}`, not one of {}", words.join(", ")));
    }
}

/// A TOML integer that fits a `u32`.
fn uint(n: &toml::de::DeInteger<'_>) -> Option<u32> {
    u32::from_str_radix(n.as_str(), n.radix()).ok()
}

// ---------------------------------------------------------------------------
// Pass 3: the meaning.

fn meaning(errors: &mut Errors<'_>, b: &Blueprint) {
    let stem = Path::new(errors.path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if b.id.get_ref() != &stem {
        errors.push(
            b.id.span(),
            format!("id `{}` is not the file name `{stem}`", b.id.get_ref()),
        );
    }
    for (what, value) in [("name", &b.name), ("source", &b.source)] {
        if value.get_ref().trim().is_empty() {
            errors.push(value.span(), format!("`{what}` is empty"));
        }
    }

    if b.classes.get_ref().is_empty() {
        errors.push(b.classes.span(), "`classes` names no class");
    }
    for class in b.classes.get_ref() {
        if !CLASSES.contains(&class.get_ref().as_str()) {
            errors.push(
                class.span(),
                format!("unknown class `{}` (the data's classes: {})", class.get_ref(), CLASSES.join(", ")),
            );
        }
    }

    let order = b.order();
    if let (Order::State, Some(word), None) = (order, &b.order, &b.relies_on) {
        errors.push(
            word.span(),
            "`order = \"state\"` needs `relies_on`: the not-established facts the route relies on, `[]` for none",
        );
    }
    if let Some(relies) = &b.relies_on {
        let mut named: Vec<&str> = Vec::new();
        for fact in relies.get_ref() {
            let fact_id = fact.get_ref().as_str();
            if named.contains(&fact_id) {
                errors.push(fact.span(), format!("`relies_on`: `{fact_id}` is named twice"));
                continue;
            }
            named.push(fact_id);
            if !NOT_ESTABLISHED.contains(&fact_id) {
                errors.push(
                    fact.span(),
                    format!(
                        "`relies_on`: `{fact_id}` is not a not-established fact (known: {})",
                        NOT_ESTABLISHED.join(", ")
                    ),
                );
            }
        }
    }

    if b.starts.is_empty() {
        errors.push(0..0, "no [[start]]: a blueprint needs at least one start alternative");
    }
    if b.steps.is_empty() {
        errors.push(0..0, "no [[step]]: a blueprint needs at least one step");
    }

    let mut first: HashMap<&str, Span> = HashMap::new();
    for step in &b.steps {
        let id = step.get_ref().id.get_ref().as_str();
        match first.get(id) {
            Some(at) => {
                let (line, _) = line_col(errors.text, at.start);
                let span = step.get_ref().id.span();
                errors.push(span, format!("duplicate step id `{id}` (first on line {line})"));
            }
            None => {
                first.insert(id, step.get_ref().id.span());
            }
        }
    }

    for step in &b.steps {
        check_step(errors, step, &first, order);
    }
}

fn check_step(errors: &mut Errors<'_>, step: &Spanned<Step>, ids: &HashMap<&str, Span>, order: Order) {
    let s = step.get_ref();
    let goal = *s.goal.get_ref();
    let id = s.id.get_ref();
    if id.trim().is_empty() {
        errors.push(s.id.span(), "the step `id` is empty");
    }
    if s.say.get_ref().trim().is_empty() {
        errors.push(s.say.span(), format!("step `{id}`: `say` is empty"));
    }
    match order {
        Order::State => {
            let listed_only = [
                ("requires", s.requires.as_ref().map(Spanned::span)),
                ("until", s.until.as_ref().map(Spanned::span)),
                ("on_hit", s.on_hit.as_ref().map(Spanned::span)),
                ("on_miss", s.on_miss.as_ref().map(Spanned::span)),
            ];
            for (key, span) in listed_only {
                if let Some(span) = span {
                    errors.push(
                        span,
                        format!(
                            "step `{id}`: `{key}` has no place in a state-ordered blueprint, whose next step is the first one whose `guard` holds"
                        ),
                    );
                }
            }
        }
        Order::Listed => {
            if let Some(rule) = &s.side_rule {
                errors.push(
                    rule.span(),
                    format!("step `{id}`: `side = \"{}\"` needs `order = \"state\"`", rule.get_ref()),
                );
            }
            if let Some(guard) = &s.guard {
                errors.push(guard.span(), format!("step `{id}`: `guard` needs `order = \"state\"`"));
            }
        }
    }
    for t in [&s.on_hit, &s.on_miss].into_iter().flatten() {
        if let Transition::Goto(target) = t.get_ref() {
            if !ids.contains_key(target.as_str()) {
                errors.push(
                    t.span(),
                    format!("step `{id}`: `goto:{target}` names no step of this blueprint"),
                );
            }
        }
    }
    if matches!(goal, Goal::Reveal | Goal::Convert) && s.bind.is_none() {
        errors.push(
            s.goal.span(),
            format!("step `{id}`: a `{goal}` step needs `bind` (the target it must produce)"),
        );
    }
    if let (Some(when), None) = (&s.when, &s.bind) {
        errors.push(when.span(), format!("step `{id}`: `when = \"bound\"` needs `bind`"));
    }
    if s.actions.is_empty() && goal != Goal::Manual {
        errors.push(
            s.goal.span(),
            format!("step `{id}` has no [[step.action]] alternatives (only a `manual` step may have none)"),
        );
    }
    for action in &s.actions {
        check_action(errors, s, action.get_ref());
    }
    check_say(errors, s);
}

/// A step's `say` promises no omen its actions lack: when it names an omen
/// (by its English name, or without the leading "Omen of" where two or more
/// words are left, "Dextral Exaltation"), an action carries that omen; when
/// it mentions an omen at all (the word, or a name), an action carries one.
/// `side-omen` counts as both of its currency's side omens, `greater-omen`
/// as Omen of Greater Exaltation.
fn check_say(errors: &mut Errors<'_>, step: &Step) {
    let id = step.id.get_ref();
    let say = step.say.get_ref().to_lowercase();

    let mut carries_one = false;
    let mut carried: Vec<&str> = Vec::new();
    for action in &step.actions {
        let action = action.get_ref();
        let currency = Currency::of(action.currency.get_ref());
        for omen in &action.omens {
            carries_one = true;
            match omen.get_ref().as_str() {
                SIDE_OMEN => {
                    for side in [Side::Prefix, Side::Suffix] {
                        if let Some(e) = currency.and_then(|c| c.side_omen(side)) {
                            carried.push(e.id);
                        }
                    }
                }
                GREATER_OMEN => carried.push(GREATER_EXALTATION),
                other => carried.push(other),
            }
        }
    }

    let mut mentions = say
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == "omen" || word == "omens");
    let mut lacking = false;
    for entry in catalogue::CATALOGUE.iter().filter(|e| e.kind == Kind::Omen) {
        if !omen_phrases(entry.name).iter().any(|p| has_phrase(&say, p)) {
            continue;
        }
        mentions = true;
        if !carried.contains(&entry.id) {
            lacking = true;
            errors.push(
                step.say.span(),
                format!("step `{id}`: `say` names {}, which no action of the step carries", entry.name),
            );
        }
    }
    if mentions && !carries_one && !lacking {
        errors.push(
            step.say.span(),
            format!("step `{id}`: `say` mentions an omen, but no action of the step carries one"),
        );
    }
}

/// The lowercase phrases that name an omen in a step's text: its English
/// name, and the name without "Omen of" (and "the") when two or more words
/// are left. "Light" or "Whittling" alone name nothing.
fn omen_phrases(name: &str) -> Vec<String> {
    let full = name.to_lowercase();
    let mut phrases = vec![full.clone()];
    let short = full.strip_prefix("omen of ").unwrap_or(&full);
    let short = short.strip_prefix("the ").unwrap_or(short);
    if short.split_whitespace().count() >= 2 {
        phrases.push(short.to_string());
    }
    phrases
}

/// Whether `phrase` is in `text` as whole words.
fn has_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + phrase.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn check_action(errors: &mut Errors<'_>, step: &Step, action: &crate::model::Action) {
    let id = step.id.get_ref();
    let goal = *step.goal.get_ref();
    let currency_id = action.currency.get_ref().as_str();
    let currency = Currency::of(currency_id);
    match currency {
        None if catalogue::entry(currency_id).is_some_and(|e| e.kind == Kind::Omen) => errors.push(
            action.currency.span(),
            format!("step `{id}`: `{currency_id}` is an omen, not a currency; list it in `omens`"),
        ),
        None => errors.push(
            action.currency.span(),
            format!("step `{id}`: unknown currency `{currency_id}`"),
        ),
        Some(c) if !c.effect().serves(goal) => errors.push(
            action.currency.span(),
            format!(
                "step `{id}`: `{currency_id}` ({}) cannot meet the `{goal}` goal",
                effect_words(c.effect())
            ),
        ),
        Some(_) => {}
    }

    let mut seen: Vec<&str> = Vec::new();
    let mut sides: Vec<(Side, &str)> = Vec::new();
    for omen in &action.omens {
        let omen_id = omen.get_ref().as_str();
        let span = omen.span();
        if seen.contains(&omen_id) {
            errors.push(span, format!("step `{id}`: `{omen_id}` is named twice on one action"));
            continue;
        }
        seen.push(omen_id);

        if omen_id == GREATER_OMEN {
            let on_an_exalt = matches!(currency, Some(Currency::Static(e)) if EXALTED_ORBS.contains(&e.id));
            // An unknown currency has had its error.
            if currency.is_some() && !on_an_exalt {
                errors.push(
                    span,
                    format!(
                        "step `{id}`: `{GREATER_OMEN}` on `{currency_id}`; it is for an Exalted Orb ({})",
                        EXALTED_ORBS.join(", ")
                    ),
                );
            }
            continue;
        }

        if omen_id == SIDE_OMEN {
            if step.bind.is_none() && step.side.is_none() && step.side_rule.is_none() {
                errors.push(
                    span.clone(),
                    format!("step `{id}`: `{SIDE_OMEN}` needs the step's `bind` or `side` to pick a side"),
                );
            }
            if let Some(c) = currency {
                if c.side_omen(Side::Prefix).is_none() && c.side_omen(Side::Suffix).is_none() {
                    errors.push(
                        span,
                        format!("step `{id}`: `{SIDE_OMEN}` on `{currency_id}`, which has no Sinistral or Dextral omen"),
                    );
                }
            }
            continue;
        }

        let Some(entry) = catalogue::entry(omen_id) else {
            errors.push(span, format!("step `{id}`: unknown omen `{omen_id}`"));
            continue;
        };
        let Some(spec) = entry.omen else {
            errors.push(
                span,
                format!("step `{id}`: `{omen_id}` is not an omen; an action has one currency"),
            );
            continue;
        };
        if let Some(c) = currency {
            if !c.takes(&spec) {
                errors.push(
                    span,
                    format!("step `{id}`: `{omen_id}` does not modify `{currency_id}` (it modifies {})", fits_words(spec.fits)),
                );
            }
        }
        if let Some(side) = spec.side {
            sides.push((side, omen_id));
        }
    }
    if sides.iter().any(|(s, _)| *s == Side::Prefix) && sides.iter().any(|(s, _)| *s == Side::Suffix) {
        errors.push(
            action.currency.span(),
            format!("step `{id}`: both a Sinistral and a Dextral omen on one action"),
        );
    }
    if !sides.is_empty() && seen.contains(&SIDE_OMEN) {
        errors.push(
            action.currency.span(),
            format!("step `{id}`: `{SIDE_OMEN}` beside `{}` on one action", sides[0].1),
        );
    }
    if seen.contains(&GREATER_OMEN) && seen.contains(&GREATER_EXALTATION) {
        errors.push(
            action.currency.span(),
            format!("step `{id}`: `{GREATER_OMEN}` beside `{GREATER_EXALTATION}` on one action"),
        );
    }
}

fn effect_words(effect: catalogue::Effect) -> &'static str {
    use catalogue::Effect::*;
    match effect {
        Transmute => "makes a Normal item Magic",
        Augment => "adds to a Magic item",
        Upgrade => "makes a Magic item Rare",
        Alchemy => "makes an item Rare with four modifiers",
        Add => "adds a modifier",
        Swap => "removes one modifier and adds one",
        Remove => "removes only",
        Convert => "converts modifiers",
        Desecrate => "desecrates",
        Reveal => "reveals",
        RerollValues => "rerolls values only",
        Fracture => "fractures",
        Manual => "is manual",
        Omen => "is an omen",
    }
}

fn fits_words(fits: catalogue::OmenFits) -> String {
    use catalogue::OmenFits::*;
    match fits {
        Currencies(ids) => ids.join(", "),
        SwapEssences => "Perfect and special essences and alloys".to_string(),
        Desecration => "bones and `desecrate`".to_string(),
        Reveal => "`reveal`".to_string(),
        Nothing => "nothing known".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Condition, SideRule};

    #[test]
    fn the_shape_pass_knows_the_models_condition_keys() {
        let start: Vec<&str> = START.iter().map(|f| f.key).collect();
        assert_eq!(start, Condition::START_KEYS);
        let step: Vec<&str> = STEP_CONDITION.iter().map(|f| f.key).collect();
        let both: Vec<&str> = Condition::START_KEYS
            .iter()
            .chain(Condition::STEP_ONLY_KEYS)
            .copied()
            .collect();
        assert_eq!(step, both);
        let guard = STEP.iter().find(|f| f.key == "guard").map(|f| f.ty);
        assert!(matches!(guard, Some(Ty::Table(fields, _)) if std::ptr::eq(fields, STEP_CONDITION)));
    }

    #[test]
    fn a_steps_side_takes_the_sides_then_the_rules() {
        let words: Vec<&str> = Side::NAMES.iter().chain(SideRule::NAMES).copied().collect();
        assert_eq!(words, Step::SIDE_WORDS);
    }

    #[test]
    fn omens_are_named_by_whole_phrases() {
        assert_eq!(omen_phrases("Omen of Light"), ["omen of light"]);
        assert_eq!(omen_phrases("Omen of Dextral Exaltation"), ["omen of dextral exaltation", "dextral exaltation"]);
        assert_eq!(omen_phrases("Omen of the Sovereign"), ["omen of the sovereign"]);
        assert!(has_phrase("exalt (dextral exaltation)", "dextral exaltation"));
        assert!(!has_phrase("sinistral or dextral exaltations", "dextral exaltation"));
        assert!(!has_phrase("omen of lights", "omen of light"));
    }

    #[test]
    fn line_col_counts_characters_from_one() {
        let text = "a = 1\nbé = 2\n";
        assert_eq!(line_col(text, 0), (1, 1));
        assert_eq!(line_col(text, 6), (2, 1));
        // "é" is two bytes: the byte after it is column 3.
        assert_eq!(line_col(text, 9), (2, 3));
    }
}
