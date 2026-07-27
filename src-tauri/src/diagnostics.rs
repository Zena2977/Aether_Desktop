//! پورت از `core/Diagnostics.kt`.
//!
//! همان خودآزمای ۴ مرحله‌ای اندروید که اعلام «Connected» را دروازه‌بانی می‌کند:
//!   ۱. پورت SOCKS5 باز است؟
//!   ۲. دست‌دادن SOCKS5 جواب می‌دهد؟
//!   ۳. TCP از دل پروکسی به IP خام (1.1.1.1:80) برقرار می‌شود؟
//!   ۴. DNS + HTTP واقعی از دل تونل (همراه با IP خروجی و کد کشور)؟
//!
//! مثل اندروید، مراحل ۳ و ۴ در یک پنجرهٔ گریس با تلاش مجدد هر ۷۵۰ms اجرا
//! می‌شوند (شروع سرد warp-in-warp چند ثانیه طول می‌کشد تا مسیر خروجی
//! واقعاً باز شود). نتیجهٔ هر مرحله زنده در `DiagnosticsLog::checks` منتشر
//! می‌شود تا پنل عیب‌یابی دقیقاً مثل موبایل رنگ عوض کند.

use crate::engine;
use crate::log::DiagnosticsLog;
use crate::probe;
use crate::profile::ConnectionProfile;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const TAG: &str = "diag";

// شناسهٔ بررسی‌ها — همان مقادیر Diagnostics.kt.
pub const C_PORT: &str = "socks_port";
pub const C_HANDSHAKE: &str = "socks_handshake";
pub const C_TCP: &str = "tcp_via_proxy";
pub const C_DNS: &str = "dns_http_via_tunnel";

/// تلاش مجدد هر ۷۵۰ms — همان مقدار اندروید.
const RETRY_DELAY_MS: u64 = 750;

/// معادل `Diagnostics.resetChecks()`.
pub fn reset_checks() {
    DiagnosticsLog::set_checks(vec![
        (C_PORT, format!("SOCKS5 port 127.0.0.1:{}", engine::LOCAL_SOCKS_PORT)),
        (C_HANDSHAKE, "SOCKS5 handshake".to_string()),
        (C_TCP, "TCP via proxy (1.1.1.1:80)".to_string()),
        (C_DNS, "DNS + HTTP via tunnel".to_string()),
    ]);
}

#[derive(Debug, Clone)]
pub struct SelfTestOutcome {
    pub ok: bool,
    pub exit: Option<probe::IpInfo>,
    pub latency_ms: Option<u64>,
}

/// معادل `Diagnostics.run()` — دروازهٔ اعلام Connected.
pub fn self_test(grace_ms: u64) -> SelfTestOutcome {
    reset_checks();
    DiagnosticsLog::i(TAG, "Starting connectivity self-test…");

    // ۱) پورت SOCKS5
    DiagnosticsLog::update_check(C_PORT, "RUNNING", None);
    let port_open = probe::socks_ready(engine::LOCAL_SOCKS_PORT);
    if port_open {
        DiagnosticsLog::update_check(C_PORT, "PASS", Some("listening"));
        DiagnosticsLog::i(TAG, "SOCKS5 port check: open");
    } else {
        DiagnosticsLog::update_check(C_PORT, "FAIL", Some("no listener"));
        DiagnosticsLog::e(TAG, "SOCKS5 port check: nothing is listening — the engine is not up.");
        DiagnosticsLog::update_check(C_HANDSHAKE, "FAIL", Some("skipped"));
        DiagnosticsLog::update_check(C_TCP, "FAIL", Some("skipped"));
        DiagnosticsLog::update_check(C_DNS, "FAIL", Some("skipped"));
        return SelfTestOutcome { ok: false, exit: None, latency_ms: None };
    }

    // ۲) دست‌دادن SOCKS5
    DiagnosticsLog::update_check(C_HANDSHAKE, "RUNNING", None);
    let hs = probe::socks_handshake_ok();
    if hs {
        DiagnosticsLog::update_check(C_HANDSHAKE, "PASS", Some("method accepted"));
        DiagnosticsLog::i(TAG, "SOCKS5 handshake: OK");
    } else {
        DiagnosticsLog::update_check(C_HANDSHAKE, "FAIL", Some("no SOCKS5 reply"));
        DiagnosticsLog::e(TAG, "SOCKS5 handshake failed — the port is open but it is not a SOCKS5 server.");
    }

    // ۳+۴) TCP و DNS/HTTP — با تلاش مجدد در پنجرهٔ گریس (معادل اجرای هم‌زمان موبایل)
    DiagnosticsLog::update_check(C_TCP, "RUNNING", None);
    DiagnosticsLog::update_check(C_DNS, "RUNNING", None);
    let deadline = Instant::now() + Duration::from_millis(grace_ms);
    let mut tcp_ok = false;
    let mut dns: Option<(probe::IpInfo, u64)> = None;
    loop {
        if !tcp_ok {
            tcp_ok = probe::tcp_via_proxy("1.1.1.1", 80);
            if tcp_ok {
                DiagnosticsLog::update_check(C_TCP, "PASS", Some("connected"));
                DiagnosticsLog::i(TAG, "TCP via proxy: OK");
            }
        }
        if dns.is_none() {
            let started = Instant::now();
            if let Some(info) = probe::fetch_ip_via_socks(6_000) {
                let ms = started.elapsed().as_millis() as u64;
                let cc = info.country_code.clone().unwrap_or_else(|| "??".to_string());
                DiagnosticsLog::update_check(C_DNS, "PASS", Some(&format!("exit {} {}", info.ip, cc)));
                // v8 audit: the full IP stays in the UI check detail only; the
                // persistent log gets a masked last octet.
                DiagnosticsLog::i(TAG, &format!("DNS+HTTP via tunnel: OK — exit {} {} ({} ms)", mask_ip(&info.ip), cc, ms));
                dns = Some((info, ms));
            }
        }
        if tcp_ok && dns.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
    }

    if !tcp_ok {
        DiagnosticsLog::update_check(C_TCP, "FAIL", Some("could not reach 1.1.1.1:80 through the proxy"));
        DiagnosticsLog::e(TAG, "TCP via proxy failed — the engine accepted the connection but no upstream is flowing.");
    }
    let ok = dns.is_some();
    if !ok {
        DiagnosticsLog::update_check(C_DNS, "FAIL", Some("no HTTP response through the tunnel"));
        if tcp_ok {
            DiagnosticsLog::w(TAG, "Raw TCP works but DNS+HTTP failed — upstream DNS looks broken.");
        } else {
            DiagnosticsLog::w(TAG, "No outbound path at all — the tunnel has no upstream yet.");
        }
    }

    let (exit, latency_ms) = match dns {
        Some((info, ms)) => (Some(info), Some(ms)),
        None => (None, None),
    };
    SelfTestOutcome { ok, exit, latency_ms }
}

