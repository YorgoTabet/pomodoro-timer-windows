use crate::core::{engine, phase::Phase, settings::Settings};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};

/// What the windows and the tray draw. Sent as `timer-state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    pub phase: Phase,
    pub is_running: bool,
    pub remaining_seconds: i64,
    pub total_seconds: i64,
    pub completed_focus_sessions: u32,
}

/// A phase that ended, by running out or by being skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Elapsed {
    pub from: Phase,
    pub to: Phase,
    /// Seconds to record when a focus session ran to its end. None for skips and breaks.
    #[serde(skip)]
    pub focus_seconds: Option<i64>,
}

/// Drives the pomodoro cycle. No clock and no Tauri in here: callers pass `now`,
/// which keeps every rule testable.
///
/// Timing is deadline-based: the core stores an absolute wall-clock time and derives
/// the remaining seconds from it on every tick. That makes it immune to tick drift
/// and to system sleep. A deadline that passed while asleep fires on the next tick.
/// Wall clock on purpose: the monotonic `Instant` may stop counting during sleep
/// (it does on macOS), which would resume the countdown behind.
#[derive(Debug)]
pub struct TimerCore {
    phase: Phase,
    is_running: bool,
    remaining_seconds: i64,
    /// The length this phase actually started with. Not re-read from settings, so
    /// editing "focus = 45" halfway through a 25-minute session cannot make the ring
    /// jump backwards. The new length applies from the next phase.
    total_seconds: i64,
    completed_focus_sessions: u32,
    deadline: Option<SystemTime>,
}

impl TimerCore {
    pub fn new(settings: &Settings) -> Self {
        let total = engine::duration(Phase::Focus, settings);
        TimerCore {
            phase: Phase::Focus,
            is_running: false,
            remaining_seconds: total,
            total_seconds: total,
            completed_focus_sessions: 0,
            deadline: None,
        }
    }

    pub fn state(&self) -> TimerState {
        TimerState {
            phase: self.phase,
            is_running: self.is_running,
            remaining_seconds: self.remaining_seconds,
            total_seconds: self.total_seconds,
            completed_focus_sessions: self.completed_focus_sessions,
        }
    }

    pub fn toggle(&mut self, now: SystemTime, settings: &Settings) {
        if self.is_running {
            self.pause(now);
        } else {
            self.start(now, settings);
        }
    }

    pub fn start(&mut self, now: SystemTime, settings: &Settings) {
        if self.is_running {
            return;
        }
        if self.remaining_seconds <= 0 {
            self.total_seconds = engine::duration(self.phase, settings);
            self.remaining_seconds = self.total_seconds;
        }
        self.deadline = Some(now + Duration::from_secs(self.remaining_seconds as u64));
        self.is_running = true;
    }

    pub fn pause(&mut self, now: SystemTime) {
        if !self.is_running {
            return;
        }
        self.sync_remaining(now);
        self.is_running = false;
        self.deadline = None;
    }

    /// Abandon the current phase and move to the next one without recording it.
    pub fn skip(&mut self, settings: &Settings) -> Elapsed {
        let from = self.phase;
        let to = self.next_phase(settings);
        self.transition(to, settings);
        Elapsed { from, to, focus_seconds: None }
    }

    /// Back to a fresh, stopped focus session. Does not clear today's stats.
    pub fn reset(&mut self, settings: &Settings) {
        self.completed_focus_sessions = 0;
        self.transition(Phase::Focus, settings);
    }

    /// Re-read durations after the user edits settings. A phase that is already
    /// running keeps the length it started with; a stopped one adopts the new value
    /// straight away, since nothing is mid-flight to disturb.
    pub fn settings_changed(&mut self, settings: &Settings) {
        if !self.is_running {
            self.total_seconds = engine::duration(self.phase, settings);
            self.remaining_seconds = self.total_seconds;
        }
    }

    /// Call every few hundred milliseconds. Returns the phase change when the
    /// deadline has passed, however long ago.
    pub fn tick(&mut self, now: SystemTime, settings: &Settings) -> Option<Elapsed> {
        let deadline = self.deadline.filter(|_| self.is_running)?;
        if deadline <= now {
            Some(self.elapse(now, settings))
        } else {
            self.sync_remaining(now);
            None
        }
    }

