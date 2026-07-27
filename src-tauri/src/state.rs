//! پورت از `core/AetherController.kt` + `vpn/AetherVpnService.kt` + `model/ConnectionState.kt`.
//!
//! ریشهٔ باگ «هیچ پروتکلی کانکت نمی‌شود» در نسخهٔ قبلی دسکتاپ:
//! `connect()` بلافاصله بعد از اجرای موتور، `Tunnel::establish` را صدا می‌زد؛
//! ساخت آداپتور Wintun بدون دسترسی Administrator شکست می‌خورد، خطا از
//! `connect()` بیرون می‌رفت و ماشین حالت برای همیشه روی StartingEngine گیر
//! می‌کرد — در حالی که موتور واقعاً وصل می‌شد (لاگ کاربر: «socks5 server
//! listening on 127.0.0.1:1819» بدون هیچ «I/state: Connecting» بعد از آن).
//!
//! حالا دقیقاً ترتیب اندروید (`connectAttempt`) اجرا می‌شود:
//!   ۱. StartingEngine → آزادشدن پورت → اجرای موتور → Connecting
//!   ۲. انتظار برای بازشدن پورت SOCKS5 (ground truth — همان PortProbe)
//!   ۳. فقط بعد از آن، مسیر داده برپا می‌شود (معادل VpnService.establish):
//!      پل HTTP/SOCKS محلی + پروکسی سیستمی ویندوز؛ Wintun هم اگر ممکن بود
//!      (شکست Wintun دیگر کل اتصال را نمی‌کُشد — فقط یک هشدار لاگ می‌شود).
//!   ۴. Verifying: خودآزمای ۴ مرحله‌ای (Diagnostics.kt) در ترد پس‌زمینه
//!   ۵. فقط بعد از قبولی همهٔ بررسی‌ها، Connected اعلام می‌شود
//!   ۶. شکست هر پله ← پلهٔ بعدی نردبان (معادل runLadder)، نه گیرکردن ابدی.

use crate::diagnostics;
use crate::engine::{self, AetherProcess};
use crate::log::DiagnosticsLog;
use crate::probe;
use crate::profile::{ConnectionProfile, Protocol};
use crate::share::ShareBridge;
use crate::smart_auto::{self, Candidate};
use crate::store::ProfileStore;
use crate::sysproxy;
use crate::tun::Tunnel;
use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TAG: &str = "state";

/// همان مقادیر اندروید: MAX_RETRIES=3، BACKOFF = 2s/5s/10s.
const MAX_RETRIES: u32 = 3;
const BACKOFF_MS: [u64; 3] = [2_000, 5_000, 10_000];
/// پنجرهٔ گریس خودآزما — همان `OUTBOUND_GRACE_MS` (شروع سرد warp-in-warp).
const OUTBOUND_GRACE_MS: u64 = 90_000;
/// معادل `PORT_RELEASE_WAIT_MS` اندروید.
const PORT_RELEASE_WAIT_MS: u64 = 3_000;

/// معادل دقیق `ConnectionState.kt` — همان هشت حالت، همان ترتیب.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionState {
    Disconnected,
    StartingEngine,
    Connecting,
    Verifying,
    Connected,
    Reconnecting,
    Disconnecting,
    Failed,
}

impl ConnectionState {
    pub fn is_busy(self) -> bool {
        matches!(
            self,
            Self::StartingEngine | Self::Connecting | Self::Verifying | Self::Reconnecting | Self::Disconnecting
        )
    }
    pub fn is_active(self) -> bool {
        self.is_busy() || self == Self::Connected
    }
}

/// معادل `IpInfo` در UI اندروید — خوراک نشان «IP + پرچم».
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpEndpoint {
    pub ip: String,
    pub country_code: Option<String>,
    /// true = IP خروجی سرور (از دل تونل)، false = IP واقعی کاربر.
    pub via_tunnel: bool,
}

/// حالت مشترک جست‌وجوی IP — معادل `ipInfo`/`ipLoading` در MainActivity.
struct IpSlot {
    info: Option<IpEndpoint>,
    loading: bool,
    /// شمارندهٔ نسل — نتیجهٔ جست‌وجوهای قدیمی دور ریخته می‌شود.
    session: u64,
}

