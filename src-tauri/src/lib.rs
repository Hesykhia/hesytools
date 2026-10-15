mod macro_store;
mod settings;

use engine::clicker::{Clicker, ClickerConfig, ClickerStatus, ExitReason};
use engine::hooks::InputKind;
use engine::hotkey::{Hotkey, HotkeyManager, Trigger};
use engine::macros::{self, Macro, RecordOptions, Recorder, Step};
use engine::timer::Waiter;
use engine::{elevation, hooks, input, webhook};
use macro_store::MacroStore;
use parking_lot::Mutex;
use serde::Serialize;
use settings::{HotkeyMode, Settings, Store, WebhookSettings};
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

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
        self.playing.lock().as_ref().filter(|p| !p.thread.is_finished()).map(|p| p.id.clone())
    }
}

// ── Quick Clicker ───────────────────────────────────────────────────────────

fn start_clicker(app: &AppHandle, a: &Arc<App>) -> Result<(), String> {
    let cfg = a.settings.lock().clicker.clone();
    let (handle, a2) = (app.clone(), a.clone());
    a.clicker
        .start(cfg, move |reason| {
            let _ = handle.emit("clicker-state", false);
            if reason == ExitReason::Finished {
                let clicks = a2.clicker.status().clicks;
                if a2.settings.lock().webhook.notify_clicker_finished {
                    notify(&handle, &a2, "Quick Clicker finished", &format!("Done after {clicks} clicks."));
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
    let m = a.macros.lock().iter().find(|m| m.id == id).cloned().ok_or("Macro not found")?;
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
            if result.blocked > 0 {
                let _ = handle.emit("toast", "Windows blocked some input: that window runs as admin. See Settings → Admin mode.");
            }
            if result.finished && m.repeat != 0 && a2.settings.lock().webhook.notify_macro_finished {
                let runs = if m.repeat == 1 { String::new() } else { format!(" ({} runs)", result.iterations) };
                notify(&handle, &a2, &format!("Macro “{}” finished", m.name), &format!("Completed{runs}."));
            }
        })
        .map_err(|e| e.to_string())?;
    *a.playing.lock() = Some(Playing { id: mid.clone(), waiter, thread });
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
    *slot = Some(Recorder::start(opts, move |ev| {
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
        // Nothing done inside hesytools' own window gets recorded.
        match ev.kind {
            InputKind::Key { .. } => input::foreground_window() != own,
            _ => input::root_window_at(ev.x, ev.y) != own,
        }
    }));
    let _ = app.emit("recording-state", true);
    Ok(())
}

fn stop_recording(app: &AppHandle, a: &App) {
    let Some(rec) = a.recorder.lock().take() else { return };
    let steps: Vec<Step> = rec.stop();
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
    let Some(a) = app.try_state::<Arc<App>>() else { return };
    let a = a.inner().clone();
    let result = match id {
        HK_PANIC if pressed => {
            stop_everything(app, &a);
            let _ = app.emit("toast", "Panic key: everything stopped");
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
            a.hotkeys.bind(&format!("{HK_MACRO_PREFIX}{}", m.id), Some(hk));
        }
    }
}

/// Who already uses `hk` (other than `except`), as a human-readable name.
fn hotkey_owner(a: &App, hk: Hotkey, except: &str) -> Option<String> {
    let s = a.settings.lock();
    let globals = [
        (HK_CLICKER, s.clicker_hotkey, "the Quick Clicker".to_string()),
        (HK_PANIC, s.panic_hotkey, "the panic key".to_string()),
        (HK_RECORD, s.record_hotkey, "recording".to_string()),
    ];
    let macro_ids: Vec<(String, Option<Hotkey>, String)> = a
        .macros
        .lock()
        .iter()
        .map(|m| (format!("{HK_MACRO_PREFIX}{}", m.id), m.hotkey, format!("macro “{}”", m.name)))
        .collect();
    globals
        .into_iter()
        .map(|(id, h, n)| (id.to_string(), h, n))
        .chain(macro_ids)
        .find(|(id, h, _)| id != except && *h == Some(hk))
        .map(|(_, _, name)| name)
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
fn set_hotkey(a: AppState, id: String, hotkey: Option<String>) -> Result<Settings, String> {
    let hk: Option<Hotkey> = hotkey.as_deref().map(str::parse).transpose()?;
    if let Some(owner) = hk.and_then(|h| hotkey_owner(&a, h, &id)) {
        return Err(format!("That key is already used by {owner}"));
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
    tauri::async_runtime::spawn_blocking(hooks::pick_point).await.map_err(|e| e.to_string())
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
fn save_macro(a: AppState, mut entry: Macro) -> Result<Macro, String> {
    if entry.id.is_empty() {
        entry.id = macro_store::new_id();
    }
    let hk_id = format!("{HK_MACRO_PREFIX}{}", entry.id);
    if let Some(owner) = entry.hotkey.and_then(|h| hotkey_owner(&a, h, &hk_id)) {
        return Err(format!("That key is already used by {owner}"));
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
    ActivityStatus { playing: a.playing_id(), recording: a.recorder.lock().is_some() }
}

#[tauri::command]
fn recording_start(app: AppHandle, a: AppState) -> Result<(), String> {
    start_recording(&app, &a)
}

#[tauri::command]
fn recording_stop(app: AppHandle, a: AppState) {
    stop_recording(&app, &a);
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
    let n = notification(&webhook, "hesytools is connected", "Notifications will show up here.");
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    elevation::wait_for_previous_instance();
    tauri::Builder::default()
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
            });
            apply_hotkeys(&a);
            app.manage(a);
            Ok(())
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
            play_macro,
            stop_macro,
            activity_status,
            recording_start,
            recording_stop,
            set_webhook,
            test_webhook,
            system_info,
            relaunch_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
