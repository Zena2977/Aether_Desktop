//! پورت ۱:۱ از `core/DiagnosticsLog.kt` + وضعیت زندهٔ بررسی‌های `Diagnostics.kt`.
//!
//! رفع نشت حافظهٔ ۱.۲.۲ عیناً حفظ شده:
//!   * رینگ‌بافر کرانمند ۸۰۰ خطی (نه لیست بی‌انتها)
//!   * نوشتن روی دیسک دسته‌ای و روی ترد پس‌زمینه
//!   * سقف فایل ۵۱۲ KiB با چرخش
//!   * به‌روزرسانی UI حداکثر ~۵ بار در ثانیه
//!
//! جدید: همان `ComponentCheck`های پنل عیب‌یابی اندروید (PENDING/RUNNING/
//! PASS/FAIL) این‌جا نگه‌داری می‌شوند تا UI بتواند زنده آن‌ها را بخواند.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LINES: usize = 800;
const MAX_FILE_BYTES: u64 = 512 * 1024;
pub const UI_THROTTLE: Duration = Duration::from_millis(200); // ~۵ بار در ثانیه

struct Inner {
    ring: VecDeque<String>,
    pending: Vec<String>,
    file: Option<PathBuf>,
}

static LOG: OnceLock<Mutex<Inner>> = OnceLock::new();

fn inner() -> &'static Mutex<Inner> {
    LOG.get_or_init(|| {
        Mutex::new(Inner { ring: VecDeque::with_capacity(MAX_LINES), pending: Vec::new(), file: None })
    })
}

/// معادل `ComponentCheck` اندروید — یک ردیف پنل عیب‌یابی.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentCheck {
    pub id: String,
    pub label: String,
    /// "PENDING" | "RUNNING" | "PASS" | "FAIL"
    pub state: String,
    pub detail: String,
}

static CHECKS: OnceLock<Mutex<Vec<ComponentCheck>>> = OnceLock::new();

fn checks_inner() -> &'static Mutex<Vec<ComponentCheck>> {
    CHECKS.get_or_init(|| Mutex::new(Vec::new()))
}

pub struct DiagnosticsLog;

impl DiagnosticsLog {
    pub fn init(data_dir: &Path) {
        let path = data_dir.join("logs").join("aether.log");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        inner().lock().file = Some(path);
        Self::spawn_flusher();
    }

    pub fn d(tag: &str, msg: &str) { Self::push('D', tag, msg) }
    pub fn i(tag: &str, msg: &str) { Self::push('I', tag, msg) }
    pub fn w(tag: &str, msg: &str) { Self::push('W', tag, msg) }
    pub fn e(tag: &str, msg: &str) { Self::push('E', tag, msg) }

    fn push(level: char, tag: &str, msg: &str) {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let line = format!("{ts} {level}/{tag}: {msg}");
        let mut g = inner().lock();
        if g.ring.len() == MAX_LINES { g.ring.pop_front(); }
        g.ring.push_back(line.clone());
        g.pending.push(line);
    }

    /// خوانده می‌شود توسط پنل Diagnostics — فقط وقتی کنسول باز است
    /// (معادل بهینه‌سازی ۱.۲.۲: فقط کنسولِ باز subscribe می‌کند).
    pub fn tail(limit: usize) -> Vec<String> {
        let g = inner().lock();
        g.ring.iter().rev().take(limit).rev().cloned().collect()
    }

    /// معادل `DiagnosticsLog.exportText()` — خوراک دکمهٔ «Copy logs».
    pub fn export_text() -> String {
        Self::tail(MAX_LINES).join("\n")
    }

    pub fn clear() {
        let mut g = inner().lock();
        g.ring.clear();
        g.pending.clear();
    }

    // ---- وضعیت زندهٔ بررسی‌ها (معادل Diagnostics.checks در اندروید) ----

    /// بازنشانی فهرست بررسی‌ها — معادل `Diagnostics.resetChecks()`.
    pub fn set_checks(items: Vec<(&str, String)>) {
        let mut g = checks_inner().lock();
        *g = items
            .into_iter()
            .map(|(id, label)| ComponentCheck {
                id: id.to_string(),
                label,
                state: "PENDING".to_string(),
                detail: String::new(),
            })
            .collect();
    }

    pub fn update_check(id: &str, state: &str, detail: Option<&str>) {
        let mut g = checks_inner().lock();
        if let Some(c) = g.iter_mut().find(|c| c.id == id) {
            c.state = state.to_string();
            if let Some(d) = detail {
                c.detail = d.to_string();
            }
        }
    }

    pub fn checks() -> Vec<ComponentCheck> {
        checks_inner().lock().clone()
    }

    /// نوشتن دسته‌ای روی ترد پس‌زمینه — مسیر بحرانی هرگز دیسک را لمس نمی‌کند.
    fn spawn_flusher() {
        std::thread::Builder::new()
            .name("aether-log-flush".into())
            .spawn(|| loop {
                std::thread::sleep(UI_THROTTLE);
                let (batch, path) = {
                    let mut g = inner().lock();
                    if g.pending.is_empty() { continue; }
                    (std::mem::take(&mut g.pending), g.file.clone())
                };
                let Some(path) = path else { continue };

                // چرخش فایل با سقف ۵۱۲ KiB.
                if std::fs::metadata(&path).map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(false) {
                    let _ = std::fs::rename(&path, path.with_extension("log.1"));
                }
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
                    for line in batch { let _ = writeln!(f, "{line}"); }
                }
            })
            .ok();
    }
}
