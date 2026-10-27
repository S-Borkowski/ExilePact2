//! The screen reader's thread (Context "Design: the reader thread"): with the
//! `screen_reading` setting on, it copies the regions of the game's window
//! the worker's demand names, reads them with the backend's OCR, feeds the
//! trackers of `qt-screen`, and sends what they report to the worker on the
//! worker's own channel.
//!
//! - **Demand.** The worker publishes a [`DemandSnapshot`] by value after
//!   every batch: the screen signals that could fire now, their regions and
//!   remark texts, and the confirmed game pid. The reader takes the newest
//!   at each tick with `try_lock` and never waits for it. It captures
//!   nothing while the demand is empty, the pid unconfirmed, or the game's
//!   window missing, minimized or not in front, and its `idle (<reason>)`
//!   line names the first of those that held.
//! - **A death with a bar up.** The demand also counts the character's live
//!   deaths. When the count changes, each demanded boss whose bar the reader
//!   has seen and whose going it has not reported is marked: its next going
//!   is not counted, so a bar that stays up through the death screen and
//!   goes on the walk back ticks nothing. The bar must show again before a
//!   going counts: a mark ends with the going it was for, or, when the
//!   band was out of sight and the tracker could not report that going,
//!   once the bar has been absent for a going's run and then present for
//!   the tracker's minimum ([`DeathMarks`]). A lost or refused window
//!   forgets the marks with the trackers. The worker still drops a going
//!   last seen before the death, for one already queued when the line came.
//! - **Locks.** The reader takes no `App` state lock and never calls the
//!   shell's `Window`: everything it needs of the system goes through the
//!   backend, and everything the view needs goes to the worker as a
//!   [`ScreenMsg`]. The backend sits behind its own mutex, taken with
//!   `try_lock` for one tick at a time, so a reader started while a stopped
//!   one is still inside an OCR call waits for it without missing its stop.
//! - **Privacy.** OCR text lives only in the observations of one tick and in
//!   the trackers' matches; what leaves the thread is an event kind, a name
//!   from the quest data, times, counts and region names. No line here
//!   formats OCR text, and an error names only the backend call.
//! - **Diagnostic lines.** Each is `screen: ` and one of the shapes the
//!   Context pins ("Privacy and diagnostic lines"); an error's detail goes on
//!   its own `error: screen reader: ` line. Beside the bar's counts, the
//!   per-minute line is followed by one line per NPC whose dialogue window
//!   and per boss whose voiced lines were read that minute, so a route that
//!   ticked nothing says which of its parts never matched ([`RouteWatch`]).
//! - **The dialogue window's gate.** While no demanded NPC's heading has
//!   read lately, a due read of the dialogue window copies and reads only
//!   its heading strip; the whole region once the heading shows there, in
//!   the same tick ([`DialogueGate`]). Its counts go on a `minute: dialogue
//!   window gate, ...` line.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use qt_engine::{Engine, ScreenOutcome, ScreenWhat};
use qt_screen::{
    aspect_is_16_9, classify_bar, fuzzy_score, is_banner, is_heading, remark_body, BackendError,
    BarClass, Demand, DialogueGate, DialogueRead, GateCounts, Hidden, Millis, Observation,
    OcrReadiness, PixelRect, Region, RemarkDemand, ScreenBackend, ScreenEvent, Trackers,
    BANNER_AREA, BANNER_TEXT, GONE_RUN, HEADING_STRIP, HEADING_WINDOW_MS, MATCH_THRESHOLD,
    MIN_PRESENT,
};
use qt_viewmodel::StatusLine;

use crate::app::{App, Inbound};

/// The longest an idle reader sleeps before it looks at its stop flag and
/// the demand again.
pub const IDLE_SLEEP: Duration = Duration::from_millis(100);

/// The shortest sleep between two ticks.
const MIN_SLEEP: Duration = Duration::from_millis(5);

/// How many failed copies in a row, or black copies in a row, set a status
/// line. The two runs are counted apart: a black copy is a dark scene or a
/// loading screen as often as a copy that got nothing back.
pub const CAPTURE_FAILURE_LIMIT: u32 = 10;

/// How often the reader writes its per-minute line, on the shell's clock.
pub const MINUTE_MS: u64 = 60_000;

/// The status line when Windows has no English OCR.
pub const NO_ENGLISH_STATUS: &str = "Screen reading is off: Windows has no English OCR.";

/// The status line when the reader ended without being stopped.
pub const STOPPED_STATUS: &str =
    "Screen reading stopped after an error; turn it off and on to retry.";

/// The status line when a panel, or a name in the bar's place, has hidden a
/// boss bar that was up for [`GONE_RUN`] reads with no sight of it since:
/// its going would have counted by now, and cannot until the bar shows again.
pub const PANEL_STATUS: &str = "A panel or a name over the boss bar hides it; screen reading counts the bar going only once it shows again.";

/// The status line when a boss bar that was up has been out of sight (the
/// game not in front or minimized, the overlay over it, a failed copy or OCR)
/// for [`GONE_RUN`] reads with no sight of it since: as with a panel, its
/// going cannot count until the bar shows again.
pub const OUT_OF_SIGHT_STATUS: &str = "The boss bar was out of sight after it showed; screen reading counts the bar going only once it shows again.";

/// The backend, shared by every reader the app starts (one at a time).
pub(crate) type SharedBackend = Arc<Mutex<Box<dyn ScreenBackend>>>;

/// What the worker publishes for the reader: what the screen could finish
/// now, for which steps, and the confirmed game pid.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DemandSnapshot {
    pub demand: Demand,
    /// The `<quest>/<step>` pairs the demand reads for, in data order.
    pub steps: Vec<(String, String)>,
    /// The game's pid, once a live line or its window in front confirmed it.
    pub pid: Option<u32>,
    /// The current character's live deaths the worker received. Only a
    /// change means anything: the reader marks the bars up at that moment.
    pub deaths: u64,
}

impl DemandSnapshot {
    /// The engine's screen demand as the reader takes it: each boss's bar,
    /// each NPC's window, and each boss's kill remark that has a text, with
    /// the steps they would tick. A `kill_remark` with no text reads nothing
    /// and names no step.
    pub fn of(engine: &Engine, pid: Option<u32>, deaths: u64) -> DemandSnapshot {
        let mut demand = Demand::default();
        let mut steps: Vec<(String, String)> = Vec::new();
        for want in engine.screen_demand() {
            match want.what {
                ScreenWhat::BarGone => {
                    if !demand.bars.contains(&want.name) {
                        demand.bars.push(want.name.clone());
                    }
                }
                ScreenWhat::QuestComplete => {
                    if !demand.dialogues.contains(&want.name) {
                        demand.dialogues.push(want.name.clone());
                    }
                }
                ScreenWhat::KillRemark => {
                    let texts: Vec<String> = engine
                        .data()
                        .remark_texts(&want.name)
                        .into_iter()
                        .map(str::to_string)
                        .collect();
                    if texts.is_empty() {
                        continue;
                    }
                    if !demand.remarks.iter().any(|r| r.boss == want.name) {
                        demand.remarks.push(RemarkDemand {
                            boss: want.name.clone(),
                            texts,
                        });
                    }
                }
            }
            let step = (want.quest_id, want.step_id);
            if !steps.contains(&step) {
                steps.push(step);
            }
        }
        DemandSnapshot {
            demand,
            steps,
            pid,
            deaths,
        }
    }
}

/// What the reader sends the worker, tagged with its generation.
#[derive(Debug)]
pub(crate) enum ScreenMsg {
    /// A tracker's event, with the wall-clock time (Unix milliseconds) its
    /// line names: when the bar was last seen, or when the event was seen.
    /// `after_death` is set on a bar's first going after a death the reader
    /// saw while the bar was up, which is not counted.
    Seen {
        event: ScreenEvent,
        wall_ms: i64,
        after_death: bool,
    },
    /// The `Source::Screen` status line, or `None` to clear it.
    Status(Option<StatusLine>),
}

/// A running reader, as the app keeps it.
pub(crate) struct ReaderHandle {
    pub stop: Arc<AtomicBool>,
    pub handle: thread::JoinHandle<()>,
}

