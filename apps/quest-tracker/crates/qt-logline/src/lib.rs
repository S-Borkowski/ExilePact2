//! Client.txt line parser: a pure function from one decoded log line (and the
//! allowlists from the quest data) to one typed event or none. Chat, unknown
//! speakers and every unrecognised line produce nothing.
//!
//! The line shapes are the ones measured in the owner's English client log
//! (`docs/quest-tracker/quest-tracker-research.md`):
//!
//! ```text
//! YYYY/MM/DD HH:MM:SS <ms counter> <hex> [<LEVEL> Client <pid>] <message>
//! YYYY/MM/DD HH:MM:SS ***** LOG FILE OPENING *****
//! ```
//!
//! A line without a leading timestamp is the continuation of a multi-line
//! warning and is ignored. The parser keeps no state: the same line always
//! gives the same result, so the tailer, the engine and the replay CLI can
//! call it from anywhere.

use std::collections::HashSet;
use std::fmt;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// The wall-clock time a line was written, as the client logs it (local time,
/// second resolution). Orders chronologically; serialises as the log's own
/// `YYYY/MM/DD HH:MM:SS` text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Timestamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Timestamp {
    /// Parses `YYYY/MM/DD HH:MM:SS`; `None` for anything else.
    pub fn parse(text: &str) -> Option<Timestamp> {
        let b = text.as_bytes();
        if b.len() != 19
            || b[4] != b'/'
            || b[7] != b'/'
            || b[10] != b' '
            || b[13] != b':'
            || b[16] != b':'
        {
            return None;
        }
        let num = |from: usize, to: usize| -> Option<u16> {
            let part = &text[from..to];
            if part.bytes().all(|c| c.is_ascii_digit()) {
                part.parse().ok()
            } else {
                None
            }
        };
        let ts = Timestamp {
            year: num(0, 4)?,
            month: num(5, 7)? as u8,
            day: num(8, 10)? as u8,
            hour: num(11, 13)? as u8,
            minute: num(14, 16)? as u8,
            second: num(17, 19)? as u8,
        };
        let valid = (1..=12).contains(&ts.month)
            && (1..=31).contains(&ts.day)
            && ts.hour < 24
            && ts.minute < 60
            && ts.second < 61;
        valid.then_some(ts)
    }

    /// Seconds from `earlier` to this time, negative when `earlier` is
    /// later. Both are the log's local wall-clock time, so a clock change
    /// between them shows up in the result.
    pub fn seconds_since(&self, earlier: &Timestamp) -> i64 {
        self.seconds() - earlier.seconds()
    }

    /// This time advanced by `seconds` (back, when negative) on the same
    /// wall clock, carrying into minutes, hours, days, months and years. The
    /// shell uses it for "now" in log time: the last line's timestamp plus
    /// the whole seconds elapsed since it was handled.
    pub fn plus_seconds(&self, seconds: i64) -> Timestamp {
        let total = self.seconds() + seconds;
        let (days, secs) = (total.div_euclid(86_400), total.rem_euclid(86_400));
        // The civil date from days since 1970-01-01, the inverse of
        // `seconds` (Howard Hinnant's algorithm).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Timestamp {
            year: year as u16,
            month: month as u8,
            day: day as u8,
            hour: (secs / 3_600) as u8,
            minute: (secs % 3_600 / 60) as u8,
            second: (secs % 60) as u8,
        }
    }

    /// Seconds since 1970-01-01 00:00:00 of the same wall clock.
    fn seconds(&self) -> i64 {
        // Days from the civil date (proleptic Gregorian), Howard Hinnant's
        // algorithm.
        let (m, d) = (i64::from(self.month), i64::from(self.day));
        let y = i64::from(self.year) - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        days * 86_400
            + i64::from(self.hour) * 3_600
            + i64::from(self.minute) * 60
            + i64::from(self.second)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}/{:02}/{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

impl From<Timestamp> for String {
    fn from(ts: Timestamp) -> String {
        ts.to_string()
    }
}

impl TryFrom<String> for Timestamp {
    type Error = String;

    fn try_from(text: String) -> Result<Timestamp, String> {
        Timestamp::parse(&text).ok_or_else(|| format!("not a log timestamp: {text:?}"))
    }
}

/// Which passive tree a `You have received <n> ... Passive Skill Points.`
/// line adds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointPool {
    Passive,
    WeaponSet,
}

/// Whether a `has joined/left the area.` line is a join or a leave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyChange {
    Joined,
    Left,
}

