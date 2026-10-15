//! Discord webhook notifications ("macro finished", "text appeared", ...).

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub title: String,
    pub description: String,
    /// Embed side-bar colour, 0xRRGGBB.
    pub color: u32,
    /// Optional plain message text, e.g. `<@1234>` to ping someone.
    pub content: Option<String>,
}

pub fn validate_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    let ok = ["https://discord.com/api/webhooks/", "https://discordapp.com/api/webhooks/"]
        .iter()
        .chain(&["https://ptb.discord.com/api/webhooks/", "https://canary.discord.com/api/webhooks/"])
        .any(|p| u.starts_with(p));
    if ok {
        Ok(())
    } else {
        Err("Not a Discord webhook URL. In Discord: channel settings → Integrations → Webhooks → Copy Webhook URL.".into())
    }
}

/// Blocking POST; call from a background thread.
pub fn send(url: &str, n: &Notification) -> Result<(), String> {
    validate_url(url)?;
    let body = json!({
        "username": "hesytools",
        "content": n.content,
        "embeds": [{
            "title": n.title,
            "description": n.description,
            "color": n.color,
            "timestamp": rfc3339_now(),
        }],
        // Only ping users explicitly mentioned in `content`, never @everyone.
        "allowed_mentions": { "parse": ["users"] },
    });
    ureq::post(url.trim())
        .send_json(&body)
        .map(|_| ())
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => "Webhook not found (was it deleted?)".to_string(),
            ureq::Error::StatusCode(429) => "Rate limited by Discord; try again shortly".to_string(),
            other => format!("Webhook failed: {other}"),
        })
}

fn rfc3339_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant), UTC.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_validation() {
        assert!(validate_url("https://discord.com/api/webhooks/1/abc").is_ok());
        assert!(validate_url("http://203.0.113.7:9203").is_err());
    }

    #[test]
    fn timestamp_format() {
        let t = rfc3339_now();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.starts_with("20") && t.ends_with('Z'));
    }
}
