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
    /// Shut down (or restart) after a one-minute countdown, so the panic
    /// key or Cancel can still stop it.
    Shutdown {
        restart: bool,
    },
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
    pub region: Region,
    pub condition: Condition,
    pub poll_ms: f64,
    pub action: WatchAction,
    /// Also send a Discord alert when it triggers.
    pub discord: bool,
    /// Don't trigger again within this long.
    pub cooldown_ms: f64,
}

impl Default for WatchRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "New watch".into(),
            enabled: false,
            region: Region::default(),
            condition: Condition::TextAppears {
                text: String::new(),
                digits_after: None,
            },
            poll_ms: 250.0,
            action: WatchAction::Notify,
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
        }
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
        ];
        for (json, want) in cases {
            assert_eq!(serde_json::from_str::<WatchAction>(json).unwrap(), want, "{json}");
        }
    }
}
