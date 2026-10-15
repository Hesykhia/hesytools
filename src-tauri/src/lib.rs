mod macro_store;
mod settings;
mod watch_runner;

use engine::capture::Region;
use engine::clicker::{Clicker, ClickerConfig, ClickerStatus, ExitReason};
use engine::hooks::InputKind;
use engine::hotkey::{Hotkey, HotkeyManager, Trigger};
use engine::macros::{self, Macro, RecordOptions, Recorder, Step};
use engine::timer::Waiter;
use engine::watch::{Check, Condition, Watcher};
use engine::{elevation, hooks, input, picker, webhook};
use macro_store::MacroStore;
use parking_lot::Mutex;
use serde::Serialize;
use settings::{HotkeyMode, Settings, Store, WatchAction, WatchRule, WebhookSettings};
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton as TrayButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use watch_runner::WatchRunner;

const HK_CLICKER: &str = "clicker";
const HK_PANIC: &str = "panic";
const HK_RECORD: &str = "record";
const HK_MACRO_PREFIX: &str = "macro:";

struct Playing {
    id: String,
    waiter: Arc<Waiter>,
    thread: JoinHandle<()>,
}

struct App {
    clicker: Clicker,
    hotkeys: Arc<HotkeyManager>,
    settings: Mutex<Settings>,
    store: Store,
    macros: Mutex<Vec<Macro>>,
    macro_store: MacroStore,
    playing: Mutex<Option<Playing>>,
    recorder: Mutex<Option<Recorder>>,
    /// Our main window, so the recorder can ignore clicks on hesytools itself.
    main_hwnd: AtomicIsize,
    /// Red border around every monitor while recording.
    outline: Mutex<Option<engine::outline::ScreenOutline>>,
    /// Live viewers by id, each with its watcher kept between polls so OCR is
    /// skipped while the region is unchanged (plus the region+condition it
    /// was built for).
    viewers: Mutex<std::collections::HashMap<String, (String, Watcher)>>,
    watches: WatchRunner,
    /// An update found by `check_update`, waiting for "Update now".
    pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    /// The shutdown/restart timer hesytools set, if any.
    shutdown: Mutex<Option<ShutdownTimer>>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShutdownTimer {
    /// Unix ms when Windows will shut down / restart.
    ends_at: u64,
    restart: bool,
}

type AppState<'a> = State<'a, Arc<App>>;

impl App {
    fn update(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut s = self.settings.lock();
        f(&mut s);
        self.store.save(&s)?;
        Ok(s.clone())
    }

