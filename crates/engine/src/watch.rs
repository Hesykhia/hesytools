//! Watching a screen region for a condition.
//!
//! The key to staying cheap: every check captures the region (sub-ms), but
//! OCR only runs when the pixels actually changed since the last check. A
//! static screen therefore costs almost nothing however often it's polled.

use crate::capture::{self, Frame, Region};
use crate::imagematch::{self, Gray, Template};
use crate::ocr::Ocr;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Cmp {
    #[serde(rename = ">=")]
    AtLeast,
    #[serde(rename = "<=")]
    AtMost,
    #[serde(rename = "==")]
    Equal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Condition {
    /// Any of the comma-separated phrases appears (case-insensitive).
    ///
    /// `digits_after` pins how many digits must directly follow the phrase:
    /// "e25" with `Some(1)` matches e250..e259, "e2" with `Some(2)` matches
    /// e200..e299, and `Some(0)` means exactly "e2" (not e25). `None` is a
    /// plain "contains". "e+25" is read the same as "e25".
    #[serde(rename_all = "camelCase")]
    TextAppears {
        text: String,
        #[serde(default)]
        digits_after: Option<u32>,
    },
    /// None of the phrases are visible any more (same matching rules).
    #[serde(rename_all = "camelCase")]
    TextGone {
        text: String,
        #[serde(default)]
        digits_after: Option<u32>,
    },
    /// Any number in the region compares true against `value`, so a box
    /// showing "WAVE 12 · 1.25e+25" still matches `>= 1e25`.
    /// Understands 1,234 · 1.25e+25 · 12.5K / 3M / 1.2B / 4T.
    Number { op: Cmp, value: f64 },
    /// Some pixel anywhere in the region is within `tolerance` of `rgb`
    /// (e.g. a red health bar or a green "ready" light appears).
    ColorAppears { rgb: [u8; 3], tolerance: u8 },
    /// The pixel at (dx, dy) inside the region is within `tolerance` of `rgb`.
    #[serde(rename_all = "camelCase")]
    ColorAt { dx: u32, dy: u32, rgb: [u8; 3], tolerance: u8 },
    /// A picture (button, icon...) shows up anywhere in the region, at
    /// least `threshold` similar (0..1, ~0.9 is a good start).
    ImageAppears { template: Template, threshold: f64 },
    /// Anything in the region changed since watching began.
    Changes,
}

impl Condition {
    pub fn needs_ocr(&self) -> bool {
        matches!(self, Condition::TextAppears { .. } | Condition::TextGone { .. } | Condition::Number { .. })
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub matched: bool,
    /// What OCR read (if the condition uses OCR).
    pub text: Option<String>,
    /// For number conditions: the number that matched, else the first found.
    pub number: Option<f64>,
    /// Every number found in the text.
    pub numbers: Vec<f64>,
    /// Image conditions: best match position (inside the region) and how
    /// similar it was.
    pub found: Option<imagematch::Found>,
    /// Whether this check had to run OCR (false = pixels unchanged, reused).
    pub ocr_ran: bool,
    /// Screen position of the region's top-left corner at this check (it
    /// moves with the window when watching a single app).
    pub origin: (i32, i32),
    pub capture_ms: f64,
    pub ocr_ms: f64,
}

pub struct Watcher {
    pub region: Region,
    pub condition: Condition,
    ocr: Option<Ocr>,
    template: Option<Gray>,
    last_fp: Option<u64>,
    baseline_fp: Option<u64>,
    last: Option<Check>,
    /// The most recent capture, for the live viewer.
    pub frame: Option<Frame>,
    /// Only look at this app's window; `region` is then relative to it.
    pub window: Option<crate::window::WindowTarget>,
    /// The window last found for `window`.
    handle: Option<isize>,
}

impl Watcher {
    pub fn new(region: Region, condition: Condition) -> Result<Self, String> {
        let ocr = if condition.needs_ocr() { Some(Ocr::new()?) } else { None };
        let template = match &condition {
            Condition::ImageAppears { template, .. } => Some(template.gray()?),
            _ => None,
        };
        Ok(Self { region, condition, ocr, template, last_fp: None, baseline_fp: None, last: None, frame: None, window: None, handle: None })
    }

    /// Watch inside one app's window instead of the whole screen.
    pub fn in_window(mut self, window: Option<crate::window::WindowTarget>) -> Self {
        self.window = window;
        self
    }

    fn grab(&mut self) -> Result<(Frame, (i32, i32)), String> {
        let Some(target) = &self.window else {
            return Ok((capture::capture(self.region)?, (self.region.x, self.region.y)));
        };
        let alive = self.handle.filter(|h| crate::window::rect(*h).is_some());
        let handle = match alive {
            Some(h) => h,
            None => crate::window::find(target).ok_or_else(|| format!("{} isn't open", target.exe))?,
        };
        self.handle = Some(handle);
        capture::capture_window(handle, self.region)
    }

    pub fn check(&mut self) -> Result<Check, String> {
        let t = Instant::now();
        let (frame, origin) = self.grab()?;
        let capture_ms = t.elapsed().as_secs_f64() * 1000.0;
        let fp = frame.fingerprint();
        let baseline = *self.baseline_fp.get_or_insert(fp);
        let unchanged = self.last_fp == Some(fp);
        self.last_fp = Some(fp);

        let check = match &self.condition {
            Condition::Changes => Check { matched: fp != baseline, ..empty(capture_ms) },
            Condition::ColorAppears { rgb, tolerance } => {
                let hit = find_color(&frame, *rgb, *tolerance);
                let found = hit.map(|(x, y)| imagematch::Found { x, y, score: 1.0 });
                Check { matched: hit.is_some(), found, ..empty(capture_ms) }
            }
            Condition::ColorAt { dx, dy, rgb, tolerance } => {
                let matched = frame.pixel(*dx, *dy).is_some_and(|p| close(p, *rgb, *tolerance));
                Check { matched, ..empty(capture_ms) }
            }
            Condition::ImageAppears { threshold, .. } => {
                if unchanged && let Some(prev) = &self.last {
                    Check { capture_ms, ..prev.clone() }
                } else {
                    let tpl = self.template.as_ref().ok_or("No image to look for")?;
                    let t = Instant::now();
                    let found = imagematch::find(&Gray::from_frame(&frame), tpl);
                    let ms = t.elapsed().as_secs_f64() * 1000.0;
                    let matched = found.is_some_and(|f| f.score >= *threshold);
                    Check { matched, found, ocr_ms: ms, ..empty(capture_ms) }
                }
            }
            cond => {
                if unchanged && let Some(prev) = &self.last {
                    // Same pixels as last time: same answer, no OCR.
                    Check { ocr_ran: false, capture_ms, ocr_ms: 0.0, ..prev.clone() }
                } else {
                    let ocr = self.ocr.as_ref().ok_or("OCR not initialised")?;
                    let t = Instant::now();
                    let text = ocr.read(&frame)?;
                    let ocr_ms = t.elapsed().as_secs_f64() * 1000.0;
                    let numbers = parse_numbers(&text);
                    let hit = match cond {
                        Condition::Number { op, value } => numbers.iter().copied().find(|n| compare(*n, *op, *value)),
                        _ => None,
                    };
                    let matched = match cond {
                        Condition::TextAppears { text: want, digits_after } => contains_any(&text, want, *digits_after),
                        Condition::TextGone { text: want, digits_after } => !contains_any(&text, want, *digits_after),
                        Condition::Number { .. } => hit.is_some(),
                        _ => unreachable!(),
                    };
                    let number = hit.or(numbers.first().copied());
                    Check { matched, text: Some(text), number, numbers, found: None, ocr_ran: true, capture_ms, ocr_ms, origin: (0, 0) }
                }
            }
        };
        let check = Check { origin, ..check };
        self.frame = Some(frame);
        self.last = Some(check.clone());
        Ok(check)
    }
}

/// Screen point of what `check` found: the image's centre or the matching
/// pixel; text has no position, so the middle of the area.
pub fn found_point(condition: &Condition, check: &Check, region: Region) -> (i32, i32) {
    let (dx, dy) = match (condition, check.found) {
        (Condition::ImageAppears { template, .. }, Some(f)) => (f.x + template.w / 2, f.y + template.h / 2),
        (Condition::ColorAppears { .. }, Some(f)) => (f.x, f.y),
        (Condition::ColorAt { dx, dy, .. }, _) => (*dx, *dy),
        _ => (region.w / 2, region.h / 2),
    };
    (check.origin.0 + dx as i32, check.origin.1 + dy as i32)
}

fn close(p: [u8; 3], rgb: [u8; 3], tolerance: u8) -> bool {
    p.iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

/// First pixel (row by row) within `tolerance` of `rgb`.
fn find_color(f: &Frame, rgb: [u8; 3], tolerance: u8) -> Option<(u32, u32)> {
    f.bgra.chunks_exact(4).position(|px| close([px[2], px[1], px[0]], rgb, tolerance)).map(|i| {
        let i = i as u32;
        (i % f.w, i / f.w)
    })
}

fn empty(capture_ms: f64) -> Check {
    Check {
        matched: false,
        text: None,
        number: None,
        numbers: Vec::new(),
        found: None,
        ocr_ran: false,
        capture_ms,
        ocr_ms: 0.0,
        origin: (0, 0),
    }
}

fn contains_any(haystack: &str, phrases: &str, digits_after: Option<u32>) -> bool {
    let hay = normalize(haystack);
    phrases
        .split(',')
        .map(normalize)
        .filter(|p| !p.is_empty())
        .any(|p| phrase_matches(&hay, &p, digits_after))
}

fn phrase_matches(hay: &str, phrase: &str, digits_after: Option<u32>) -> bool {
    let Some(want) = digits_after else { return hay.contains(phrase) };
    hay.match_indices(phrase).any(|(i, _)| {
        // OCR often reads a 0 as the letter o; count it as a digit here.
        let digits = hay[i + phrase.len()..].chars().take_while(|c| c.is_ascii_digit() || *c == 'o').count();
        digits == want as usize
    })
}

/// Lowercase, collapse whitespace (so line breaks don't break matches), and
/// write exponents one way: "e+25" and "e 25" both become "e25".
fn normalize(s: &str) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let c: Vec<char> = collapsed.chars().collect();
    let mut out = String::with_capacity(collapsed.len());
    let mut i = 0;
    while i < c.len() {
        out.push(c[i]);
        if c[i] == 'e' && matches!(c.get(i + 1), Some('+') | Some(' ')) && c.get(i + 2).is_some_and(|d| d.is_ascii_digit()) {
            i += 2;
            continue;
        }
        i += 1;
    }
    out
}

fn compare(n: f64, op: Cmp, v: f64) -> bool {
    match op {
        Cmp::AtLeast => n >= v,
        Cmp::AtMost => n <= v,
        // Relative tolerance so 1.25e+25 == 1.25e25 despite float noise.
        Cmp::Equal => (n - v).abs() <= v.abs().max(1.0) * 1e-9,
    }
}

/// First number in the text.
pub fn parse_number(text: &str) -> Option<f64> {
    parse_numbers(text).into_iter().next()
}

/// Every number in the text, in order. Handles thousands separators,
/// decimals, scientific notation and K/M/B/T suffixes (as games love).
pub fn parse_numbers(text: &str) -> Vec<f64> {
    let s: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if !s[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let negative = i > 0 && s[i - 1] == '-';
        let mut num = String::new();
        // Integer part with optional thousands separators.
        while i < s.len() && (s[i].is_ascii_digit() || (s[i] == ',' && s.get(i + 1).is_some_and(|c| c.is_ascii_digit()))) {
            if s[i] != ',' {
                num.push(s[i]);
            }
            i += 1;
        }
        if i < s.len() && s[i] == '.' && s.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            num.push('.');
            i += 1;
            while i < s.len() && s[i].is_ascii_digit() {
                num.push(s[i]);
                i += 1;
            }
        }
        // Exponent: e25, e+25, E-3
        if i < s.len() && (s[i] == 'e' || s[i] == 'E') {
            let mut j = i + 1;
            let mut exp = String::from("e");
            if j < s.len() && (s[j] == '+' || s[j] == '-') {
                exp.push(s[j]);
                j += 1;
            }
            if j < s.len() && s[j].is_ascii_digit() {
                while j < s.len() && s[j].is_ascii_digit() {
                    exp.push(s[j]);
                    j += 1;
                }
                num.push_str(&exp);
                i = j;
            }
        }
        let Ok(mut value) = num.parse::<f64>() else { continue };
        // Suffix like 12.5K, 3M, 1.2B, 4T, 2Qa (only directly after the number).
        let rest: String = s[i..].iter().take(2).collect::<String>().to_lowercase();
        let mult = if rest.starts_with("qa") {
            1e15
        } else if rest.starts_with("qi") {
            1e18
        } else {
            match rest.chars().next() {
                Some('k') => 1e3,
                Some('m') => 1e6,
                Some('b') => 1e9,
                Some('t') => 1e12,
                _ => 1.0,
            }
        };
        // Avoid reading "5 min" or "3 ms" as millions: suffix must not start a word.
        let is_word = s.get(i + 1).is_some_and(|c| c.is_alphabetic()) && !rest.starts_with("qa") && !rest.starts_with("qi");
        if mult != 1.0 && !is_word {
            value *= mult;
            i += if mult >= 1e15 { 2 } else { 1 };
        }
        out.push(if negative { -value } else { value });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(parse_number("Coins: 1,234,567"), Some(1_234_567.0));
        assert_eq!(parse_number("WAVE 12"), Some(12.0));
        assert_eq!(parse_number("1.25e+25"), Some(1.25e25));
        assert_eq!(parse_number("x 3E-3 y"), Some(0.003));
        assert_eq!(parse_number("12.5K"), Some(12_500.0));
        assert_eq!(parse_number("$3M"), Some(3_000_000.0));
        assert_eq!(parse_number("1.2B gems"), Some(1.2e9));
        assert_eq!(parse_number("2Qa"), Some(2e15));
        assert_eq!(parse_number("5 min left"), Some(5.0));
        assert_eq!(parse_number("wait 3ms"), Some(3.0));
        assert_eq!(parse_number("-40 hp"), Some(-40.0));
        assert_eq!(parse_number("no digits"), None);
        assert_eq!(parse_numbers("WAVE 12\n1.25e+25 · 3.5K"), vec![12.0, 1.25e25, 3500.0]);
    }

    #[test]
    fn exponent_digits_after() {
        let m = |hay: &str, p: &str, n: Option<u32>| contains_any(hay, p, n);
        // +1: e250..e259 only
        assert!(m("1.5e250", "e25", Some(1)));
        assert!(m("Coins: e+259", "e25", Some(1)));
        assert!(!m("e25", "e25", Some(1)));
        assert!(!m("e2500", "e25", Some(1)));
        // +2: e200..e299
        assert!(m("3.1e+287", "e2", Some(2)));
        assert!(!m("e25", "e2", Some(2)));
        // +0: exactly e2
        assert!(m("x 4.2e2 y", "e2", Some(0)));
        assert!(!m("4.2e25", "e2", Some(0)));
        // plain contains is the old loose behaviour
        assert!(m("4.2e25", "e2", None));
        // OCR read a zero as "o"
        assert!(m("e25o", "e25", Some(1)));
        // alternatives and "inf"
        assert!(m("Rebirth: inf", "e306, inf", Some(0)));
        assert!(m("1e306", "e306, inf", Some(0)));
    }

    #[test]
    fn text_matching() {
        assert!(contains_any("Rebirth\nREADY!", "ready", None));
        assert!(contains_any("Press  Start", "nope, press start", None));
        assert!(!contains_any("Loading", "ready, done", None));
        assert!(compare(1.25e25, Cmp::Equal, 1.25e25));
        assert!(compare(10.0, Cmp::AtLeast, 10.0));
    }

    #[test]
    fn colour_anywhere() {
        let mut bgra = [10u8, 10, 10, 255].repeat(20 * 10);
        let i = ((4 * 20 + 7) * 4) as usize; // pixel (7, 4) turns red-ish
        bgra[i..i + 3].copy_from_slice(&[20, 30, 240]);
        let f = Frame { w: 20, h: 10, bgra };
        assert_eq!(find_color(&f, [255, 0, 0], 40), Some((7, 4)));
        assert_eq!(find_color(&f, [255, 0, 0], 10), None);
    }

    #[test]
    fn condition_json() {
        let c = Condition::Number { op: Cmp::AtLeast, value: 1e25 };
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, r#"{"kind":"number","op":">=","value":1e+25}"#);
        let c: Condition = serde_json::from_str(r#"{"kind":"colorAt","dx":1,"dy":2,"rgb":[255,0,0],"tolerance":20}"#).unwrap();
        assert!(matches!(c, Condition::ColorAt { tolerance: 20, .. }));
    }
}
