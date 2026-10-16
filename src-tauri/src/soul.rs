//! The Soul Clicker helper's link to hesytools: a tiny HTTP server that only
//! listens on this PC. The userscript in the browser posts what it sees and
//! gets its settings back; the Dev tab shows the stats.
//!
//! GET  /soul-helper.user.js  the userscript, with this install's token baked in
//! POST /soul                 stats in, settings out (needs the token)

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

pub const PORT: u16 = 47629;
const SCRIPT: &str = include_str!("../assets/soul-helper.user.js");

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RitualRule {
    /// "off" | "worth" | "shards" | "always"
    pub mode: String,
    /// "worth": go when output after ÷ output now is at least this.
    pub worth: f64,
    /// "shards": go when this many shards are waiting.
    pub shards: f64,
}

impl Default for RitualRule {
    fn default() -> Self {
        Self { mode: "off".into(), worth: 1.5, shards: 10.0 }
    }
}

/// What the helper should do. Everything starts off.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SoulConfig {
    pub enabled: bool,
    pub favors: bool,
    pub crystals: bool,
    /// Claim the Soul Leech (offline earnings).
    pub leech: bool,
    /// Release banked blessings straight away.
    pub blessings: bool,
    /// Click the orb flat out while a click blessing is running.
    pub frenzy: bool,
    pub buy_gens: bool,
    pub buy_upgrades: bool,
    pub orb: bool,
    pub orb_cps: f64,
    /// Keep this many minutes of income unspent (Lucky Souls pays up to
    /// 15% of what you hold).
    pub reserve_minutes: f64,
    /// After a Ritual, stay on the skill tree instead of starting the next
    /// run straight away (the tree can only be opened then).
    #[serde(default = "yes")]
    pub tree_stop: bool,
    pub ritual: RitualRule,
}

/// What the helper last reported, passed through to the Dev tab as is.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoulReport {
    pub at: u64,
    pub data: serde_json::Value,
}

/// How the server reaches the rest of the app.
pub trait Host: Send + Sync + 'static {
    fn token(&self) -> String;
    /// `None` while the Dev tab is locked: the helper then gets nothing.
    fn config(&self) -> Option<SoulConfig>;
    fn report(&self, r: SoulReport);
}

/// The helper version bundled with this build (its @version line).
pub fn helper_version() -> String {
    SCRIPT
        .lines()
        .find_map(|l| l.strip_prefix("// @version"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

pub fn script(token: &str) -> String {
    SCRIPT.replace("__PORT__", &PORT.to_string()).replace("__TOKEN__", token)
}

pub fn install_url() -> String {
    format!("http://127.0.0.1:{PORT}/soul-helper.user.js")
}

pub fn start(host: Arc<dyn Host>) {
    std::thread::Builder::new()
        .name("soul-bridge".into())
        .spawn(move || {
            let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("soul bridge: can't listen on {PORT}: {e}");
                    return;
                }
            };
            for stream in listener.incoming().flatten() {
                let host = host.clone();
                std::thread::spawn(move || {
                    let _ = handle(stream, &*host);
                });
            }
        })
        .expect("spawn soul bridge");
}

fn handle(mut stream: TcpStream, host: &dyn Host) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut length = 0usize;
    let mut token = String::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => length = v.trim().parse().unwrap_or(0).min(1 << 20),
                "x-hesy-token" => token = v.trim().to_string(),
                _ => {}
            }
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;

    let (status, kind, out) = match (method, path.split('?').next().unwrap_or("")) {
        ("GET", "/soul-helper.user.js") => match host.config() {
            Some(_) => ("200 OK", "text/javascript; charset=utf-8", script(&host.token())),
            None => ("403 Forbidden", "text/plain", "The Dev tab isn't unlocked in hesytools.".into()),
        },
        ("POST", "/soul") if !token.is_empty() && token == host.token() => {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&body) {
                host.report(SoulReport { at: crate::macro_store::now_ms(), data });
            }
            // Locked: tell the helper to sit still.
            let cfg = host.config().unwrap_or_default();
            ("200 OK", "application/json", serde_json::to_string(&cfg).unwrap_or_default())
        }
        ("POST", "/soul") => ("401 Unauthorized", "text/plain", "Bad token. Reinstall the helper from hesytools.".into()),
        _ => ("404 Not Found", "text/plain", String::new()),
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{out}",
        out.len()
    )?;
    stream.flush()
}
