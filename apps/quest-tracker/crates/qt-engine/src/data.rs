//! The quest data file (`data/quests.json`): its typed form and the validator
//! that checks a file against the contract before the engine ever sees it.
//!
//! Validation walks the JSON value by hand rather than leaning on serde's
//! errors, so every failure names the quest (by id, or by index when the id
//! itself is the problem) and the field path inside it, e.g.
//! `quest "beira": field "steps[0].signals[1].speaker": "Nobody" is not in npcs`.
//! Only a value that passed is turned into the typed structs.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

use qt_logline::{Allowlist, PointPool};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The `schema` number this crate reads and writes.
pub const SCHEMA: u64 = 1;

/// A quest's status for one character (Context "Quest state rules").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Locked,
    Available,
    InProgress,
    Done,
    Skipped,
    Unknown,
}

impl Status {
    pub const NAMES: [&'static str; 6] = [
        "locked",
        "available",
        "in_progress",
        "done",
        "skipped",
        "unknown",
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Locked => "locked",
            Status::Available => "available",
            Status::InProgress => "in_progress",
            Status::Done => "done",
            Status::Skipped => "skipped",
            Status::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a quest is worth doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Worth {
    Yes,
    Optional,
    No,
}

impl Worth {
    pub const NAMES: [&'static str; 3] = ["yes", "optional", "no"];
}

/// How much of a quest the log can follow on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Detection {
    Auto,
    Partial,
    Manual,
}

impl Detection {
    pub const NAMES: [&'static str; 3] = ["auto", "partial", "manual"];

    /// The manual-tick marker shows for `partial` and `manual`.
    pub fn needs_tick(self) -> bool {
        self != Detection::Auto
    }
}

/// How a fact in the data file is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Evidence {
    #[serde(rename = "measured")]
    Measured,
    #[serde(rename = "static")]
    Static,
    #[serde(rename = "not-established")]
    NotEstablished,
}

impl Evidence {
    pub const NAMES: [&'static str; 3] = ["measured", "static", "not-established"];
}

/// What a reward gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardCategory {
    PassivePoints,
    WeaponSetPoints,
    Spirit,
    Resistance,
    Life,
    Mana,
    Attribute,
    Charm,
    Flask,
    Defences,
    OtherStat,
    SkillGem,
    SupportGem,
    Ascendancy,
    Item,
    Unlock,
    Story,
}

impl RewardCategory {
    pub const NAMES: [&'static str; 17] = [
        "passive_points",
        "weapon_set_points",
        "spirit",
        "resistance",
        "life",
        "mana",
        "attribute",
        "charm",
        "flask",
        "defences",
        "other_stat",
        "skill_gem",
        "support_gem",
        "ascendancy",
        "item",
        "unlock",
        "story",
    ];

    /// The categories that are a permanent character stat (worth `yes`).
    pub fn is_permanent_stat(self) -> bool {
        use RewardCategory::*;
        matches!(
            self,
            PassivePoints
                | WeaponSetPoints
                | Spirit
                | Resistance
                | Life
                | Mana
                | Attribute
                | Charm
                | Flask
                | Defences
                | OtherStat
        )
    }
}

/// A resistance reward's element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Cold,
    Lightning,
    Chaos,
    All,
}

impl Element {
    pub const NAMES: [&'static str; 5] = ["fire", "cold", "lightning", "chaos", "all"];
}

/// Sorts the stat text of a `has received` line into a reward category, for
/// `reward` signals that name a category instead of an exact text. The order
/// matters: `Life Recovery from Flasks` is a flask reward, not a life one.
pub fn classify_stat(stat: &str) -> RewardCategory {
    let has = |needle: &str| stat.contains(needle);
    if has("Weapon Set Passive Skill Point") {
        RewardCategory::WeaponSetPoints
    } else if has("Passive Skill Point") {
        RewardCategory::PassivePoints
    } else if has("Flask") {
        RewardCategory::Flask
    } else if has("Charm") {
        RewardCategory::Charm
    } else if has("Spirit") {
        RewardCategory::Spirit
    } else if has("Resistance") {
        RewardCategory::Resistance
    } else if has("maximum Life") {
        RewardCategory::Life
    } else if has("maximum Mana") {
        RewardCategory::Mana
    } else if has("Strength") || has("Dexterity") || has("Intelligence") || has("Attributes") {
        RewardCategory::Attribute
    } else if has("Defences") || has("Armour") || has("Evasion") || has("Energy Shield") {
        RewardCategory::Defences
    } else {
        RewardCategory::OtherStat
    }
}

/// One condition the log (or the engine's own state) can satisfy. A list of
/// signals fires when any one of them fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Signal {
    /// Entering an area, by its code (`Generating level ... area "<code>"`)
    /// or by its display name (`[SCENE] Set Source [<name>]`).
    Area {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// An allowlisted speaker's line, optionally an exact line and in an area.
    Npc {
        speaker: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        area: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<String>,
    },
    /// An allowlisted speakerless line, optionally in an area.
    System {
        line: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        area: Option<String>,
    },
    /// A `has received` line of the character, by exact stat text or category.
    Reward {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<RewardCategory>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        area: Option<String>,
    },
    /// A `You have received <n> ... Passive Skill Points.` line.
    Points {
        #[serde(rename = "type")]
        pool: PointPool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        area: Option<String>,
    },
    /// The character's level is at least `min` (evaluated against its state).
    Level { min: u32 },
    /// Another quest has this status (evaluated at computation time).
    Quest { id: String, status: Status },
}

impl Signal {
    /// `level` and `quest` signals are read from the character's state when a
    /// status is computed; every other kind fires on a log event and is kept
    /// as evidence.
    pub fn is_state_based(&self) -> bool {
        matches!(self, Signal::Level { .. } | Signal::Quest { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Area {
    pub name: String,
    pub act: u32,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    pub category: RewardCategory,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<Element>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub text: String,
    pub signals: Vec<Signal>,
    /// The owner's short "how to find it" text, shown under the step: one
    /// line of at most [`TIP_MAX_CHARS`] characters, curated by hand (the
    /// poe2db extract never writes it). Absent in the file means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip: Option<String>,
}

/// The longest a step's `tip` may be, in characters.
pub const TIP_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quest {
    pub id: String,
    pub name: String,
    pub act: u32,
    pub poe2db_type: String,
    pub giver: Option<String>,
    pub zones: Vec<String>,
    pub rewards: Vec<Reward>,
    pub worth: Worth,
    pub worth_reason: String,
    pub steps: Vec<Step>,
    pub unlock: Vec<Signal>,
    /// Signals that make the quest in progress before any step fires (a
    /// talk the log does not record, read from a later event); absent in
    /// the file means empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start: Vec<Signal>,
    pub done: Vec<Signal>,
    pub window_closes: Vec<Signal>,
    pub detection: Detection,
    pub evidence: Evidence,
    pub source_url: String,
    pub fetched: String,
    #[serde(default)]
    pub notes: String,
    /// The quest's `done` signals are a later event, not its own completion,
    /// so a done status is an inference (absent in the file means false).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub done_inferred: bool,
}

/// The whole quest data file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestData {
    pub schema: u64,
    pub game_version: String,
    pub poe2db_fetched: String,
    pub source: String,
    pub source_note: String,
    pub npcs: Vec<String>,
    pub system_lines: Vec<String>,
    pub areas: BTreeMap<String, Area>,
    pub quests: Vec<Quest>,
}

/// Why a data file was refused: which quest (when the problem is inside one),
/// which field, and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    /// The quest's id, or `#<index>` when it has no usable id.
    pub quest: Option<String>,
    /// The field path, e.g. `steps[0].signals[1].speaker`.
    pub field: String,
    pub message: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.quest {
            Some(q) => write!(f, "quest {q:?}: field {:?}: {}", self.field, self.message),
            None => write!(f, "field {:?}: {}", self.field, self.message),
        }
    }
}

impl std::error::Error for DataError {}

impl QuestData {
    /// Parses and validates a data file's text.
    pub fn parse(text: &str) -> Result<QuestData, DataError> {
        let value: Value = serde_json::from_str(text).map_err(|e| DataError {
            quest: None,
            field: String::new(),
            message: format!("not valid JSON: {e}"),
        })?;
        QuestData::from_value(&value)
    }