// ---------------------------------------------------------------------------
// The diagnostic lines.

/// `hh:mm:ss.mmm` of a Unix time in milliseconds, in UTC.
pub fn utc_clock(unix_ms: i64) -> String {
    let ms = unix_ms.rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1_000 % 60,
        ms % 1_000
    )
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(names: &[String]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// The regions, as a line names them: `boss bar`, `boss bar and voiced
/// lines`, `boss bar, voiced lines and dialogue window`.
fn region_list(regions: &[Region]) -> String {
    let names: Vec<String> = regions.iter().map(|r| r.name().to_string()).collect();
    and_list(&names)
}

/// `reading <regions> for <quest>/<step>[, <quest>/<step>]...`
fn reading_line(regions: &[Region], steps: &[(String, String)]) -> String {
    let steps: Vec<String> = steps.iter().map(|(q, s)| format!("{q}/{s}")).collect();
    format!(
        "screen: reading {} for {}",
        region_list(regions),
        steps.join(", ")
    )
}

/// The status line for the regions the overlay covers.
fn covered_status(covered: &BTreeSet<Region>) -> StatusLine {
    let names: Vec<String> = covered.iter().map(|r| format!("the {}", r.name())).collect();
    StatusLine::warn(format!(
        "The overlay covers {}; move it to let screen reading see {}.",
        and_list(&names),
        if names.len() == 1 { "it" } else { "them" }
    ))
}

/// The head of an event's line, before its outcome.
pub(crate) fn seen_head(event: &ScreenEvent, wall_ms: i64) -> String {
    let at = utc_clock(wall_ms);
    match event {
        ScreenEvent::BarGone { boss, .. } => format!("screen: bar of {boss} gone (last seen {at}Z)"),
        ScreenEvent::QuestComplete { npc, .. } => {
            format!("screen: quest complete in {npc}'s window ({at}Z)")
        }
        ScreenEvent::KillRemark { boss, .. } => format!("screen: kill remark for {boss} ({at}Z)"),
    }
}

/// Why a screen event was not counted, as its line says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotCounted {
    AfterDeath,
    AfterAreaChange,
    AreaUnknown,
    NoOpenStep,
}

impl NotCounted {
    fn why(self) -> &'static str {
        match self {
            NotCounted::AfterDeath => "after a death",
            NotCounted::AfterAreaChange => "after an area change",
            NotCounted::AreaUnknown => "area unknown",
            NotCounted::NoOpenStep => "no open step",
        }
    }
}

/// `ticked <quest>/<step>` or `not counted (<why>)`.
pub(crate) fn outcome_text(outcome: Result<(String, String), NotCounted>) -> String {
    match outcome {
        Ok((quest, step)) => format!("ticked {quest}/{step}"),
        Err(why) => format!("not counted ({})", why.why()),
    }
}

/// The engine's outcome as the line says it. A screen event that would
/// wait for a character is never handed to the engine (the worker drops it
/// as having no open step), so `Waiting` reads the same.
pub(crate) fn engine_outcome(outcome: &ScreenOutcome) -> Result<(String, String), NotCounted> {
    match outcome {
        ScreenOutcome::Ticked { quest_id, step_id } => Ok((quest_id.clone(), step_id.clone())),
        ScreenOutcome::AreaUnknown => Err(NotCounted::AreaUnknown),
        ScreenOutcome::NoOpenStep | ScreenOutcome::Waiting => Err(NotCounted::NoOpenStep),
    }
}

/// The engine's screen event for a tracker's: the kind and the name, which
/// came from the demand.
pub(crate) fn engine_event(event: &ScreenEvent) -> qt_engine::ScreenEvent {
    match event {
        ScreenEvent::BarGone { boss, .. } => qt_engine::ScreenEvent::bar_gone(boss),
        ScreenEvent::QuestComplete { npc, .. } => qt_engine::ScreenEvent::quest_complete(npc),
        ScreenEvent::KillRemark { boss, .. } => qt_engine::ScreenEvent::kill_remark(boss),
    }
}

// ---------------------------------------------------------------------------
// The thread.

/// Runs `f` on the backend when no other reader holds it; `None`, after a
/// short sleep, when one does (a stopped reader still inside a call).
fn with_backend<R>(
    backend: &SharedBackend,
    f: impl FnOnce(&mut dyn ScreenBackend) -> R,
) -> Option<R> {
    let mut guard = match backend.try_lock() {
        Ok(g) => g,
        Err(TryLockError::Poisoned(p)) => p.into_inner(),
        Err(TryLockError::WouldBlock) => {
            thread::sleep(MIN_SLEEP);
            return None;
        }
    };
    Some(f(&mut **guard))
}

/// How a reader's run ended.
enum End {
    /// Its stop flag was set.
    Stopped,
    /// It refused for good (no English OCR) and said so.
    Refused,
}

/// What the reader last wrote about what it does.
#[derive(Debug, Clone, PartialEq)]
enum Activity {
    Unknown,
    /// Idle, for the first gate of the tick that stopped it: `pid
    /// unconfirmed`, `no game window`, `minimized`, `no demand` or `not in
    /// front`.
    Idle(&'static str),
    Reading(Vec<Region>, Vec<(String, String)>),
}

/// The reader's counts for its per-minute line.
#[derive(Default)]
struct Minute {
    frames: u64,
    /// Black copies of any region (each also a covered read).
    black_copies: u64,
    ocr_failures: u64,
    busy: Duration,
}

/// One demanded bar as [`BarWatch`] follows it.
#[derive(Clone, Copy, Debug, Default)]
struct BarSight {
    /// The bar read present since the tracker last (re)armed.
    seen: bool,
    /// A covered read came after the bar was seen, with no present read
    /// since: the tracker is spoiled.
    hidden: bool,
    /// One of those covered reads was visible: a panel's title or a name
    /// over the bar's column, rather than the band out of sight.
    panel: bool,
    /// Reads since the first of them, none of them present.
    reads_since: u32,
}

/// Why a bar that was up cannot count its going, as the status line says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarHidden {
    /// A panel or a name over the bar's column ([`PANEL_STATUS`]).
    Panel,
    /// The band out of sight ([`OUT_OF_SIGHT_STATUS`]).
    OutOfSight,
}

/// Beside the bar trackers, what tells the person why a bar's going cannot
/// count: any covered read after the bar was up spoils it until the bar shows
/// again (`qt-screen`'s `BarTracker`), whatever covered it. The overlay and a
/// run of failed copies have lines of their own while they last; a panel, a
/// name, the game out of view or a single failed read have none, and the
/// spoiled tracker outlives all of them. Each read is classed with the
/// trackers' own [`classify_bar`]; only the classes are kept, never text.
#[derive(Clone, Debug, Default)]
struct BarWatch {
    bars: BTreeMap<String, BarSight>,
}

impl BarWatch {
    /// Forget the bosses no longer demanded, as the trackers do.
    fn follow(&mut self, bosses: &[String]) {
        self.bars.retain(|boss, _| bosses.contains(boss));
    }

    /// One read, after the trackers took it and reported `events`.
    fn observe(&mut self, bosses: &[String], obs: &Observation, events: &[ScreenEvent]) {
        if obs.region != Region::BossBar {
            return;
        }
        for boss in bosses {
            let sight = self.bars.entry(boss.clone()).or_default();
            match classify_bar(boss, obs) {
                BarClass::Present => {
                    *sight = BarSight {
                        seen: true,
                        ..BarSight::default()
                    };
                    continue;
                }
                BarClass::Covered if sight.seen => {
                    sight.hidden = true;
                    sight.panel |= obs.is_visible();
                }
                BarClass::Covered | BarClass::Absent => {}
            }
            if sight.hidden {
                sight.reads_since += 1;
            }
        }
        for event in events {
            if let ScreenEvent::BarGone { boss, .. } = event {
                // The tracker rearmed: the bar must show again first.
                self.bars.remove(boss);
            }
        }
    }