/// What one recognised line says. Serialised with a snake_case `kind` tag
/// beside the variant's fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// `***** LOG FILE OPENING *****`: the client started writing (a launch
    /// can write it two or three times within seconds).
    SessionStart,
    /// The client's build tag, from the HTTP user agent line.
    ClientVersion { tag: String },
    /// `Generating level <level> area "<code>" with seed <seed>`: one per
    /// instance joined. Towns and hideouts have seed 1.
    AreaEntered { code: String, level: u32, seed: u64 },
    /// `[SCENE] Set Source [<name>]`, trimmed, never `(null)`, `(unknown)` or
    /// `Act <n>`.
    AreaName { name: String },
    /// `<speaker>: <text>` with the speaker on the NPC allowlist.
    NpcLine { speaker: String, text: String },
    /// A speakerless line whose whole text is on the `system_lines` allowlist.
    SystemLine { text: String },
    /// `: <name> (<class>) is now level <level>`, party members included.
    LevelUp {
        name: String,
        class: String,
        level: u32,
    },
    /// `: <name> has been slain.`, party members included.
    Slain { name: String },
    /// `: <name> has received <stat>.` with the markup resolved.
    Received { name: String, stat: String },
    /// `: <name> has lost <stat>.` with the markup resolved.
    Lost { name: String, stat: String },
    /// `: You have received <amount> [Weapon Set ]Passive Skill Points.`
    /// These lines name no character.
    Points { pool: PointPool, amount: u32 },
    /// `: <name> has joined the area.` / `has left the area.`
    Party { name: String, change: PartyChange },
    /// `[WARN Client <pid>] Character <name> has unknown build of path: None`,
    /// that exact shape only. The client wrote it within about 2 s of a
    /// relog into The Riverbank, naming the character; it is not written for
    /// every new character, so on its own it names nobody new.
    CharacterNamed { name: String },
}

/// One recognised line: its timestamp and what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub time: Timestamp,
    #[serde(flatten)]
    pub kind: EventKind,
}

/// The speakers and speakerless texts the quest data treats as signals
/// (`npcs` and `system_lines` in `data/quests.json`). Only these turn a
/// `<speaker>: <text>` or bare line into an event: other players' local chat
/// has exactly the NPC shape, so the allowlist is the only thing that tells
/// the two apart. Matching is exact and case-sensitive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Allowlist {
    npcs: HashSet<String>,
    system_lines: HashSet<String>,
}

impl Allowlist {
    pub fn new<N, S>(npcs: N, system_lines: S) -> Allowlist
    where
        N: IntoIterator,
        N::Item: Into<String>,
        S: IntoIterator,
        S::Item: Into<String>,
    {
        Allowlist {
            npcs: npcs.into_iter().map(Into::into).collect(),
            system_lines: system_lines.into_iter().map(Into::into).collect(),
        }
    }

    /// Adds speakers (the replay CLI's `--speakers`).
    pub fn add_npcs<N>(&mut self, npcs: N)
    where
        N: IntoIterator,
        N::Item: Into<String>,
    {
        self.npcs.extend(npcs.into_iter().map(Into::into));
    }

    pub fn is_npc(&self, speaker: &str) -> bool {
        self.npcs.contains(speaker)
    }

    pub fn is_system_line(&self, text: &str) -> bool {
        self.system_lines.contains(text)
    }
}

/// Message prefixes of the chat channels (global, trade, whisper and the
/// rest, guild): a line starting with one is someone talking, never a signal.
const CHAT_PREFIXES: [char; 5] = ['#', '$', '%', '@', '&'];

/// The three `[SCENE] Set Source` values that are not area names (`Act <n>`
/// is matched separately).
const NOT_AREA_NAMES: [&str; 2] = ["(null)", "(unknown)"];

struct Patterns {
    header: Regex,
    area: Regex,
    scene: Regex,
    act: Regex,
    user_agent: Regex,
    level_up: Regex,
    slain: Regex,
    received: Regex,
    lost: Regex,
    points: Regex,
    party: Regex,
    character_build: Regex,
    markup: Regex,
}

