mod settings;

use engine::clicker::{Clicker, ClickerConfig, ClickerStatus, ExitReason};
use engine::hotkey::{Hotkey, HotkeyManager};
use engine::{elevation, hooks, webhook};
use parking_lot::Mutex;
use serde::Serialize;
use settings::{HotkeyMode, Settings, Store, WebhookSettings};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

const HK_CLICKER: &str = "clicker";
const HK_PANIC: &str = "panic";

struct App {
    clicker: Clicker,
    hotkeys: Arc<HotkeyManager>,
    settings: Mutex<Settings>,
    store: Store,
}

type AppState<'a> = State<'a, Arc<App>>;

impl App {
    fn update(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut s = self.settings.lock();
        f(&mut s);
        self.store.save(&s)?;
        Ok(s.clone())
    }
}

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

fn stop_everything(a: &App) {
    a.clicker.stop();
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
    match id {
        HK_PANIC if pressed => {
            stop_everything(&a);
            let _ = app.emit("toast", "Panic hotkey: everything stopped");
        }
        HK_CLICKER => {
            let mode = a.settings.lock().clicker_mode;
            let result = match (mode, pressed) {
                (HotkeyMode::Toggle, true) if a.clicker.is_running() => Ok(a.clicker.stop()),
                (HotkeyMode::Toggle, true) | (HotkeyMode::Hold, true) => start_clicker(app, &a),
                (HotkeyMode::Hold, false) => Ok(a.clicker.stop()),
                _ => Ok(()),
            };
            if let Err(e) = result {
                eprintln!("clicker: {e}");
                let _ = app.emit("toast", e);
            }
        }
        _ => {}
    }
}

fn apply_hotkeys(a: &App, s: &Settings) {
    a.hotkeys.bind(HK_CLICKER, s.clicker_hotkey);
    a.hotkeys.bind(HK_PANIC, s.panic_hotkey);
}

// ── Commands ────────────────────────────────────────────────────────────────

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
fn set_hotkey(a: AppState, id: String, hotkey: Option<String>) -> Result<Settings, String> {
    let hk: Option<Hotkey> = hotkey.as_deref().map(str::parse).transpose()?;
    let s = a.update(|s| {
        let (this, other) = match id.as_str() {
            HK_CLICKER => (&mut s.clicker_hotkey, s.panic_hotkey),
            _ => (&mut s.panic_hotkey, s.clicker_hotkey),
        };
        *this = hk;
        if hk.is_some() && hk == other {
            *this = None;
        }
    })?;
    let saved = if id == HK_CLICKER { s.clicker_hotkey } else { s.panic_hotkey };
    if hk.is_some() && saved.is_none() {
        return Err("That hotkey is already used by the other action".into());
    }
    apply_hotkeys(&a, &s);
    Ok(s)
}

#[tauri::command]
async fn capture_hotkey(a: AppState<'_>) -> Result<Option<String>, String> {
    let mgr = a.hotkeys.clone();
    // Don't let the hotkey being captured also fire its old action.
    for id in [HK_CLICKER, HK_PANIC] {
        mgr.bind(id, None);
    }
    let got = tauri::async_runtime::spawn_blocking(move || mgr.capture_next())
        .await
        .map_err(|e| e.to_string())?;
    apply_hotkeys(&a, &a.settings.lock());
    Ok(got.map(|h| h.to_string()))
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
    stop_everything(&a);
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
            let store = Store::new(&app.path().app_config_dir()?);
            let settings = store.load();
            let handle = app.handle().clone();
            let hotkeys = HotkeyManager::start(move |id, pressed| on_hotkey(&handle, id, pressed));
            let a = Arc::new(App {
                clicker: Clicker::new(),
                hotkeys,
                settings: Mutex::new(settings.clone()),
                store,
            });
            apply_hotkeys(&a, &settings);
            app.manage(a);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_clicker_config,
            set_clicker_mode,
            set_hotkey,
            capture_hotkey,
            cancel_capture,
            pick_point,
            cancel_pick,
            clicker_toggle,
            clicker_status,
            set_webhook,
            test_webhook,
            system_info,
            relaunch_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