    /// Whether a seen bar has been hidden for [`GONE_RUN`] reads or more, so
    /// its going would have counted by now: by a panel if any such bar was,
    /// else out of sight.
    fn hides(&self) -> Option<BarHidden> {
        let long: Vec<&BarSight> = self
            .bars
            .values()
            .filter(|s| s.hidden && s.reads_since >= GONE_RUN)
            .collect();
        if long.iter().any(|s| s.panel) {
            Some(BarHidden::Panel)
        } else if long.is_empty() {
            None
        } else {
            Some(BarHidden::OutOfSight)
        }
    }

    /// The bosses whose bar is up: seen since the tracker last (re)armed,
    /// its going not yet reported (hidden since or not).
    fn up(&self) -> impl Iterator<Item = &String> {
        self.bars.iter().filter(|(_, s)| s.seen).map(|(boss, _)| boss)
    }
}

/// One boss marked at a death, its bar up then.
#[derive(Clone, Copy, Debug, Default)]
struct DeathMark {
    /// Absent reads since the bar's last present read. A covered read
    /// neither counts nor breaks the run: the band out of sight on the death
    /// screen or behind another window says nothing about the bar.
    absent: u32,
    /// The bar went after the death: [`GONE_RUN`] absent reads with no
    /// present one between, whether or not its tracker reported the going (a
    /// covered read spoils the tracker, which then reports nothing).
    gone: bool,
    /// Present reads since it went.
    shown: u32,
}

/// The bosses whose next going is not counted because their bar was up when
/// a death came. A mark ends when that going is reported (not counted), or,
/// when the tracker could not report it, once the bar has gone and then
/// shown again [`MIN_PRESENT`] times: the going after that is the fight's
/// own and counts. Each read is classed with the trackers' own
/// [`classify_bar`]; only counts are kept, never text.
#[derive(Clone, Debug, Default)]
struct DeathMarks {
    marks: BTreeMap<String, DeathMark>,
}

impl DeathMarks {
    /// A death: each boss in `up` has its next going not counted, a mark it
    /// already had starting over.
    fn mark<'a>(&mut self, up: impl Iterator<Item = &'a String>) {
        for boss in up {
            self.marks.insert(boss.clone(), DeathMark::default());
        }
    }

    /// Forget the bosses no longer demanded, as the trackers do.
    fn follow(&mut self, bosses: &[String]) {
        self.marks.retain(|boss, _| bosses.contains(boss));
    }

    /// Forget every mark, as fresh trackers forget the bars they saw.
    fn clear(&mut self) {
        self.marks.clear();
    }

    /// Whether `boss`'s reported going is not counted; its mark ends.
    fn take(&mut self, boss: &str) -> bool {
        self.marks.remove(boss).is_some()
    }

    /// One read, after its events were taken.
    fn observe(&mut self, obs: &Observation) {
        if obs.region != Region::BossBar {
            return;
        }
        self.marks.retain(|boss, m| {
            match classify_bar(boss, obs) {
                BarClass::Present if m.gone => {
                    m.shown += 1;
                    return m.shown < MIN_PRESENT;
                }
                BarClass::Present => m.absent = 0,
                BarClass::Absent if !m.gone => {
                    m.absent += 1;
                    m.gone = m.absent >= GONE_RUN;
                }
                BarClass::Absent | BarClass::Covered => {}
            }
            true
        });
    }
}

/// One NPC's dialogue window over a minute, for its `minute: dialogue
/// window, ...` line. Each count is of visible reads; a hidden read is a
/// frame of the per-minute line and nothing here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DialogueMinute {
    /// Visible reads of the region.
    reads: u64,
    /// Reads with the NPC's heading ([`is_heading`]).
    heading: u64,
    /// Reads with the banner in its place ([`is_banner`]).
    in_place: u64,
    /// Reads with a line matching [`BANNER_TEXT`] outside [`BANNER_AREA`].
    elsewhere: u64,
    /// Reads with the banner in its place and no heading within
    /// [`HEADING_WINDOW_MS`] before.
    late: u64,
}

/// One boss's voiced lines over a minute, for its `minute: voiced lines,
/// ...` line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RemarkMinute {
    /// Visible reads of the region.
    reads: u64,
    /// Reads with a `<prefix>: <text>` line whose text matches one of the
    /// boss's known remarks ([`remark_body`], [`fuzzy_score`]).
    matched: u64,
    /// The trackers' `kill_remark` events.
    new: u64,
}

/// Beside the dialogue and remark trackers, the positive control on their
/// routes: what each read of the dialogue window and the voiced lines held
/// for each demanded NPC or boss, by the trackers' own public predicates, so
/// a minute in which a route ticked nothing tells a heading that never
/// matched from a banner read out of its place or too long after the
/// heading, and a remark never read from one read but not new. Only counts
/// and the last heading's time are kept, never text.
#[derive(Clone, Debug, Default)]
struct RouteWatch {
    /// Per NPC, when its heading was last read, as its tracker keeps it.
    last_heading: BTreeMap<String, Millis>,
    /// This minute's counts, per NPC and per boss. They outlive the demand,
    /// so the minute an NPC's banner ticked its step still has its line.
    dialogues: BTreeMap<String, DialogueMinute>,
    remarks: BTreeMap<String, RemarkMinute>,
}

impl RouteWatch {
    /// Forget the headings of NPCs no longer demanded, as the trackers do.
    fn follow(&mut self, npcs: &[String]) {
        self.last_heading.retain(|npc, _| npcs.contains(npc));
    }

    /// Forget every heading, as fresh trackers do; the minute's counts stay.
    fn reset(&mut self) {
        self.last_heading.clear();
    }

    /// One read, after the trackers took it and reported `events`.
    fn observe(&mut self, demand: &Demand, obs: &Observation, events: &[ScreenEvent]) {
        let visible = obs.is_visible();
        match obs.region {
            Region::BossBar => {}
            Region::DialogueWindow => {
                let in_place = visible && obs.lines.iter().any(is_banner);
                let elsewhere = visible
                    && obs.lines.iter().any(|l| {
                        !BANNER_AREA.contains(&l.bbox)
                            && fuzzy_score(BANNER_TEXT, &l.text) >= MATCH_THRESHOLD
                    });
                for npc in &demand.dialogues {
                    let m = self.dialogues.entry(npc.clone()).or_default();
                    if !visible {
                        continue;
                    }
                    m.reads += 1;
                    if obs.lines.iter().any(|l| is_heading(npc, l)) {
                        m.heading += 1;
                        self.last_heading.insert(npc.clone(), obs.at);
                    }
                    if in_place {
                        m.in_place += 1;
                        let in_window = self
                            .last_heading
                            .get(npc)
                            .is_some_and(|&h| h <= obs.at && obs.at - h <= HEADING_WINDOW_MS);
                        if !in_window {
                            m.late += 1;
                        }
                    }
                    if elsewhere {
                        m.elsewhere += 1;
                    }
                }
            }
            Region::VoicedLines => {
                for want in demand.remarks.iter().filter(|r| !r.texts.is_empty()) {
                    let m = self.remarks.entry(want.boss.clone()).or_default();
                    if !visible {
                        continue;
                    }
                    m.reads += 1;
                    let said = obs
                        .lines
                        .iter()
                        .filter_map(|l| remark_body(&l.text))
                        .any(|body| {
                            want.texts
                                .iter()
                                .any(|known| fuzzy_score(known, body) >= MATCH_THRESHOLD)
                        });
                    if said {
                        m.matched += 1;
                    }
                }
            }
        }
        for event in events {
            if let ScreenEvent::KillRemark { boss, .. } = event {
                self.remarks.entry(boss.clone()).or_default().new += 1;
            }
        }
    }

    /// Whether the dialogue window was read this minute, a hidden read
    /// included.
    fn read_dialogue_window(&self) -> bool {
        !self.dialogues.is_empty()
    }

    /// The minute's lines, one per NPC or boss whose region was read (a
    /// hidden read included, which reads `0 of 0`), and the counts reset.
    fn take_lines(&mut self) -> Vec<String> {
        let dialogues = std::mem::take(&mut self.dialogues).into_iter().map(|(npc, m)| {
            format!(
                "screen: minute: dialogue window, {npc}'s heading in {} of {} reads, {} banners in place, {} elsewhere, {} after the heading window",
                m.heading, m.reads, m.in_place, m.elsewhere, m.late
            )
        });
        let remarks = std::mem::take(&mut self.remarks).into_iter().map(|(boss, m)| {
            format!(
                "screen: minute: voiced lines, kill remark for {boss} in {} of {} reads, {} new",
                m.matched, m.reads, m.new
            )
        });
        dialogues.chain(remarks).collect()
    }
}

