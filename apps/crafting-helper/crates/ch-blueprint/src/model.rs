//! The blueprint model: what one `apps/crafting-helper/blueprints/*.toml`
//! file says, deserialised from TOML.
//!
//! This is the format's contract with `ch-engine`. Every value the
//! validator, or the engine's data-backed check, may refuse keeps its byte
//! span ([`Spanned`]) so the refusal can name its line and column.
//!
//! A file, in short:
//!
//! ```toml
//! id = "rings-resist-whittle"          # the file stem
//! name = "Rings: resistances by whittling"
//! classes = ["Rings"]                  # data `class` strings
//! source = "Discord post 10, owner's paste, 2026-10-28"
//! notes = "optional free text"
//!
//! [[start]]                            # one or more start alternatives
//! rarity = ["rare"]
//! wanted_at_least = 2
//!
//! [[step]]                             # ordered steps
//! id = "fill"
//! say = "Exalt the open side"
//! goal = "add-wanted"
//! bind = "any-remaining"
//! until = { wanted_remaining_at_most = 2 }
//! on_miss = "goto:whittle"
//!
//! [[step.action]]                      # alternatives, preferred first
//! currency = "exalted-orb"
//! omens = ["side-omen"]
//! ```
//!
//! Keys are snake case; enumerated values are kebab case. Every table
//! refuses a key it does not know.
//!
//! A file that says `order = "state"` is state-ordered: the engine takes the
//! first step whose `guard` holds (and whose goal is not met) on the item as
//! it stands, so it has no transitions, and `side` may name a rule
//! (`target`, `close`, `blocked`) instead of a side. It names the
//! not-established facts its route relies on in `relies_on`. The rules are
//! docs/models/crafting-slots-spec.md, "Route rules".
//!
//! ```toml
//! order = "state"
//! relies_on = ["greater-with-side-omen", "greater-needs-two-free"]
//!
//! [[step]]
//! id = "slam"
//! say = "Exalt for a target on its side (Sinistral or Dextral Exaltation)"
//! goal = "add-wanted"
//! side = "target"
//! guard = { blocked_side = false }
//!
//! [[step.action]]
//! currency = "perfect-exalted-orb"
//! omens = ["side-omen", "greater-omen"]
//! ```

use std::fmt;

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};

pub use toml::Spanned;

/// A byte range in the blueprint's text.
pub type Span = std::ops::Range<usize>;

/// Declares a kebab-case string enum: its variants, the names a file may
/// write ([`NAMES`](Rarity::NAMES), which the key check also reads, so the
/// two never drift), and its `Deserialize` from and `Serialize` to those
/// names.
macro_rules! named_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident = $text:literal,)+ }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
        }

        impl $name {
            /// Every name a blueprint may write, in declaration order.
            pub const NAMES: &'static [&'static str] = &[$($text,)+];

            /// The value a name stands for.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// The name a blueprint writes for this value.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.name())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.name())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = String::deserialize(d)?;
                Self::from_name(&text).ok_or_else(|| {
                    D::Error::custom(format!(
                        "`{text}` is not one of {}",
                        Self::NAMES.join(", ")
                    ))
                })
            }
        }
    };
}

named_enum! {
    /// An item's rarity, as a start or step condition names it.
    Rarity {
        Normal = "normal",
        Magic = "magic",
        Rare = "rare",
    }
}

named_enum! {
    /// A modifier's side.
    Side {
        Prefix = "prefix",
        Suffix = "suffix",
    }
}

named_enum! {
    /// A side rule: the side a step of a state-ordered blueprint works on,
    /// resolved on the item as it stands (the spec's "Side rules").
    SideRule {
        /// The bound target's side; unbound, the open side with the most
        /// missing targets the step can place there.
        Target = "target",
        /// The closable side.
        Close = "close",
        /// The blocked side (with two, the larger deficit first).
        Blocked = "blocked",
    }
}

named_enum! {
    /// How the engine picks the next step.
    Order {
        /// Steps in file order, moved between by transitions (`on_hit`,
        /// `on_miss`, `until`); what every blueprint without `order` does.
        Listed = "listed",
        /// The first step whose `guard` holds and whose goal is not met, on
        /// the item as it stands.
        State = "state",
    }
}