/// معادل مجموع StateFlow‌هایی که HomeScreen.kt جمع می‌کرد.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub state: ConnectionState,
    pub detail: String,
    pub error: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: Option<String>,
    pub latency_ms: Option<u64>,
    pub uptime_secs: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub share_socks: Option<String>,
    pub share_http: Option<String>,
    pub ip_info: Option<IpEndpoint>,
    pub ip_loading: bool,
}

pub struct AetherController {
    data_dir: PathBuf,
    store: ProfileStore,
    profile: ConnectionProfile,
    state: ConnectionState,
    detail: String,
    error: Option<String>,
    endpoint: Option<String>,
    effective_protocol: Option<Protocol>,
    latency_ms: Option<u64>,
    connected_at: Option<Instant>,
    engine: AetherProcess,
    tunnel: Option<Tunnel>,
    share: ShareBridge,
    sysproxy_on: bool,
    /// نردبان تلاش‌ها — معادل `runLadder` در AetherVpnService.kt.
    plan: Vec<Candidate>,
    plan_index: usize,
    /// تلاش‌های اتصال مجدد پشت‌سرهم — معادل `reconnectAttempts`.
    attempts: u32,
    deadline: Option<Instant>,
    reconnect_at: Option<Instant>,
    /// نتیجهٔ خودآزمای در حال اجرا (ترد پس‌زمینه — UI فریز نمی‌شود).
    verify_slot: Option<Arc<Mutex<Option<diagnostics::SelfTestOutcome>>>>,
    ip_slot: Arc<Mutex<IpSlot>>,
    /// پینگ زنده: نتیجهٔ آخرین اندازه‌گیری دوره‌ای در ترد پس‌زمینه.
    latency_slot: Arc<Mutex<Option<u64>>>,
    /// زمان اندازه‌گیری بعدی پینگ.
    latency_probe_at: Option<Instant>,
}

impl AetherController {
    pub fn new(data_dir: &Path) -> Self {
        let store = ProfileStore::new(data_dir);
        let profile = store.load();
        let install_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| data_dir.to_path_buf());

        let ip_slot = Arc::new(Mutex::new(IpSlot { info: None, loading: false, session: 0 }));

        let me = Self {
            data_dir: data_dir.to_path_buf(),
            store,
            profile,
            state: ConnectionState::Disconnected,
            detail: String::new(),
            error: None,
            endpoint: None,
            effective_protocol: None,
            latency_ms: None,
            connected_at: None,
            engine: AetherProcess::new(&install_dir, data_dir),
            tunnel: None,
            share: ShareBridge::new(),
            sysproxy_on: false,
            plan: Vec::new(),
            plan_index: 0,
            attempts: 0,
            deadline: None,
            reconnect_at: None,
            verify_slot: None,
            ip_slot,
            latency_slot: Arc::new(Mutex::new(None)),
            latency_probe_at: None,
        };