struct Reader {
    app: Arc<App>,
    generation: u64,
    stop: Arc<AtomicBool>,
    snapshot: Arc<DemandSnapshot>,
    trackers: Trackers,
    bar_watch: BarWatch,
    routes: RouteWatch,
    activity: Activity,
    /// Whether a read of the dialogue window is of its heading strip alone.
    gate: DialogueGate,
    /// The regions the overlay covered at the last check.
    covered: BTreeSet<Region>,
    /// The client size refused for its aspect ratio, while it lasts.
    aspect: Option<(i32, i32)>,
    /// Whether the last overlap check had no overlay window to check, and
    /// said so.
    overlay_off: bool,
    /// Failed copies in a row.
    capture_failures: u32,
    /// Black copies in a row.
    black_copies: u32,
    /// The deaths count of the demand last taken.
    deaths: u64,
    /// The bosses whose next going is not counted: their bar was up when a
    /// death came.
    death_marked: DeathMarks,
    last_read: BTreeMap<Region, Millis>,
    minute: Minute,
    minute_start: u64,
    /// The status line last sent.
    status: Option<StatusLine>,
    /// The shell's clock and the wall clock at the reader's start, so an
    /// event's line can name a UTC time.
    base_ms: u64,
    base_unix_ms: i64,
}

/// The reader thread's body: runs until `stop` is set, the OCR refuses, or a
/// backend call it cannot go on without fails. A failure or a panic ends it
/// with a warning in the status line and the cause on an `error: screen
/// reader:` line (a panic's message is not written: it might hold anything).
pub(crate) fn run_reader(
    app: Arc<App>,
    backend: SharedBackend,
    generation: u64,
    stop: Arc<AtomicBool>,
) {
    let now = app.clock_ms();
    let base_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut reader = Reader {
        app,
        generation,
        stop,
        snapshot: Arc::default(),
        trackers: Trackers::new(),
        bar_watch: BarWatch::default(),
        routes: RouteWatch::default(),
        gate: DialogueGate::new(),
        activity: Activity::Unknown,
        covered: BTreeSet::new(),
        aspect: None,
        overlay_off: false,
        capture_failures: 0,
        black_copies: 0,
        deaths: 0,
        death_marked: DeathMarks::default(),
        last_read: BTreeMap::new(),
        minute: Minute::default(),
        minute_start: now,
        status: None,
        base_ms: now,
        base_unix_ms,
    };
    let ended = panic::catch_unwind(AssertUnwindSafe(|| reader.run(&backend)));
    match ended {
        Ok(Ok(End::Stopped)) | Ok(Ok(End::Refused)) => {}
        Ok(Err(e)) => reader.failed(&e.message),
        Err(_) => reader.failed("the reader thread panicked"),
    }
}

impl Reader {
    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn send(&self, msg: ScreenMsg) {
        let _ = self.app.sender().send(Inbound::Screen(self.generation, msg));
    }

    fn set_status(&mut self, status: Option<StatusLine>) {
        if self.status != status {
            self.status = status.clone();
            self.send(ScreenMsg::Status(status));
        }
    }

    /// The wall-clock time of a reading of the shell's clock.
    fn wall(&self, at: Millis) -> i64 {
        self.base_unix_ms + (at - self.base_ms as i64)
    }

    fn run(&mut self, backend: &SharedBackend) -> Result<End, BackendError> {
        // The OCR's language, once, before anything is read.
        let readiness = loop {
            if self.stopped() {
                return Ok(End::Stopped);
            }
            if let Some(r) = with_backend(backend, |b| b.ocr_readiness()) {
                break r?;
            }
        };
        match readiness {
            OcrReadiness::Ready { language } => {
                self.app.diag(&format!("screen: on (OCR {language})"));
            }
            OcrReadiness::NoEnglish => {
                self.app.diag("screen: refused: no English OCR");
                self.set_status(Some(StatusLine::warn(NO_ENGLISH_STATUS)));
                return Ok(End::Refused);
            }
        }
        while !self.stopped() {
            let started = Instant::now();
            let wait = match with_backend(backend, |b| self.tick(b)) {
                Some(r) => r?,
                None => MIN_SLEEP,
            };
            self.minute.busy += started.elapsed();
            self.minute_line();
            self.sleep(wait);
        }
        Ok(End::Stopped)
    }

    /// Sleeps `wait`, in steps of at most [`IDLE_SLEEP`], until the stop.
    fn sleep(&self, wait: Duration) {
        let until = Instant::now() + wait;
        while !self.stopped() {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            thread::sleep(left.min(IDLE_SLEEP));
        }
    }

    /// The newest demand, if the worker is not writing it right now.
    fn refresh_demand(&mut self) {
        let newest = match self.app.demand_slot().try_lock() {
            Ok(slot) => Arc::clone(&slot),
            Err(TryLockError::Poisoned(p)) => Arc::clone(&p.into_inner()),
            Err(TryLockError::WouldBlock) => return,
        };
        if !Arc::ptr_eq(&newest, &self.snapshot) {
            // A death since the last demand: each bar up now has its next
            // going not counted, whether it goes at the death or after the
            // walk back. A bar not up is not marked.
            if newest.deaths != self.deaths {
                self.deaths = newest.deaths;
                self.death_marked.mark(self.bar_watch.up());
            }
            self.trackers.set_demand(&newest.demand);
            self.bar_watch.follow(&newest.demand.bars);
            self.routes.follow(&newest.demand.dialogues);
            self.death_marked.follow(&newest.demand.bars);
            self.snapshot = newest;
        }
    }

    /// Idle, for `reason`: a line when the reason differs from the last.
    fn go_idle(&mut self, reason: &'static str) {
        if self.activity != Activity::Idle(reason) {
            self.activity = Activity::Idle(reason);
            self.app.diag(&format!("screen: idle ({reason})"));
        }
    }

    fn go_reading(&mut self, regions: &[Region]) {
        let now = Activity::Reading(regions.to_vec(), self.snapshot.steps.clone());
        if self.activity != now {
            self.app.diag(&reading_line(regions, &self.snapshot.steps));
            self.activity = now;
        }
    }

    /// Forgets every tracker's state: what follows cannot continue what was
    /// read before (the window is gone, or its shape was refused).
    fn reset_trackers(&mut self) {
        self.trackers = Trackers::for_demand(&self.snapshot.demand);
        self.bar_watch = BarWatch::default();
        self.death_marked.clear();
        self.routes.reset();
        self.gate.reset();
        self.last_read.clear();
    }

    /// The demanded regions due at `now`.
    fn due(&self, now: Millis) -> Vec<Region> {
        self.trackers
            .regions()
            .into_iter()
            .filter(|r| {
                self.last_read
                    .get(r)
                    .is_none_or(|last| now - last >= r.period_ms())
            })
            .collect()
    }

    /// How long to sleep before the next region is due.
    fn next_wait(&self) -> Duration {
        let regions = self.trackers.regions();
        if regions.is_empty() || matches!(self.activity, Activity::Idle(_)) {
            return IDLE_SLEEP;
        }
        let now = self.app.clock_ms() as Millis;
        let soonest = regions
            .iter()
            .map(|r| {
                self.last_read
                    .get(r)
                    .map_or(0, |last| last + r.period_ms() - now)
            })
            .min()
            .unwrap_or(0);
        Duration::from_millis(soonest.max(0) as u64).clamp(MIN_SLEEP, IDLE_SLEEP)
    }

    /// Feeds one read of each due region to the trackers, sending what they
    /// report.
    fn observe(&mut self, obs: Observation) {
        self.minute.frames += 1;
        let events = self.trackers.observe(&obs);
        self.bar_watch
            .observe(&self.snapshot.demand.bars, &obs, &events);
        self.routes.observe(&self.snapshot.demand, &obs, &events);
        for event in events {
            let (wall_ms, after_death) = match &event {
                ScreenEvent::BarGone {
                    boss, last_present, ..
                } => (self.wall(*last_present), self.death_marked.take(boss)),
                other => (self.wall(other.at()), false),
            };
            self.send(ScreenMsg::Seen {
                event,
                wall_ms,
                after_death,
            });
        }
        self.death_marked.observe(&obs);
    }