named_enum! {
    /// What a step is for, and so what the engine checks after it.
    Goal {
        /// A wanted modifier appears (on `side`, when given).
        AddWanted = "add-wanted",
        /// An unwanted modifier goes (from `side`, when given).
        RemoveUnwanted = "remove-unwanted",
        /// A free slot is filled with anything.
        Fill = "fill",
        /// An unrevealed desecrated modifier appears.
        Desecrate = "desecrate",
        /// The unrevealed modifier is revealed as the bound target at its
        /// floor.
        Reveal = "reveal",
        /// The desecrated modifier goes.
        RemoveDesecrated = "remove-desecrated",
        /// A flux turns one modifier into the bound target.
        Convert = "convert",
        /// The player does it and confirms: no check, no odds.
        Manual = "manual",
    }
}

named_enum! {
    /// Which target a step aims at, resolved when the engine picks a route.
    Binding {
        /// The target an unblocked essence (or alloy, or liquid) of the
        /// step's actions adds at its floor.
        Essence = "essence",
        /// The target with the lowest add odds (unknown odds count as
        /// hardest).
        Hardest = "hardest",
        /// Any target not yet on the item.
        AnyRemaining = "any-remaining",
        /// The one target not yet on the item.
        LastRemaining = "last-remaining",
    }
}

named_enum! {
    /// When a step is part of the route at all.
    When {
        /// Only when its binding resolved (an essence step with no essence
        /// for any target is skipped).
        Bound = "bound",
    }
}

/// Where the guide goes after an outcome. A file writes `next`, `repeat`,
/// `restart`, `abandon`, `done` or `goto:<step id>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Transition {
    /// The following step.
    Next,
    /// This step again.
    Repeat,
    /// The blueprint's first step.
    Restart,
    /// Give the item up.
    Abandon,
    /// The craft is finished.
    Done,
    /// The step with this id.
    Goto(String),
}

impl Transition {
    /// The keywords a file may write, beside `goto:<step id>`.
    pub const KEYWORDS: &'static [&'static str] = &["next", "repeat", "restart", "abandon", "done"];
    /// The prefix of a go-to.
    pub const GOTO: &'static str = "goto:";

    /// Reads a transition, or says why it is not one.
    pub fn parse(text: &str) -> Result<Self, String> {
        Ok(match text {
            "next" => Self::Next,
            "repeat" => Self::Repeat,
            "restart" => Self::Restart,
            "abandon" => Self::Abandon,
            "done" => Self::Done,
            _ => match text.strip_prefix(Self::GOTO) {
                Some(step) if !step.trim().is_empty() => Self::Goto(step.trim().to_string()),
                Some(_) => return Err(format!("`{text}` names no step after `{}`", Self::GOTO)),
                None => {
                    return Err(format!(
                        "`{text}` is not one of {}, or `{}<step id>`",
                        Self::KEYWORDS.join(", "),
                        Self::GOTO
                    ))
                }
            },
        })
    }
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Next => f.write_str("next"),
            Self::Repeat => f.write_str("repeat"),
            Self::Restart => f.write_str("restart"),
            Self::Abandon => f.write_str("abandon"),
            Self::Done => f.write_str("done"),
            Self::Goto(step) => write!(f, "{}{step}", Self::GOTO),
        }
    }
}

impl<'de> Deserialize<'de> for Transition {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// One blueprint file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    /// Equals the file stem.
    pub id: Spanned<String>,
    /// The title the player sees.
    pub name: Spanned<String>,
    /// The data `class` strings the blueprint is for.
    pub classes: Spanned<Vec<Spanned<String>>>,
    /// Where the technique came from ("Discord post 10, owner's paste,
    /// 2026-10-28", or "roadmap").
    pub source: Spanned<String>,
    /// Free text: caveats, mechanics not established.
    #[serde(default)]
    pub notes: Option<Spanned<String>>,
    /// How the next step is picked; [`Order::Listed`] when left out
    /// ([`Blueprint::order`]).
    #[serde(default)]
    pub order: Option<Spanned<Order>>,
    /// The not-established facts the route relies on, by their id in the
    /// spec's facts table ([`crate::catalogue::NOT_ESTABLISHED`]). Required,
    /// possibly empty, when `order = "state"`.
    #[serde(default)]
    pub relies_on: Option<Spanned<Vec<Spanned<String>>>>,
    /// The items the route may start from; any one suffices (`[[start]]`).
    #[serde(default, rename = "start")]
    pub starts: Vec<Spanned<Condition>>,
    /// The steps, in order (`[[step]]`).
    #[serde(default, rename = "step")]
    pub steps: Vec<Spanned<Step>>,
}

impl Blueprint {
    /// How the next step is picked, with the default applied.
    pub fn order(&self) -> Order {
        self.order.as_ref().map_or(Order::Listed, |o| *o.get_ref())
    }
}