    /// Reads, parses and validates a data file.
    pub fn load(path: &Path) -> Result<QuestData, DataError> {
        let text = std::fs::read_to_string(path).map_err(|e| DataError {
            quest: None,
            field: String::new(),
            message: format!("cannot read {}: {e}", path.display()),
        })?;
        QuestData::parse(&text)
    }

    /// Validates an already parsed value against the contract.
    pub fn from_value(value: &Value) -> Result<QuestData, DataError> {
        validate(value)?;
        serde_json::from_value(value.clone()).map_err(|e| DataError {
            quest: None,
            field: String::new(),
            message: format!("passed validation but did not deserialise: {e}"),
        })
    }

    /// The parser's allowlist: the data's `npcs` and `system_lines`.
    pub fn allowlist(&self) -> Allowlist {
        Allowlist::new(self.npcs.iter().cloned(), self.system_lines.iter().cloned())
    }

    pub fn quest(&self, id: &str) -> Option<&Quest> {
        self.quests.iter().find(|q| q.id == id)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

struct Checker<'a> {
    quest: Option<String>,
    npcs: HashSet<&'a str>,
    system_lines: HashSet<&'a str>,
    areas: HashSet<&'a str>,
    quest_ids: HashSet<&'a str>,
}

impl Checker<'_> {
    fn err(&self, field: &str, message: impl Into<String>) -> DataError {
        DataError {
            quest: self.quest.clone(),
            field: field.to_string(),
            message: message.into(),
        }
    }