fn patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let re = |p: &str| Regex::new(p).expect("built-in pattern");
        Patterns {
            // `<ms counter> <hex> [<LEVEL> Client <pid>] <message>`; the
            // message may be empty.
            header: re(r"^\d+ (?:[0-9A-Fa-f]+ )?\[([A-Z]+) Client \d+\](?: (.*))?$"),
            area: re(r#"^Generating level (\d+) area "([^"]+)" with seed (\d+)$"#),
            scene: re(r"^\[SCENE\] Set Source \[(.*)\]$"),
            act: re(r"^Act \d+$"),
            user_agent: re(r"^\[HTTP2\] User agent: PoE \S*/tags/(\S+)"),
            level_up: re(r"^: (\S+) \(([^)]+)\) is now level (\d+)$"),
            slain: re(r"^: (\S+) has been slain\.$"),
            received: re(r"^: (\S+) has received (.+)\.$"),
            lost: re(r"^: (\S+) has lost (.+)\.$"),
            points: re(r"^: You have received (\d+) (Weapon Set )?Passive Skill Points?\.$"),
            party: re(r"^: (\S+) has (joined|left) the area\.$"),
            // Only on a `[WARN ...]` line.
            character_build: re(r"^Character (\S+) has unknown build of path: None$"),
            // `[Group|Shown]` or `[Shown]`: keep what is shown.
            markup: re(r"\[(?:[^\]|]*\|)?([^\]|]*)\]"),
        }
    })
}

/// Resolves the client's stat markup: `[Group|Shown]` and `[Shown]` both
/// become `Shown`, so `+10% to [Resistances|Cold Resistance]` reads
/// `+10% to Cold Resistance`.
pub fn strip_markup(text: &str) -> String {
    patterns().markup.replace_all(text, "$1").into_owned()
}

/// Parses one decoded log line into one event, or `None` for chat, an
/// unknown speaker, a continuation line and anything else not recognised.
/// A trailing `\r\n` (or a leading byte-order mark) is tolerated.
pub fn parse_line(line: &str, allow: &Allowlist) -> Option<Event> {
    let line = line.strip_prefix('\u{feff}').unwrap_or(line);
    let line = line.trim_end_matches(['\r', '\n']);
    // Continuation lines carry no timestamp.
    let time = Timestamp::parse(line.get(..19)?)?;
    let rest = line[19..].strip_prefix(' ')?;
    if rest.trim_end() == "***** LOG FILE OPENING *****" {
        return Some(Event {
            time,
            kind: EventKind::SessionStart,
        });
    }
    let caps = patterns().header.captures(rest)?;
    let message = caps.get(2).map_or("", |m| m.as_str());
    if &caps[1] == "WARN" {
        if let Some(c) = patterns().character_build.captures(message) {
            return Some(Event {
                time,
                kind: EventKind::CharacterNamed {
                    name: c[1].to_string(),
                },
            });
        }
    }
    parse_message(message, allow).map(|kind| Event { time, kind })
}

fn parse_message(message: &str, allow: &Allowlist) -> Option<EventKind> {
    let p = patterns();
    let first = message.chars().next()?;
    if CHAT_PREFIXES.contains(&first) {
        return None;
    }
    if let Some(c) = p.area.captures(message) {
        return Some(EventKind::AreaEntered {
            code: c[2].to_string(),
            level: c[1].parse().ok()?,
            seed: c[3].parse().ok()?,
        });
    }
    if let Some(c) = p.scene.captures(message) {
        let name = c[1].trim();
        if name.is_empty() || NOT_AREA_NAMES.contains(&name) || p.act.is_match(name) {
            return None;
        }
        return Some(EventKind::AreaName {
            name: name.to_string(),
        });
    }
    if let Some(c) = p.user_agent.captures(message) {
        return Some(EventKind::ClientVersion {
            tag: c[1].to_string(),
        });
    }
    if message.starts_with(": ") {
        return parse_system_message(message);
    }
    if allow.is_system_line(message) {
        return Some(EventKind::SystemLine {
            text: message.to_string(),
        });
    }
    let (speaker, text) = message.split_once(": ")?;
    if allow.is_npc(speaker) {
        return Some(EventKind::NpcLine {
            speaker: speaker.to_string(),
            text: text.to_string(),
        });
    }
    None
}