/// v8 audit: "1.2.3.4" -> "1.2.3.xxx" for the persistent rotating log so a
/// leaked log file cannot reveal the exact exit IP. IPv6 keeps only the /48.
fn mask_ip(ip: &str) -> String {
    if let Some((head, _)) = ip.rsplit_once('.') {
        return format!("{head}.xxx");
    }
    if ip.contains(':') {
        let parts: Vec<&str> = ip.split(':').take(3).collect();
        return format!("{}::xxxx", parts.join(":"));
    }
    ip.to_string()
}

// ---------------------------------------------------------------------------
// گزارش محیطی دسکتاپ (مکمل، مخصوص ویندوز) — همان گزارش قبلی حفظ شده
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub verdict: Verdict,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub checks: Vec<Check>,
    pub summary: String,
}

fn check(name: &str, verdict: Verdict, detail: impl Into<String>) -> Check {
    Check { name: name.into(), verdict, detail: detail.into() }
}

/// گزارش سلامت محیط نصب — دکمهٔ «Environment check».
pub fn run(profile: &ConnectionProfile) -> Report {
    let mut checks = Vec::new();

    // ۱) باینری موتور
    let engine_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("engine").join("aether.exe")));
    checks.push(match &engine_exe {
        Some(p) if p.exists() => check("Engine binary", Verdict::Pass, p.display().to_string()),
        Some(p) => check("Engine binary", Verdict::Fail, format!("Missing: {}", p.display())),
        None => check("Engine binary", Verdict::Fail, "Could not resolve the install directory"),
    });

    // ۲) درایور Wintun — اختیاری؛ مسیر دادهٔ اصلی پروکسی سیستمی است
    let wintun = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("engine").join("wintun.dll")));
    checks.push(match &wintun {
        Some(p) if p.exists() => check("Wintun driver", Verdict::Pass, "Present"),
        _ => check("Wintun driver", Verdict::Warn, "wintun.dll missing — system-proxy mode still works"),
    });

    // ۳) دسترسی مدیر — فقط برای آداپتور Wintun لازم است، نه برای اتصال
    checks.push(if is_elevated() {
        check("Administrator rights", Verdict::Pass, "Running elevated")
    } else {
        check(
            "Administrator rights",
            Verdict::Warn,
            "Not elevated — connection uses the Windows system proxy instead of a TUN adapter",
        )
    });

    // ۴) پورت SOCKS5 محلی
    checks.push(if probe::socks_ready(engine::LOCAL_SOCKS_PORT) {
        check("Local SOCKS5", Verdict::Pass, format!("127.0.0.1:{}", engine::LOCAL_SOCKS_PORT))
    } else {
        check("Local SOCKS5", Verdict::Warn, "Not listening (expected while disconnected)")
    });

    // ۵) خروج واقعی
    checks.push(match probe::verify_egress() {
        Some(r) => check("Tunnel egress", Verdict::Pass, format!("{} in {} ms", r.endpoint, r.latency_ms)),
        None => check("Tunnel egress", Verdict::Warn, "No verified egress yet"),
    });

    // ۶) سلامت پروفایل
    checks.push(if profile.mtu >= 576 && profile.mtu <= 9000 {
        check("Profile", Verdict::Pass, format!("MTU {}", profile.mtu))
    } else {
        check("Profile", Verdict::Warn, format!("Unusual MTU: {}", profile.mtu))
    });

    let failed = checks.iter().filter(|c| c.verdict == Verdict::Fail).count();
    let warned = checks.iter().filter(|c| c.verdict == Verdict::Warn).count();
    let summary = if failed > 0 {
        format!("{failed} blocking problem(s) found")
    } else if warned > 0 {
        format!("{warned} warning(s), nothing blocking")
    } else {
        "Everything looks healthy".to_string()
    };

    Report { checks, summary }
}

#[cfg(windows)]
fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_always_has_every_check() {
        let r = run(&ConnectionProfile::default());
        assert_eq!(r.checks.len(), 6);
        assert!(!r.summary.is_empty());
    }

    #[test]
    fn reset_populates_the_four_android_checks() {
        reset_checks();
        let checks = DiagnosticsLog::checks();
        assert_eq!(checks.len(), 4);
        assert!(checks.iter().all(|c| c.state == "PENDING"));
    }
}