        // پروکسی سیستمی به‌جامانده از کرش احتمالی جلسهٔ قبل را پاک می‌کنیم.
        sysproxy::disable();
        // معادل LaunchedEffect فاز idle در MainActivity: نمایش IP واقعی کاربر از لحظهٔ اجرا.
        spawn_ip_lookup(me.ip_slot.clone(), false);
        me
    }

    pub fn profile(&self) -> ConnectionProfile {
        self.profile.clone()
    }

    pub fn set_profile(&mut self, profile: ConnectionProfile) -> Result<()> {
        self.store.save(&profile)?;
        let lan_toggled = profile.lan_share != self.profile.lan_share;
        self.profile = profile;
        // Root fix for "Share over LAN shows no IP:port": flipping the switch
        // while a connection is active must rebind the bridge immediately
        // (mobile restarts its ShareBridge the same way), so the UI gets the
        // fresh endpoints in the very next snapshot instead of never.
        if lan_toggled && self.state.is_active() {
            if let Err(e) = self.share.start(
                engine::SHARE_SOCKS_PORT,
                engine::SHARE_HTTP_PORT,
                self.profile.lan_share,
            ) {
                DiagnosticsLog::e(TAG, &format!("Bridge restart after LAN toggle failed: {e}"));
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        let (tun_rx, tun_tx) = self.tunnel.as_ref().map(Tunnel::counters).unwrap_or((0, 0));
        let (br_rx, br_tx) = self.share.traffic();
        let (ip_info, ip_loading) = {
            let g = self.ip_slot.lock();
            (g.info.clone(), g.loading)
        };
        Snapshot {
            state: self.state,
            detail: self.detail.clone(),
            error: self.error.clone(),
            endpoint: self.endpoint.clone(),
            protocol: self.effective_protocol.map(|p| format!("{p:?}").to_uppercase()),
            latency_ms: self.latency_ms,
            uptime_secs: self.connected_at.map(|t| t.elapsed().as_secs()).unwrap_or(0),
            rx_bytes: tun_rx + br_rx,
            tx_bytes: tun_tx + br_tx,
            share_socks: self.share.socks_endpoint(),
            share_http: self.share.http_endpoint(),
            ip_info,
            ip_loading,
        }
    }

    /// معادل `onToggleConnection` — خطای اتصال دیگر به بیرون پرتاب نمی‌شود؛
    /// همیشه به حالت Failed ترجمه می‌شود تا UI هرگز در StartingEngine گیر نکند.
    pub fn toggle(&mut self) -> Result<()> {
        if self.state.is_active() {
            self.disconnect();
        } else if let Err(e) = self.connect() {
            let msg = e.to_string();
            self.fail(&msg);
        }
        Ok(())
    }

    /// معادل `connect()` سرویس اندروید — فقط برنامه‌ریزی و اجرای پلهٔ اول؛
    /// بقیهٔ مراحل در tick() دنبال می‌شوند.
    fn connect(&mut self) -> Result<()> {
        self.error = None;
        self.attempts = 0;
        // معادل DiagnosticsLog.clear + resetChecks در شروع اتصال اندروید.
        diagnostics::reset_checks();
        self.set_state(ConnectionState::StartingEngine, "Starting engine…");
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Connect requested — protocol={:?} scan={:?} ip={:?}",
                self.profile.protocol, self.profile.scan_mode, self.profile.ip_version
            ),
        );
        // SmartAuto.kt parity: fingerprint the network before planning. On a
        // filtered network the ladder leads with the hardened anti-DPI
        // candidate, so the plain first pass can no longer waste 35-75s
        // (slow connects) or win with a tunnel that cannot carry real
        // browser traffic afterwards.
        let hostile = probe::network_looks_filtered();
        self.plan = smart_auto::build_plan(&self.profile, hostile);
        self.plan_index = 0;
        self.start_candidate()
    }

    /// اجرای یک پله از نردبان — معادل یک دور `runLadder`.
    fn start_candidate(&mut self) -> Result<()> {
        let cand = self.plan[self.plan_index].clone();
        DiagnosticsLog::i(
            TAG,
            &format!("Attempt {}/{} → {}", self.plan_index + 1, self.plan.len(), cand.label),
        );

        // معادل PortProbe.awaitClosed — ریشهٔ باگ «تعویض پروتکل گیر می‌کند».
        if !engine::wait_for_port_release(
            engine::LOCAL_SOCKS_PORT,
            Duration::from_millis(PORT_RELEASE_WAIT_MS),
        ) {
            DiagnosticsLog::w(
                TAG,
                &format!(
                    "Local port {} is still busy after {}s — starting anyway.",
                    engine::LOCAL_SOCKS_PORT,
                    PORT_RELEASE_WAIT_MS / 1000
                ),
            );
        }

        self.effective_protocol = Some(cand.profile.protocol);
        self.engine.start(&cand.profile)?;
        self.deadline = Some(Instant::now() + Duration::from_millis(cand.timeout_ms));
        self.set_state(ConnectionState::Connecting, "Connecting…");
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Waiting for SOCKS5 on 127.0.0.1:{}… (timeout={}s)",
                engine::LOCAL_SOCKS_PORT,
                cand.timeout_ms / 1000
            ),
        );
        Ok(())
    }

    /// معادل بخش establish در connectAttempt — فقط بعد از بازشدن پورت SOCKS5.
    fn bring_up_data_path(&mut self) {
        let profile = self
            .plan
            .get(self.plan_index)
            .map(|c| c.profile.clone())
            .unwrap_or_else(|| self.profile.clone());

        // ۱) پل محلی HTTP/SOCKS — معادل hev-socks5-tunnel/ShareBridge (مسیر دادهٔ واقعی).
        if let Err(e) = self.share.start(engine::SHARE_SOCKS_PORT, engine::SHARE_HTTP_PORT, profile.lan_share) {
            DiagnosticsLog::e(TAG, &format!("Bridge failed to start: {e}"));
        }

        // ۲) پروکسی سیستمی ویندوز — معادل کارکرد VpnService (کل سیستم از تونل می‌رود).
        self.sysproxy_on = sysproxy::enable(engine::SHARE_HTTP_PORT);

        // ۳) Wintun — اختیاری. شکست آن دیگر اتصال را نمی‌کُشد (رفع ریشه‌ای گیر StartingEngine).
        if self.tunnel.is_none() {
            let wintun = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("engine").join("wintun.dll")))
                .unwrap_or_default();
            match Tunnel::establish(&profile, &wintun) {
                Ok(t) => self.tunnel = Some(t),
                Err(e) => DiagnosticsLog::w(
                    "tun",
                    &format!("Wintun unavailable ({e}); continuing with the system-proxy data path."),
                ),
            }
        }
    }

    /// خودآزمای ۴ مرحله‌ای در ترد پس‌زمینه — حلقهٔ tick هرگز مسدود نمی‌شود.
    fn begin_verification(&mut self) {
        let slot: Arc<Mutex<Option<diagnostics::SelfTestOutcome>>> = Arc::new(Mutex::new(None));
        self.verify_slot = Some(slot.clone());
        let remaining = self
            .deadline
            .map(|d| d.saturating_duration_since(Instant::now()).as_millis() as u64)
            .unwrap_or(OUTBOUND_GRACE_MS);
        let grace = remaining.clamp(20_000, OUTBOUND_GRACE_MS);
        std::thread::Builder::new()
            .name("aether-selftest".into())
            .spawn(move || {
                let outcome = diagnostics::self_test(grace);
                *slot.lock() = Some(outcome);
            })
            .ok();
        self.set_state(ConnectionState::Verifying, "Verifying…");
    }

    fn disconnect(&mut self) {
        self.set_state(ConnectionState::Disconnecting, "Disconnecting…");
        self.cleanup_native();
        // v16: تیک‌های سبز Diagnostics باید بلافاصله بعد از دیسکانکت
        // ریست شوند تا برای اتصال بعدی آماده باشند (معادل resetChecks اندروید).
        diagnostics::reset_checks();
        self.latency_probe_at = None;
        *self.latency_slot.lock() = None;
        self.connected_at = None;
        self.endpoint = None;
        self.latency_ms = None;
        self.effective_protocol = None;
        self.deadline = None;
        self.reconnect_at = None;
        self.verify_slot = None;
        self.plan.clear();
        self.plan_index = 0;
        self.set_state(ConnectionState::Disconnected, "");
    }

    /// ترتیب ۱.۲.۲: اول پروکسی سیستمی (تا مرورگر به پل مُرده نچسبد)، بعد
    /// اشتراک، بعد تونل، بعد موتور — بدون فریز.
    fn cleanup_native(&mut self) {
        if self.sysproxy_on {
            sysproxy::disable();
            self.sysproxy_on = false;
        }
        self.share.stop();
        if let Some(mut t) = self.tunnel.take() {
            t.close();
        }
        self.engine.stop();
    }

    /// شکست یک پله → پلهٔ بعدی نردبان؛ تمام‌شدن نردبان → Failed با پیام روشن.
    fn advance_or_fail(&mut self, why: &str) {
        DiagnosticsLog::w(TAG, &format!("{why} — tearing down this attempt."));
        // فقط موتور/مسیر داده را جمع می‌کنیم، وضعیت UI همچنان busy می‌ماند.
        self.cleanup_native();
        self.verify_slot = None;
        diagnostics::reset_checks();
        self.plan_index += 1;
        if self.plan_index < self.plan.len() {
            if let Err(e) = self.start_candidate() {
                let msg = e.to_string();
                self.fail(&msg);
            }
        } else if self.profile.protocol == Protocol::Smart {
            self.fail("Smart Auto tried every strategy and none passed the self-test on this network.");
        } else {
            self.fail(
                "This protocol could not establish a working tunnel on this network, even with anti-DPI hardening. Try Smart Auto or another protocol.",
            );
        }
    }

    /// هر ۲۰۰ms از main.rs صدا زده می‌شود — معادل حلقهٔ نظارت اندروید.
    pub fn tick(&mut self) {
        match self.state {
            ConnectionState::Connecting => {
                if !self.engine.is_alive() {
                    self.advance_or_fail("Engine exited before it opened the SOCKS5 port");
                    return;
                }
                if probe::socks_ready(engine::LOCAL_SOCKS_PORT) {
                    DiagnosticsLog::i(TAG, "SOCKS5 port is up — bringing up the data path.");
                    self.bring_up_data_path();
                    self.begin_verification();
                } else if self.past_deadline() {
                    self.advance_or_fail("Engine still scanning — the SOCKS5 port never opened in time");
                }
            }
            ConnectionState::Verifying => {
                let outcome = self.verify_slot.as_ref().and_then(|s| s.lock().take());
                if let Some(out) = outcome {
                    self.verify_slot = None;
                    if out.ok {
                        if let Some(exit) = &out.exit {
                            self.endpoint = Some(match &exit.country_code {
                                Some(cc) => format!("{} · {cc}", exit.ip),
                                None => exit.ip.clone(),
                            });
                            // IP خروجی از خودآزما مستقیماً به نشان IP می‌رود —
                            // معادل offerTunnelIpInfo در Diagnostics.kt.
                            let mut g = self.ip_slot.lock();
                            g.session += 1;
                            g.info = Some(IpEndpoint {
                                ip: exit.ip.clone(),
                                country_code: exit.country_code.clone(),
                                via_tunnel: true,
                            });
                            g.loading = false;
                        }
                        self.latency_ms = out.latency_ms;
                        self.connected_at = Some(Instant::now());
                        self.attempts = 0;
                        self.set_state(ConnectionState::Connected, "");
                        DiagnosticsLog::i(TAG, "All checks passed — tunnel is ready.");
                        if out.exit.is_none() {
                            spawn_ip_lookup(self.ip_slot.clone(), true);
                        }
                    } else {
                        self.advance_or_fail("Tunnel started, but the end-to-end self-test failed");
                    }
                } else if !self.engine.is_alive() {
                    self.advance_or_fail("The engine stopped during verification");
                }
            }
            ConnectionState::Connected => {
                // v16: پینگ نمایشی قبلاً فقط یک‌بار هنگام خودآزمای اتصال اندازه
                // گرفته می‌شد (شامل زمان دریافت HTTP در شلوغی لحظهٔ اتصال)
                // و دیگر به‌روز نمی‌شد — برای همین عددی مثل ۸۰۰۰ms می‌ماند.
                // حالا هر ۱۵ ثانیه یک اتصال TCP سبک از داخل تونل زمان‌گیری
                // می‌شود تا پینگ واقعی و زنده نمایش داده شود (بدون فریز UI).
                if let Some(ms) = self.latency_slot.lock().take() {
                    self.latency_ms = Some(ms);
                }
                let latency_due = self
                    .latency_probe_at
                    .map(|t| Instant::now() >= t)
                    .unwrap_or(true);
                if latency_due {
                    self.latency_probe_at = Some(Instant::now() + Duration::from_secs(15));
                    let slot = self.latency_slot.clone();
                    std::thread::Builder::new()
                        .name("aether-latency".into())
                        .spawn(move || {
                            let started = Instant::now();
                            if probe::tcp_via_proxy("1.1.1.1", 80) {
                                *slot.lock() = Some(started.elapsed().as_millis() as u64);
                            }
                        })
                        .ok();
                }
                if !self.engine.is_alive() {
                    // معادل superviseEngine: بک‌آف پلکانی ۲/۵/۱۰ ثانیه، حداکثر ۳ تلاش.
                    if self.attempts >= MAX_RETRIES {
                        self.fail("The engine keeps dying — giving up after repeated restarts.");
                        return;
                    }
                    let backoff = BACKOFF_MS[(self.attempts as usize).min(BACKOFF_MS.len() - 1)];
                    self.attempts += 1;
                    self.connected_at = None;
                    self.reconnect_at = Some(Instant::now() + Duration::from_millis(backoff));
                    DiagnosticsLog::w(
                        TAG,
                        &format!("Engine died while connected — restarting in {}s.", backoff / 1000),
                    );
                    let detail = format!("Attempt {} of {}", self.attempts, MAX_RETRIES);
                    self.set_state(ConnectionState::Reconnecting, &detail);
                }
            }
            ConnectionState::Reconnecting => {
                if let Some(at) = self.reconnect_at {
                    if Instant::now() >= at {
                        self.reconnect_at = None;
                        // همان پلهٔ برنده دوباره اجرا می‌شود — معادل restart در superviseEngine.
                        if self.plan.is_empty() {
                            self.plan = smart_auto::build_plan(&self.profile, probe::network_looks_filtered());
                            self.plan_index = 0;
                        }
                        self.cleanup_native();
                        if let Err(e) = self.start_candidate() {
                            let msg = e.to_string();
                            self.fail(&msg);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn past_deadline(&self) -> bool {
        self.deadline.map(|d| Instant::now() > d).unwrap_or(false)
    }

    fn fail(&mut self, why: &str) {
        DiagnosticsLog::e(TAG, why);
        self.cleanup_native();
        self.error = Some(why.to_string());
        self.connected_at = None;
        self.deadline = None;
        self.reconnect_at = None;
        self.verify_slot = None;
        self.set_state(ConnectionState::Failed, "Connection failed");
    }

    fn set_state(&mut self, state: ConnectionState, detail: &str) {
        let prev = self.state;
        self.state = state;
        self.detail = detail.to_string();
        DiagnosticsLog::i(TAG, &format!("{state:?} {detail}"));
        if prev != state {
            self.on_phase_change(state);
        }
    }

    /// معادل LaunchedEffect فازهای IP در MainActivity.kt:
    ///   connected → IP سرور از دل تونل — idle/failed → IP واقعی کاربر — busy → خالی.
    fn on_phase_change(&mut self, state: ConnectionState) {
        match state {
            ConnectionState::Connected => { /* خودآزما قبلاً IP را تحویل داده است */ }
            ConnectionState::Disconnected | ConnectionState::Failed => {
                spawn_ip_lookup(self.ip_slot.clone(), false);
            }
            _ => {
                let mut g = self.ip_slot.lock();
                g.session += 1;
                g.info = None;
                g.loading = false;
            }
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

impl Drop for AetherController {
    fn drop(&mut self) {
        // خروج برنامه هرگز نباید پروکسی سیستمی را فعال رها کند.
        self.cleanup_native();
    }
}

/// جست‌وجوی IP در ترد پس‌زمینه — همان تعداد تلاش/تأخیرهای NetProbe اندروید:
/// مستقیم ۶×۲۰۰۰ms، از دل تونل ۱۲×۱۰۰۰ms.
fn spawn_ip_lookup(slot: Arc<Mutex<IpSlot>>, via_tunnel: bool) {
    let session = {
        let mut g = slot.lock();
        g.session += 1;
        g.loading = true;
        if !via_tunnel {
            g.info = None;
        }
        g.session
    };
    std::thread::Builder::new()
        .name("aether-ipinfo".into())
        .spawn(move || {
            let result = if via_tunnel {
                probe::fetch_ip_via_socks_retry(12, 1_000, 6_000)
            } else {
                probe::fetch_ip_direct_retry(6, 2_000, 6_000)
            };
            let mut g = slot.lock();
            if g.session != session {
                return; // نتیجهٔ کهنه — فاز عوض شده است.
            }
            g.info = result.map(|i| IpEndpoint {
                ip: i.ip,
                country_code: i.country_code,
                via_tunnel,
            });
            g.loading = false;
        })
        .ok();
}