    /// A covered frame for each due region: the game is not in front, or is
    /// minimized, so nothing is copied, but a bar's run of absent frames
    /// cannot go on across the gap.
    fn hidden_frames(&mut self, why: Hidden) {
        let now = self.app.clock_ms() as Millis;
        for region in self.due(now) {
            self.last_read.insert(region, now);
            self.observe(Observation::hidden(region, now, why));
        }
    }

    /// Writes a line for each region the overlay newly covers or uncovers.
    fn note_covered(&mut self, covered: BTreeSet<Region>) {
        for r in covered.difference(&self.covered) {
            self.app
                .diag(&format!("screen: covered: {} by the overlay", r.name()));
        }
        for r in self.covered.difference(&covered) {
            self.app.diag(&format!("screen: uncovered: {}", r.name()));
        }
        self.covered = covered;
    }

    /// The status line for what holds now: a refused shape, failing copies,
    /// black copies, the overlay over a region, or a bar that was up hidden
    /// since, by a panel or out of sight.
    fn current_status(&self) -> Option<StatusLine> {
        Self::status_for(
            self.aspect,
            self.capture_failures,
            self.black_copies,
            &self.covered,
            &self.bar_watch,
        )
    }

    fn status_for(
        aspect: Option<(i32, i32)>,
        capture_failures: u32,
        black_copies: u32,
        covered: &BTreeSet<Region>,
        bar_watch: &BarWatch,
    ) -> Option<StatusLine> {
        if let Some((w, h)) = aspect {
            Some(StatusLine::warn(format!(
                "Screen reading is paused while the game's window is not 16:9 ({w}x{h})."
            )))
        } else if capture_failures >= CAPTURE_FAILURE_LIMIT {
            Some(StatusLine::warn(format!(
                "Screen reading cannot copy the game's window (the capture failed {CAPTURE_FAILURE_LIMIT} times in a row)."
            )))
        } else if black_copies >= CAPTURE_FAILURE_LIMIT {
            Some(StatusLine::warn(format!(
                "Screen reading sees the game's window copied black ({CAPTURE_FAILURE_LIMIT} times in a row); a dark scene or a loading screen looks the same."
            )))
        } else if !covered.is_empty() {
            Some(covered_status(covered))
        } else {
            match bar_watch.hides()? {
                BarHidden::Panel => Some(StatusLine::warn(PANEL_STATUS)),
                BarHidden::OutOfSight => Some(StatusLine::warn(OUT_OF_SIGHT_STATUS)),
            }
        }
    }

    /// One tick: the steps of Context "Design: the reader thread" › "A
    /// tick". Returns how long to sleep before the next.
    fn tick(&mut self, backend: &mut dyn ScreenBackend) -> Result<Duration, BackendError> {
        self.refresh_demand();
        self.tick_inner(backend)?;
        let status = self.current_status();
        self.set_status(status);
        Ok(self.next_wait())
    }

    fn tick_inner(&mut self, backend: &mut dyn ScreenBackend) -> Result<(), BackendError> {
        // The game's window, by the confirmed pid.
        let Some(pid) = self.snapshot.pid else {
            self.go_idle("pid unconfirmed");
            return Ok(());
        };
        let Some(window) = backend.game_window(pid)? else {
            self.reset_trackers();
            self.go_idle("no game window");
            return Ok(());
        };
        let Some(client) = backend.client_rect(window)? else {
            self.hidden_frames(Hidden::Minimized);
            self.go_idle("minimized");
            return Ok(());
        };
        // Its shape: positions were measured at 16:9 only.
        if !aspect_is_16_9(client.width(), client.height()) {
            let size = (client.width(), client.height());
            if self.aspect != Some(size) {
                self.app.diag(&format!(
                    "screen: refused: the game window is not 16:9 ({}x{})",
                    size.0, size.1
                ));
                self.aspect = Some(size);
                self.reset_trackers();
            }
            self.activity = Activity::Unknown;
            return Ok(());
        }
        self.aspect = None;
        // The overlay over a region, demand or not. With no overlay window
        // to check, nothing counts as covered and the copy holds whatever
        // is in front, the overlay included: say so once, until it is back.
        let overlay = backend.overlay()?;
        match (overlay.is_some(), self.overlay_off) {
            (false, false) => {
                self.app
                    .diag("screen: overlay check off (no overlay window)");
                self.overlay_off = true;
            }
            (true, true) => {
                self.app.diag("screen: overlay check on");
                self.overlay_off = false;
            }
            _ => {}
        }
        let covered: BTreeSet<Region> = Region::ALL
            .into_iter()
            .filter(|r| overlay.is_some_and(|o| o.visible && r.covered_by(client, o.rect)))
            .collect();
        self.note_covered(covered);
        let regions = self.trackers.regions();
        if regions.is_empty() {
            self.go_idle("no demand");
            return Ok(());
        }
        if backend.foreground_pid()? != Some(pid) {
            self.hidden_frames(Hidden::NotForeground);
            self.go_idle("not in front");
            return Ok(());
        }
        self.go_reading(&regions);
        for region in self.due(self.app.clock_ms() as Millis) {
            if self.stopped() {
                break;
            }
            let at = self.app.clock_ms() as Millis;
            self.last_read.insert(region, at);
            if self.covered.contains(&region) {
                self.observe(Observation::hidden(region, at, Hidden::Overlay));
            } else if region == Region::DialogueWindow {
                for obs in self.read_dialogue(backend, window, client, at) {
                    self.observe(obs);
                }
            } else {
                let obs = self.read(backend, window, client, region, region.pixels(client), at);
                self.observe(obs);
            }
        }
        Ok(())
    }

    /// Reads the dialogue window through its gate: the heading strip alone
    /// while the gate is shut, and the whole region while it is open or once
    /// that strip read opens it, at once. Returns the reads in order, each
    /// one an observation of the region for the trackers.
    fn read_dialogue(
        &mut self,
        backend: &mut dyn ScreenBackend,
        window: qt_screen::WindowHandle,
        client: PixelRect,
        at: Millis,
    ) -> Vec<Observation> {
        let region = Region::DialogueWindow;
        let npcs = self.snapshot.demand.dialogues.clone();
        let mut reads = Vec::new();
        if !self.gate.is_open(at) {
            let strip_area = HEADING_STRIP.to_pixels(client);
            let strip = self.read(backend, window, client, region, strip_area, at);
            let open = self.gate.observe(&npcs, &strip, DialogueRead::Strip);
            reads.push(strip);
            if !open {
                return reads;
            }
        }
        let at = self.app.clock_ms() as Millis;
        let full = self.read(backend, window, client, region, region.pixels(client), at);
        self.gate.observe(&npcs, &full, DialogueRead::Full);
        reads.push(full);
        reads
    }

    /// Copies and reads `area` (screen pixels) of `region`: the region, or
    /// the dialogue window's heading strip. A failed or black copy, or a
    /// failed OCR call, is a covered frame. Failed and black copies run
    /// apart: each run ends at a copy of the other kind or a good copy.
    fn read(
        &mut self,
        backend: &mut dyn ScreenBackend,
        window: qt_screen::WindowHandle,
        client: PixelRect,
        region: Region,
        area: PixelRect,
        at: Millis,
    ) -> Observation {
        let image = match backend.capture(window, area) {
            Ok(image) if image.is_black() => {
                self.end_failures();
                self.copied_black();
                return Observation::hidden(region, at, Hidden::Black);
            }
            Ok(image) => image,
            Err(e) => {
                self.black_copies = 0;
                self.capture_failed(e);
                return Observation::hidden(region, at, Hidden::CaptureFailed);
            }
        };
        self.end_failures();
        self.black_copies = 0;
        match backend.ocr(&image) {
            Ok(lines) => Observation::from_ocr(region, at, client, area, lines),
            Err(_) => {
                self.minute.ocr_failures += 1;
                Observation::hidden(region, at, Hidden::OcrFailed)
            }
        }
    }

