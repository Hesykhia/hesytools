//! Importing macros saved by the old Python hesytools (v4.x JSON).
//!
//! Old format: a flat list of events, each with a `delay` (seconds to wait
//! *before* it): key_press / key_release (pynput names like "l",
//! "Key.page_up", "<96>"), mouse_move, mouse_click (pressed true/false),
//! mouse_scroll (dy in notches) and screen_wait (OCR region + target).

use crate::capture::Region;
use crate::hotkey::Hotkey;
use crate::keys::{Key, MouseButton};
use crate::macros::{Macro, OnTimeout, PathPoint, PressAction, Step};
use crate::watch::{Cmp, Condition, parse_number};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct OldMacro {
    #[serde(default)]
    events: Vec<Value>,
    #[serde(default = "one")]
    speed: f64,
    #[serde(default)]
    play_hotkey: String,
}

fn one() -> f64 {
    1.0
}

/// Moves closer together than this are treated as one continuous path.
const PATH_GAP_MS: f64 = 100.0;

pub fn import_v4(json: &str, name: &str) -> Result<Macro, String> {
    let old: OldMacro = serde_json::from_str(json).map_err(|e| format!("Not a hesytools macro file: {e}"))?;
    if old.events.is_empty() {
        return Err("That file has no events in it".into());
    }
    let mut steps = Vec::new();
    let mut skipped = 0;
    for ev in &old.events {
        let delay_ms = ev["delay"].as_f64().unwrap_or(0.0) * 1000.0;
        if delay_ms >= 1.0 {
            steps.push(Step::Wait { ms: delay_ms.round() });
        }
        match convert(ev) {
            Some(s) => steps.push(s),
            None => skipped += 1,
        }
    }
    let steps = crate::macros::merge_taps(collapse_paths(steps));
    if steps.is_empty() {
        return Err(format!("Couldn't convert any of the {skipped} events"));
    }
    Ok(Macro {
        name: name.to_string(),
        steps,
        speed: if old.speed > 0.0 { old.speed } else { 1.0 },
        repeat: 1,
        hotkey: old_key(&old.play_hotkey).map(Hotkey::key),
        ..Default::default()
    })
}

fn convert(ev: &Value) -> Option<Step> {
    let num = |k: &str| ev[k].as_f64().unwrap_or(0.0);
    let int = |k: &str| num(k).round() as i32;
    match ev["type"].as_str()? {
        "key_press" | "key_release" => Some(Step::Key {
            key: old_key(ev["key"].as_str()?)?,
            action: if ev["type"] == "key_press" { PressAction::Down } else { PressAction::Up },
            hold_ms: 0.0,
        }),
        "mouse_move" => Some(Step::Move { x: int("x"), y: int("y") }),
        "mouse_click" => {
            let b = ev["button"].as_str().unwrap_or("left").to_lowercase();
            let button = if b.contains("right") {
                MouseButton::Right
            } else if b.contains("middle") {
                MouseButton::Middle
            } else if b.contains("x1") {
                MouseButton::X1
            } else if b.contains("x2") {
                MouseButton::X2
            } else {
                MouseButton::Left
            };
            let pressed = ev["pressed"].as_bool().unwrap_or(true);
            Some(Step::Click {
                button,
                action: if pressed { PressAction::Down } else { PressAction::Up },
                hold_ms: 0.0,
                at: Some((int("x"), int("y"))),
            })
        }
        "mouse_scroll" => Some(Step::Scroll { amount: (num("dy") * 120.0).round() as i32, horizontal: false }),
        "screen_wait" => {
            let target = ev["target"].as_str().unwrap_or("").to_string();
            let condition = match ev["condition"].as_str().unwrap_or("contains") {
                op @ (">=" | "<=" | "==") => Condition::Number {
                    op: match op {
                        ">=" => Cmp::AtLeast,
                        "<=" => Cmp::AtMost,
                        _ => Cmp::Equal,
                    },
                    value: parse_number(&target).unwrap_or(0.0),
                },
                _ => {
                    // The old "min_after" = how many characters must follow
                    // the target (e25 + 1 → e250..e259).
                    let min_after = num("min_after").round() as u32;
                    Condition::TextAppears { text: target, digits_after: (min_after > 0).then_some(min_after) }
                }
            };
            Some(Step::WaitFor {
                region: Region { x: int("rx"), y: int("ry"), w: int("rw").max(0) as u32, h: int("rh").max(0) as u32 },
                condition,
                timeout_ms: num("timeout") * 1000.0,
                // Old files often polled every 10ms (part of why it ate CPU).
                poll_ms: ev["poll_ms"].as_f64().unwrap_or(250.0).max(50.0),
                // The old player carried on after a timeout.
                on_timeout: OnTimeout::Continue,
            })
        }
        _ => None,
    }
}