/// A predicate on the item. A start alternative takes the first group of
/// keys; a step's `requires`, `until` and `guard` take them all. Every key
/// left out holds; a bool key set to `false` asks for the opposite.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// The item's rarity is one of these.
    #[serde(default)]
    pub rarity: Option<Spanned<Vec<Rarity>>>,
    /// At least this many wanted modifiers are present (at their floor).
    #[serde(default)]
    pub wanted_at_least: Option<Spanned<u32>>,
    /// At least this many wanted prefixes are present.
    #[serde(default)]
    pub wanted_prefixes_at_least: Option<Spanned<u32>>,
    /// At least this many wanted suffixes are present.
    #[serde(default)]
    pub wanted_suffixes_at_least: Option<Spanned<u32>>,
    /// `true`: a fractured modifier is present and every fractured
    /// modifier is a wanted one. `false`: no fractured modifier is present.
    #[serde(default)]
    pub fractured_wanted: Option<Spanned<bool>>,
    /// At most this many unwanted modifiers are present.
    #[serde(default)]
    pub unwanted_at_most: Option<Spanned<u32>>,
    /// An unblocked essence (or alloy, or liquid) adds one of the targets
    /// at its floor on this item.
    #[serde(default)]
    pub essence_binds: Option<Spanned<bool>>,
    /// Some side is blocked (the spec's "Slot accounting").
    #[serde(default)]
    pub blocked_side: Option<Spanned<bool>>,
    /// Some side is closable.
    #[serde(default)]
    pub closable_side: Option<Spanned<bool>>,
    /// Exactly one side is open.
    #[serde(default)]
    pub one_side_open: Option<Spanned<bool>>,
    /// A revealed desecrated modifier on the item is not a target.
    #[serde(default)]
    pub desecrated_unwanted: Option<Spanned<bool>>,
    /// Step conditions only: at least this many free prefix slots.
    #[serde(default)]
    pub free_prefixes_at_least: Option<Spanned<u32>>,
    /// Step conditions only: at least this many free suffix slots.
    #[serde(default)]
    pub free_suffixes_at_least: Option<Spanned<u32>>,
    /// Step conditions only: `true` when no desecrated modifier (revealed or
    /// not) may be present.
    #[serde(default)]
    pub no_desecrated: Option<Spanned<bool>>,
    /// Step conditions only: `true` when an unrevealed desecrated modifier
    /// must be present.
    #[serde(default)]
    pub unrevealed: Option<Spanned<bool>>,
    /// Step conditions only: at most this many targets are still missing.
    #[serde(default)]
    pub wanted_remaining_at_most: Option<Spanned<u32>>,
    /// Step conditions only: exactly this many targets are still missing.
    #[serde(default)]
    pub wanted_remaining_exactly: Option<Spanned<u32>>,
}

impl Condition {
    /// The keys a start alternative may use.
    pub const START_KEYS: &'static [&'static str] = &[
        "rarity",
        "wanted_at_least",
        "wanted_prefixes_at_least",
        "wanted_suffixes_at_least",
        "fractured_wanted",
        "unwanted_at_most",
        "essence_binds",
        "blocked_side",
        "closable_side",
        "one_side_open",
        "desecrated_unwanted",
    ];
    /// The keys only a step's `requires`, `until` and `guard` may use.
    pub const STEP_ONLY_KEYS: &'static [&'static str] = &[
        "free_prefixes_at_least",
        "free_suffixes_at_least",
        "no_desecrated",
        "unrevealed",
        "wanted_remaining_at_most",
        "wanted_remaining_exactly",
    ];
}