    /// Counts a failed copy; the tenth in a row writes its line, with the
    /// failure's detail.
    fn capture_failed(&mut self, error: BackendError) {
        self.capture_failures += 1;
        if self.capture_failures == CAPTURE_FAILURE_LIMIT {
            self.app.diag(&format!(
                "screen: refused: the capture failed {CAPTURE_FAILURE_LIMIT} times in a row"
            ));
            self.app.diag(&format!("error: screen reader: {error}"));
        }
    }

    /// Ends a run of failed copies: a copy came back.
    fn end_failures(&mut self) {
        if self.capture_failures >= CAPTURE_FAILURE_LIMIT {
            // Copies work again: say what is read, as after any refusal.
            self.activity = Activity::Unknown;
        }
        self.capture_failures = 0;
    }

    /// Counts a black copy; the tenth in a row writes its line. It is not a
    /// failure: a dark scene or a loading screen copies the same.
    fn copied_black(&mut self) {
        self.minute.black_copies += 1;
        self.black_copies += 1;
        if self.black_copies == CAPTURE_FAILURE_LIMIT {
            self.app.diag(&format!(
                "screen: black: {CAPTURE_FAILURE_LIMIT} copies in a row"
            ));
        }
    }

    /// Writes the per-minute line when a minute has passed with any read,
    /// then the dialogue window's and the voiced lines' lines, then the
    /// dialogue window gate's when the window was read that minute.
    fn minute_line(&mut self) {
        let now = self.app.clock_ms();
        if now.saturating_sub(self.minute_start) < MINUTE_MS {
            return;
        }
        let counts = self.trackers.take_counts();
        let window_read = self.routes.read_dialogue_window();
        let mut routes = self.routes.take_lines();
        let gate = self.gate.take_counts();
        if window_read || gate != GateCounts::default() {
            routes.push(format!(
                "screen: minute: dialogue window gate, {} full reads, {} skipped",
                gate.full, gate.skipped
            ));
        }
        let m = std::mem::take(&mut self.minute);
        self.minute_start = now;
        if m.frames == 0 {
            return;
        }
        self.app.diag(&format!(
            "screen: minute: {} frames, {} present, {} absent, {} covered, {} black copies, {} OCR failures, busy {} ms",
            m.frames,
            counts.present,
            counts.absent,
            counts.covered,
            m.black_copies,
            m.ocr_failures,
            m.busy.as_millis()
        ));
        for line in routes {
            self.app.diag(&line);
        }
    }

