//! The mod data file (`data/waystone-mods.json`): its typed form and the
//! validator that checks a file against the contract before anything uses it.
//!
//! Each family is read on its own, so every refusal names the family (by id,
//! or by index when the id itself is the problem) and the field, e.g.
//! `family "monster-have-chance-to-bleat" (families[3]): field "lines[0]": is empty`.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `schema` number this crate reads.
pub const SCHEMA: u64 = 1;

/// Which poe2db list a family comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Waystone,
    Desecrated,
}

/// Whether a mod is a prefix or a suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Prefix,
    Suffix,
}

/// One modifier as the player sees it: the poe2db rows whose displayed lines
/// are the same, merged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub id: String,
    pub group: Group,
    pub kind: Kind,
    /// The desecrated affix name (`Abyssal`, `of Siphoning`); null for a
    /// regular mod.
    pub name: Option<String>,
    /// The displayed lines, case kept, each number or range replaced by `#`.
    pub lines: Vec<String>,
    /// One per line: the line is one of the shared reward templates.
    pub bonus: Vec<bool>,
    /// How many poedb rows were merged into this family.
    pub tiers: u32,
}

impl Family {
    /// The lines that are the mod's own, not a shared reward line.
    pub fn own_lines(&self) -> impl Iterator<Item = &str> {
        self.lines
            .iter()
            .zip(&self.bonus)
            .filter(|(_, b)| !**b)
            .map(|(l, _)| l.as_str())
    }
}

/// A poedb row the data tool could not turn into a family, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skipped {
    pub section: String,
    /// Where the row was, as the data tool wrote it.
    pub row: Value,
    pub reason: String,
}

/// The whole data file, validated.
#[derive(Debug, Clone, PartialEq)]
pub struct ModData {
    pub schema: u64,
    pub source: String,
    pub sections: Vec<String>,
    pub fetched: String,
    pub source_note: String,
    pub families: Vec<Family>,
    pub skipped: Vec<Skipped>,
    index: HashMap<String, usize>,
}

/// Why a data file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    /// The family's id, or `families[i]` when the id is the problem.
    pub family: Option<String>,
    pub field: String,
    pub message: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(family) = &self.family {
            write!(f, "family {family}: ")?;
        }
        if !self.field.is_empty() {
            write!(f, "field \"{}\": ", self.field)?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for DataError {}

const TOP_FIELDS: [&str; 7] = [
    "schema",
    "source",
    "sections",
    "fetched",
    "source_note",
    "families",
    "skipped",
];

fn top(field: &str, message: impl Into<String>) -> DataError {
    DataError {
        family: None,
        field: field.to_string(),
        message: message.into(),
    }
}

fn string_field(obj: &serde_json::Map<String, Value>, field: &str) -> Result<String, DataError> {
    match obj.get(field) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(top(field, "is empty")),
        Some(_) => Err(top(field, "is not a string")),
        None => Err(top(field, "is missing")),
    }
}