    /// The current phase ran to completion.
    fn elapse(&mut self, now: SystemTime, settings: &Settings) -> Elapsed {
        let from = self.phase;
        let to = self.next_phase(settings);
        let focus_seconds = (from == Phase::Focus).then(|| {
            self.completed_focus_sessions += 1;
            engine::duration(Phase::Focus, settings)
        });

        // next_phase counts sessions finished before this phase, so it ran above
        // before the increment. Moving on only needs the target phase.
        self.transition(to, settings);

        if engine::should_auto_start(from, settings) {
            self.start(now, settings);
        }
        Elapsed { from, to, focus_seconds }
    }

    /// Sessions finished *before* the current phase decide the break length.
    fn next_phase(&self, settings: &Settings) -> Phase {
        engine::next(
            self.phase,
            self.completed_focus_sessions,
            settings.pomodoros_until_long_break as u32,
        )
    }

    fn transition(&mut self, to: Phase, settings: &Settings) {
        self.is_running = false;
        self.deadline = None;
        self.phase = to;
        self.total_seconds = engine::duration(to, settings);
        self.remaining_seconds = self.total_seconds;
    }

    fn sync_remaining(&mut self, now: SystemTime) {
        if let Some(deadline) = self.deadline {
            let millis = deadline.duration_since(now).map_or(0, |d| d.as_millis() as i64);
            // Capped at the total: a clock set back must not push the ring past full.
            self.remaining_seconds = ((millis + 999) / 1000).min(self.total_seconds);
        }
    }
}

/// The timer and the settings it reads, shared between the ticker, the tray and the commands.
pub struct Shared {
    core: Mutex<TimerCore>,
    settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    /// Held while a change is made and announced, so a stale state can never be
    /// published after a newer one.
    ///
    /// RULE: never call tray or window APIs while holding it. Those wait on the main
    /// thread, and sync commands run on the main thread and wait on this lock. The
    /// tray is updated through `tray::refresh`, which never blocks.
    publish_lock: Mutex<()>,
}

impl Shared {
    pub fn new(settings: Settings, settings_path: PathBuf) -> Self {
        Shared { core: Mutex::new(TimerCore::new(&settings)), settings: Mutex::new(settings), settings_path, publish_lock: Mutex::new(()) }
    }

    pub fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    pub fn state(&self) -> TimerState {
        lock(&self.core).state()
    }

    /// Save to disk first, then keep in memory, so the two never disagree.
    /// Returns the clamped settings that were kept.
    pub fn replace_settings(&self, settings: Settings) -> std::io::Result<Settings> {
        let clamped = settings.clamped();
        // Saved under the lock so it cannot interleave with a bar drag's save.
        let mut guard = lock(&self.settings);
        clamped.save(&self.settings_path)?;
        *guard = clamped.clone();
        Ok(clamped)
    }
}

impl Shared {
    /// Change only the bar's position, under the settings lock, so a drag never
    /// overwrites a setting saved at the same moment.
    pub fn set_bar_position(&self, x: f64, y: f64) -> std::io::Result<Settings> {
        let mut guard = lock(&self.settings);
        let mut next = guard.clone();
        next.floating_bar_x = Some(x);
        next.floating_bar_y = Some(y);
        next.save(&self.settings_path)?;
        *guard = next.clone();
        Ok(next)
    }
}

// A panic elsewhere must not wedge the timer, so a poisoned lock is still usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run one change against the timer, then tell everyone if anything moved.
/// Every path that changes the timer goes through here, so alerts and music can
/// hook `publish` later without chasing individual commands.
pub fn apply(app: &AppHandle, change: impl FnOnce(&mut TimerCore, &Settings, SystemTime) -> Option<Elapsed>) {
    let shared = app.state::<Shared>();
    let _order = lock(&shared.publish_lock);
    let settings = shared.settings();
    let (before, after, elapsed) = {
        let mut core = lock(&shared.core);
        let before = core.state();
        let elapsed = change(&mut core, &settings, SystemTime::now());
        (before, core.state(), elapsed)
    };
    if before != after || elapsed.is_some() {
        publish(app, &after, elapsed);
    }
}