/// One step of the route.
///
/// The file's `side` key fills either [`side`](Step::side) (a side) or
/// [`side_rule`](Step::side_rule) (a rule word), never both.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "StepFile")]
pub struct Step {
    /// Unique within the blueprint; `goto:` names it.
    pub id: Spanned<String>,
    /// The short instruction the player sees.
    pub say: Spanned<String>,
    /// What the step is for.
    pub goal: Spanned<Goal>,
    /// The side an add or remove goal is about, when the file names one
    /// side (`prefix`, `suffix`).
    pub side: Option<Spanned<Side>>,
    /// State order only: the rule that picks the side on the item as it
    /// stands, when the file's `side` names one (`target`, `close`,
    /// `blocked`).
    pub side_rule: Option<Spanned<SideRule>>,
    /// Which target the step aims at.
    pub bind: Option<Spanned<Binding>>,
    /// Listed order only: what must hold before the step is taken; when it
    /// does not, the guide says which part failed and stops ("stuck").
    pub requires: Option<Spanned<Condition>>,
    /// When the step is part of the route at all.
    pub when: Option<Spanned<When>>,
    /// Listed order only: on a hit, repeat the step until this holds.
    pub until: Option<Spanned<Condition>>,
    /// The ways to take the step, preferred first (`[[step.action]]`). The
    /// route takes the first one none of whose ids is blocked. A `manual`
    /// step may have none.
    pub actions: Vec<Spanned<Action>>,
    /// Listed order only: where to go on a hit; `next` (or `repeat` while
    /// `until` fails) when left out.
    pub on_hit: Option<Spanned<Transition>>,
    /// Listed order only: where to go on a miss; `repeat` when left out.
    pub on_miss: Option<Spanned<Transition>>,
    /// State order only: the step is a candidate only while this holds.
    pub guard: Option<Spanned<Condition>>,
}

/// A `[[step]]` table as the file writes it, before its `side` is split
/// into a side or a rule.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepFile {
    id: Spanned<String>,
    say: Spanned<String>,
    goal: Spanned<Goal>,
    #[serde(default)]
    side: Option<Spanned<SideWord>>,
    #[serde(default)]
    bind: Option<Spanned<Binding>>,
    #[serde(default)]
    requires: Option<Spanned<Condition>>,
    #[serde(default)]
    when: Option<Spanned<When>>,
    #[serde(default)]
    until: Option<Spanned<Condition>>,
    #[serde(default, rename = "action")]
    actions: Vec<Spanned<Action>>,
    #[serde(default)]
    on_hit: Option<Spanned<Transition>>,
    #[serde(default)]
    on_miss: Option<Spanned<Transition>>,
    #[serde(default)]
    guard: Option<Spanned<Condition>>,
}

/// A word a step's `side` may hold: a side, or a side rule.
#[derive(Clone, Copy)]
enum SideWord {
    Side(Side),
    Rule(SideRule),
}

impl<'de> Deserialize<'de> for SideWord {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Side::from_name(&text)
            .map(Self::Side)
            .or_else(|| SideRule::from_name(&text).map(Self::Rule))
            .ok_or_else(|| D::Error::custom(format!("`{text}` is not one of {}", Step::SIDE_WORDS.join(", "))))
    }
}

impl From<StepFile> for Step {
    fn from(f: StepFile) -> Self {
        let (mut side, mut side_rule) = (None, None);
        if let Some(word) = f.side {
            let span = word.span();
            match *word.get_ref() {
                SideWord::Side(s) => side = Some(Spanned::new(span, s)),
                SideWord::Rule(r) => side_rule = Some(Spanned::new(span, r)),
            }
        }
        Step {
            id: f.id,
            say: f.say,
            goal: f.goal,
            side,
            side_rule,
            bind: f.bind,
            requires: f.requires,
            when: f.when,
            until: f.until,
            actions: f.actions,
            on_hit: f.on_hit,
            on_miss: f.on_miss,
            guard: f.guard,
        }
    }
}

impl Step {
    /// Every word a step's `side` may hold: the sides, then the rules.
    pub const SIDE_WORDS: &'static [&'static str] = &["prefix", "suffix", "target", "close", "blocked"];

    /// The transition on a hit, with the default applied. `until_holds`
    /// says whether the step's `until` holds on the new item (`true` when
    /// it has none).
    pub fn hit_transition(&self, until_holds: bool) -> Transition {
        match &self.on_hit {
            Some(t) => t.get_ref().clone(),
            None if self.until.is_some() && !until_holds => Transition::Repeat,
            None => Transition::Next,
        }
    }

    /// The transition on a miss, with the default applied.
    pub fn miss_transition(&self) -> Transition {
        self.on_miss
            .as_ref()
            .map(|t| t.get_ref().clone())
            .unwrap_or(Transition::Repeat)
    }
}

/// One way to take a step: a currency and the omens to have in the
/// inventory when using it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// A catalogue id, a data-backed essence, alloy or liquid id, or the
    /// placeholder `desecrate` (the bones that fit the item) or `reveal`
    /// (the reveal at the Well of Souls).
    pub currency: Spanned<String>,
    /// Omen ids, or the placeholder `side-omen` (the Sinistral or Dextral
    /// omen of this currency for the bound target's side).
    #[serde(default)]
    pub omens: Vec<Spanned<String>>,
}
