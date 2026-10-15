//! Macros on disk: one pretty-printed JSON file per macro, so they're easy to
//! back up, share, or hand-edit.

use engine::macros::Macro;
use std::path::{Path, PathBuf};

pub struct MacroStore {
    dir: PathBuf,
}

impl MacroStore {
    pub fn new(config_dir: &Path) -> Self {
        Self { dir: config_dir.join("macros") }
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Newest first. Unreadable files are skipped, not deleted.
    pub fn load_all(&self) -> Vec<Macro> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<Macro> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| {
                let text = std::fs::read_to_string(e.path()).ok()?;
                serde_json::from_str(&text)
                    .map_err(|err| eprintln!("macros: skipping {}: {err}", e.path().display()))
                    .ok()
            })
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        out
    }

    pub fn save(&self, m: &Macro) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.path(&m.id).with_extension("json.tmp");
        let json = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.path(&m.id)).map_err(|e| e.to_string())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        match std::fs::remove_file(self.path(id)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

/// Short unique id, safe as a filename.
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let ms = now_ms();
    format!("m{ms:x}{:x}", COUNTER.fetch_add(1, Ordering::Relaxed) & 0xff)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