/// Save new settings, then tell the timer, the windows and the tray.
/// The one path for every settings change, so the tray toggle and the settings
/// window behave the same.
pub fn commit_settings(app: &AppHandle, settings: Settings) -> Result<Settings, String> {
    let shared = app.state::<Shared>();
    let bar_was_shown = shared.settings().show_floating_bar;
    let saved = shared.replace_settings(settings).map_err(|e| e.to_string())?;
    let _ = app.emit("settings-changed", &saved);
    apply(app, |c, s, _| {
        c.settings_changed(s);
        None
    });
    if saved.show_floating_bar != bar_was_shown {
        crate::tray::sync_bar(app, &saved);
    }
    Ok(saved)
}

/// The single place a change becomes visible: windows, tray, and phase hooks.
pub fn publish(app: &AppHandle, state: &TimerState, elapsed: Option<Elapsed>) {
    if let Some(e) = elapsed {
        on_phase_elapsed(app, e);
    }
    let _ = app.emit("timer-state", state);
    crate::tray::refresh(app, state);
}

fn on_phase_elapsed(app: &AppHandle, e: Elapsed) {
    if let Some(seconds) = e.focus_seconds {
        on_focus_completed(seconds);
    }
    let _ = app.emit("phase-elapsed", e);
}

/// Task 6 records stats here.
fn on_focus_completed(_seconds: i64) {}