    fn object<'v>(&self, v: &'v Value, field: &str) -> Result<&'v Map<String, Value>, DataError> {
        v.as_object()
            .ok_or_else(|| self.err(field, format!("must be an object, found {}", kind_of(v))))
    }

    fn array<'v>(&self, v: &'v Value, field: &str) -> Result<&'v Vec<Value>, DataError> {
        v.as_array()
            .ok_or_else(|| self.err(field, format!("must be a list, found {}", kind_of(v))))
    }

    fn string<'v>(&self, v: &'v Value, field: &str) -> Result<&'v str, DataError> {
        v.as_str()
            .ok_or_else(|| self.err(field, format!("must be a string, found {}", kind_of(v))))
    }

    fn nonempty<'v>(&self, v: &'v Value, field: &str) -> Result<&'v str, DataError> {
        let s = self.string(v, field)?;
        if s.trim().is_empty() {
            return Err(self.err(field, "must not be empty"));
        }
        Ok(s)
    }

    /// A step's `tip`: non-empty after trimming, one line, at most
    /// [`TIP_MAX_CHARS`] characters.
    fn tip(&self, v: &Value, field: &str) -> Result<(), DataError> {
        let s = self.nonempty(v, field)?;
        if s.contains(['\n', '\r']) {
            return Err(self.err(field, "must be one line"));
        }
        let n = s.chars().count();
        if n > TIP_MAX_CHARS {
            return Err(self.err(
                field,
                format!("is {n} characters long, at most {TIP_MAX_CHARS} allowed"),
            ));
        }
        Ok(())
    }

    fn positive(&self, v: &Value, field: &str) -> Result<u64, DataError> {
        match v.as_u64() {
            Some(n) if n >= 1 && n <= u32::MAX as u64 => Ok(n),
            _ => Err(self.err(
                field,
                format!("must be a whole number of 1 or more, found {v}"),
            )),
        }
    }

    fn one_of(&self, v: &Value, field: &str, allowed: &[&str]) -> Result<(), DataError> {
        let s = self.string(v, field)?;
        if allowed.contains(&s) {
            Ok(())
        } else {
            Err(self.err(field, format!("{s:?} is not one of {}", allowed.join(", "))))
        }
    }

    fn date(&self, v: &Value, field: &str) -> Result<(), DataError> {
        let s = self.string(v, field)?;
        if is_date(s) {
            Ok(())
        } else {
            Err(self.err(field, format!("{s:?} is not a YYYY-MM-DD date")))
        }
    }

    fn strings(&self, v: &Value, field: &str) -> Result<(), DataError> {
        for (i, item) in self.array(v, field)?.iter().enumerate() {
            self.nonempty(item, &format!("{field}[{i}]"))?;
        }
        Ok(())
    }

    /// Fails on a missing required key or any key outside the two lists.
    fn keys(
        &self,
        map: &Map<String, Value>,
        prefix: &str,
        required: &[&str],
        optional: &[&str],
    ) -> Result<(), DataError> {
        for key in required {
            if !map.contains_key(*key) {
                return Err(self.err(&join(prefix, key), "is missing"));
            }
        }
        for key in map.keys() {
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                return Err(self.err(&join(prefix, key), "is not a field of the contract"));
            }
        }
        Ok(())
    }

    fn area_code(&self, v: &Value, field: &str) -> Result<(), DataError> {
        let code = self.nonempty(v, field)?;
        if self.areas.contains(code) {
            Ok(())
        } else {
            Err(self.err(field, format!("area code {code:?} is not in areas")))
        }
    }

    fn signals(&self, v: &Value, field: &str) -> Result<(), DataError> {
        for (i, s) in self.array(v, field)?.iter().enumerate() {
            self.signal(s, &format!("{field}[{i}]"))?;
        }
        Ok(())
    }

    fn signal(&self, v: &Value, path: &str) -> Result<(), DataError> {
        let m = self.object(v, path)?;
        let kind_field = join(path, "kind");
        let kind = match m.get("kind") {
            Some(k) => self.string(k, &kind_field)?,
            None => return Err(self.err(&kind_field, "is missing")),
        };
        let (required, optional): (&[&str], &[&str]) = match kind {
            "area" => (&["kind"], &["code", "name"]),
            "npc" => (&["kind", "speaker"], &["area", "line"]),
            "system" => (&["kind", "line"], &["area"]),
            "reward" => (&["kind"], &["text", "category", "area"]),
            "points" => (&["kind", "type"], &["area"]),
            "level" => (&["kind", "min"], &[]),
            "quest" => (&["kind", "id", "status"], &[]),
            other => {
                return Err(self.err(
                    &kind_field,
                    format!(
                        "unknown signal kind {other:?} (one of area, npc, system, reward, points, level, quest)"
                    ),
                ))
            }
        };
        self.keys(m, path, required, optional)?;
        if let Some(a) = m.get("area") {
            self.area_code(a, &join(path, "area"))?;
        }
        match kind {
            "area" => match (m.get("code"), m.get("name")) {
                (Some(c), None) => self.area_code(c, &join(path, "code"))?,
                (None, Some(n)) => {
                    self.nonempty(n, &join(path, "name"))?;
                }
                _ => return Err(self.err(path, "an area signal needs exactly one of code or name")),
            },
            "npc" => {
                let f = join(path, "speaker");
                let speaker = self.nonempty(&m["speaker"], &f)?;
                if !self.npcs.contains(speaker) {
                    return Err(self.err(&f, format!("{speaker:?} is not in npcs")));
                }
                if let Some(l) = m.get("line") {
                    self.nonempty(l, &join(path, "line"))?;
                }
            }
            "system" => {
                let f = join(path, "line");
                let line = self.nonempty(&m["line"], &f)?;
                if !self.system_lines.contains(line) {
                    return Err(self.err(&f, format!("{line:?} is not in system_lines")));
                }
            }
            "reward" => match (m.get("text"), m.get("category")) {
                (Some(t), None) => {
                    self.nonempty(t, &join(path, "text"))?;
                }
                (None, Some(c)) => {
                    self.one_of(c, &join(path, "category"), &RewardCategory::NAMES)?
                }
                _ => {
                    return Err(self.err(
                        path,
                        "a reward signal needs exactly one of text or category",
                    ))
                }
            },
            "points" => self.one_of(&m["type"], &join(path, "type"), &["passive", "weapon_set"])?,
            "level" => {
                self.positive(&m["min"], &join(path, "min"))?;
            }
            "quest" => {
                let f = join(path, "id");
                let id = self.nonempty(&m["id"], &f)?;
                if !self.quest_ids.contains(id) {
                    return Err(self.err(&f, format!("names no quest: {id:?}")));
                }
                self.one_of(&m["status"], &join(path, "status"), &Status::NAMES)?;
            }
            _ => unreachable!("kind checked above"),
        }
        Ok(())
    }

    fn quest(&self, q: &Map<String, Value>) -> Result<(), DataError> {
        self.keys(
            q,
            "",
            &[
                "id",
                "name",
                "act",
                "poe2db_type",
                "giver",
                "zones",
                "rewards",
                "worth",
                "worth_reason",
                "steps",
                "unlock",
                "done",
                "window_closes",
                "detection",
                "evidence",
                "source_url",
                "fetched",
            ],
            &["notes", "done_inferred", "start"],
        )?;
        self.nonempty(&q["name"], "name")?;
        self.positive(&q["act"], "act")?;
        self.nonempty(&q["poe2db_type"], "poe2db_type")?;
        if !q["giver"].is_null() {
            self.nonempty(&q["giver"], "giver")?;
        }
        self.strings(&q["zones"], "zones")?;
        for (i, r) in self.array(&q["rewards"], "rewards")?.iter().enumerate() {
            let p = format!("rewards[{i}]");
            let m = self.object(r, &p)?;
            self.keys(m, &p, &["category", "text"], &["element"])?;
            self.one_of(
                &m["category"],
                &join(&p, "category"),
                &RewardCategory::NAMES,
            )?;
            self.nonempty(&m["text"], &join(&p, "text"))?;
            let is_res = m["category"].as_str() == Some("resistance");
            match m.get("element") {
                Some(e) if is_res => self.one_of(e, &join(&p, "element"), &Element::NAMES)?,
                Some(_) => {
                    return Err(self.err(
                        &join(&p, "element"),
                        "only a resistance reward has an element",
                    ))
                }
                None if is_res => {
                    return Err(self.err(
                        &join(&p, "element"),
                        "is missing (a resistance reward names its element)",
                    ))
                }
                None => {}
            }
        }
        self.one_of(&q["worth"], "worth", &Worth::NAMES)?;
        self.nonempty(&q["worth_reason"], "worth_reason")?;
        let mut step_ids = HashSet::new();
        for (i, s) in self.array(&q["steps"], "steps")?.iter().enumerate() {
            let p = format!("steps[{i}]");
            let m = self.object(s, &p)?;
            self.keys(m, &p, &["id", "title", "text", "signals"], &["tip"])?;
            let id = self.nonempty(&m["id"], &join(&p, "id"))?;
            if !step_ids.insert(id) {
                return Err(self.err(&join(&p, "id"), format!("duplicate step id {id:?}")));
            }
            self.nonempty(&m["title"], &join(&p, "title"))?;
            self.string(&m["text"], &join(&p, "text"))?;
            self.signals(&m["signals"], &join(&p, "signals"))?;
            if let Some(t) = m.get("tip") {
                self.tip(t, &join(&p, "tip"))?;
            }
        }
        self.signals(&q["unlock"], "unlock")?;
        if let Some(st) = q.get("start") {
            self.signals(st, "start")?;
        }
        self.signals(&q["done"], "done")?;
        self.signals(&q["window_closes"], "window_closes")?;
        self.one_of(&q["detection"], "detection", &Detection::NAMES)?;
        self.one_of(&q["evidence"], "evidence", &Evidence::NAMES)?;
        let url = self.string(&q["source_url"], "source_url")?;
        if !is_quest_url(url) {
            return Err(self.err(
                "source_url",
                format!("{url:?} is not a https://poe2db.tw/us/<page> URL"),
            ));
        }
        self.date(&q["fetched"], "fetched")?;
        if let Some(n) = q.get("notes") {
            self.string(n, "notes")?;
        }
        if let Some(d) = q.get("done_inferred") {
            if !d.is_boolean() {
                return Err(self.err(
                    "done_inferred",
                    format!("must be true or false, found {}", kind_of(d)),
                ));
            }
        }
        Ok(())
    }
}