/// The game's own notices, written after a `: ` with no speaker.
fn parse_system_message(message: &str) -> Option<EventKind> {
    let p = patterns();
    if let Some(c) = p.points.captures(message) {
        let pool = if c.get(2).is_some() {
            PointPool::WeaponSet
        } else {
            PointPool::Passive
        };
        return Some(EventKind::Points {
            pool,
            amount: c[1].parse().ok()?,
        });
    }
    if let Some(c) = p.level_up.captures(message) {
        return Some(EventKind::LevelUp {
            name: c[1].to_string(),
            class: c[2].to_string(),
            level: c[3].parse().ok()?,
        });
    }
    if let Some(c) = p.slain.captures(message) {
        return Some(EventKind::Slain {
            name: c[1].to_string(),
        });
    }
    if let Some(c) = p.party.captures(message) {
        let change = if &c[2] == "joined" {
            PartyChange::Joined
        } else {
            PartyChange::Left
        };
        return Some(EventKind::Party {
            name: c[1].to_string(),
            change,
        });
    }
    if let Some(c) = p.received.captures(message) {
        return Some(EventKind::Received {
            name: c[1].to_string(),
            stat: strip_markup(&c[2]),
        });
    }
    if let Some(c) = p.lost.captures(message) {
        return Some(EventKind::Lost {
            name: c[1].to_string(),
            stat: strip_markup(&c[2]),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_round_trips_and_rejects_garbage() {
        let ts = Timestamp::parse("2026/03/07 09:05:01").unwrap();
        assert_eq!(ts.to_string(), "2026/03/07 09:05:01");
        assert!(Timestamp::parse("2026/13/07 09:05:01").is_none());
        assert!(Timestamp::parse("2026-03-07 09:05:01").is_none());
        assert!(Timestamp::parse("2026/03/07 9:05:01").is_none());
    }

    #[test]
    fn seconds_between_timestamps() {
        let t = |s: &str| Timestamp::parse(s).unwrap();
        let entry = t("2026/03/14 12:00:00");
        assert_eq!(t("2026/03/14 12:00:05").seconds_since(&entry), 5);
        assert_eq!(entry.seconds_since(&t("2026/03/14 12:00:05")), -5);
        // Across midnight, a month end and a leap day.
        assert_eq!(
            t("2026/04/01 00:00:02").seconds_since(&t("2026/03/31 23:59:58")),
            4
        );
        assert_eq!(
            t("2028/03/01 00:00:00").seconds_since(&t("2028/02/28 00:00:00")),
            2 * 86_400
        );
        assert_eq!(
            t("2027/01/01 00:00:00").seconds_since(&t("2026/12/31 23:59:59")),
            1
        );
    }

    #[test]
    fn timestamp_plus_seconds_carries() {
        let t = |s: &str| Timestamp::parse(s).unwrap();
        let entry = t("2026/03/14 12:00:00");
        assert_eq!(entry.plus_seconds(0), entry);
        assert_eq!(entry.plus_seconds(5), t("2026/03/14 12:00:05"));
        assert_eq!(entry.plus_seconds(-1), t("2026/03/14 11:59:59"));
        // Across midnight, a month end, a leap day and a year end.
        assert_eq!(
            t("2026/03/31 23:59:58").plus_seconds(4),
            t("2026/04/01 00:00:02")
        );
        assert_eq!(
            t("2028/02/28 23:59:59").plus_seconds(1),
            t("2028/02/29 00:00:00")
        );
        assert_eq!(
            t("2027/02/28 23:59:59").plus_seconds(1),
            t("2027/03/01 00:00:00")
        );
        assert_eq!(
            t("2026/12/31 23:59:59").plus_seconds(1),
            t("2027/01/01 00:00:00")
        );
        assert_eq!(
            t("2027/01/01 00:00:00").plus_seconds(-1),
            t("2026/12/31 23:59:59")
        );
        // It is the inverse of seconds_since.
        for step in [1_i64, 59, 3_601, 86_399, 86_400 * 40 + 7, -86_400 * 400 - 3] {
            assert_eq!(entry.plus_seconds(step).seconds_since(&entry), step);
        }
    }

    #[test]
    fn markup_keeps_the_shown_part() {
        assert_eq!(
            strip_markup(
                "30% increased Global [Armour], [Evasion] and [EnergyShield|Energy Shield]"
            ),
            "30% increased Global Armour, Evasion and Energy Shield"
        );
        assert_eq!(strip_markup("+20 to maximum Life"), "+20 to maximum Life");
    }

    #[test]
    fn event_serialises_with_kind_tag() {
        let ev = Event {
            time: Timestamp::parse("2026/03/07 09:05:01").unwrap(),
            kind: EventKind::Points {
                pool: PointPool::WeaponSet,
                amount: 2,
            },
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert_eq!(
            json,
            r#"{"time":"2026/03/07 09:05:01","kind":"points","pool":"weapon_set","amount":2}"#
        );
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ev);
    }
}