/// Ticks twice a second. The state only changes, and is only sent, once per
/// displayed second.
pub fn spawn_ticker(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(500));
        apply(&app, |core, settings, now| core.tick(now, settings));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn short() -> Settings {
        Settings { focus_minutes: 1, short_break_minutes: 1, long_break_minutes: 2, pomodoros_until_long_break: 2, auto_start_breaks: false, auto_start_focus: false, ..Default::default() }
    }

    #[test]
    fn starts_stopped_on_a_full_focus_session() {
        let core = TimerCore::new(&Settings::default());
        let s = core.state();
        assert_eq!((s.phase, s.is_running, s.remaining_seconds, s.total_seconds), (Phase::Focus, false, 1500, 1500));
    }

    #[test]
    fn pause_and_resume_keep_the_remaining_time() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        assert_eq!(core.tick(t0 + secs(20), &s), None);
        assert_eq!(core.state().remaining_seconds, 40);
        core.pause(t0 + secs(20));
        assert!(!core.state().is_running);
        // Time passing while paused costs nothing.
        assert_eq!(core.tick(t0 + secs(500), &s), None);
        assert_eq!(core.state().remaining_seconds, 40);
        core.start(t0 + secs(500), &s);
        core.tick(t0 + secs(510), &s);
        assert_eq!(core.state().remaining_seconds, 30);
    }

    #[test]
    fn remaining_rounds_up_so_it_never_shows_zero_early() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + Duration::from_millis(59_500), &s);
        assert_eq!(core.state().remaining_seconds, 1);
    }

    #[test]
    fn elapse_moves_to_a_stopped_break_without_auto_start() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        let e = core.tick(t0 + secs(60), &s).expect("deadline reached");
        assert_eq!((e.from, e.to, e.focus_seconds), (Phase::Focus, Phase::ShortBreak, Some(60)));
        let st = core.state();
        assert_eq!((st.phase, st.is_running, st.remaining_seconds, st.completed_focus_sessions), (Phase::ShortBreak, false, 60, 1));
    }

    #[test]
    fn elapse_auto_starts_the_break_when_asked() {
        let s = Settings { auto_start_breaks: true, ..short() };
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + secs(60), &s);
        let st = core.state();
        assert!(st.is_running);
        assert_eq!(st.phase, Phase::ShortBreak);
        // The break's own deadline counts from the moment focus ended.
        core.tick(t0 + secs(90), &s);
        assert_eq!(core.state().remaining_seconds, 30);
    }

    #[test]
    fn break_leads_back_to_focus_and_follows_the_focus_flag() {
        let mut s = Settings { auto_start_focus: true, ..short() };
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + secs(60), &s);
        core.start(t0 + secs(60), &s);
        let e = core.tick(t0 + secs(120), &s).unwrap();
        assert_eq!((e.from, e.to, e.focus_seconds), (Phase::ShortBreak, Phase::Focus, None));
        assert!(core.state().is_running);
        s.auto_start_focus = false;
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + secs(60), &s);
        core.start(t0 + secs(60), &s);
        core.tick(t0 + secs(120), &s);
        assert!(!core.state().is_running);
    }

    #[test]
    fn every_nth_focus_gets_a_long_break() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + secs(60), &s);
        core.skip(&s);
        core.start(t0, &s);
        let e = core.tick(t0 + secs(60), &s).unwrap();
        assert_eq!(e.to, Phase::LongBreak);
        assert_eq!(core.state().total_seconds, 120);
    }

    #[test]
    fn skip_moves_on_without_counting_a_session() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        let e = core.skip(&s);
        assert_eq!((e.from, e.to, e.focus_seconds), (Phase::Focus, Phase::ShortBreak, None));
        let st = core.state();
        assert_eq!((st.is_running, st.completed_focus_sessions, st.remaining_seconds), (false, 0, 60));
        assert_eq!(core.skip(&s).to, Phase::Focus);
    }

    #[test]
    fn reset_returns_to_a_fresh_stopped_focus() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 + secs(60), &s);
        core.reset(&s);
        let st = core.state();
        assert_eq!((st.phase, st.is_running, st.remaining_seconds, st.completed_focus_sessions), (Phase::Focus, false, 60, 0));
    }

    #[test]
    fn a_long_gap_such_as_sleep_elapses_on_the_next_tick() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        // The lid was shut for eight hours.
        let e = core.tick(t0 + secs(8 * 3600), &s).expect("fires immediately");
        assert_eq!(e.to, Phase::ShortBreak);
        assert_eq!(core.state().remaining_seconds, 60);
        assert_eq!(core.state().completed_focus_sessions, 1);
    }

    #[test]
    fn a_wall_clock_jump_forward_counts_as_elapsed_time() {
        let s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        // The clock leaps 45 s, as when the PC wakes from sleep.
        core.tick(t0 + secs(45), &s);
        assert_eq!(core.state().remaining_seconds, 15);
    }

    #[test]
    fn a_clock_set_back_never_panics_or_goes_negative() {
        let s = short();
        let t0 = SystemTime::now() + secs(100);
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        core.tick(t0 - secs(30), &s);
        assert_eq!(core.state().remaining_seconds, 60);
        core.pause(t0 + secs(500));
        assert_eq!(core.state().remaining_seconds, 0);
    }

    #[test]
    fn a_running_phase_keeps_its_length_when_settings_change() {
        let mut s = short();
        let t0 = SystemTime::now();
        let mut core = TimerCore::new(&s);
        core.start(t0, &s);
        s.focus_minutes = 45;
        core.settings_changed(&s);
        assert_eq!(core.state().total_seconds, 60);
        core.pause(t0 + secs(10));
        core.settings_changed(&s);
        assert_eq!((core.state().total_seconds, core.state().remaining_seconds), (2700, 2700));
    }

    #[test]
    fn start_after_a_zero_remaining_refills_from_settings() {
        let s = short();
        let mut core = TimerCore::new(&s);
        core.remaining_seconds = 0;
        core.start(SystemTime::now(), &s);
        assert_eq!(core.state().remaining_seconds, 60);
    }

    #[test]
    fn state_serializes_in_the_contract_shape() {
        let json = serde_json::to_value(TimerCore::new(&Settings::default()).state()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"phase":"focus","isRunning":false,"remainingSeconds":1500,"totalSeconds":1500,"completedFocusSessions":0})
        );
        let e = Elapsed { from: Phase::Focus, to: Phase::ShortBreak, focus_seconds: Some(5) };
        assert_eq!(serde_json::to_value(e).unwrap(), serde_json::json!({"from":"focus","to":"shortBreak"}));
    }
}
