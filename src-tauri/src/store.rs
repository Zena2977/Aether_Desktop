//! پورت از `data/ProfileStore.kt`.
//!
//! در اندروید DataStore بود؛ در ویندوز یک فایل JSON در %LOCALAPPDATA%\Aether.
//! رفتار یکسان است: خواندن هرگز خطا نمی‌دهد — فایل خراب یعنی پیش‌فرض‌ها.

use crate::log::DiagnosticsLog;
use crate::profile::ConnectionProfile;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn new(data_dir: &Path) -> Self {
        Self { path: data_dir.join("profile.json") }
    }

    pub fn load(&self) -> ConnectionProfile {
        match std::fs::read_to_string(&self.path) {
            Ok(raw) => match serde_json::from_str::<ConnectionProfile>(&raw) {
                Ok(p) => p,
                Err(e) => {
                    DiagnosticsLog::w("store", &format!("Profile unreadable ({e}); using defaults."));
                    ConnectionProfile::default()
                }
            },
            Err(_) => ConnectionProfile::default(),
        }
    }

    /// نوشتن اتمیک: اول فایل موقت، بعد rename. قطع برق وسط ذخیره
    /// نباید تنظیمات کاربر را نابود کند.
    pub fn save(&self, profile: &ConnectionProfile) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(profile)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
