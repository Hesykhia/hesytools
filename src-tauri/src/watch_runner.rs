//! Runs enabled Screen Watch rules in the background, one light thread each.
//! A rule fires when its condition *becomes* true (not on every check while
//! it stays true), and not again until its cooldown has passed.

use crate::settings::WatchRule;
use engine::timer::Waiter;
use engine::watch::Watcher;
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

pub type FireFn = Arc<dyn Fn(&WatchRule) + Send + Sync>;

#[derive(Default)]
pub struct WatchRunner {
    threads: Mutex<HashMap<String, (Arc<Waiter>, JoinHandle<()>)>>,
}

impl WatchRunner {
    /// Stop everything, then start a thread for every enabled rule.
    pub fn sync(&self, app: &AppHandle, rules: &[WatchRule], fire: FireFn) {
        self.stop_all();
        let mut threads = self.threads.lock();
        for rule in rules.iter().filter(|r| r.enabled && r.region.w > 0 && r.region.h > 0) {
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
        let _ = app.emit("watch-status", WatchStatus { id: rule.id.clone(), matched, text, error });
    };
    let mut watcher = match Watcher::new(rule.region, rule.condition.clone()) {
        Ok(w) => w,
        Err(e) => {
            status(false, None, Some(e));
            return;
        }
    };
    let poll = Duration::from_secs_f64(rule.poll_ms.clamp(50.0, 60_000.0) / 1000.0);
    let cooldown = Duration::from_secs_f64(rule.cooldown_ms.max(0.0) / 1000.0);
    let mut was_matched = None::<bool>;
    let mut last_text = None::<String>;
    let mut last_fire: Option<Instant> = None;
    loop {
        match watcher.check() {
            Ok(c) => {
                // Only tell the UI when something it shows actually changed.
                if was_matched != Some(c.matched) || (c.ocr_ran && c.text != last_text) {
                    status(c.matched, c.text.clone(), None);
                    last_text = c.text.clone();
                }
                let rising = c.matched && was_matched == Some(false);
                if rising && last_fire.is_none_or(|t| t.elapsed() >= cooldown) {
                    last_fire = Some(Instant::now());
                    fire(&rule);
                }
                // The first check only establishes the starting state, so a
                // rule enabled while its text is already showing doesn't fire.
                was_matched = Some(c.matched);
            }
            Err(e) => status(false, None, Some(e)),
        }
        if !waiter.wait(poll) {
            return;
        }
    }
}
