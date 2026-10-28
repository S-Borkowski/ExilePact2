//! The view the window renders, as the `wr:view` event and the `get_view`
//! command carry it (Context "The IPC contract"). `src/ipc.ts` mirrors these
//! types field for field; every type here denies unknown fields, so a field
//! either side adds, drops or renames fails
//! `view_fixtures_round_trip_through_the_view_type`.
//!
//! The selection and the search parts have view types of their own rather
//! than `wr-regex`'s, so the engine's types stay free of the contract's
//! strictness and the contract stays free of the engine's.

use serde::{Deserialize, Serialize};
use wr_regex::{
    Builder, Corruption, Group, Kind, Outcome, Part, PartWhat, Rarity, Selection, WantMode, LIMIT,
};

/// Where the mod data came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataInfo {
    pub source: String,
    pub fetched: String,
}

/// One modifier family the player can want or avoid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModView {
    pub id: String,
    pub group: Group,
    pub kind: Kind,
    pub name: Option<String>,
    /// The first line.
    pub label: String,
    pub lines: Vec<String>,
    pub tiers: u32,
    pub token: Option<String>,
    pub why_not: Option<String>,
}

/// The rarities to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RarityView {
    pub normal: bool,
    pub magic: bool,
    pub rare: bool,
}

/// The player's rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionView {
    pub want: Vec<String>,
    pub want_mode: WantMode,
    pub avoid: Vec<String>,
    pub tier_min: u8,
    pub tier_max: u8,
    pub rarity: RarityView,
    pub corruption: Corruption,
}

impl From<&Selection> for SelectionView {
    fn from(s: &Selection) -> SelectionView {
        SelectionView {
            want: s.want.clone(),
            want_mode: s.want_mode,
            avoid: s.avoid.clone(),
            tier_min: s.tier_min,
            tier_max: s.tier_max,
            rarity: RarityView {
                normal: s.rarity.normal,
                magic: s.rarity.magic,
                rare: s.rarity.rare,
            },
            corruption: s.corruption,
        }
    }
}

impl From<&SelectionView> for Selection {
    fn from(s: &SelectionView) -> Selection {
        Selection {
            want: s.want.clone(),
            want_mode: s.want_mode,
            avoid: s.avoid.clone(),
            tier_min: s.tier_min,
            tier_max: s.tier_max,
            rarity: Rarity {
                normal: s.rarity.normal,
                magic: s.rarity.magic,
                rare: s.rarity.rare,
            },
            corruption: s.corruption,
        }
    }
}

/// What a selection builds now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultState {
    Ok,
    Empty,
    TooLong,
    Unmatchable,
}

/// One rule's text in the search, and its length in characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartView {
    pub what: PartWhat,
    pub text: String,
    pub length: usize,
}

impl From<&Part> for PartView {
    fn from(p: &Part) -> PartView {
        PartView {
            what: p.what,
            text: p.text.clone(),
            length: p.length,
        }
    }
}

/// The search for the current selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchResult {
    pub state: ResultState,
    /// The search string; empty unless `state` is `ok`.
    pub regex: String,
    /// The string's length; when too long, the length it would have had.
    pub length: usize,
    pub limit: usize,
    pub parts: Vec<PartView>,
    pub unmatchable: Vec<String>,
}

impl SearchResult {
    pub fn from_outcome(outcome: &Outcome) -> SearchResult {
        let base = SearchResult {
            state: ResultState::Empty,
            regex: String::new(),
            length: 0,
            limit: LIMIT,
            parts: Vec::new(),
            unmatchable: Vec::new(),
        };
        match outcome {
            Outcome::Ok { regex, parts } => SearchResult {
                state: ResultState::Ok,
                regex: regex.clone(),
                length: regex.chars().count(),
                parts: parts.iter().map(PartView::from).collect(),
                ..base
            },
            Outcome::Empty => base,
            Outcome::TooLong { length, parts } => SearchResult {
                state: ResultState::TooLong,
                length: *length,
                parts: parts.iter().map(PartView::from).collect(),
                ..base
            },
            Outcome::Unmatchable { ids } => SearchResult {
                state: ResultState::Unmatchable,
                unmatchable: ids.clone(),
                ..base
            },
        }
    }
}

/// Whether the status line reports or warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Info,
    Error,
}

/// The one status line under the search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusLine {
    pub kind: StatusKind,
    pub text: String,
}

impl StatusLine {
    pub fn info(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Info,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Error,
            text: text.into(),
        }
    }
}

/// Everything the window shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub data: DataInfo,
    pub mods: Vec<ModView>,
    pub selection: SelectionView,
    pub result: SearchResult,
    pub hotkey: String,
    pub status: Option<StatusLine>,
    /// The newer version a check found this run (`1.3.0`), which the
    /// header's *Settings* button names; `None` while none is known.
    pub update_available: Option<String>,
}

/// The mod list of `builder`'s data, in the data's order, with each
/// family's token or the reason it has none.
pub fn mod_views(builder: &Builder) -> Vec<ModView> {
    builder
        .data()
        .families
        .iter()
        .zip(builder.tokens().all())
        .map(|(f, t)| ModView {
            id: f.id.clone(),
            group: f.group,
            kind: f.kind,
            name: f.name.clone(),
            label: f.lines.first().cloned().unwrap_or_default(),
            lines: f.lines.clone(),
            tiers: f.tiers,
            token: t.token.clone(),
            why_not: t.why_not.clone(),
        })
        .collect()
}
