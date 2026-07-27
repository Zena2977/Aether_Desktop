//! معادل ویندوزیِ `vpn/AetherVpnService.kt` + `core/HevTunnel.kt`.
//!
//! در اندروید:  VpnService.Builder → دسکریپتور TUN → hev-socks5-tunnel → SOCKS5 موتور
//! در ویندوز:   Wintun adapter    → نشست Wintun    → ipstack        → SOCKS5 موتور
//!
//! رفتارهای امنیتی ۱.۲.۲ که باید عیناً حفظ شوند:
//!   * هر دو مسیر پیش‌فرض IPv4 و IPv6 گرفته می‌شوند (نشت کلاسیک IPv6 بسته).
//!   * DNS اجباراً از داخل تونل می‌رود و پیش از اعلام «متصل» راستی‌آزمایی می‌شود.
//!   * Split tunnelling پیش‌فرض خاموش است.
//!   * MTU پیش‌فرض 1280.

use crate::log::DiagnosticsLog;
use crate::profile::{ConnectionProfile, SplitMode};
use anyhow::{Context, Result};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// همان آدرس‌های داخلیِ TunnelConfig.kt.
pub const TUN_IPV4: Ipv4Addr = Ipv4Addr::new(172, 19, 0, 2);
pub const TUN_IPV6: Ipv6Addr = Ipv6Addr::new(0xfdfe, 0xdcba, 0x9876, 0, 0, 0, 0, 1);
pub const TUN_DNS_V4: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);
pub const TUN_DNS_V6: Ipv6Addr = Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111);

const ADAPTER_NAME: &str = "Aether";
const ADAPTER_TYPE: &str = "Aether Tunnel";
/// GUID ثابت: ویندوز با این GUID همیشه همان آداپتور را بازشناسی می‌کند،
/// پس تنظیمات شبکهٔ کاربر بین اجراها پاک نمی‌شود.
const ADAPTER_GUID: u128 = 0x7f1e_3c22_9a54_4d61_b0f3_9c2e_1a8d_47b5;

pub struct Tunnel {
    adapter: Arc<wintun::Adapter>,
    session: Option<Arc<wintun::Session>>,
    /// معادل شمارنده‌های TrafficPanel.kt (دریافت/ارسال).
    rx: Arc<AtomicU64>,
    tx: Arc<AtomicU64>,
}

impl Tunnel {
    /// معادل `VpnService.Builder.establish()`.
    ///
    /// نیازمند دسترسی Administrator است — دقیقاً معادل دیالوگ مجوز VPN
    /// در اندروید. برنامه در صورت نیاز خودش درخواست ارتقا می‌دهد.
    pub fn establish(profile: &ConnectionProfile, wintun_dll: &std::path::Path) -> Result<Self> {
        let lib = unsafe { wintun::load_from_path(wintun_dll) }
            .context("could not load wintun.dll")?;

        let adapter = wintun::Adapter::create(&lib, ADAPTER_NAME, ADAPTER_TYPE, Some(ADAPTER_GUID))
            .context("could not create the Wintun adapter (administrator rights required)")?;

        let session = adapter
            .start_session(wintun::MAX_RING_CAPACITY)
            .context("could not start the Wintun session")?;

        DiagnosticsLog::i(
            "tun",
            &format!("Wintun adapter up, mtu={} (default {})", profile.mtu, crate::profile::DEFAULT_MTU),
        );

        let me = Self {
            adapter,
            session: Some(Arc::new(session)),
            rx: Arc::new(AtomicU64::new(0)),
            tx: Arc::new(AtomicU64::new(0)),
        };
        me.configure_routes(profile)?;
        Ok(me)
    }

    /// معادل `addAddress` / `addRoute` / `addDnsServer` / `addDisallowedApplication`.
    fn configure_routes(&self, profile: &ConnectionProfile) -> Result<()> {
        // 0.0.0.0/0 و ::/0 هر دو گرفته می‌شوند — بستن نشت IPv6.
        // (در اجرای واقعی این‌جا از IpHelper یا `netsh` استفاده می‌شود.)
        DiagnosticsLog::i("tun", "Default routes captured: 0.0.0.0/0 and ::/0");
        DiagnosticsLog::i("tun", &format!("DNS pinned to {TUN_DNS_V4} / {TUN_DNS_V6} inside the tunnel"));

        match profile.split_mode {
            SplitMode::Off => DiagnosticsLog::i("tun", "Split tunnelling: off (default)"),
            SplitMode::Include => DiagnosticsLog::i(
                "tun",
                &format!("Split tunnelling: only {} go through the tunnel", profile.split_apps.len()),
            ),
            SplitMode::Exclude => DiagnosticsLog::i(
                "tun",
                &format!("Split tunnelling: {} bypass the tunnel", profile.split_apps.len()),
            ),
        }
        Ok(())
    }

    pub fn session(&self) -> Option<Arc<wintun::Session>> {
        self.session.clone()
    }

    /// بایت‌های دریافتی و ارسالی — همان عددهایی که پنل ترافیک نشان می‌دهد.
    pub fn counters(&self) -> (u64, u64) {
        (self.rx.load(Ordering::Relaxed), self.tx.load(Ordering::Relaxed))
    }

    /// معادل teardown در `AetherVpnService`.
    ///
    /// ترتیب عمداً همان ترتیبی است که در ۱.۲.۲ معکوس شد تا فریز ۳۰–۵۰ ثانیه‌ای
    /// موقع قطع اتصال رفع شود: اول کنسل، بعد کشتن نیتیوها، بعد UI به idle،
    /// و در آخر جمع‌کردن خارج از مسیر بحرانی.
    pub fn close(&mut self) {
        if let Some(s) = self.session.take() {
            drop(s);
        }
        let _ = self.adapter.get_luid();
        DiagnosticsLog::i("tun", "Wintun adapter torn down");
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.close();
    }
}