    fn playing_id(&self) -> Option<String> {
        self.playing
            .lock()
            .as_ref()
            .filter(|p| !p.thread.is_finished())
            .map(|p| p.id.clone())
    }
}

// ── Autoclicker ─────────────────────────────────────────────────────────────

fn start_clicker(app: &AppHandle, a: &Arc<App>) -> Result<(), String> {
    let cfg = a.settings.lock().clicker.clone();
    let (handle, a2) = (app.clone(), a.clone());
    a.clicker
        .start(cfg, move |reason| {
            let _ = handle.emit("clicker-state", false);
            if reason == ExitReason::Finished {
                let clicks = a2.clicker.status().clicks;
                if a2.settings.lock().webhook.notify_clicker_finished {
                    notify(
                        &handle,
                        &a2,
                        "Autoclicker finished",
                        &format!("Done after {clicks} clicks."),
                    );
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("clicker-state", true);
    Ok(())
}

// ── Macros: playback ────────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MacroStep {
    id: String,
    index: usize,
    iteration: u32,
}

fn play_macro_by_id(app: &AppHandle, a: &Arc<App>, id: &str) -> Result<(), String> {
    stop_macro_playback(a);
    let m = a
        .macros
        .lock()
        .iter()
        .find(|m| m.id == id)
        .cloned()
        .ok_or("Macro not found")?;
    let waiter = Arc::new(Waiter::new().map_err(|e| e.to_string())?);
    let (w, handle, a2, mid) = (waiter.clone(), app.clone(), a.clone(), m.id.clone());
    let thread = std::thread::Builder::new()
        .name("macro".into())
        .spawn(move || {
            // Step highlights in the UI; fast macros are throttled so we
            // don't flood the webview with events.
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            let mut last_index = usize::MAX;
            let result = macros::play(&m, &w, |index, iteration| {
                if index != last_index && last_emit.elapsed() >= Duration::from_millis(16) {
                    let _ = handle.emit("macro-step", MacroStep { id: m.id.clone(), index, iteration });
                    last_emit = Instant::now();
                    last_index = index;
                }
            });
            let _ = handle.emit("macro-state", Option::<String>::None);
            if let Some(err) = &result.error {
                let _ = handle.emit("toast", format!("“{}” stopped: {err}", m.name));
            }
            if result.blocked > 0 {
                let _ = handle.emit("toast", "Windows blocked some input: that window runs as admin. See Settings → Admin mode.");
            }
            if result.finished && m.repeat != 0 && a2.settings.lock().webhook.notify_macro_finished {
                let runs = if m.repeat == 1 { String::new() } else { format!(" ({} runs)", result.iterations) };
                notify(&handle, &a2, &format!("Macro “{}” finished", m.name), &format!("Completed{runs}."));
            }
        })
        .map_err(|e| e.to_string())?;
    *a.playing.lock() = Some(Playing {
        id: mid.clone(),
        waiter,
        thread,
    });
    let _ = app.emit("macro-state", Some(mid));
    Ok(())
}

fn stop_macro_playback(a: &App) {
    if let Some(p) = a.playing.lock().take() {
        p.waiter.cancel();
        let _ = p.thread.join();
    }
}

// ── Macros: recording ───────────────────────────────────────────────────────

fn start_recording(app: &AppHandle, a: &Arc<App>) -> Result<(), String> {
    let mut slot = a.recorder.lock();
    if slot.is_some() {
        return Ok(());
    }
    let (opts, record_hk) = {
        let s = a.settings.lock();
        (s.record, s.record_hotkey)
    };
    let own = a.main_hwnd.load(Ordering::Relaxed);
    let progress_app = app.clone();
    let on_progress = move |steps: &[Step]| {
        let _ = progress_app.emit("recording-progress", steps);
    };
    *slot = Some(Recorder::start(
        opts,
        move |ev| {
            // The record hotkey itself is never part of the macro.
            if let Some(hk) = record_hk {
                let is_hotkey = match (hk.trigger, ev.kind) {
                    (Trigger::Key(k), InputKind::Key { key, .. }) => k == key,
                    (Trigger::Mouse(b), InputKind::Button { button, .. }) => b == button,
                    _ => false,
                };
                if is_hotkey {
                    return false;
                }
            }
            // Nothing done inside hesytools' own window gets recorded. (The
            // outline is click-through, so clicks over it reach the app below.)
            let target = match ev.kind {
                InputKind::Key { .. } => input::foreground_window(),
                _ => input::root_window_at(ev.x, ev.y),
            };
            target != own
        },
        on_progress,
    ));
    drop(slot);
    let _ = app.emit("recording-state", true);
    if a.settings.lock().recording_overlay {
        show_outline(a);
    }
    Ok(())
}

fn show_outline(a: &App) {
    let hint = a
        .settings
        .lock()
        .record_hotkey
        .map(|h| format!("  \u{b7}  press {h} to stop"))
        .unwrap_or_default();
    *a.outline.lock() = Some(engine::outline::ScreenOutline::show(&format!("REC{hint}")));
}

fn stop_recording(app: &AppHandle, a: &App) {
    let Some(rec) = a.recorder.lock().take() else {
        return;
    };
    let steps: Vec<Step> = rec.stop();
    *a.outline.lock() = None;
    let _ = app.emit("recording-state", false);
    let _ = app.emit("recording-finished", steps);
}

// ── Shared ──────────────────────────────────────────────────────────────────

fn stop_everything(app: &AppHandle, a: &App) {
    a.clicker.stop();
    stop_macro_playback(a);
    stop_recording(app, a);
}

/// Fire-and-forget Discord notification; failures surface as a UI toast.
fn notify(app: &AppHandle, a: &App, title: &str, description: &str) {
    let wh = a.settings.lock().webhook.clone();
    if wh.url.trim().is_empty() {
        return;
    }
    let n = notification(&wh, title, description);
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = webhook::send(&wh.url, &n) {
            let _ = app.emit("toast", format!("Discord: {e}"));
        }
    });
}

fn notification(wh: &WebhookSettings, title: &str, description: &str) -> webhook::Notification {
    let id = wh.mention_user_id.trim();
    webhook::Notification {
        title: title.into(),
        description: description.into(),
        color: 0x7C5CFF,
        content: (!id.is_empty()).then(|| format!("<@{id}>")),
    }
}

fn on_hotkey(app: &AppHandle, id: &str, pressed: bool) {
    let Some(a) = app.try_state::<Arc<App>>() else {
        return;
    };
    let a = a.inner().clone();
    let result = match id {
        HK_PANIC if pressed => {
            stop_everything(app, &a);
            if a.shutdown.lock().take().is_some() && engine::power::cancel().is_ok() {
                let _ = app.emit("shutdown-state", Option::<ShutdownTimer>::None);
            }
            // Watches could immediately restart a macro, so they go off too.
            let had_watches = a.settings.lock().watches.iter().any(|w| w.enabled);
            if had_watches {
                let _ = set_watches_enabled(app, &a, false);
            }
            let msg = if had_watches {
                "Panic key: everything stopped, screen watches turned off"
            } else {
                "Panic key: everything stopped"
            };
            let _ = app.emit("toast", msg);
            let _ = app.emit("settings-changed", a.settings.lock().clone());
            Ok(())
        }
        HK_CLICKER => {
            let mode = a.settings.lock().clicker_mode;
            match (mode, pressed) {
                (HotkeyMode::Toggle, true) if a.clicker.is_running() => Ok(a.clicker.stop()),
                (HotkeyMode::Toggle, true) | (HotkeyMode::Hold, true) => start_clicker(app, &a),
                (HotkeyMode::Hold, false) => Ok(a.clicker.stop()),
                _ => Ok(()),
            }
        }
        HK_RECORD if pressed => {
            if a.recorder.lock().is_some() {
                stop_recording(app, &a);
                Ok(())
            } else {
                start_recording(app, &a)
            }
        }
        _ if pressed && id.starts_with(HK_MACRO_PREFIX) => {
            let mid = &id[HK_MACRO_PREFIX.len()..];
            if a.playing_id().as_deref() == Some(mid) {
                stop_macro_playback(&a);
                Ok(())
            } else {
                play_macro_by_id(app, &a, mid)
            }
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        let _ = app.emit("toast", e);
    }
}

fn apply_hotkeys(a: &App) {
    let s = a.settings.lock().clone();
    a.hotkeys.bind(HK_CLICKER, s.clicker_hotkey);
    a.hotkeys.bind(HK_PANIC, s.panic_hotkey);
    a.hotkeys.bind(HK_RECORD, s.record_hotkey);
    a.hotkeys.clear_prefix(HK_MACRO_PREFIX);
    for m in a.macros.lock().iter() {
        if let Some(hk) = m.hotkey {
            a.hotkeys
                .bind(&format!("{HK_MACRO_PREFIX}{}", m.id), Some(hk));
        }
    }
}

/// Who already uses `hk` (other than `except`), as a human-readable name.
fn hotkey_owner(a: &App, hk: Hotkey, except: &str) -> Option<String> {
    let s = a.settings.lock();
    let globals = [
        (HK_CLICKER, s.clicker_hotkey, "the autoclicker".to_string()),
        (HK_PANIC, s.panic_hotkey, "the panic key".to_string()),
        (HK_RECORD, s.record_hotkey, "recording".to_string()),
    ];
    let macro_ids: Vec<(String, Option<Hotkey>, String)> = a
        .macros
        .lock()
        .iter()
        .map(|m| {
            (
                format!("{HK_MACRO_PREFIX}{}", m.id),
                m.hotkey,
                format!("macro “{}”", m.name),
            )
        })
        .collect();
    globals
        .into_iter()
        .map(|(id, h, n)| (id.to_string(), h, n))
        .chain(macro_ids)
        .find(|(id, h, _)| id != except && *h == Some(hk))
        .map(|(_, _, name)| name)
}

/// Choosing a key that's already in use moves it: the previous owner loses
/// it, and the UI is told (toast + refreshed settings/macros).
fn take_hotkey(app: &AppHandle, a: &App, hk: Hotkey, new_owner: &str) -> Result<(), String> {
    if hk.trigger == Trigger::Key(engine::keys::Key::ESCAPE) {
        return Err("Esc is reserved: it cancels picking and capturing".into());
    }
    let Some(owner) = hotkey_owner(a, hk, new_owner) else {
        return Ok(());
    };
    {
        let mut guard = a.settings.lock();
        let s = &mut *guard;
        for slot in [
            &mut s.clicker_hotkey,
            &mut s.panic_hotkey,
            &mut s.record_hotkey,
        ] {
            if *slot == Some(hk) {
                *slot = None;
            }
        }
        a.store.save(s)?;
    }
    let moved: Vec<Macro> = {
        let mut list = a.macros.lock();
        list.iter_mut()
            .filter(|m| m.hotkey == Some(hk) && format!("{HK_MACRO_PREFIX}{}", m.id) != new_owner)
            .map(|m| {
                m.hotkey = None;
                m.clone()
            })
            .collect()
    };
    for m in &moved {
        a.macro_store.save(m)?;
    }
    let _ = app.emit("toast", format!("{hk} moved here from {owner}"));
    let _ = app.emit("settings-changed", a.settings.lock().clone());
    let _ = app.emit("macros-changed", a.macros.lock().clone());
    Ok(())
}

// ── Screen Watch ────────────────────────────────────────────────────────────

fn sync_watches(app: &AppHandle, a: &Arc<App>) {
    let rules = a.settings.lock().watches.clone();
    let (handle, a2) = (app.clone(), Arc::downgrade(a));
    a.watches.sync(
        app,
        &rules,
        Arc::new(move |rule: &WatchRule| {
            let Some(a) = a2.upgrade() else { return };
            let _ = handle.emit("watch-hit", rule.id.clone());
            let result = match &rule.action {
                WatchAction::Notify => {
                    let _ = handle.emit("toast", format!("Screen watch “{}” triggered", rule.name));
                    Ok(())
                }
                WatchAction::RunMacro { macro_id } => play_macro_by_id(&handle, &a, macro_id),
                WatchAction::StopEverything => {
                    stop_everything(&handle, &a);
                    Ok(())
                }
            };
            if let Err(e) = result {
                let _ = handle.emit("toast", format!("Screen watch “{}”: {e}", rule.name));
            }
            if rule.discord {
                notify(
                    &handle,
                    &a,
                    &format!("Screen watch “{}” triggered", rule.name),
                    &describe_watch(rule),
                );
            }
        }),
    );
}

fn describe_watch(rule: &WatchRule) -> String {
    use engine::watch::Cmp;
    let what = match &rule.condition {
        Condition::TextAppears { text, .. } => format!("“{text}” appeared"),
        Condition::TextGone { text, .. } => format!("“{text}” disappeared"),
        Condition::Number { op, value } => {
            let op = match op {
                Cmp::AtLeast => "≥",
                Cmp::AtMost => "≤",
                Cmp::Equal => "=",
            };
            format!("The number reached {op} {value}")
        }
        Condition::ColorAt { .. } | Condition::ColorAppears { .. } => "The colour appeared".into(),
        Condition::ImageAppears { .. } => "The image appeared".into(),
        Condition::Changes => "The area changed".into(),
    };
    let action = match &rule.action {
        WatchAction::Notify => String::new(),
        WatchAction::RunMacro { .. } => " Running its macro.".into(),
        WatchAction::StopEverything => " Stopped everything.".into(),
    };
    format!("{what}.{action}")
}

fn set_watches_enabled(app: &AppHandle, a: &Arc<App>, on: bool) -> Result<(), String> {
    a.update(|s| s.watches.iter_mut().for_each(|w| w.enabled = on))?;
    sync_watches(app, a);
    Ok(())
}

// ── Commands: settings & clicker ────────────────────────────────────────────

#[tauri::command]
fn get_settings(a: AppState) -> Settings {
    a.settings.lock().clone()
}

#[tauri::command]
fn set_clicker_config(app: AppHandle, a: AppState, config: ClickerConfig) -> Result<(), String> {
    a.update(|s| s.clicker = config)?;
    // Apply live: restart a running clicker with the new settings.
    if a.clicker.is_running() {
        a.clicker.stop();
        start_clicker(&app, &a)?;
    }
    Ok(())
}

#[tauri::command]
fn set_clicker_mode(a: AppState, mode: HotkeyMode) -> Result<(), String> {
    a.update(|s| s.clicker_mode = mode).map(|_| ())
}

#[tauri::command]
fn set_record_options(a: AppState, options: RecordOptions) -> Result<(), String> {
    a.update(|s| s.record = options).map(|_| ())
}

/// Set one of the global hotkeys: "clicker", "panic" or "record".
#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    a: AppState,
    id: String,
    hotkey: Option<String>,
) -> Result<Settings, String> {
    let hk: Option<Hotkey> = hotkey.as_deref().map(str::parse).transpose()?;
    if let Some(h) = hk {
        take_hotkey(&app, &a, h, &id)?;
    }
    let s = a.update(|s| match id.as_str() {
        HK_CLICKER => s.clicker_hotkey = hk,
        HK_PANIC => s.panic_hotkey = hk,
        HK_RECORD => s.record_hotkey = hk,
        _ => {}
    })?;
    apply_hotkeys(&a);
    Ok(s)
}

#[tauri::command]
async fn capture_hotkey(a: AppState<'_>) -> Result<Option<String>, String> {
    let mgr = a.hotkeys.clone();
    // Don't let the key being captured also fire whatever it's bound to.
    mgr.suspend(true);
    let got = tauri::async_runtime::spawn_blocking(move || mgr.capture_next()).await;
    a.hotkeys.suspend(false);
    Ok(got.map_err(|e| e.to_string())?.map(|h| h.to_string()))
}

#[tauri::command]
fn cancel_capture(a: AppState) {
    a.hotkeys.cancel_capture();
}

#[tauri::command]
async fn pick_point() -> Result<Option<(i32, i32)>, String> {
    tauri::async_runtime::spawn_blocking(hooks::pick_point)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_pick() {
    hooks::cancel_pick();
}

#[tauri::command]
fn clicker_toggle(app: AppHandle, a: AppState) -> Result<bool, String> {
    if a.clicker.is_running() {
        a.clicker.stop();
        Ok(false)
    } else {
        start_clicker(&app, &a)?;
        Ok(true)
    }
}

#[tauri::command]
fn clicker_status(a: AppState) -> ClickerStatus {
    a.clicker.status()
}

// ── Commands: macros ────────────────────────────────────────────────────────

#[tauri::command]
fn list_macros(a: AppState) -> Vec<Macro> {
    a.macros.lock().clone()
}

#[tauri::command]
fn save_macro(app: AppHandle, a: AppState, mut entry: Macro) -> Result<Macro, String> {
    if entry.id.is_empty() {
        entry.id = macro_store::new_id();
    }
    let hk_id = format!("{HK_MACRO_PREFIX}{}", entry.id);
    if let Some(h) = entry.hotkey {
        take_hotkey(&app, &a, h, &hk_id)?;
    }
    entry.name = entry.name.trim().to_string();
    if entry.name.is_empty() {
        entry.name = "Untitled macro".into();
    }
    entry.updated_at = macro_store::now_ms();
    a.macro_store.save(&entry)?;
    {
        let mut list = a.macros.lock();
        list.retain(|m| m.id != entry.id);
        list.insert(0, entry.clone());
    }
    apply_hotkeys(&a);
    Ok(entry)
}

/// Import old hesytools v4 macro files: (file name, JSON contents) pairs.
#[tauri::command]
fn import_macros(a: AppState, files: Vec<(String, String)>) -> Result<Vec<Macro>, String> {
    let mut imported = Vec::new();
    let mut errors = Vec::new();
    for (file, json) in files {
        let name = file.trim_end_matches(".json").to_string();
        match engine::import::import_v4(&json, &name) {
            Ok(mut m) => {
                m.id = macro_store::new_id();
                // Keep the old hotkey only if it's still free.
                if m.hotkey.is_some_and(|h| hotkey_owner(&a, h, "").is_some()) {
                    m.hotkey = None;
                }
                m.updated_at = macro_store::now_ms();
                a.macro_store.save(&m)?;
                a.macros.lock().insert(0, m.clone());
                imported.push(m);
            }
            Err(e) => errors.push(format!("{file}: {e}")),
        }
    }
    apply_hotkeys(&a);
    if imported.is_empty() && !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(imported)
}

#[tauri::command]
fn delete_macro(a: AppState, id: String) -> Result<(), String> {
    if a.playing_id().as_deref() == Some(id.as_str()) {
        stop_macro_playback(&a);
    }
    a.macro_store.delete(&id)?;
    a.macros.lock().retain(|m| m.id != id);
    apply_hotkeys(&a);
    Ok(())
}

#[tauri::command]
fn play_macro(app: AppHandle, a: AppState, id: String) -> Result<(), String> {
    play_macro_by_id(&app, &a, &id)
}

#[tauri::command]
fn stop_macro(a: AppState) {
    stop_macro_playback(&a);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityStatus {
    playing: Option<String>,
    recording: bool,
}

#[tauri::command]
fn activity_status(a: AppState) -> ActivityStatus {
    ActivityStatus {
        playing: a.playing_id(),
        recording: a.recorder.lock().is_some(),
    }
}

#[tauri::command]
async fn recording_start(app: AppHandle, a: AppState<'_>) -> Result<(), String> {
    start_recording(&app, &a)
}

#[tauri::command]
async fn recording_stop(app: AppHandle, a: AppState<'_>) -> Result<(), String> {
    stop_recording(&app, &a);
    Ok(())
}

#[tauri::command]
fn set_recording_overlay(a: AppState, on: bool) -> Result<(), String> {
    a.update(|s| s.recording_overlay = on).map(|_| ())
}

/// Monitor rectangles in physical pixels, for drawing the click map.
#[tauri::command]
fn screen_layout(app: AppHandle) -> Vec<Region> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| Region {
            x: m.position().x,
            y: m.position().y,
            w: m.size().width,
            h: m.size().height,
        })
        .collect()
}

// ── Commands: screen watch ──────────────────────────────────────────────────

/// Hide hesytools, let the user drag a box on screen, then come back.
#[tauri::command]
async fn pick_region(app: AppHandle) -> Result<Option<Region>, String> {
    let win = app.get_webview_window("main");
    if let Some(w) = &win {
        let _ = w.minimize();
    }
    // Let the minimize animation finish so our own window isn't in the way.
    tokio_sleep(250).await;
    let region = tauri::async_runtime::spawn_blocking(picker::pick_region)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(w) = &win {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    Ok(region)
}

async fn tokio_sleep(ms: u64) {
    let _ =
        tauri::async_runtime::spawn_blocking(move || std::thread::sleep(Duration::from_millis(ms)))
            .await;
}

/// Drag a box around a button/icon; returns it as an image to look for.
#[tauri::command]
async fn capture_template(app: AppHandle) -> Result<Option<engine::imagematch::Template>, String> {
    let Some(region) = pick_region(app).await? else {
        return Ok(None);
    };
    if region.w > 400 || region.h > 400 {
        return Err("That's quite big for an image to find; select just the button or icon".into());
    }
    let frame = engine::capture::capture(region)?;
    Ok(Some(engine::imagematch::Template::from_frame(&frame)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewerCheck {
    #[serde(flatten)]
    check: Check,
    width: u32,
    height: u32,
}

/// One live-viewer tick: capture + evaluate. OCR only re-runs when the
/// region's pixels changed since the previous tick.
#[tauri::command]
async fn viewer_check(
    a: AppState<'_>,
    id: String,
    region: Region,
    condition: Condition,
) -> Result<ViewerCheck, String> {
    let key = serde_json::to_string(&(region, &condition)).unwrap_or_default();
    let a = a.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut viewers = a.viewers.lock();
        if viewers.get(&id).is_none_or(|(k, _)| *k != key) {
            viewers.insert(id.clone(), (key, Watcher::new(region, condition)?));
        }
        let (_, w) = viewers.get_mut(&id).expect("just set");
        let check = w.check()?;
        let (width, height) = w.frame.as_ref().map_or((0, 0), |f| (f.w, f.h));
        Ok(ViewerCheck {
            check,
            width,
            height,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The viewer's latest capture as raw bytes: u32 width, u32 height (LE),
/// then RGBA pixels, scaled down to at most `max_width` wide.
#[tauri::command]
fn viewer_frame(a: AppState, id: String, max_width: u32) -> tauri::ipc::Response {
    let viewers = a.viewers.lock();
    let Some(frame) = viewers.get(&id).and_then(|(_, w)| w.frame.as_ref()) else {
        return tauri::ipc::Response::new(Vec::new());
    };
    let t = frame.thumbnail(max_width.clamp(16, 1600));
    let mut out = Vec::with_capacity(8 + t.bgra.len());
    out.extend_from_slice(&t.w.to_le_bytes());
    out.extend_from_slice(&t.h.to_le_bytes());
    for px in t.bgra.chunks_exact(4) {
        out.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }
    tauri::ipc::Response::new(out)
}

#[tauri::command]
fn viewer_stop(a: AppState, id: String) {
    a.viewers.lock().remove(&id);
}

#[tauri::command]
fn save_watches(
    app: AppHandle,
    a: AppState,
    mut watches: Vec<WatchRule>,
) -> Result<Vec<WatchRule>, String> {
    for w in &mut watches {
        if w.id.is_empty() {
            w.id = macro_store::new_id().replacen('m', "w", 1);
        }
    }
    let s = a.update(|s| s.watches = watches)?;
    sync_watches(&app, &a);
    Ok(s.watches)
}

// ── Commands: webhook & system ──────────────────────────────────────────────

#[tauri::command]
fn set_webhook(a: AppState, webhook: WebhookSettings) -> Result<(), String> {
    if !webhook.url.trim().is_empty() {
        webhook::validate_url(&webhook.url)?;
    }
    a.update(|s| s.webhook = webhook).map(|_| ())
}

#[tauri::command]
async fn test_webhook(webhook: WebhookSettings) -> Result<(), String> {
    let n = notification(
        &webhook,
        "hesytools is connected",
        "Notifications will show up here.",
    );
    tauri::async_runtime::spawn_blocking(move || webhook::send(&webhook.url, &n))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    elevated: bool,
    version: String,
}

#[tauri::command]
fn system_info(app: AppHandle) -> SystemInfo {
    SystemInfo {
        elevated: elevation::is_elevated(),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn relaunch_as_admin(app: AppHandle, a: AppState) -> Result<(), String> {
    elevation::relaunch_as_admin()?;
    stop_everything(&app, &a);
    app.exit(0);
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn quit(app: &AppHandle) {
    if let Some(a) = app.try_state::<Arc<App>>() {
        stop_everything(app, &a);
        a.watches.stop_all();
    }
    app.exit(0);
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show hesytools", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop everything", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &stop,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("hesytools")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => show_main(app),
            "stop" => {
                if let Some(a) = app.try_state::<Arc<App>>() {
                    stop_everything(app, &a);
                }
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: TrayButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
fn startup_enabled() -> bool {
    engine::startup::is_enabled()
}

#[tauri::command]
fn set_startup_enabled(on: bool) -> Result<(), String> {
    engine::startup::set_enabled(on)
}

#[tauri::command]
fn set_startup_macro(a: AppState, id: Option<String>) -> Result<(), String> {
    a.update(|s| s.startup_macro = id).map(|_| ())
}

/// Eyedropper: the next click anywhere on screen returns that pixel's colour
/// (the click itself is swallowed). Esc cancels.
#[tauri::command]
async fn pick_color() -> Result<Option<[u8; 3]>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some((x, y)) = hooks::pick_point() else {
            return Ok(None);
        };
        let f = engine::capture::capture(Region { x, y, w: 1, h: 1 })?;
        Ok(f.pixel(0, 0))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Shutdown timer ──────────────────────────────────────────────────────────

#[tauri::command]
fn schedule_shutdown(app: AppHandle, a: AppState, minutes: f64, restart: bool) -> Result<ShutdownTimer, String> {
    if !(minutes > 0.0) {
        return Err("Pick how many minutes to wait".into());
    }
    let secs = (minutes * 60.0).round() as u64;
    engine::power::schedule(secs, restart)?;
    let timer = ShutdownTimer { ends_at: macro_store::now_ms() + secs * 1000, restart };
    *a.shutdown.lock() = Some(timer);
    let _ = app.emit("shutdown-state", Some(timer));
    Ok(timer)
}

#[tauri::command]
fn cancel_shutdown(app: AppHandle, a: AppState) -> Result<(), String> {
    a.shutdown.lock().take();
    let _ = app.emit("shutdown-state", Option::<ShutdownTimer>::None);
    // If Windows had nothing pending (e.g. it was cancelled elsewhere), fine.
    let _ = engine::power::cancel();
    Ok(())
}

#[tauri::command]
fn shutdown_status(a: AppState) -> Option<ShutdownTimer> {
    let mut slot = a.shutdown.lock();
    if slot.is_some_and(|t| t.ends_at <= macro_store::now_ms()) {
        *slot = None;
    }
    *slot
}

// ── Updates ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    version: String,
    current_version: String,
    notes: Option<String>,
    date: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
    done: bool,
}

/// Ask GitHub Releases whether there's a newer version. Updates are only
/// accepted if they're signed with hesy's key (pubkey in tauri.conf.json).
#[tauri::command]
async fn check_update(app: AppHandle, a: AppState<'_>) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let mut builder = app.updater_builder();
    // For testing against a local server; signatures are still checked.
    if let Ok(url) = std::env::var("HESYTOOLS_UPDATE_URL") {
        let url = url.parse().map_err(|e| format!("Bad HESYTOOLS_UPDATE_URL: {e}"))?;
        builder = builder.endpoints(vec![url]).map_err(|e| e.to_string())?;
    }
    let update = builder
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Couldn't check for updates: {e}"))?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        date: u.date.map(|d| d.date().to_string()),
    });
    *a.pending_update.lock() = update;
    Ok(info)
}

/// Download and run the new installer. It closes hesytools, upgrades it in
/// place and reopens it. Macros and settings live in %APPDATA% and aren't
/// touched.
#[tauri::command]
async fn install_update(app: AppHandle, a: AppState<'_>) -> Result<(), String> {
    let update = a.pending_update.lock().take().ok_or("No update is ready to install")?;
    stop_everything(&app, &a);
    a.watches.stop_all();
    let mut downloaded = 0u64;
    let progress = app.clone();
    let finished = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress.emit("update-progress", UpdateProgress { downloaded, total, done: false });
            },
            move || {
                let _ = finished.emit("update-progress", UpdateProgress { downloaded: 0, total: None, done: true });
            },
        )
        .await
        .map_err(|e| format!("Update failed: {e}"))
}

#[tauri::command]
fn set_auto_check_updates(a: AppState, on: bool) -> Result<(), String> {
    a.update(|s| s.auto_check_updates = on).map(|_| ())
}

#[tauri::command]
fn set_close_to_tray(a: AppState, on: bool) -> Result<(), String> {
    a.update(|s| s.close_to_tray = on).map(|_| ())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    quit(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    elevation::wait_for_previous_instance();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch just brings the existing window forward; two
            // instances would both react to every hotkey.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let store = Store::new(&dir);
            let macro_store = MacroStore::new(&dir);
            let handle = app.handle().clone();
            let hotkeys = HotkeyManager::start(move |id, pressed| on_hotkey(&handle, id, pressed));
            let main_hwnd = app
                .get_webview_window("main")
                .and_then(|w| w.hwnd().ok())
                .map_or(0, |h| h.0 as isize);
            let a = Arc::new(App {
                clicker: Clicker::new(),
                hotkeys,
                settings: Mutex::new(store.load()),
                store,
                macros: Mutex::new(macro_store.load_all()),
                macro_store,
                playing: Mutex::new(None),
                recorder: Mutex::new(None),
                main_hwnd: AtomicIsize::new(main_hwnd),
                outline: Mutex::new(None),
                viewers: Mutex::new(Default::default()),
                watches: WatchRunner::default(),
                pending_update: Mutex::new(None),
                shutdown: Mutex::new(None),
            });
            apply_hotkeys(&a);
            sync_watches(app.handle(), &a);
            app.manage(a);
            build_tray(app)?;
            if engine::startup::launched_at_startup() {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            let startup_macro = app
                .state::<Arc<App>>()
                .settings
                .lock()
                .startup_macro
                .clone();
            if let Some(id) = startup_macro {
                // A short grace period so a startup macro never fires the
                // instant you sign in.
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    if let Some(a) = handle.try_state::<Arc<App>>() {
                        if let Err(e) = play_macro_by_id(&handle, &a, &id) {
                            let _ = handle.emit("toast", format!("Startup macro: {e}"));
                        }
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides it to the tray so hotkeys and
            // watches keep running; "Quit" in the tray menu really exits.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                let app = window.app_handle();
                let to_tray = app
                    .try_state::<Arc<App>>()
                    .is_some_and(|a| a.settings.lock().close_to_tray);
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    quit(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_clicker_config,
            set_clicker_mode,
            set_record_options,
            set_hotkey,
            capture_hotkey,
            cancel_capture,
            pick_point,
            cancel_pick,
            clicker_toggle,
            clicker_status,
            list_macros,
            save_macro,
            delete_macro,
            import_macros,
            play_macro,
            stop_macro,
            activity_status,
            recording_start,
            recording_stop,
            set_recording_overlay,
            set_close_to_tray,
            schedule_shutdown,
            cancel_shutdown,
            shutdown_status,
            check_update,
            install_update,
            set_auto_check_updates,
            startup_enabled,
            set_startup_enabled,
            set_startup_macro,
            pick_color,
            quit_app,
            screen_layout,
            pick_region,
            capture_template,
            viewer_check,
            viewer_frame,
            viewer_stop,
            save_watches,
            set_webhook,
            test_webhook,
            system_info,
            relaunch_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