fn validate(value: &Value) -> Result<(), DataError> {
    let mut c = Checker {
        quest: None,
        npcs: HashSet::new(),
        system_lines: HashSet::new(),
        areas: HashSet::new(),
        quest_ids: HashSet::new(),
    };
    let top = c.object(value, "")?;
    c.keys(
        top,
        "",
        &[
            "schema",
            "game_version",
            "poe2db_fetched",
            "source",
            "source_note",
            "npcs",
            "system_lines",
            "areas",
            "quests",
        ],
        &[],
    )?;
    if top["schema"].as_u64() != Some(SCHEMA) {
        return Err(c.err(
            "schema",
            format!("must be {SCHEMA}, found {}", top["schema"]),
        ));
    }
    c.nonempty(&top["game_version"], "game_version")?;
    c.date(&top["poe2db_fetched"], "poe2db_fetched")?;
    let source = c.string(&top["source"], "source")?;
    if !source.starts_with("https://poe2db.tw/") {
        return Err(c.err(
            "source",
            format!("{source:?} is not a https://poe2db.tw/ URL"),
        ));
    }
    c.nonempty(&top["source_note"], "source_note")?;

    c.strings(&top["npcs"], "npcs")?;
    c.strings(&top["system_lines"], "system_lines")?;
    let npcs = top["npcs"].as_array().expect("checked");
    let system_lines = top["system_lines"].as_array().expect("checked");
    c.npcs = npcs.iter().filter_map(Value::as_str).collect();
    c.system_lines = system_lines.iter().filter_map(Value::as_str).collect();

    let areas = c.object(&top["areas"], "areas")?;
    for (code, area) in areas {
        let p = format!("areas.{code}");
        let m = c.object(area, &p)?;
        c.keys(m, &p, &["name", "act", "evidence"], &[])?;
        c.nonempty(&m["name"], &join(&p, "name"))?;
        c.positive(&m["act"], &join(&p, "act"))?;
        c.one_of(&m["evidence"], &join(&p, "evidence"), &Evidence::NAMES)?;
    }
    c.areas = areas.keys().map(String::as_str).collect();

    let quests = c.array(&top["quests"], "quests")?;
    // First pass: every quest is an object with a well-formed, unique id, so
    // `quest` signals can be checked against the full set.
    let mut first_index: HashMap<&str, usize> = HashMap::new();
    for (i, q) in quests.iter().enumerate() {
        c.quest = Some(format!("#{i}"));
        let m = c.object(q, "")?;
        let id = match m.get("id") {
            Some(v) => c.nonempty(v, "id")?,
            None => return Err(c.err("id", "is missing")),
        };
        c.quest = Some(id.to_string());
        if !is_kebab(id) {
            return Err(c.err("id", format!("{id:?} is not kebab-case")));
        }
        if let Some(prev) = first_index.insert(id, i) {
            return Err(c.err("id", format!("duplicate id (quests #{prev} and #{i})")));
        }
    }
    c.quest_ids = first_index.keys().copied().collect();
    for q in quests {
        let m = q.as_object().expect("checked");
        c.quest = m["id"].as_str().map(str::to_string);
        c.quest(m)?;
    }
    Ok(())
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

fn kind_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("built-in pattern"))
}

