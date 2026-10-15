//! Global hotkeys built on the low-level hooks (not `RegisterHotKey`), so they
//! can be mouse buttons, report releases (for hold-to-run), and still work
//! while a fullscreen game has focus.

use crate::hooks::{self, InputKind};
use crate::keys::{Key, Modifier, MouseButton};
use crossbeam_channel::Sender;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    Key(Key),
    Mouse(MouseButton),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub trigger: Trigger,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
}

impl Hotkey {
    pub fn key(key: Key) -> Self {
        Self::new(Trigger::Key(key))
    }

    pub fn new(trigger: Trigger) -> Self {
        Self { trigger, ctrl: false, shift: false, alt: false, win: false }
    }

    fn mods(&self) -> [bool; 4] {
        [self.ctrl, self.shift, self.alt, self.win]
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in self.mods().iter().zip(["Ctrl", "Shift", "Alt", "Win"]) {
            if *on {
                write!(f, "{name}+")?;
            }
        }
        match self.trigger {
            Trigger::Key(k) => write!(f, "{k}"),
            Trigger::Mouse(b) => f.write_str(b.name()),
        }
    }
}

impl std::str::FromStr for Hotkey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('+').map(str::trim).collect();
        let (last, mods) = parts.split_last().ok_or("empty hotkey")?;
        let trigger = MouseButton::from_name(last)
            .map(Trigger::Mouse)
            .or_else(|| Key::from_name(last).map(Trigger::Key))
            .ok_or_else(|| format!("unknown key '{last}'"))?;
        let mut hk = Hotkey::new(trigger);
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => hk.ctrl = true,
                "shift" => hk.shift = true,
                "alt" => hk.alt = true,
                "win" => hk.win = true,
                other => return Err(format!("unknown modifier '{other}'")),
            }
        }
        Ok(hk)
    }
}

impl Serialize for Hotkey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Hotkey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}

/// Watches global input and calls `callback(binding_id, pressed)` on matches.
pub struct HotkeyManager {
    bindings: RwLock<HashMap<String, Hotkey>>,
    capture: Mutex<Option<Sender<Option<Hotkey>>>>,
}

impl HotkeyManager {
    pub fn start(callback: impl Fn(&str, bool) + Send + Sync + 'static) -> Arc<Self> {
        let mgr = Arc::new(Self {
            bindings: RwLock::new(HashMap::new()),
            capture: Mutex::new(None),
        });
        let rx = hooks::subscribe(false);
        let m = mgr.clone();
        std::thread::Builder::new()
            .name("hotkeys".into())
            .spawn(move || {
                let mut held_mods = HashSet::<Modifier>::new();
                // Bindings currently held down, so we report each press once
                // (ignoring key auto-repeat) and can report the release.
                let mut active: Vec<(String, Trigger)> = Vec::new();
                for ev in rx {
                    // Our own injected events must never trigger hotkeys,
                    // e.g. an autoclicker pressing its own toggle key.
                    if ev.ours {
                        continue;
                    }
                    let (trigger, down) = match ev.kind {
                        InputKind::Key { key, down, .. } => {
                            if let Some(md) = key.modifier() {
                                if down {
                                    held_mods.insert(md);
                                } else {
                                    held_mods.remove(&md);
                                }
                                continue;
                            }
                            (Trigger::Key(key), down)
                        }
                        InputKind::Button { button, down } => (Trigger::Mouse(button), down),
                        _ => continue,
                    };
                    if !down {
                        active.retain(|(id, t)| {
                            if *t == trigger {
                                callback(id, false);
                                false
                            } else {
                                true
                            }
                        });
                        continue;
                    }
                    let pressed = Hotkey {
                        trigger,
                        ctrl: held_mods.contains(&Modifier::Ctrl),
                        shift: held_mods.contains(&Modifier::Shift),
                        alt: held_mods.contains(&Modifier::Alt),
                        win: held_mods.contains(&Modifier::Win),
                    };
                    if let Some(tx) = m.capture.lock().take() {
                        let cancelled = trigger == Trigger::Key(Key::ESCAPE) && !pressed.ctrl;
                        let _ = tx.send((!cancelled).then_some(pressed));
                        continue;
                    }
                    for id in m.matches(&pressed) {
                        if !active.iter().any(|(a, _)| *a == id) {
                            callback(&id, true);
                            active.push((id, trigger));
                        }
                    }
                }
            })
            .expect("spawn hotkey thread");
        mgr
    }

    /// Bindings that fire for `pressed`. Exact modifier matches win; failing
    /// that, a binding fires if its modifiers are a subset of those held, so
    /// "F6" still works while you're holding Shift to sprint in a game.
    fn matches(&self, pressed: &Hotkey) -> Vec<String> {
        let b = self.bindings.read();
        let same_trigger = || b.iter().filter(|(_, h)| h.trigger == pressed.trigger);
        let exact: Vec<String> =
            same_trigger().filter(|(_, h)| *h == pressed).map(|(id, _)| id.clone()).collect();
        if !exact.is_empty() {
            return exact;
        }
        let held = pressed.mods();
        same_trigger()
            .filter(|(_, h)| h.mods().iter().zip(held).all(|(need, have)| !need || have))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn bind(&self, id: &str, hotkey: Option<Hotkey>) {
        let mut b = self.bindings.write();
        match hotkey {
            Some(h) => b.insert(id.to_string(), h),
            None => b.remove(id),
        };
    }

    /// Block until the user presses the next key/button combo. Escape cancels
    /// (returns `None`). Left click is never captured, since that's how you
    /// use the app.
    pub fn capture_next(&self) -> Option<Hotkey> {
        loop {
            let (tx, rx) = crossbeam_channel::bounded(1);
            *self.capture.lock() = Some(tx);
            let hk = rx.recv().ok()??;
            if hk.trigger != Trigger::Mouse(MouseButton::Left) {
                return Some(hk);
            }
        }
    }

    pub fn cancel_capture(&self) {
        if let Some(tx) = self.capture.lock().take() {
            let _ = tx.send(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_strings_round_trip() {
        for s in ["F6", "Ctrl+Shift+End", "Mouse4", "Alt+MiddleMouse", "Win+Q"] {
            let h: Hotkey = s.parse().unwrap();
            assert_eq!(h.to_string(), s);
        }
        assert!("Ctrl+".parse::<Hotkey>().is_err());
    }
}
