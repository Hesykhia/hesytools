//! Macros: the step model, turning recorded input into steps, and playback.
//!
//! The recorder captures raw hook events; [`events_to_steps`] then turns them
//! into something a person can read and edit: a key tapped and released
//! becomes one "Press E" step instead of a down/up pair, mouse movement
//! becomes a single path step, and the gaps between actions become waits.

use crate::capture::Region;
use crate::hooks::{InputEvent, InputKind};
use crate::input;
use crate::keys::{Key, MouseButton};
use crate::timer::Waiter;
use crate::watch::{Condition, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PressAction {
    /// Down, hold, up.
    #[default]
    Tap,
    /// Press and keep holding (released by a later `Up` or when playback ends).
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct PathPoint {
    /// Milliseconds since the start of the path.
    pub t: f64,
    pub x: i32,
    pub y: i32,
}

fn default_hold() -> f64 {
    30.0
}

fn default_poll() -> f64 {
    100.0
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OnTimeout {
    /// Carry on with the next step.
    #[default]
    Continue,
    /// Stop the whole macro.
    Stop,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Step {
    #[serde(rename_all = "camelCase")]
    Key {
        key: Key,
        #[serde(default)]
        action: PressAction,
        #[serde(default = "default_hold")]
        hold_ms: f64,
    },
    #[serde(rename_all = "camelCase")]
    Click {
        button: MouseButton,
        #[serde(default)]
        action: PressAction,
        #[serde(default = "default_hold")]
        hold_ms: f64,
        /// Move here first; `None` clicks wherever the cursor is.
        #[serde(default)]
        at: Option<(i32, i32)>,
    },
    Move { x: i32, y: i32 },
    /// Recorded mouse movement, replayed with its original timing.
    Path { points: Vec<PathPoint> },
    /// `amount` in wheel units: 120 = one notch, positive = up/right.
    Scroll {
        amount: i32,
        #[serde(default)]
        horizontal: bool,
    },
    Wait { ms: f64 },
    /// Types text as Unicode characters (great for chat and forms; some
    /// games only accept real keys, so use Key steps there).
    Text { text: String },
    /// Runs `steps` `times` times; 0 = forever.
    Repeat { times: u32, steps: Vec<Step> },
    /// Pause until something shows up on screen (text, a number, a colour).
    #[serde(rename_all = "camelCase")]
    WaitFor {
        region: Region,
        condition: Condition,
        /// Give up after this long; 0 = wait forever.
        #[serde(default)]
        timeout_ms: f64,
        #[serde(default = "default_poll")]
        poll_ms: f64,
        #[serde(default)]
        on_timeout: OnTimeout,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub steps: Vec<Step>,
    /// Playback speed multiplier (2.0 = twice as fast).
    pub speed: f64,
    /// Times to run the whole macro; 0 = until stopped.
    pub repeat: u32,
    pub hotkey: Option<crate::hotkey::Hotkey>,
    /// Unix millis, for sorting.
    pub updated_at: u64,
}

impl Default for Macro {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Untitled macro".into(),
            steps: Vec::new(),
            speed: 1.0,
            repeat: 1,
            hotkey: None,
            updated_at: 0,
        }
    }
}

impl Macro {
    /// Duration of one run at speed 1.0, ignoring infinite repeats.
    pub fn duration_ms(&self) -> f64 {
        fn dur(steps: &[Step]) -> f64 {
            steps
                .iter()
                .map(|s| match s {
                    Step::Wait { ms } => *ms,
                    Step::Key { action: PressAction::Tap, hold_ms, .. } => *hold_ms,
                    Step::Click { action: PressAction::Tap, hold_ms, .. } => *hold_ms,
                    Step::Path { points } => points.last().map_or(0.0, |p| p.t),
                    Step::Repeat { times, steps } => dur(steps) * (*times).max(1) as f64,
                    // Screen waits take as long as the screen takes: not counted.
                    _ => 0.0,
                })
                .sum()
        }
        dur(&self.steps)
    }
}

// ── Recording → steps ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RecordOptions {
    /// Record mouse movement between clicks as path steps.
    pub record_moves: bool,
    /// Waits shorter than this are dropped (merged into the next action).
    pub min_wait_ms: f64,
}

impl Default for RecordOptions {
    fn default() -> Self {
        Self { record_moves: false, min_wait_ms: 1.0 }
    }
}

/// Points closer together than this in time and space are thinned out.
const PATH_MIN_DT_MS: f64 = 8.0;
const PATH_MIN_DIST: i32 = 2;
/// Wheel events this close together merge into one scroll step.
const SCROLL_MERGE_MS: f64 = 60.0;

pub fn events_to_steps(events: &[InputEvent], opts: &RecordOptions) -> Vec<Step> {
    let mut raw: Vec<(Instant, Step)> = Vec::new();
    let mut keys_down = HashSet::new();
    let mut buttons_down = HashSet::new();
    let mut path: Vec<(Instant, i32, i32)> = Vec::new();

    let flush_path = |path: &mut Vec<(Instant, i32, i32)>, raw: &mut Vec<(Instant, Step)>| {
        if path.is_empty() {
            return;
        }
        let t0 = path[0].0;
        let step = if path.len() == 1 {
            Step::Move { x: path[0].1, y: path[0].2 }
        } else {
            Step::Path {
                points: path
                    .iter()
                    .map(|(t, x, y)| PathPoint { t: ms_between(t0, *t), x: *x, y: *y })
                    .collect(),
            }
        };
        raw.push((t0, step));
        path.clear();
    };

    for ev in events.iter().filter(|e| !e.ours) {
        match ev.kind {
            InputKind::Move => {
                if !opts.record_moves {
                    continue;
                }
                if let Some(&(lt, lx, ly)) = path.last() {
                    let close = (ev.x - lx).abs() < PATH_MIN_DIST && (ev.y - ly).abs() < PATH_MIN_DIST;
                    if close || ms_between(lt, ev.at) < PATH_MIN_DT_MS {
                        continue;
                    }
                }
                path.push((ev.at, ev.x, ev.y));
            }
            InputKind::Key { key, down, .. } => {
                flush_path(&mut path, &mut raw);
                // Windows repeats keydown while a key is held; keep the first.
                if down && !keys_down.insert(key) {
                    continue;
                }
                // A release for a key held before recording started.
                if !down && !keys_down.remove(&key) {
                    continue;
                }
                let action = if down { PressAction::Down } else { PressAction::Up };
                raw.push((ev.at, Step::Key { key, action, hold_ms: 0.0 }));
            }
            InputKind::Button { button, down } => {
                flush_path(&mut path, &mut raw);
                if down {
                    buttons_down.insert(button);
                } else if !buttons_down.remove(&button) {
                    continue;
                }
                let action = if down { PressAction::Down } else { PressAction::Up };
                raw.push((ev.at, Step::Click { button, action, hold_ms: 0.0, at: Some((ev.x, ev.y)) }));
            }
            InputKind::Wheel { amount, horizontal } => {
                flush_path(&mut path, &mut raw);
                if let Some((t, Step::Scroll { amount: a, horizontal: h })) = raw.last_mut() {
                    if *h == horizontal && a.signum() == amount.signum() && ms_between(*t, ev.at) < SCROLL_MERGE_MS {
                        *a += amount;
                        *t = ev.at;
                        continue;
                    }
                }
                raw.push((ev.at, Step::Scroll { amount, horizontal }));
            }
        }
    }
    flush_path(&mut path, &mut raw);

    // Insert waits between actions. A path's "time" is its start, so the
    // wait after it must account for how long the path itself lasts.
    let mut steps = Vec::new();
    let mut prev_end: Option<Instant> = None;
    for (t, step) in raw {
        if let Some(p) = prev_end {
            let gap = ms_between(p, t);
            if gap >= opts.min_wait_ms {
                steps.push(Step::Wait { ms: gap.round() });
            }
        }
        prev_end = Some(match &step {
            Step::Path { points } => t + Duration::from_secs_f64(points.last().map_or(0.0, |p| p.t) / 1000.0),
            _ => t,
        });
        steps.push(step);
    }
    let mut steps = merge_taps(steps);
    // The stop hotkey's modifier (Ctrl of Ctrl+F9) lands at the very end.
    let trailing_noise = |s: &Step| match s {
        Step::Wait { .. } => true,
        Step::Key { action: PressAction::Down, key, .. } => key.modifier().is_some(),
        _ => false,
    };
    while steps.last().is_some_and(trailing_noise) {
        steps.pop();
    }
    steps
}

/// `Down X, (Wait), Up X` → `Tap X` holding for the wait. Holds that span
/// other actions (W held while clicking) stay as separate Down/Up steps.
pub(crate) fn merge_taps(steps: Vec<Step>) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::with_capacity(steps.len());
    let mut i = 0;
    while i < steps.len() {
        let (hold, up_at) = match steps.get(i + 1) {
            Some(Step::Wait { ms }) => (*ms, i + 2),
            _ => (0.0, i + 1),
        };
        let merged = match (&steps[i], steps.get(up_at)) {
            (
                Step::Key { key, action: PressAction::Down, .. },
                Some(Step::Key { key: k2, action: PressAction::Up, .. }),
            ) if key == k2 => Some(Step::Key { key: *key, action: PressAction::Tap, hold_ms: hold }),
            (
                Step::Click { button, action: PressAction::Down, at, .. },
                Some(Step::Click { button: b2, action: PressAction::Up, .. }),
            ) if button == b2 => Some(Step::Click { button: *button, action: PressAction::Tap, hold_ms: hold, at: *at }),
            _ => None,
        };
        match merged {
            Some(s) => {
                out.push(s);
                i = up_at + 1;
            }
            None => {
                out.push(steps[i].clone());
                i += 1;
            }
        }
    }
    out
}

fn ms_between(a: Instant, b: Instant) -> f64 {
    b.saturating_duration_since(a).as_secs_f64() * 1000.0
}

// ── Recorder ────────────────────────────────────────────────────────────────

/// Collects global input on a background thread until stopped.
pub struct Recorder {
    stop: crossbeam_channel::Sender<()>,
    thread: std::thread::JoinHandle<Vec<InputEvent>>,
    opts: RecordOptions,
}

/// How often the live view gets the macro-so-far while recording.
const PROGRESS_EVERY: Duration = Duration::from_millis(150);

impl Recorder {
    /// `keep(event)` decides which events are recorded (e.g. drop clicks on
    /// our own window, or the hotkey that stops recording).
    /// `on_progress(steps)` receives the macro-so-far whenever something new
    /// was recorded (at most every 150ms), for the live view.
    pub fn start(
        opts: RecordOptions,
        keep: impl Fn(&InputEvent) -> bool + Send + 'static,
        mut on_progress: impl FnMut(&[Step]) + Send + 'static,
    ) -> Self {
        let rx = crate::hooks::subscribe(opts.record_moves);
        let (stop, stop_rx) = crossbeam_channel::bounded::<()>(1);
        let thread = std::thread::Builder::new()
            .name("recorder".into())
            .spawn(move || {
                let mut events = Vec::new();
                let mut dirty = false;
                let mut last_progress = Instant::now();
                loop {
                    crossbeam_channel::select! {
                        recv(rx) -> ev => match ev {
                            Ok(ev) if !ev.ours && keep(&ev) => {
                                events.push(ev);
                                dirty = true;
                            }
                            Ok(_) => {}
                            Err(_) => break,
                        },
                        recv(stop_rx) -> _ => break,
                        default(PROGRESS_EVERY) => {}
                    }
                    if dirty && last_progress.elapsed() >= PROGRESS_EVERY {
                        on_progress(&events_to_steps(&events, &opts));
                        dirty = false;
                        last_progress = Instant::now();
                    }
                }
                events
            })
            .expect("spawn recorder");
        Self { stop, thread, opts }
    }

    pub fn stop(self) -> Vec<Step> {
        let _ = self.stop.send(());
        let events = self.thread.join().unwrap_or_default();
        events_to_steps(&events, &self.opts)
    }
}

// ── Playback ────────────────────────────────────────────────────────────────

/// Plays a macro on the calling thread until it finishes or `waiter` is
/// cancelled. `on_step(index, iteration)` fires as each top-level step starts.
/// Anything still held down is released at the end, no matter what.
pub fn play(m: &Macro, waiter: &Waiter, mut on_step: impl FnMut(usize, u32)) -> PlayResult {
    let mut p = Player {
        waiter,
        speed: m.speed.clamp(0.05, 20.0),
        held_keys: HashSet::new(),
        held_buttons: HashSet::new(),
        blocked: 0,
        error: None,
    };
    let mut iteration = 0u32;
    let finished = loop {
        if m.repeat != 0 && iteration >= m.repeat {
            break true;
        }
        let ok = m.steps.iter().enumerate().all(|(i, s)| {
            on_step(i, iteration);
            p.run(s)
        });
        if !ok {
            break false;
        }
        iteration += 1;
        // An empty forever-macro would spin; give it a breather.
        if m.steps.is_empty() && !p.wait(Duration::from_millis(50)) {
            break false;
        }
    };
    p.release_all();
    PlayResult { finished, iterations: iteration, blocked: p.blocked, error: p.error }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayResult {
    /// False if stopped early.
    pub finished: bool,
    pub iterations: u32,
    pub blocked: u64,
    /// Why playback stopped on its own, if it did (e.g. a screen wait timed out).
    pub error: Option<String>,
}

struct Player<'a> {
    waiter: &'a Waiter,
    speed: f64,
    held_keys: HashSet<Key>,
    held_buttons: HashSet<MouseButton>,
    blocked: u64,
    error: Option<String>,
}

impl Player<'_> {
    fn scaled(&self, ms: f64) -> Duration {
        Duration::from_secs_f64((ms / self.speed).max(0.0) / 1000.0)
    }

    fn wait(&self, d: Duration) -> bool {
        d.is_zero() && !self.waiter.is_cancelled() || self.waiter.wait(d)
    }

    fn send(&mut self, events: &[windows::Win32::UI::Input::KeyboardAndMouse::INPUT]) {
        if (input::send(events) as usize) < events.len() {
            self.blocked += 1;
        }
    }

    /// Returns false when cancelled.
    fn run(&mut self, step: &Step) -> bool {
        if self.waiter.is_cancelled() {
            return false;
        }
        match step {
            Step::Key { key, action, hold_ms } => match action {
                PressAction::Tap => {
                    self.send(&[input::key_event(*key, true)]);
                    let ok = self.wait(self.scaled(*hold_ms));
                    self.send(&[input::key_event(*key, false)]);
                    ok
                }
                PressAction::Down => {
                    self.held_keys.insert(*key);
                    self.send(&[input::key_event(*key, true)]);
                    true
                }
                PressAction::Up => {
                    self.held_keys.remove(key);
                    self.send(&[input::key_event(*key, false)]);
                    true
                }
            },
            Step::Click { button, action, hold_ms, at } => {
                if let Some((x, y)) = at {
                    self.send(&[input::move_abs_event(*x, *y)]);
                }
                match action {
                    PressAction::Tap => {
                        self.send(&[input::button_event(*button, true)]);
                        let ok = self.wait(self.scaled(*hold_ms));
                        self.send(&[input::button_event(*button, false)]);
                        ok
                    }
                    PressAction::Down => {
                        self.held_buttons.insert(*button);
                        self.send(&[input::button_event(*button, true)]);
                        true
                    }
                    PressAction::Up => {
                        self.held_buttons.remove(button);
                        self.send(&[input::button_event(*button, false)]);
                        true
                    }
                }
            }
            Step::Move { x, y } => {
                self.send(&[input::move_abs_event(*x, *y)]);
                true
            }
            Step::Path { points } => {
                let start = Instant::now();
                for pt in points {
                    if !self.waiter.wait_until(start + self.scaled(pt.t)) {
                        return false;
                    }
                    self.send(&[input::move_abs_event(pt.x, pt.y)]);
                }
                true
            }
            Step::Scroll { amount, horizontal } => {
                self.send(&[input::wheel_event(*amount, *horizontal)]);
                true
            }
            Step::Wait { ms } => self.wait(self.scaled(*ms)),
            Step::Text { text } => {
                for unit in text.encode_utf16() {
                    self.send(&[input::unicode_event(unit, true), input::unicode_event(unit, false)]);
                    if !self.wait(Duration::from_millis(2)) {
                        return false;
                    }
                }
                true
            }
            Step::WaitFor { region, condition, timeout_ms, poll_ms, on_timeout } => {
                let mut w = match Watcher::new(*region, condition.clone()) {
                    Ok(w) => w,
                    Err(e) => {
                        self.error = Some(e);
                        return false;
                    }
                };
                let start = Instant::now();
                // Polling isn't scaled by playback speed: the screen is the screen.
                let poll = Duration::from_secs_f64(poll_ms.clamp(16.0, 10_000.0) / 1000.0);
                loop {
                    if let Ok(c) = w.check() {
                        if c.matched {
                            return true;
                        }
                    }
                    if *timeout_ms > 0.0 && start.elapsed().as_secs_f64() * 1000.0 >= *timeout_ms {
                        if *on_timeout == OnTimeout::Stop {
                            self.error = Some("A screen wait timed out".into());
                            return false;
                        }
                        return true;
                    }
                    if !self.waiter.wait(poll) {
                        return false;
                    }
                }
            }
            Step::Repeat { times, steps } => {
                let mut n = 0;
                while *times == 0 || n < *times {
                    if !steps.iter().all(|s| self.run(s)) {
                        return false;
                    }
                    n += 1;
                    if steps.is_empty() && !self.wait(Duration::from_millis(50)) {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn release_all(&mut self) {
        let keys: Vec<_> = self.held_keys.drain().collect();
        for k in keys {
            self.send(&[input::key_event(k, false)]);
        }
        let buttons: Vec<_> = self.held_buttons.drain().collect();
        for b in buttons {
            self.send(&[input::button_event(b, false)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(base: Instant, ms: u64, kind: InputKind) -> InputEvent {
        InputEvent { kind, x: 100, y: 200, ours: false, foreign_injected: false, at: base + Duration::from_millis(ms) }
    }
    fn key(name: &str, down: bool) -> InputKind {
        InputKind::Key { key: Key::from_name(name).unwrap(), scan: 0, extended: false, down }
    }

    #[test]
    fn taps_become_single_steps_with_waits_between() {
        let t = Instant::now();
        let events = [
            ev(t, 0, key("E", true)),
            ev(t, 40, key("E", false)),
            ev(t, 540, InputKind::Button { button: MouseButton::Left, down: true }),
            ev(t, 560, InputKind::Button { button: MouseButton::Left, down: false }),
        ];
        let steps = events_to_steps(&events, &RecordOptions::default());
        assert_eq!(
            steps,
            vec![
                Step::Key { key: Key::from_name("E").unwrap(), action: PressAction::Tap, hold_ms: 40.0 },
                Step::Wait { ms: 500.0 },
                Step::Click { button: MouseButton::Left, action: PressAction::Tap, hold_ms: 20.0, at: Some((100, 200)) },
            ]
        );
    }

    #[test]
    fn holds_spanning_other_actions_stay_split_and_repeats_are_ignored() {
        let t = Instant::now();
        let events = [
            ev(t, 0, key("W", true)),
            ev(t, 30, key("W", true)), // auto-repeat
            ev(t, 60, key("W", true)), // auto-repeat
            ev(t, 100, key("Space", true)),
            ev(t, 150, key("Space", false)),
            ev(t, 2000, key("W", false)),
        ];
        let steps = events_to_steps(&events, &RecordOptions::default());
        let w = Key::from_name("W").unwrap();
        assert_eq!(steps[0], Step::Key { key: w, action: PressAction::Down, hold_ms: 0.0 });
        assert_eq!(steps[1], Step::Wait { ms: 100.0 });
        assert!(matches!(steps[2], Step::Key { action: PressAction::Tap, hold_ms: 50.0, .. }));
        assert_eq!(steps[3], Step::Wait { ms: 1850.0 });
        assert_eq!(steps[4], Step::Key { key: w, action: PressAction::Up, hold_ms: 0.0 });
        assert_eq!(steps.len(), 5);
    }

    #[test]
    fn moves_collapse_into_one_path_and_our_events_are_skipped() {
        let t = Instant::now();
        let mut events: Vec<InputEvent> = (0..50)
            .map(|i| InputEvent { x: 100 + i * 3, y: 200, ..ev(t, i as u64 * 4, InputKind::Move) })
            .collect();
        events.push(InputEvent { ours: true, ..ev(t, 210, key("F24", true)) });
        events.push(ev(t, 300, key("E", true)));
        events.push(ev(t, 320, key("E", false)));
        let opts = RecordOptions { record_moves: true, ..Default::default() };
        let steps = events_to_steps(&events, &opts);
        let Step::Path { points } = &steps[0] else { panic!("{steps:?}") };
        // 4ms samples thinned to >= 8ms apart.
        assert!(points.len() < 30 && points.len() > 20, "{}", points.len());
        assert_eq!(points[0].t, 0.0);
        // Wait after the path is measured from the path's *end*.
        let Step::Wait { ms } = steps[1] else { panic!() };
        assert!((ms - (300.0 - points.last().unwrap().t)).abs() <= 1.0, "{ms}");
        assert!(matches!(steps[2], Step::Key { action: PressAction::Tap, .. }));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn stray_releases_and_trailing_modifiers_are_dropped() {
        let t = Instant::now();
        let steps = events_to_steps(
            &[
                ev(t, 0, key("F9", false)), // the hotkey that started recording
                ev(t, 10, InputKind::Button { button: MouseButton::Left, down: false }),
                ev(t, 100, key("E", true)),
                ev(t, 130, key("E", false)),
                ev(t, 900, key("Ctrl", true)), // Ctrl of the Ctrl+F9 stop combo
            ],
            &RecordOptions::default(),
        );
        assert_eq!(steps, vec![Step::Key { key: Key::from_name("E").unwrap(), action: PressAction::Tap, hold_ms: 30.0 }]);
    }

    #[test]
    fn wheel_notches_merge() {
        let t = Instant::now();
        let w = |ms, a| ev(t, ms, InputKind::Wheel { amount: a, horizontal: false });
        let steps = events_to_steps(&[w(0, 120), w(20, 120), w(40, 120), w(500, -120)], &RecordOptions::default());
        assert_eq!(steps[0], Step::Scroll { amount: 360, horizontal: false });
        assert_eq!(steps[2], Step::Scroll { amount: -120, horizontal: false });
    }

    #[test]
    fn json_shape_is_stable() {
        let m = Macro {
            steps: vec![
                Step::Key { key: Key::from_name("E").unwrap(), action: PressAction::Tap, hold_ms: 30.0 },
                Step::Repeat { times: 3, steps: vec![Step::Wait { ms: 100.0 }] },
            ],
            ..Default::default()
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#"{"type":"key","key":"E","action":"tap","holdMs":30.0}"#), "{json}");
        assert_eq!(serde_json::from_str::<Macro>(&json).unwrap(), m);
        assert_eq!(m.duration_ms(), 330.0);
    }

    #[test]
    fn playback_stops_promptly_and_releases_held_keys() {
        // Only F24 (no physical key) so nothing is typed anywhere.
        let f24 = Key::from_name("F24").unwrap();
        let m = Macro {
            steps: vec![Step::Key { key: f24, action: PressAction::Down, hold_ms: 0.0 }, Step::Wait { ms: 10_000.0 }],
            ..Default::default()
        };
        let w = std::sync::Arc::new(Waiter::new().unwrap());
        let w2 = w.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            w2.cancel();
        });
        let t = Instant::now();
        let r = play(&m, &w, |_, _| {});
        assert!(!r.finished);
        assert!(t.elapsed() < Duration::from_secs(1));
    }
}