/// `kebab-case`: lower-case letters and digits in dash-separated runs.
pub fn is_kebab(id: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"^[a-z0-9]+(?:-[a-z0-9]+)*$").is_match(id)
}

/// `https://poe2db.tw/us/<page>`, the page one path segment without spaces,
/// query or fragment.
pub fn is_quest_url(url: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"^https://poe2db\.tw/us/[^/\s?#]+$").is_match(url)
}

/// `YYYY-MM-DD`, a real calendar date.
pub fn is_date(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let Some(c) = re(&RE, r"^(\d{4})-(\d{2})-(\d{2})$").captures(text) else {
        return false;
    };
    let (y, m, d): (u32, u32, u32) = (
        c[1].parse().unwrap(),
        c[2].parse().unwrap(),
        c[3].parse().unwrap(),
    );
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_texts_sort_into_categories() {
        let cases = [
            ("+10% to Cold Resistance", RewardCategory::Resistance),
            (
                "+5% to all Elemental Resistances",
                RewardCategory::Resistance,
            ),
            ("+30 to Spirit", RewardCategory::Spirit),
            ("+20 to maximum Life", RewardCategory::Life),
            ("5% increased maximum Mana", RewardCategory::Mana),
            ("+1 Charm Slot", RewardCategory::Charm),
            (
                "30% increased Life Recovery from Flasks",
                RewardCategory::Flask,
            ),
            ("+5 to Strength", RewardCategory::Attribute),
            ("15% increased Global Defences", RewardCategory::Defences),
            (
                "30% increased Global Armour, Evasion and Energy Shield",
                RewardCategory::Defences,
            ),
            ("3% increased Movement Speed", RewardCategory::OtherStat),
        ];
        for (text, want) in cases {
            assert_eq!(classify_stat(text), want, "{text}");
        }
    }

    #[test]
    fn dates_ids_and_urls() {
        assert!(is_date("2026-10-24"));
        assert!(is_date("2024-02-29"));
        assert!(!is_date("2026-02-29"));
        assert!(!is_date("2026-13-01"));
        assert!(!is_date("26-10-24"));
        assert!(is_kebab("the-king-in-the-mists"));
        assert!(!is_kebab("The_King"));
        assert!(!is_kebab("a--b"));
        assert!(is_quest_url("https://poe2db.tw/us/Reaching_Clearfell"));
        assert!(!is_quest_url("http://poe2db.tw/us/Reaching_Clearfell"));
        assert!(!is_quest_url("https://poe2db.tw/us/"));
        assert!(!is_quest_url("https://example.com/us/Reaching_Clearfell"));
    }

    #[test]
    fn enum_name_lists_match_serde() {
        for name in Status::NAMES {
            let s: Status = serde_json::from_value(Value::String(name.into())).unwrap();
            assert_eq!(s.as_str(), name);
        }
        for name in RewardCategory::NAMES {
            let _: RewardCategory = serde_json::from_value(Value::String(name.into())).unwrap();
        }
        for name in Evidence::NAMES {
            let _: Evidence = serde_json::from_value(Value::String(name.into())).unwrap();
        }
        for name in Element::NAMES {
            let _: Element = serde_json::from_value(Value::String(name.into())).unwrap();
        }
    }

    #[test]
    fn empty_data_file_validates() {
        let text = r#"{"schema":1,"game_version":"x","poe2db_fetched":"2026-10-25",
            "source":"https://poe2db.tw/us/Quest","source_note":"n",
            "npcs":[],"system_lines":[],"areas":{},"quests":[]}"#;
        QuestData::parse(text).unwrap();
    }
}
