//! Runs enabled Screen Watch rules in the background, one light thread each.
//! Each thing a rule looks for fires when it *becomes* true (not on every
//! check while it stays true), and not again until its cooldown has passed.
//! Items are independent: yellow showing up fires for yellow, red for red.

use crate::settings::WatchRule;
use engine::timer::Waiter;
use engine::watch::{Check, Watcher};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub id: String,
    pub matched: bool,
    pub text: Option<String>,
    pub error: Option<String>,
}

/// `Start` when item `item` (index into `WatchRule::items`) triggers; `End`
/// once that match goes away again (or the watch stops), for actions that
/// last while it's showing.
pub enum Edge<'a> {
    Start { item: usize, check: &'a Check },
    End { item: usize },
}

/// Per-item trigger state.
struct Item {
    watcher: Watcher,
    was_matched: Option<bool>,
    last_fire: Option<Instant>,
    /// Triggered and not yet ended.
    active: bool,
}

pub type FireFn = Arc<dyn Fn(&WatchRule, Edge) + Send + Sync>;

#[derive(Default)]
pub struct WatchRunner {
    threads: Mutex<HashMap<String, (Arc<Waiter>, JoinHandle<()>)>>,
}

impl WatchRunner {
    /// Stop everything, then start a thread for every enabled rule.
    pub fn sync(&self, app: &AppHandle, rules: &[WatchRule], fire: FireFn) {
        self.stop_all();
        let mut threads = self.threads.lock();
        for rule in rules
            .iter()
            .filter(|r| r.enabled && r.region.w > 0 && r.region.h > 0)
        {
            let Ok(waiter) = Waiter::new() else { continue };
            let waiter = Arc::new(waiter);
            let (w, rule, app, fire) = (waiter.clone(), rule.clone(), app.clone(), fire.clone());
            let id = rule.id.clone();
            let handle = std::thread::Builder::new()
                .name(format!("watch {}", rule.name))
                .spawn(move || run(rule, &w, &app, &fire))
                .expect("spawn watch thread");
            threads.insert(id, (waiter, handle));
        }
    }

    pub fn stop_all(&self) {
        let drained: Vec<_> = self.threads.lock().drain().collect();
        for (_, (w, _)) in &drained {
            w.cancel();
        }
        for (_, (_, h)) in drained {
            let _ = h.join();
        }
    }
}

fn run(rule: WatchRule, waiter: &Waiter, app: &AppHandle, fire: &FireFn) {
    let status = |matched, text, error| {
        let _ = app.emit(
            "watch-status",
            WatchStatus {
                id: rule.id.clone(),
                matched,
                text,
                error,
            },
        );
    };
    let mut items = Vec::new();
    for c in rule.items() {
        match Watcher::new(rule.region, c.clone()).map(|w| w.in_window(rule.window.clone())) {
            Ok(watcher) => items.push(Item { watcher, was_matched: None, last_fire: None, active: false }),
            Err(e) => {
                status(false, None, Some(e));
                return;
            }
        }
    }
    let poll = Duration::from_secs_f64(rule.poll_ms.clamp(50.0, 60_000.0) / 1000.0);
    let cooldown = Duration::from_secs_f64(rule.cooldown_ms.max(0.0) / 1000.0);
    let mut shown: Option<(bool, Option<String>)> = None;
    loop {
        let mut any = false;
        let mut text = None::<String>;
        let mut error = None::<String>;
        for (i, it) in items.iter_mut().enumerate() {
            match it.watcher.check() {
                Ok(c) => {
                    any |= c.matched;
                    if text.is_none() {
                        text = c.text.clone();
                    }
                    let rising = c.matched && it.was_matched == Some(false);
                    if rising && it.last_fire.is_none_or(|t| t.elapsed() >= cooldown) {
                        it.last_fire = Some(Instant::now());
                        it.active = true;
                        fire(&rule, Edge::Start { item: i, check: &c });
                    }
                    if !c.matched && std::mem::take(&mut it.active) {
                        fire(&rule, Edge::End { item: i });
                    }
                    // The first check only establishes the starting state, so
                    // a rule enabled while its text is already showing doesn't
                    // fire.
                    it.was_matched = Some(c.matched);
                }
                Err(e) => error = Some(e),
            }
        }
        if let Some(e) = error {
            status(false, None, Some(e));
            shown = None;
        } else if shown.as_ref() != Some(&(any, text.clone())) {
            // Only tell the UI when something it shows actually changed.
            status(any, text.clone(), None);
            shown = Some((any, text));
        }
        if !waiter.wait(poll) {
            for (i, it) in items.iter_mut().enumerate() {
                if std::mem::take(&mut it.active) {
                    fire(&rule, Edge::End { item: i });
                }
            }
            return;
        }
    }
}
