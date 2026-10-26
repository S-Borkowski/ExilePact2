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
//!   window missing, minimized or not in front.
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
//!   its own `error: screen reader: ` line.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use qt_engine::{Engine, ScreenOutcome, ScreenWhat};
use qt_screen::{
    aspect_is_16_9, BackendError, Demand, Hidden, Millis, Observation, OcrReadiness, PixelRect,
    Region, RemarkDemand, ScreenBackend, ScreenEvent, Trackers,
};
use qt_viewmodel::StatusLine;

use crate::app::{App, Inbound};

/// The longest an idle reader sleeps before it looks at its stop flag and
/// the demand again.
pub const IDLE_SLEEP: Duration = Duration::from_millis(100);

/// The shortest sleep between two ticks.
const MIN_SLEEP: Duration = Duration::from_millis(5);

/// How many failed or black copies in a row set a status line.
pub const CAPTURE_FAILURE_LIMIT: u32 = 10;

/// How often the reader writes its per-minute line, on the shell's clock.
pub const MINUTE_MS: u64 = 60_000;

/// The status line when Windows has no English OCR.
pub const NO_ENGLISH_STATUS: &str = "Screen reading is off: Windows has no English OCR.";

/// The status line when the reader ended without being stopped.
pub const STOPPED_STATUS: &str =
    "Screen reading stopped after an error; turn it off and on to retry.";

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
}

impl DemandSnapshot {
    /// The engine's screen demand as the reader takes it: each boss's bar,
    /// each NPC's window, and each boss's kill remark that has a text, with
    /// the steps they would tick. A `kill_remark` with no text reads nothing
    /// and names no step.
    pub fn of(engine: &Engine, pid: Option<u32>) -> DemandSnapshot {
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
        DemandSnapshot { demand, steps, pid }
    }
}

/// What the reader sends the worker, tagged with its generation.
#[derive(Debug)]
pub(crate) enum ScreenMsg {
    /// A tracker's event, with the wall-clock time (Unix milliseconds) its
    /// line names: when the bar was last seen, or when the event was seen.
    Seen { event: ScreenEvent, wall_ms: i64 },
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
    Idle,
    Reading(Vec<Region>, Vec<(String, String)>),
}

/// The reader's counts for its per-minute line.
#[derive(Default)]
struct Minute {
    frames: u64,
    ocr_failures: u64,
    busy: Duration,
}

