//! پورت ۱:۱ از `app/src/main/java/studio/cluvex/aether/model/Profile.kt`
//!
//! هر تغییری در سمت اندروید باید دقیقاً همین‌جا هم اعمال شود؛ منطق ساخت
//! آرگومان‌های خط فرمان و متغیرهای محیطیِ موتور باید بایت‌به‌بایت یکسان بماند.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Protocol {
    /// v8: "Auto" renamed to "Smart" (same as mobile). The serde alias keeps
    /// old profile.json files that stored "AUTO" loading fine.
    #[serde(alias = "AUTO")]
    Smart,
    Masque,
    Wireguard,
    Gool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScanMode { Turbo, Balanced, Thorough, Stealth, Ironclad }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IpVersion { V4, V6, Both }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Noize { Off, Light, Firewall, Balanced, Gfw, Aggressive }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndpointMode { Auto, ManualPeer, ManualRange }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SplitMode { Off, Include, Exclude }

pub const DEFAULT_MTU: u32 = 1280;
pub const MTU_PRESETS: [u32; 5] = [1280, 1380, 1420, 1500, 8500];
pub const KEEPALIVE_PRESETS: [u32; 4] = [0, 10, 25, 45];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnectionProfile {
    pub protocol: Protocol,
    pub scan_mode: ScanMode,
    pub ip_version: IpVersion,
    pub quick_reconnect: bool,
    pub masque_http2: bool,
    /// اشتراک تونل با دستگاه‌های دیگر روی همان شبکه (پورت‌های 10810/10811).
    pub lan_share: bool,
    pub noize: Noize,
    pub endpoint_mode: EndpointMode,
    pub manual_peer: String,
    pub manual_range: String,
    pub keepalive: u32,
    pub fragment: bool,
    pub ech: bool,
    pub mtu: u32,
    // ---------------------------------------------------------------------
    // عمداً حذف شده نسبت به اندروید: `proxyMode`.
    // در ویندوز هیچ اپلیکیشنی به SOCKS5 محلی «به‌جای» VPN نیاز ندارد چون
    // Wintun کل سیستم را می‌گیرد و پروکسی سیستمی هم بومی است؛ نگه داشتنش
    // فقط یک مسیر کد بلااستفاده و یک حالت خطای اضافه می‌ساخت.
    // ---------------------------------------------------------------------
    pub split_mode: SplitMode,
    /// در ویندوز به‌جای package name، مسیر یا نام فرآیند (`chrome.exe`).
    pub split_apps: Vec<String>,
}

impl Default for ConnectionProfile {
    fn default() -> Self {
        Self {
            protocol: Protocol::Smart,
            scan_mode: ScanMode::Balanced,
            ip_version: IpVersion::V4,
            quick_reconnect: true,
            masque_http2: false,
            lan_share: false,
            noize: Noize::Off,
            endpoint_mode: EndpointMode::Auto,
            manual_peer: String::new(),
            manual_range: String::new(),
            keepalive: 0,
            fragment: false,
            ech: false,
            mtu: DEFAULT_MTU,
            split_mode: SplitMode::Off,
            split_apps: Vec::new(),
        }
    }
}

impl ConnectionProfile {
    pub fn has_manual_peer(&self) -> bool {
        self.endpoint_mode == EndpointMode::ManualPeer && !self.manual_peer.trim().is_empty()
    }

    /// معادل دقیق `Profile.kt::toArgs()`
    pub fn to_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        match self.protocol {
            // AUTO هرگز به موتور نمی‌رسد: SmartAuto قبل از اجرا آن را به یک
            // پروتکل مشخص تبدیل می‌کند (دقیقاً مثل اندروید).
            Protocol::Smart => {}
            Protocol::Masque => args.push("--masque".into()),
            Protocol::Wireguard => args.push("--wg".into()),
            Protocol::Gool => args.push("--gool".into()),
        }

        if !self.has_manual_peer() {
            args.push(match self.scan_mode {
                ScanMode::Turbo => "--turbo",
                ScanMode::Balanced => "--balanced",
                ScanMode::Thorough => "--thorough",
                ScanMode::Stealth => "--stealth",
                ScanMode::Ironclad => "--ironclad",
            }.into());
        }

        args.push(match self.ip_version {
            IpVersion::V4 => "-4",
            IpVersion::V6 => "-6",
            IpVersion::Both => "--dual",
        }.into());

        args.push(if self.quick_reconnect { "--quick-reconnect" } else { "--no-quick-reconnect" }.into());

        if self.noize != Noize::Off {
            args.push("--noize".into());
            args.push(format!("{:?}", self.noize).to_lowercase());
        }

        if self.has_manual_peer() {
            args.push("--peer".into());
            args.push(self.manual_peer.trim().to_string());
        }

        if self.fragment { args.push("--fragment".into()); }
        if self.ech { args.push("--ech".into()); args.push("auto".into()); }
        if self.keepalive > 0 { args.push("--keepalive".into()); args.push(self.keepalive.to_string()); }

        args
    }

    /// معادل دقیق `Profile.kt::toEnv()`
    pub fn to_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.insert("AETHER_MASQUE_HTTP2".into(), if self.masque_http2 { "1".into() } else { "0".into() });

        let range = self.manual_range.trim();
        if self.endpoint_mode == EndpointMode::ManualRange && !range.is_empty() {
            env.insert("AETHER_SCAN_CIDRS".into(), range.to_string());
            env.insert("AETHER_MASQUE_CIDRS".into(), range.to_string());
            env.insert("AETHER_WG_CIDRS".into(), range.to_string());
        }
        env
    }

    /// معادل دقیق `Profile.kt::connectTimeoutMs()`
    pub fn connect_timeout_ms(&self) -> u64 {
        if self.has_manual_peer() { return 45_000; }
        match self.scan_mode {
            ScanMode::Turbo => 60_000,
            ScanMode::Balanced => 150_000,
            ScanMode::Stealth => 240_000,
            ScanMode::Thorough => 300_000,
            ScanMode::Ironclad => 360_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// این تست همان «قرارداد» بین اندروید و ویندوز است. اگر روزی خروجی فرق
    /// کند، CI باید قرمز شود.
    #[test]
    fn default_profile_matches_android_argv() {
        let p = ConnectionProfile::default();
        assert_eq!(p.to_args(), vec!["--balanced", "-4", "--quick-reconnect"]);
    }

    #[test]
    fn manual_peer_skips_scan_mode() {
        let p = ConnectionProfile {
            endpoint_mode: EndpointMode::ManualPeer,
            manual_peer: "188.114.96.1:2408".into(),
            protocol: Protocol::Masque,
            ..Default::default()
        };
        assert_eq!(p.to_args(), vec!["--masque", "-4", "--quick-reconnect", "--peer", "188.114.96.1:2408"]);
        assert_eq!(p.connect_timeout_ms(), 45_000);
    }

    #[test]
    fn noize_and_hardening_flags() {
        let p = ConnectionProfile {
            protocol: Protocol::Wireguard,
            noize: Noize::Gfw,
            fragment: true,
            ech: true,
            keepalive: 25,
            ..Default::default()
        };
        assert_eq!(
            p.to_args(),
            vec!["--wg", "--balanced", "-4", "--quick-reconnect", "--noize", "gfw",
                 "--fragment", "--ech", "auto", "--keepalive", "25"]
        );
    }
}
