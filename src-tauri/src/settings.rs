//! Persistent user settings, stored as JSON in the app config directory.

use engine::capture::Region;
use engine::clicker::ClickerConfig;
use engine::hotkey::Hotkey;
use engine::keys::{Key, MouseButton};
use engine::macros::RecordOptions;
use engine::watch::Condition;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyMode {
    /// Press once to start, again to stop.
    #[default]
    Toggle,
    /// Runs only while the hotkey is held.
    Hold,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WebhookSettings {
    pub url: String,
    /// Optional Discord user ID to @mention in notifications.
    pub mention_user_id: String,
    pub notify_clicker_finished: bool,
    pub notify_macro_finished: bool,
}

impl Default for WebhookSettings {
    fn default() -> Self {
        Self {
            url: String::new(),
            mention_user_id: String::new(),
            notify_clicker_finished: true,
            notify_macro_finished: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WatchAction {
    /// Only alert (Discord, if set up) / show in the app.
    Notify,
    #[serde(rename_all = "camelCase")]
    RunMacro {
        macro_id: String,
    },
    StopEverything,
    /// Click `times` times, then put the cursor back.
    Click {
        button: MouseButton,
        times: u32,
        at: ClickAt,
    },
    PressKey {
        key: Key,
        times: u32,
    },
    /// Pause playing macros while it matches; they carry on once it's gone.
    PauseMacros,
    /// Shut down (or restart) after a one-minute countdown, so the panic
    /// key or Cancel can still stop it.
    Shutdown {
        restart: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCode {
    pub name: String,
    pub code: String,
    /// Unix ms.
    pub made_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClickAt {
    /// On the thing that was found (image, colour), else the area's middle.
    Found,
    Point { x: i32, y: i32 },
}

/// A background rule: when `condition` becomes true in `region`, do `action`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// With `window` set, relative to that window's top-left corner.
    pub region: Region,
    /// Only look at this app (like OBS window capture), so anything on top
    /// of it, e.g. a taskbar preview, can't trigger the watch.
    pub window: Option<engine::window::WindowTarget>,
    pub condition: Condition,
    pub poll_ms: f64,
    pub action: WatchAction,
    /// More things to look for in the same area. Each one triggers on its
    /// own (and a click goes to whichever was found).
    pub also: Vec<Condition>,
    /// Also send a Discord alert when it triggers.
    pub discord: bool,
    /// Don't trigger again within this long.
    pub cooldown_ms: f64,
}

impl WatchRule {
    /// Everything this watch looks for: the main condition, then `also`.
    pub fn items(&self) -> impl Iterator<Item = &Condition> {
        std::iter::once(&self.condition).chain(&self.also)
    }
}

impl Default for WatchRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "New watch".into(),
            enabled: false,
            region: Region::default(),
            window: None,
            condition: Condition::TextAppears {
                text: String::new(),
                digits_after: None,
            },
            poll_ms: 250.0,
            action: WatchAction::Notify,
            also: Vec::new(),
            discord: false,
            cooldown_ms: 5000.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub clicker: ClickerConfig,
    pub clicker_hotkey: Option<Hotkey>,
    pub clicker_mode: HotkeyMode,
    /// Stops everything, always.
    pub panic_hotkey: Option<Hotkey>,
    /// Starts/stops macro recording from anywhere.
    pub record_hotkey: Option<Hotkey>,
    pub record: RecordOptions,
    /// Draw a red, click-through outline around every monitor while recording.
    pub recording_overlay: bool,
    /// Macro to play (after a few seconds) whenever hesytools starts.
    pub startup_macro: Option<String>,
    /// Closing the window hides it to the tray (hotkeys and watches keep
    /// working) instead of quitting.
    pub close_to_tray: bool,
    /// Look for a new version on GitHub when the app starts.
    pub auto_check_updates: bool,
    pub webhook: WebhookSettings,
    pub watches: Vec<WatchRule>,
    /// Signed code that unlocks the Dev tab (see dev.rs).
    pub dev_code: Option<String>,
    /// Shared secret between hesytools and its browser helper.
    pub helper_token: String,
    /// Codes the owner has handed out (owner's PC only).
    pub issued_codes: Vec<IssuedCode>,
    pub soul: crate::soul::SoulConfig,
    /// GUI scale per page, 1 (smallest) to 5; 3 is normal size.
    pub ui_scale: UiScale,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            clicker: ClickerConfig::default(),
            clicker_hotkey: Key::from_name("F6").map(Hotkey::key),
            clicker_mode: HotkeyMode::Toggle,
            panic_hotkey: Key::from_name("F8").map(Hotkey::key),
            record_hotkey: Key::from_name("F9").map(Hotkey::key),
            record: RecordOptions::default(),
            recording_overlay: true,
            startup_macro: None,
            close_to_tray: true,
            auto_check_updates: true,
            webhook: WebhookSettings::default(),
            watches: Vec::new(),
            dev_code: None,
            helper_token: String::new(),
            issued_codes: Vec::new(),
            soul: Default::default(),
            ui_scale: UiScale::default(),
        }
    }
}

/// How big each page is drawn, like Minecraft's GUI Scale.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct UiScale {
    pub macros: u8,
    pub watch: u8,
    pub dev: u8,
    pub settings: u8,
}

impl Default for UiScale {
    fn default() -> Self {
        Self { macros: 3, watch: 3, dev: 3, settings: 3 }
    }
}

impl UiScale {
    /// Set one page's scale (clamped to 1..=5). False for an unknown page.
    pub fn set(&mut self, page: &str, level: u8) -> bool {
        let level = level.clamp(1, 5);
        match page {
            "macros" => self.macros = level,
            "watch" => self.watch = level,
            "dev" => self.dev = level,
            "settings" => self.settings = level,
            _ => return false,
        }
        true
    }
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("settings.json"),
        }
    }

    /// Missing or unreadable files fall back to defaults; a corrupt file is
    /// kept as `settings.json.bad` rather than silently overwritten.
    pub fn load(&self) -> Settings {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Settings::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("settings: {e}; starting from defaults");
            let _ = std::fs::rename(&self.path, self.path.with_extension("json.bad"));
            Settings::default()
        })
    }

    /// Atomic write: a crash mid-save can't leave a half-written file.
    pub fn save(&self, s: &Settings) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_actions_match_the_ui() {
        let cases = [
            (r#"{"kind":"click","button":"LeftMouse","times":3,"at":{"kind":"found"}}"#,
             WatchAction::Click { button: MouseButton::Left, times: 3, at: ClickAt::Found }),
            (r#"{"kind":"click","button":"RightMouse","times":1,"at":{"kind":"point","x":-1500,"y":40}}"#,
             WatchAction::Click { button: MouseButton::Right, times: 1, at: ClickAt::Point { x: -1500, y: 40 } }),
            (r#"{"kind":"pressKey","key":"E","times":2}"#,
             WatchAction::PressKey { key: Key::from_name("E").unwrap(), times: 2 }),
            (r#"{"kind":"shutdown","restart":true}"#, WatchAction::Shutdown { restart: true }),
            (r#"{"kind":"pauseMacros"}"#, WatchAction::PauseMacros),
        ];
        for (json, want) in cases {
            assert_eq!(serde_json::from_str::<WatchAction>(json).unwrap(), want, "{json}");
        }
    }

    #[test]
    fn gui_scale_defaults_to_3_and_stays_in_range() {
        // Settings saved before GUI scale existed get the normal size.
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.ui_scale, UiScale::default());
        let mut scale = UiScale::default();
        assert!(scale.set("dev", 9));
        assert!(scale.set("macros", 0));
        assert!(!scale.set("nowhere", 2));
        assert_eq!((scale.dev, scale.macros, scale.watch), (5, 1, 3));
    }
}