struct Reader {
    app: Arc<App>,
    generation: u64,
    stop: Arc<AtomicBool>,
    snapshot: Arc<DemandSnapshot>,
    trackers: Trackers,
    activity: Activity,
    /// The regions the overlay covered at the last check.
    covered: BTreeSet<Region>,
    /// The client size refused for its aspect ratio, while it lasts.
    aspect: Option<(i32, i32)>,
    /// Failed or black copies in a row.
    capture_failures: u32,
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
        activity: Activity::Unknown,
        covered: BTreeSet::new(),
        aspect: None,
        capture_failures: 0,
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
            self.trackers.set_demand(&newest.demand);
            self.snapshot = newest;
        }
    }

    fn go_idle(&mut self) {
        if self.activity != Activity::Idle {
            self.activity = Activity::Idle;
            self.app.diag("screen: idle");
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
        if regions.is_empty() || self.activity == Activity::Idle {
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
        for event in self.trackers.observe(&obs) {
            let wall_ms = self.wall(match &event {
                ScreenEvent::BarGone { last_present, .. } => *last_present,
                other => other.at(),
            });
            self.send(ScreenMsg::Seen { event, wall_ms });
        }
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
    /// or the overlay over a region.
    fn current_status(&self) -> Option<StatusLine> {
        if let Some((w, h)) = self.aspect {
            Some(StatusLine::warn(format!(
                "Screen reading is paused while the game's window is not 16:9 ({w}x{h})."
            )))
        } else if self.capture_failures >= CAPTURE_FAILURE_LIMIT {
            Some(StatusLine::warn(format!(
                "Screen reading cannot copy the game's window (the capture failed {CAPTURE_FAILURE_LIMIT} times in a row)."
            )))
        } else if !self.covered.is_empty() {
            Some(covered_status(&self.covered))
        } else {
            None
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
            self.go_idle();
            return Ok(());
        };
        let Some(window) = backend.game_window(pid)? else {
            self.reset_trackers();
            self.go_idle();
            return Ok(());
        };
        let Some(client) = backend.client_rect(window)? else {
            self.hidden_frames(Hidden::Minimized);
            self.go_idle();
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
        // The overlay over a region, demand or not.
        let overlay = backend.overlay()?;
        let covered: BTreeSet<Region> = Region::ALL
            .into_iter()
            .filter(|r| overlay.is_some_and(|o| o.visible && r.covered_by(client, o.rect)))
            .collect();
        self.note_covered(covered);
        let regions = self.trackers.regions();
        if regions.is_empty() {
            self.go_idle();
            return Ok(());
        }
        if backend.foreground_pid()? != Some(pid) {
            self.hidden_frames(Hidden::NotForeground);
            self.go_idle();
            return Ok(());
        }
        self.go_reading(&regions);
        for region in self.due(self.app.clock_ms() as Millis) {
            if self.stopped() {
                break;
            }
            let at = self.app.clock_ms() as Millis;
            self.last_read.insert(region, at);
            let obs = if self.covered.contains(&region) {
                Observation::hidden(region, at, Hidden::Overlay)
            } else {
                self.read(backend, window, client, region, at)
            };
            self.observe(obs);
        }
        Ok(())
    }

    /// Copies and reads one region. A failed or black copy, or a failed OCR
    /// call, is a covered frame.
    fn read(
        &mut self,
        backend: &mut dyn ScreenBackend,
        window: qt_screen::WindowHandle,
        client: PixelRect,
        region: Region,
        at: Millis,
    ) -> Observation {
        let area = region.pixels(client);
        let image = match backend.capture(window, area) {
            Ok(image) if image.is_black() => {
                self.capture_failed(None);
                return Observation::hidden(region, at, Hidden::Black);
            }
            Ok(image) => image,
            Err(e) => {
                self.capture_failed(Some(e));
                return Observation::hidden(region, at, Hidden::CaptureFailed);
            }
        };
        if self.capture_failures >= CAPTURE_FAILURE_LIMIT {
            // Copies work again: say what is read, as after any refusal.
            self.activity = Activity::Unknown;
        }
        self.capture_failures = 0;
        match backend.ocr(&image) {
            Ok(lines) => Observation::from_ocr(region, at, client, area, lines),
            Err(_) => {
                self.minute.ocr_failures += 1;
                Observation::hidden(region, at, Hidden::OcrFailed)
            }
        }
    }

    /// Counts a failed (`Some`) or black (`None`) copy; the tenth in a row
    /// writes its line, with the last failure's detail.
    fn capture_failed(&mut self, error: Option<BackendError>) {
        self.capture_failures += 1;
        if self.capture_failures == CAPTURE_FAILURE_LIMIT {
            self.app.diag(&format!(
                "screen: refused: the capture failed {CAPTURE_FAILURE_LIMIT} times in a row"
            ));
            if let Some(e) = error {
                self.app.diag(&format!("error: screen reader: {e}"));
            }
        }
    }

    /// Writes the per-minute line when a minute has passed with any read.
    fn minute_line(&mut self) {
        let now = self.app.clock_ms();
        if now.saturating_sub(self.minute_start) < MINUTE_MS {
            return;
        }
        let counts = self.trackers.take_counts();
        let m = std::mem::take(&mut self.minute);
        self.minute_start = now;
        if m.frames == 0 {
            return;
        }
        self.app.diag(&format!(
            "screen: minute: {} frames, {} present, {} absent, {} covered, {} OCR failures, busy {} ms",
            m.frames,
            counts.present,
            counts.absent,
            counts.covered,
            m.ocr_failures,
            m.busy.as_millis()
        ));
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
}