    /// The thread ends after an error: the cause, the refusal, the warning.
    fn failed(&mut self, cause: &str) {
        self.app.diag(&format!("error: screen reader: {cause}"));
        self.app.diag("screen: refused: stopped after an error");
        self.set_status(Some(StatusLine::warn(STOPPED_STATUS)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_clock_is_hours_minutes_seconds_millis() {
        assert_eq!(utc_clock(0), "00:00:00.000");
        assert_eq!(utc_clock(86_400_000 + 3_723_045), "01:02:03.045");
        assert_eq!(utc_clock(-1), "23:59:59.999");
    }

    #[test]
    fn lines_name_regions_and_steps() {
        assert_eq!(region_list(&[Region::BossBar]), "boss bar");
        assert_eq!(
            region_list(&[Region::BossBar, Region::DialogueWindow]),
            "boss bar and dialogue window"
        );
        assert_eq!(
            region_list(&Region::ALL),
            "boss bar, voiced lines and dialogue window"
        );
        assert_eq!(
            reading_line(
                &[Region::BossBar],
                &[("q".into(), "fight".into()), ("r".into(), "s".into())]
            ),
            "screen: reading boss bar for q/fight, r/s"
        );
        let one: BTreeSet<Region> = [Region::BossBar].into();
        assert_eq!(
            covered_status(&one).text,
            "The overlay covers the boss bar; move it to let screen reading see it."
        );
        let two: BTreeSet<Region> = [Region::BossBar, Region::DialogueWindow].into();
        assert_eq!(
            covered_status(&two).text,
            "The overlay covers the boss bar and the dialogue window; move it to let screen reading see them."
        );
        let all: BTreeSet<Region> = Region::ALL.into();
        assert_eq!(
            covered_status(&all).text,
            "The overlay covers the boss bar, the voiced lines and the dialogue window; move it to let screen reading see them."
        );
    }

    #[test]
    fn outcomes_read_as_the_context_pins_them() {
        let bar = ScreenEvent::BarGone {
            boss: "Beira of the Rotten Pack".into(),
            last_present: 0,
            at: 3_000,
        };
        assert_eq!(
            seen_head(&bar, 45_296_789),
            "screen: bar of Beira of the Rotten Pack gone (last seen 12:34:56.789Z)"
        );
        let done = ScreenEvent::QuestComplete {
            npc: "Renly".into(),
            at: 0,
        };
        assert_eq!(
            seen_head(&done, 0),
            "screen: quest complete in Renly's window (00:00:00.000Z)"
        );
        let remark = ScreenEvent::KillRemark {
            boss: "The Bloated Miller".into(),
            at: 0,
        };
        assert_eq!(
            seen_head(&remark, 0),
            "screen: kill remark for The Bloated Miller (00:00:00.000Z)"
        );
        assert_eq!(
            outcome_text(Ok(("q".into(), "fight".into()))),
            "ticked q/fight"
        );
        for (why, text) in [
            (NotCounted::AfterDeath, "not counted (after a death)"),
            (NotCounted::AfterAreaChange, "not counted (after an area change)"),
            (NotCounted::AreaUnknown, "not counted (area unknown)"),
            (NotCounted::NoOpenStep, "not counted (no open step)"),
        ] {
            assert_eq!(outcome_text(Err(why)), text);
        }
        assert_eq!(
            engine_outcome(&ScreenOutcome::Waiting),
            Err(NotCounted::NoOpenStep)
        );
    }

    const BEIRA: &str = "Beira of the Rotten Pack";

    fn bar_read(at: Millis, lines: Vec<qt_screen::Line>) -> Observation {
        Observation::visible(Region::BossBar, at, lines)
    }
    fn beira_name() -> qt_screen::Line {
        qt_screen::Line::new(
            "BEIRA OF THE ROTTEN PACK",
            qt_screen::Rect::new(0.456, 0.047, 0.544, 0.06),
        )
    }
    /// A panel's title over the name's column (the passive tree's, say).
    fn panel_title() -> qt_screen::Line {
        qt_screen::Line::new(
            "PASSIVE SKILL TREE",
            qt_screen::Rect::new(
                qt_screen::NAME_COLUMN_LEFT,
                qt_screen::NAME_COLUMN_MAX_TOP / 2.0,
                qt_screen::NAME_COLUMN_RIGHT,
                qt_screen::NAME_COLUMN_MAX_TOP,
            ),
        )
    }

    /// Feeds `reads` to a watch over Beira's bar and to her tracker, as the
    /// reader does, and returns the watch and how many `bar_gone` it fired.
    fn watch(reads: &[Observation]) -> (BarWatch, usize) {
        let bosses = vec![BEIRA.to_string()];
        let mut tracker = qt_screen::BarTracker::new(BEIRA);
        let mut w = BarWatch::default();
        let mut fired = 0;
        for obs in reads {
            let events: Vec<ScreenEvent> = tracker.observe(obs).1.into_iter().collect();
            fired += events.len();
            w.observe(&bosses, obs, &events);
        }
        (w, fired)
    }

    fn status_of(w: &BarWatch) -> Option<String> {
        Reader::status_for(None, 0, 0, &BTreeSet::new(), w).map(|s| s.text)
    }

    /// Beira's bar up for two reads, then `cover`, then absent reads until
    /// her going would have counted had nothing covered it.
    fn covered_then_absent(cover: Observation) -> Vec<Observation> {
        let mut reads = vec![
            bar_read(0, vec![beira_name()]),
            bar_read(500, vec![beira_name()]),
        ];
        reads.push(cover);
        for n in 0..qt_screen::GONE_RUN as i64 {
            reads.push(bar_read(1_500 + 500 * n, Vec::new()));
        }
        reads
    }

    #[test]
    fn a_panel_over_a_seen_bar_sets_the_status_once_its_going_could_not_count() {
        let mut reads = vec![bar_read(0, vec![beira_name()]), bar_read(500, vec![beira_name()])];
        let mut at = 1_000;
        // The passive tree opens over the bar; she dies behind it.
        reads.push(bar_read(at, vec![panel_title()]));
        for _ in 2..qt_screen::GONE_RUN {
            at += 500;
            reads.push(bar_read(at, Vec::new()));
        }
        assert_eq!(watch(&reads).0.hides(), None, "not before GONE_RUN reads");
        at += 500;
        reads.push(bar_read(at, Vec::new()));
        let (w, fired) = watch(&reads);
        assert_eq!(fired, 0, "the covered read spoiled the tracker");
        assert_eq!(w.hides(), Some(BarHidden::Panel));
        assert_eq!(status_of(&w), Some(PANEL_STATUS.to_string()));
        // The overlay's own line comes first.
        let over: BTreeSet<Region> = [Region::BossBar].into();
        assert_eq!(
            Reader::status_for(None, 0, 0, &over, &w).map(|s| s.text),
            Some(covered_status(&over).text)
        );
        // Her bar back clears it.
        at += 500;
        reads.push(bar_read(at, vec![beira_name()]));
        assert_eq!(status_of(&watch(&reads).0), None);
    }

    #[test]
    fn a_seen_bar_out_of_sight_then_absent_sets_the_out_of_sight_status() {
        // Each way the band can be out of sight for one read spoils the
        // tracker as a panel does, and the line says so, not nothing.
        for why in [
            Hidden::NotForeground,
            Hidden::Minimized,
            Hidden::Overlay,
            Hidden::CaptureFailed,
            Hidden::Black,
            Hidden::OcrFailed,
        ] {
            let mut reads = covered_then_absent(Observation::hidden(Region::BossBar, 1_000, why));
            let (w, fired) = watch(&reads);
            assert_eq!(fired, 0, "{why:?}: the tracker is spoiled");
            assert_eq!(w.hides(), Some(BarHidden::OutOfSight), "{why:?}");
            assert_eq!(
                status_of(&w),
                Some(OUT_OF_SIGHT_STATUS.to_string()),
                "{why:?}"
            );
            // The next present read clears it.
            reads.push(bar_read(10_000, vec![beira_name()]));
            assert_eq!(status_of(&watch(&reads).0), None, "{why:?}");
        }
        // The negative control: the same reads with an absent one in the
        // cover's place fire `bar_gone` and leave no line.
        let (w, fired) = watch(&covered_then_absent(bar_read(1_000, Vec::new())));
        assert_eq!(fired, 1);
        assert_eq!(status_of(&w), None);
        // A panel among the covers names the panel.
        let mut both = covered_then_absent(bar_read(1_000, vec![panel_title()]));
        both.insert(
            3,
            Observation::hidden(Region::BossBar, 1_200, Hidden::NotForeground),
        );
        assert_eq!(watch(&both).0.hides(), Some(BarHidden::Panel));
    }

    #[test]
    fn a_bar_never_seen_or_gone_in_plain_sight_sets_no_status() {
        // A panel, or the band out of sight, before the bar ever showed
        // spoils nothing.
        let early: Vec<Observation> = (0..2 * qt_screen::GONE_RUN as i64)
            .map(|n| {
                if n % 2 == 0 {
                    bar_read(500 * n, vec![panel_title()])
                } else {
                    Observation::hidden(Region::BossBar, 500 * n, Hidden::NotForeground)
                }
            })
            .collect();
        assert_eq!(status_of(&watch(&early).0), None);
        // A bar that went in plain sight leaves nothing behind.
        let mut gone = vec![bar_read(0, vec![beira_name()]), bar_read(500, vec![beira_name()])];
        for n in 0..2 * qt_screen::GONE_RUN as i64 {
            gone.push(bar_read(1_000 + 500 * n, Vec::new()));
        }
        assert_eq!(status_of(&watch(&gone).0), None);
        // A boss no longer demanded is forgotten.
        let mut w = watch(&{
            let mut r = vec![bar_read(0, vec![beira_name()]), bar_read(500, vec![beira_name()])];
            for n in 0..qt_screen::GONE_RUN as i64 {
                r.push(bar_read(1_000 + 500 * n, vec![panel_title()]));
            }
            r
        })
        .0;
        assert_eq!(w.hides(), Some(BarHidden::Panel));
        w.follow(&[]);
        assert_eq!(w.hides(), None);
    }

    /// Feeds `before` to Beira's tracker and watch, comes to a death as the
    /// reader does when the demand's deaths change, then feeds `after`.
    /// Returns, for each `bar_gone` after the death, whether it counted.
    fn through_a_death(before: &[Observation], after: &[Observation]) -> Vec<bool> {
        let bosses = vec![BEIRA.to_string()];
        let mut tracker = qt_screen::BarTracker::new(BEIRA);
        let mut w = BarWatch::default();
        let mut marks = DeathMarks::default();
        let mut counted = Vec::new();
        let mut feed = |obs: &Observation, w: &mut BarWatch, marks: &mut DeathMarks| {
            let events: Vec<ScreenEvent> = tracker.observe(obs).1.into_iter().collect();
            w.observe(&bosses, obs, &events);
            let gone: Vec<bool> = events
                .iter()
                .filter_map(|e| match e {
                    ScreenEvent::BarGone { boss, .. } => Some(!marks.take(boss)),
                    _ => None,
                })
                .collect();
            marks.observe(obs);
            gone
        };
        for obs in before {
            assert!(
                feed(obs, &mut w, &mut marks).is_empty(),
                "no going before the death"
            );
        }
        marks.mark(w.up());
        for obs in after {
            counted.extend(feed(obs, &mut w, &mut marks));
        }
        counted
    }

    /// Reads from `at`, 500 ms apart: `n` of Beira's bar present, or absent.
    fn bar_run(at: &mut Millis, n: u32, present: bool) -> Vec<Observation> {
        (0..n)
            .map(|_| {
                *at += 500;
                let lines = if present {
                    vec![beira_name()]
                } else {
                    Vec::new()
                };
                bar_read(*at, lines)
            })
            .collect()
    }

    fn hidden_run(at: &mut Millis, n: u32, why: Hidden) -> Vec<Observation> {
        (0..n)
            .map(|_| {
                *at += 500;
                Observation::hidden(Region::BossBar, *at, why)
            })
            .collect()
    }

    #[test]
    fn a_death_mark_ends_when_the_bar_shows_again_after_a_going_its_tracker_could_not_report() {
        let mut at = 0;
        let up = bar_run(&mut at, 2, true);
        // Death with her bar up; the person alt-tabs on the death screen,
        // respawns and walks back, her bar out of sight all the while. The
        // covered reads spoil the tracker, so that going reports nothing.
        let mut after = hidden_run(&mut at, 3, Hidden::NotForeground);
        after.extend(hidden_run(&mut at, 2, Hidden::Black));
        after.extend(bar_run(&mut at, GONE_RUN, false));
        // Back at the fight: her bar shows again, and she dies.
        after.extend(bar_run(&mut at, MIN_PRESENT, true));
        after.extend(bar_run(&mut at, GONE_RUN, false));
        assert_eq!(through_a_death(&up, &after), vec![true], "the kill counts");

        // A bar that flickered once before the death: the same.
        let mut at = 0;
        let flicker = bar_run(&mut at, 1, true);
        let mut after = bar_run(&mut at, GONE_RUN, false);
        after.extend(bar_run(&mut at, MIN_PRESENT, true));
        after.extend(bar_run(&mut at, GONE_RUN, false));
        assert_eq!(through_a_death(&flicker, &after), vec![true]);
    }

    #[test]
    fn a_death_mark_outlives_cover_a_flicker_and_a_single_showing() {
        // The negative controls: a going after each of these is the one the
        // death was for, and does not count.
        let mut at = 0;
        let up = bar_run(&mut at, 2, true);

        // Her bar stays up through the death screen and goes on the walk.
        let mut a = at;
        let mut after = bar_run(&mut a, 2, true);
        after.extend(bar_run(&mut a, GONE_RUN, false));
        // Then she is fought again: that going counts.
        after.extend(bar_run(&mut a, MIN_PRESENT, true));
        after.extend(bar_run(&mut a, GONE_RUN, false));
        assert_eq!(through_a_death(&up, &after), vec![false, true]);

        // The death screen covers the band, her bar shows again on it, and
        // goes on the walk: cover alone is no going.
        let mut a = at;
        let mut after = hidden_run(&mut a, 4, Hidden::Black);
        after.extend(bar_run(&mut a, MIN_PRESENT, true));
        after.extend(bar_run(&mut a, GONE_RUN, false));
        assert_eq!(through_a_death(&up, &after), vec![false]);

        // One read missing her bar is no going either.
        let mut a = at;
        let mut after = bar_run(&mut a, 1, false);
        after.extend(bar_run(&mut a, MIN_PRESENT, true));
        after.extend(bar_run(&mut a, GONE_RUN, false));
        assert_eq!(through_a_death(&up, &after), vec![false]);

        // Out of sight, gone, then one present read: the tracker's present
        // count carries over and reports the next going, which does not
        // count until the bar has shown again the tracker's minimum.
        let mut a = at;
        let mut after = hidden_run(&mut a, 1, Hidden::NotForeground);
        after.extend(bar_run(&mut a, GONE_RUN, false));
        after.extend(bar_run(&mut a, MIN_PRESENT - 1, true));
        after.extend(bar_run(&mut a, GONE_RUN, false));
        after.extend(bar_run(&mut a, MIN_PRESENT, true));
        after.extend(bar_run(&mut a, GONE_RUN, false));
        assert_eq!(through_a_death(&up, &after), vec![false, true]);

        // A bar not up at the death is not marked.
        let mut a = 0;
        let mut after = bar_run(&mut a, MIN_PRESENT, true);
        after.extend(bar_run(&mut a, GONE_RUN, false));
        assert_eq!(through_a_death(&[], &after), vec![true]);
    }

    #[test]
    fn death_marks_follow_the_demand_and_clear_with_the_trackers() {
        let beira = vec![BEIRA.to_string()];
        let mut marks = DeathMarks::default();
        marks.mark(beira.iter());
        marks.follow(&beira);
        assert!(marks.take(BEIRA), "still demanded");
        marks.mark(beira.iter());
        marks.follow(&[]);
        assert!(!marks.take(BEIRA), "no longer demanded");
        marks.mark(beira.iter());
        marks.clear();
        assert!(!marks.take(BEIRA), "the trackers were reset");
    }

    const RENLY: &str = "Renly";
    // Invented remark text and character name.
    const REMARK: &str = "That one won't be grinding any more.";

    fn dialogue_read(at: Millis, lines: Vec<qt_screen::Line>) -> Observation {
        Observation::visible(Region::DialogueWindow, at, lines)
    }
    fn renly_heading() -> qt_screen::Line {
        qt_screen::Line::new("RENLY", qt_screen::Rect::new(0.45, 0.20, 0.55, 0.23))
    }
    /// The banner, in its place or above it.
    fn banner(in_place: bool) -> qt_screen::Line {
        let a = qt_screen::BANNER_AREA;
        let (top, bottom) = if in_place {
            (a.top + 0.01, a.top + 0.03)
        } else {
            (a.top - 0.2, a.top - 0.18)
        };
        qt_screen::Line::new(
            BANNER_TEXT,
            qt_screen::Rect::new(a.left + 0.05, top, a.right - 0.05, bottom),
        )
    }
    fn remark_line(text: &str) -> qt_screen::Line {
        qt_screen::Line::new(
            format!("Tavi: {text}"),
            qt_screen::Rect::new(0.02, 0.70, 0.30, 0.72),
        )
    }
    fn route_demand() -> Demand {
        Demand {
            dialogues: vec![RENLY.into()],
            remarks: vec![RemarkDemand {
                boss: BEIRA.into(),
                texts: vec![REMARK.into()],
            }],
            ..Demand::default()
        }
    }

    #[test]
    fn the_dialogue_route_line_tells_each_way_a_banner_can_miss() {
        let demand = route_demand();
        let mut w = RouteWatch::default();
        let late = qt_screen::HEADING_WINDOW_MS + 1_000;
        let reads = [
            // Renly's window, then its banner in place: the tracker fires.
            dialogue_read(0, vec![renly_heading()]),
            dialogue_read(1_000, vec![banner(true)]),
            // The banner's text out of its place.
            dialogue_read(2_000, vec![banner(false)]),
            // A banner in place too long after the heading.
            dialogue_read(1_000 + late, vec![banner(true)]),
            // A hidden read is no read.
            Observation::hidden(Region::DialogueWindow, 2_000 + late, Hidden::Overlay),
        ];
        let mut tracker = qt_screen::DialogueTracker::new(RENLY);
        let mut fired = 0;
        for obs in &reads {
            let events: Vec<ScreenEvent> = tracker.observe(obs).into_iter().collect();
            fired += events.len();
            w.observe(&demand, obs, &events);
        }
        assert_eq!(fired, 1);
        // Renly is no longer demanded once his step ticked: his line stays.
        w.follow(&[]);
        assert_eq!(
            w.take_lines(),
            vec![
                "screen: minute: dialogue window, Renly's heading in 1 of 4 reads, 2 banners in place, 1 elsewhere, 1 after the heading window"
                    .to_string()
            ]
        );
        assert!(w.take_lines().is_empty(), "the counts reset");
    }

    #[test]
    fn the_dialogue_route_forgets_a_heading_on_reset_but_keeps_the_minute() {
        let demand = route_demand();
        let mut w = RouteWatch::default();
        w.observe(&demand, &dialogue_read(0, vec![renly_heading()]), &[]);
        w.reset();
        w.observe(&demand, &dialogue_read(1_000, vec![banner(true)]), &[]);
        assert_eq!(
            w.take_lines(),
            vec![
                "screen: minute: dialogue window, Renly's heading in 1 of 2 reads, 1 banners in place, 0 elsewhere, 1 after the heading window"
                    .to_string()
            ]
        );
    }

    #[test]
    fn the_voiced_lines_route_line_counts_reads_with_the_remark_and_new_ones() {
        let demand = route_demand();
        let mut w = RouteWatch::default();
        let said = Observation::visible(Region::VoicedLines, 0, vec![remark_line(REMARK)]);
        let event = ScreenEvent::KillRemark {
            boss: BEIRA.into(),
            at: 0,
        };
        w.observe(&demand, &said, std::slice::from_ref(&event));
        let other = Observation::visible(Region::VoicedLines, 500, vec![remark_line("Onward.")]);
        w.observe(&demand, &other, &[]);
        // Text with no speaker prefix is not a remark.
        let bare = Observation::visible(
            Region::VoicedLines,
            1_000,
            vec![qt_screen::Line::new(REMARK, qt_screen::Rect::new(0.02, 0.70, 0.30, 0.72))],
        );
        w.observe(&demand, &bare, &[]);
        // Reads of other regions leave it alone.
        w.observe(&demand, &bar_read(1_500, vec![beira_name()]), &[]);
        assert_eq!(
            w.take_lines(),
            vec![
                "screen: minute: voiced lines, kill remark for Beira of the Rotten Pack in 1 of 3 reads, 1 new"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_route_read_only_while_hidden_reads_zero_of_zero() {
        let demand = route_demand();
        let mut w = RouteWatch::default();
        w.observe(
            &demand,
            &Observation::hidden(Region::VoicedLines, 0, Hidden::NotForeground),
            &[],
        );
        assert_eq!(
            w.take_lines(),
            vec![
                "screen: minute: voiced lines, kill remark for Beira of the Rotten Pack in 0 of 0 reads, 0 new"
                    .to_string()
            ]
        );
        // Nothing demanded, nothing read: no line.
        w.observe(&Demand::default(), &dialogue_read(0, vec![renly_heading()]), &[]);
        assert!(w.take_lines().is_empty());
    }
}