impl ModData {
    /// Parses and validates the text of a data file.
    pub fn parse(text: &str) -> Result<ModData, DataError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| top("", format!("not valid JSON: {e}")))?;
        ModData::from_value(&value)
    }

    /// Reads, parses and validates a data file.
    pub fn load(path: &Path) -> Result<ModData, DataError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| top("", format!("cannot read {}: {e}", path.display())))?;
        ModData::parse(&text)
    }

    /// Validates a parsed data file.
    pub fn from_value(value: &Value) -> Result<ModData, DataError> {
        let obj = value
            .as_object()
            .ok_or_else(|| top("", "the file is not a JSON object"))?;
        for key in obj.keys() {
            if !TOP_FIELDS.contains(&key.as_str()) {
                return Err(top(key, "is not a field of the data file"));
            }
        }

        let schema = obj
            .get("schema")
            .and_then(Value::as_u64)
            .ok_or_else(|| top("schema", "is missing or not a number"))?;
        if schema != SCHEMA {
            return Err(top(
                "schema",
                format!("is {schema}, this app reads {SCHEMA}"),
            ));
        }
        let source = string_field(obj, "source")?;
        if !source.starts_with("https://") {
            return Err(top("source", format!("{source:?} is not an https URL")));
        }
        let fetched = string_field(obj, "fetched")?;
        let date = Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("date pattern");
        if !date.is_match(&fetched) {
            return Err(top(
                "fetched",
                format!("{fetched:?} is not a YYYY-MM-DD date"),
            ));
        }
        let source_note = string_field(obj, "source_note")?;
        let sections: Vec<String> = match obj.get("sections") {
            Some(v) => serde_json::from_value(v.clone())
                .map_err(|e| top("sections", format!("is not a list of strings: {e}")))?,
            None => return Err(top("sections", "is missing")),
        };
        if sections.is_empty() || sections.iter().any(|s| s.trim().is_empty()) {
            return Err(top("sections", "must list at least one non-empty tab id"));
        }
        let skipped: Vec<Skipped> = match obj.get("skipped") {
            Some(v) => serde_json::from_value(v.clone())
                .map_err(|e| top("skipped", format!("is not a list of skipped rows: {e}")))?,
            None => return Err(top("skipped", "is missing")),
        };
        for (i, s) in skipped.iter().enumerate() {
            if s.section.trim().is_empty() || s.reason.trim().is_empty() {
                return Err(top(
                    &format!("skipped[{i}]"),
                    "needs a section and a reason",
                ));
            }
        }

        let raw = match obj.get("families") {
            Some(Value::Array(a)) => a,
            Some(_) => return Err(top("families", "is not a list")),
            None => return Err(top("families", "is missing")),
        };
        if raw.is_empty() {
            return Err(top("families", "is empty"));
        }
        let slug = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").expect("slug pattern");
        let mut families = Vec::with_capacity(raw.len());
        let mut index = HashMap::new();
        let mut by_lines: BTreeMap<Vec<String>, String> = BTreeMap::new();
        for (i, v) in raw.iter().enumerate() {
            let label = match v.get("id").and_then(Value::as_str) {
                Some(id) => format!("\"{id}\" (families[{i}])"),
                None => format!("families[{i}]"),
            };
            let err = |field: &str, message: String| DataError {
                family: Some(label.clone()),
                field: field.to_string(),
                message,
            };
            let f: Family =
                serde_json::from_value(v.clone()).map_err(|e| err("", e.to_string()))?;
            if !slug.is_match(&f.id) {
                return Err(err("id", format!("{:?} is not a kebab-case slug", f.id)));
            }
            if index.insert(f.id.clone(), i).is_some() {
                return Err(err(
                    "id",
                    format!("{:?} is used by an earlier family", f.id),
                ));
            }
            match (&f.group, &f.name) {
                (_, Some(n)) if n.trim().is_empty() || n.trim() != n => {
                    return Err(err("name", format!("{n:?} is empty or padded")));
                }
                (Group::Waystone, Some(n)) => {
                    return Err(err(
                        "name",
                        format!("{n:?}: only a desecrated mod carries an affix name"),
                    ));
                }
                _ => {}
            }
            if f.lines.is_empty() {
                return Err(err("lines", "is empty".to_string()));
            }
            for (j, line) in f.lines.iter().enumerate() {
                if line.trim().is_empty() {
                    return Err(err(&format!("lines[{j}]"), "is empty".to_string()));
                }
                if line.trim() != line || line.contains('\n') || line.contains('\r') {
                    return Err(err(
                        &format!("lines[{j}]"),
                        format!("{line:?} is padded or holds a line break"),
                    ));
                }
            }
            if f.bonus.len() != f.lines.len() {
                return Err(err(
                    "bonus",
                    format!("has {} entries for {} lines", f.bonus.len(), f.lines.len()),
                ));
            }
            if let Some(first) = by_lines.insert(f.lines.clone(), f.id.clone()) {
                return Err(err(
                    "lines",
                    format!(
                        "are the same as family {first:?}'s; families are keyed by their lines"
                    ),
                ));
            }
            if f.tiers == 0 {
                return Err(err(
                    "tiers",
                    "is 0; a family merges at least one row".to_string(),
                ));
            }
            families.push(f);
        }

        Ok(ModData {
            schema,
            source,
            sections,
            fetched,
            source_note,
            families,
            skipped,
            index,
        })
    }

    /// The family with this id.
    pub fn family(&self, id: &str) -> Option<&Family> {
        self.index.get(id).map(|&i| &self.families[i])
    }

    /// The position of the family with this id in [`ModData::families`].
    pub fn position(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
}

/// Ids of the families with a line holding `corr`, which would make the
/// corruption terms (`corr`, `!corr`) match a mod instead of the
/// `Corrupted` line. Empty for usable data.
pub fn corruption_collisions(data: &ModData) -> Vec<String> {
    data.families
        .iter()
        .filter(|f| f.lines.iter().any(|l| l.to_lowercase().contains("corr")))
        .map(|f| f.id.clone())
        .collect()
}