/// pynput key names → our keys: "l", "Key.page_up", "Key.ctrl_l", "<96>".
pub fn old_key(s: &str) -> Option<Key> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(vk) = s.strip_prefix('<').and_then(|r| r.strip_suffix('>')) {
        return vk.parse::<u16>().ok().map(Key);
    }
    let Some(name) = s.strip_prefix("Key.") else {
        // A typed character: letters/digits map directly, punctuation by name.
        let c = s.chars().next()?;
        let named = match c {
            ' ' => "Space",
            ';' | ':' => "Semicolon",
            '=' | '+' => "Equals",
            ',' | '<' => "Comma",
            '-' | '_' => "Minus",
            '.' | '>' => "Period",
            '/' | '?' => "Slash",
            '`' | '~' => "Backtick",
            '[' | '{' => "LBracket",
            '\\' | '|' => "Backslash",
            ']' | '}' => "RBracket",
            '\'' | '"' => "Quote",
            _ => return Key::from_name(&c.to_string()),
        };
        return Key::from_name(named);
    };
    let mapped = match name {
        "space" => "Space",
        "enter" => "Enter",
        "esc" => "Escape",
        "tab" => "Tab",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "page_up" => "PageUp",
        "page_down" => "PageDown",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "shift" | "shift_l" => "LShift",
        "shift_r" => "RShift",
        "ctrl" | "ctrl_l" => "LCtrl",
        "ctrl_r" => "RCtrl",
        "alt" | "alt_l" => "LAlt",
        "alt_r" | "alt_gr" => "RAlt",
        "cmd" | "cmd_l" => "LWin",
        "cmd_r" => "RWin",
        "caps_lock" => "CapsLock",
        "num_lock" => "NumLock",
        "scroll_lock" => "ScrollLock",
        "print_screen" => "PrintScreen",
        "pause" => "Pause",
        "menu" => "Menu",
        f if f.starts_with('f') => return Key::from_name(&f.to_uppercase()),
        _ => return None,
    };
    Key::from_name(mapped)
}

/// Runs of moves with short waits between them → one path step.
fn collapse_paths(steps: Vec<Step>) -> Vec<Step> {
    let mut out = Vec::with_capacity(steps.len());
    let mut i = 0;
    while i < steps.len() {
        let Step::Move { x, y } = steps[i] else {
            out.push(steps[i].clone());
            i += 1;
            continue;
        };
        let mut points = vec![PathPoint { t: 0.0, x, y }];
        let mut t = 0.0;
        let mut j = i + 1;
        loop {
            match (steps.get(j), steps.get(j + 1)) {
                (Some(Step::Wait { ms }), Some(Step::Move { x, y })) if *ms < PATH_GAP_MS => {
                    t += ms;
                    points.push(PathPoint { t, x: *x, y: *y });
                    j += 2;
                }
                (Some(Step::Move { x, y }), _) => {
                    points.push(PathPoint { t, x: *x, y: *y });
                    j += 1;
                }
                _ => break,
            }
        }
        if points.len() >= 3 {
            out.push(Step::Path { points });
        } else {
            out.extend(steps[i..j].iter().cloned());
        }
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_the_real_v4_macro() {
        let json = r#"{"version":"4.1.1","events":[
            {"type":"key_press","key":"l","delay":0.8},
            {"type":"key_release","key":"l","delay":0.1},
            {"type":"mouse_move","x":1250,"y":470,"delay":0.018},
            {"type":"mouse_move","x":1250,"y":466,"delay":0.022},
            {"type":"mouse_move","x":1251,"y":465,"delay":0.02},
            {"type":"mouse_click","x":1249,"y":464,"button":"Button.left","pressed":true,"delay":0.2},
            {"type":"mouse_click","x":1249,"y":464,"button":"Button.left","pressed":false,"delay":0.05},
            {"type":"screen_wait","rx":706,"ry":35,"rw":97,"rh":30,"condition":"contains","target":"inf","min_after":0,"timeout":30.0,"poll_ms":10,"delay":0.0},
            {"type":"screen_wait","rx":1,"ry":2,"rw":3,"rh":4,"condition":"contains","target":"e25","min_after":1,"timeout":0,"delay":0.0},
            {"type":"key_press","key":"Key.page_up","delay":0.1},
            {"type":"key_release","key":"Key.page_up","delay":0.05}
          ],"speed":1.0,"mode":"Toggle","play_hotkey":"Key.page_up","rec_hotkey":""}"#;
        let m = import_v4(json, "macro5.0").unwrap();
        let l = Key::from_name("L").unwrap();
        assert_eq!(m.steps[0], Step::Wait { ms: 800.0 });
        assert_eq!(m.steps[1], Step::Key { key: l, action: PressAction::Tap, hold_ms: 100.0 });
        assert!(matches!(&m.steps[3], Step::Path { points } if points.len() == 3), "{:?}", m.steps[3]);
        assert!(m.steps.iter().any(|s| matches!(s, Step::Click { action: PressAction::Tap, hold_ms, at: Some((1249, 464)), .. } if *hold_ms == 50.0)));
        let waits: Vec<&Step> = m.steps.iter().filter(|s| matches!(s, Step::WaitFor { .. })).collect();
        assert!(matches!(waits[0], Step::WaitFor { condition: Condition::TextAppears { digits_after: None, .. }, timeout_ms, .. } if *timeout_ms == 30000.0));
        assert!(matches!(waits[1], Step::WaitFor { condition: Condition::TextAppears { digits_after: Some(1), .. }, .. }));
        assert_eq!(m.hotkey.map(|h| h.to_string()), Some("PageUp".into()));
    }

    #[test]
    fn old_key_names() {
        assert_eq!(old_key("Key.ctrl_l"), Key::from_name("LCtrl"));
        assert_eq!(old_key("Key.f5"), Key::from_name("F5"));
        assert_eq!(old_key("<96>"), Some(Key(96)));
        assert_eq!(old_key(" "), None); // trimmed to nothing
        assert_eq!(old_key("/"), Key::from_name("Slash"));
        assert_eq!(old_key("Key.nonsense"), None);
    }
}
