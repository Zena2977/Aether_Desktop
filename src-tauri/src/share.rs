//! پورت از `core/ShareBridge.kt` — حالا یک «رلهٔ واقعی» است، نه فقط اعلانِ آدرس.
//!
//! ریشهٔ باگ ترافیک: نسخهٔ قبلی این فایل فقط لاگ می‌نوشت و هیچ سوکتی باز
//! نمی‌کرد. حالا دقیقاً مثل ShareBridge اندروید دو شنونده داریم:
//!   * SOCKS5 (10810): پس‌دهی خام به SOCKS5 موتور روی 127.0.0.1:1819
//!   * HTTP  (10811): پروکسی HTTP کامل (CONNECT + absolute-form) که هر اتصال
//!     را از داخل SOCKS5 موتور بیرون می‌برد — مسیر دادهٔ پروکسی سیستمی ویندوز.
//!
//! رفع ریشه‌ای «کانکت می‌شود ولی هیچ سایتی باز نمی‌شود» (v6 → v7):
//!   شنونده‌ها با `set_nonblocking(true)` ساخته می‌شوند تا حلقهٔ accept
//!   قابل‌توقف باشد. در ویندوز (WinSock) سوکتِ پذیرفته‌شده حالتِ
//!   non-blocking را «به ارث می‌برد»؛ نتیجه: اولین `read()` روی اتصال مرورگر
//!   بلافاصله WouldBlock برمی‌گرداند، read_head/relay آن را خطا حساب می‌کرد
//!   و اتصال را می‌بست — مرورگر CONNECT می‌فرستاد (tx>0) ولی هرگز پاسخی
//!   نمی‌گرفت (rx=0). خودآزمای داخلی مستقیم به موتور (1819) وصل می‌شود و از
//!   این پل رد نمی‌شود، برای همین «سبز» بود. حالا هر سوکت پذیرفته‌شده صریحاً
//!   به حالت blocking برمی‌گردد.
//!
//! رفع ریشه‌ای «os error 10048» هنگام روشن/خاموش‌کردن اشتراک LAN:
//!   stop() فقط پرچم را false می‌کرد و بلافاصله start() دوباره bind می‌کرد؛
//!   ترد شنوندهٔ قبلی هنوز پورت را نگه داشته بود (و چون همان Arc دوباره true
//!   می‌شد، اصلاً خارج نمی‌شد). حالا هر نسل پرچم مخصوص خودش را دارد،
//!   stop() ترد‌ها را join می‌کند و bind هم چند بار با درنگ تلاش می‌شود.
//!
//! سخت‌سازی امنیتی: بدون «اشتراک LAN» فقط روی 127.0.0.1 گوش می‌دهیم و در
//! حالت LAN فقط همتاهای شبکهٔ خصوصی (RFC1918/link-local) پذیرفته می‌شوند تا
//! اگر دستگاه مستقیماً IP عمومی داشته باشد، پروکسی به اینترنت باز نشود.

use crate::log::DiagnosticsLog;
use crate::probe;
use anyhow::Result;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

const TAG: &str = "share";
const UPSTREAM_TIMEOUT: Duration = Duration::from_millis(10_000);
/// bind پس از ری‌استارت ممکن است تا آزادشدن پورتِ نسل قبل چند صد میلی‌ثانیه
/// طول بکشد؛ ۲۰ تلاش × ۱۵۰ms = حداکثر ۳ ثانیه صبر.
const BIND_RETRIES: u32 = 20;
const BIND_RETRY_DELAY: Duration = Duration::from_millis(150);

pub struct ShareBridge {
    running: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    lan_ip: Option<IpAddr>,
    lan_enabled: bool,
    socks_port: u16,
    http_port: u16,
    rx: Arc<AtomicU64>,
    tx: Arc<AtomicU64>,
}

impl Default for ShareBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl ShareBridge {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            threads: Vec::new(),
            lan_ip: None,
            lan_enabled: false,
            socks_port: 0,
            http_port: 0,
            rx: Arc::new(AtomicU64::new(0)),
            tx: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// شمارندهٔ ترافیک (دانلود، آپلود) بر حسب بایت — خوراک TrafficPanel.
    pub fn traffic(&self) -> (u64, u64) {
        (self.rx.load(Ordering::Relaxed), self.tx.load(Ordering::Relaxed))
    }

    /// معادل `ShareBridge.start()` — رلهٔ واقعی را بالا می‌آورد.
    ///
    /// `lan=false`: فقط 127.0.0.1 (مسیر دادهٔ پروکسی سیستمی خود دستگاه).
    /// `lan=true`: علاوه بر آن، روی IP شبکهٔ محلی هم گوش می‌دهد.
    pub fn start(&mut self, socks_port: u16, http_port: u16, lan: bool) -> Result<()> {
        // نسل قبلی کاملاً پایین بیاید (تردها join می‌شوند) تا پورت‌ها آزاد شوند.
        self.stop();
        // هر نسل پرچم مخصوص خودش را دارد؛ وگرنه start بعدی پرچم مشترک را دوباره
        // true می‌کرد و ترد نسل قبل هرگز خارج نمی‌شد (ریشهٔ os error 10048).
        self.running = Arc::new(AtomicBool::new(true));
        self.rx.store(0, Ordering::Relaxed);
        self.tx.store(0, Ordering::Relaxed);
        self.socks_port = socks_port;
        self.http_port = http_port;
        self.lan_enabled = lan;
        self.lan_ip = if lan { detect_lan_ip() } else { None };

        let mut binds: Vec<IpAddr> = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
        if let Some(ip) = self.lan_ip {
            if ip != IpAddr::V4(Ipv4Addr::LOCALHOST) {
                binds.push(ip);
            }
        }
        let mut localhost_http_up = false;
        for ip in binds {
            let is_local = ip == IpAddr::V4(Ipv4Addr::LOCALHOST);
            if let Some(h) = spawn_listener(ip, http_port, self.running.clone(), self.rx.clone(), self.tx.clone()) {
                self.threads.push(h);
                if is_local {
                    localhost_http_up = true;
                }
            }
            if let Some(h) = spawn_listener(ip, socks_port, self.running.clone(), self.rx.clone(), self.tx.clone()) {
                self.threads.push(h);
            }
        }
        if !localhost_http_up {
            DiagnosticsLog::e(
                TAG,
                "HTTP bridge could not bind on 127.0.0.1 — the system-proxy data path is down!",
            );
            return Err(anyhow::anyhow!("http bridge bind failed"));
        }
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Bridge up: socks5={socks_port} http={http_port}{}",
                match self.lan_ip {
                    Some(ip) => format!(" (shared on {ip})"),
                    None => " (localhost only)".to_string(),
                }
            ),
        );
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            DiagnosticsLog::i(TAG, "Bridge stopped.");
        }
        // شنونده‌ها حداکثر ~۸۰ms بعد پرچم را می‌بینند، سوکت را رها می‌کنند و
        // خارج می‌شوند؛ join یعنی وقتی برمی‌گردیم پورت‌ها واقعاً آزادند.
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        self.lan_ip = None;
        self.lan_enabled = false;
    }

    /// آدرس اشتراک LAN — فقط وقتی اشتراک فعال است (رفتار قبلی UI حفظ می‌شود).
    pub fn socks_endpoint(&self) -> Option<String> {
        if !self.lan_enabled {
            return None;
        }
        self.lan_ip.map(|ip| format!("{ip}:{}", self.socks_port))
    }

    pub fn http_endpoint(&self) -> Option<String> {
        if !self.lan_enabled {
            return None;
        }
        self.lan_ip.map(|ip| format!("{ip}:{}", self.http_port))
    }
}

impl Drop for ShareBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

/// bind با تلاش مجدد — بلافاصله بعد از ری‌استارت، پورت ممکن است هنوز در
/// TIME_WAIT/انتظار بستهٔ نسل قبل باشد (os error 10048).
fn bind_with_retry(ip: IpAddr, port: u16) -> Option<TcpListener> {
    let mut last_err = None;
    for _ in 0..BIND_RETRIES {
        match TcpListener::bind((ip, port)) {
            Ok(l) => return Some(l),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(BIND_RETRY_DELAY);
            }
        }
    }
    if let Some(e) = last_err {
        DiagnosticsLog::e(TAG, &format!("Could not listen on {ip}:{port} — {e}"));
    }
    None
}

/// فقط همتاهایی که واقعاً «محلی» هستند حق استفاده از پل را دارند:
/// خود دستگاه (loopback) یا دستگاه‌های شبکهٔ خصوصی در حالت اشتراک LAN.
fn peer_allowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn spawn_listener(
    ip: IpAddr,
    port: u16,
    running: Arc<AtomicBool>,
    rx: Arc<AtomicU64>,
    tx: Arc<AtomicU64>,
) -> Option<JoinHandle<()>> {
    let listener = bind_with_retry(ip, port)?;
    let _ = listener.set_nonblocking(true);
    std::thread::Builder::new()
        .name(format!("aether-bridge-{port}"))
        .spawn(move || {
            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((client, peer)) => {
                        // رفع ریشه‌ای مشکل «هیچ سایتی باز نمی‌شود»: سوکت
                        // پذیرفته‌شده در ویندوز non-blocking بودنِ شنونده را به
                        // ارث می‌برد و همهٔ read/write ها فوراً WouldBlock
                        // می‌شدند. صریحاً blocking می‌کنیم.
                        if client.set_nonblocking(false).is_err() {
                            let _ = client.shutdown(Shutdown::Both);
                            continue;
                        }
                        if !peer_allowed(peer.ip()) {
                            DiagnosticsLog::w(TAG, &format!("Rejected non-local peer {peer}"));
                            let _ = client.shutdown(Shutdown::Both);
                            continue;
                        }
                        let (rx, tx) = (rx.clone(), tx.clone());
                        std::thread::spawn(move || handle_client(client, rx, tx));
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(80));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
            // خروج از حلقه = drop شدن listener = آزادشدن قطعی پورت.
        })
        .ok()
}

/// v8 root fix: Android Wi-Fi proxy settings only speak HTTP, so phones sent
/// HTTP to the SOCKS port and got nothing (and Telegram only uses its own
/// in-app SOCKS5 proxy). Instead of trusting the port, sniff the first byte:
/// SOCKS5 always starts with 0x05, HTTP with an ASCII letter. Both bridge
/// ports (10810/10811) now accept BOTH protocols - no "wrong port" exists.
fn handle_client(client: TcpStream, rx: Arc<AtomicU64>, tx: Arc<AtomicU64>) {
    let _ = client.set_nodelay(true);
    // A silent client must not pin this thread forever while we sniff.
    let _ = client.set_read_timeout(Some(Duration::from_millis(15_000)));
    let mut first = [0u8; 1];
    match client.peek(&mut first) {
        Ok(n) if n > 0 => {
            if first[0] == 0x05 {
                handle_socks(client, rx, tx)
            } else {
                handle_http(client, rx, tx)
            }
        }
        _ => {
            let _ = client.shutdown(Shutdown::Both);
        }
    }
}

/// پس‌دهی خام SOCKS5: بایت‌ها همان‌طور که هستند به موتور می‌روند و برمی‌گردند.
fn handle_socks(client: TcpStream, rx: Arc<AtomicU64>, tx: Arc<AtomicU64>) {
    let upstream_addr = SocketAddr::from(([127, 0, 0, 1], crate::engine::LOCAL_SOCKS_PORT));
    match TcpStream::connect_timeout(&upstream_addr, UPSTREAM_TIMEOUT) {
        Ok(upstream) => {
            let _ = upstream.set_nodelay(true);
            relay(client, upstream, rx, tx);
        }
        Err(_) => {
            let _ = client.shutdown(Shutdown::Both);
        }
    }
}

/// پروکسی HTTP: هم CONNECT (برای HTTPS) هم درخواست‌های absolute-form (HTTP خام).
fn handle_http(mut client: TcpStream, rx: Arc<AtomicU64>, tx: Arc<AtomicU64>) {
    let _ = client.set_read_timeout(Some(Duration::from_millis(15_000)));
    let Some((head, remainder)) = read_head(&mut client) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let text = String::from_utf8_lossy(&head).into_owned();
    let Some(first_line) = text.lines().next().map(|s| s.to_string()) else {
        return;
    };
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        let _ = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        return;
    }

    if parts[0].eq_ignore_ascii_case("CONNECT") {
        // HTTPS: CONNECT host:port
        let (host, port) = split_host_port(parts[1], 443);
        match probe::socks5_stream(&host, port, UPSTREAM_TIMEOUT) {
            Some(upstream) => {
                if client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").is_err() {
                    return;
                }
                relay(client, upstream, rx, tx);
            }
            None => {
                let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
            }
        }
        return;
    }

    // HTTP خام: METHOD http://host[:port]/path HTTP/1.1
    let Some(url_rest) = parts[1].strip_prefix("http://") else {
        let _ = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        return;
    };
    let (host_port, path) = match url_rest.find('/') {
        Some(i) => (&url_rest[..i], &url_rest[i..]),
        None => (url_rest, "/"),
    };
    let (host, port) = split_host_port(host_port, 80);
    let Some(mut upstream) = probe::socks5_stream(&host, port, UPSTREAM_TIMEOUT) else {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
        return;
    };

    // بازنویسی خط اول به origin-form و حذف سرآیندهای مخصوص پروکسی.
    let mut rebuilt = format!("{} {} {}\r\n", parts[0], path, parts[2]);
    for line in text.split("\r\n").skip(1) {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("proxy-connection:") || lower.starts_with("proxy-authorization:") {
            continue;
        }
        rebuilt.push_str(line);
        rebuilt.push_str("\r\n");
    }
    rebuilt.push_str("\r\n");
    if upstream.write_all(rebuilt.as_bytes()).is_err() || upstream.write_all(&remainder).is_err() {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
        return;
    }
    tx.fetch_add((rebuilt.len() + remainder.len()) as u64, Ordering::Relaxed);
    relay(client, upstream, rx, tx);
}

/// خواندن سرآیند HTTP تا `\r\n\r\n`؛ بایت‌های اضافه (شروع بدنه) جدا برمی‌گردند.
fn read_head(client: &mut TcpStream) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    loop {
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            let rest = buf.split_off(pos + 4);
            return Some((buf, rest));
        }
        if buf.len() > 32_768 {
            return None;
        }
        match client.read(&mut chunk) {
            Ok(0) => return None,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

fn find_subsequence(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn split_host_port(input: &str, default_port: u16) -> (String, u16) {
    match input.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(port) => (h.to_string(), port),
            Err(_) => (input.to_string(), default_port),
        },
        None => (input.to_string(), default_port),
    }
}

/// کپی دوطرفه با شمارش بایت‌ها — دانلود = سرور→کلاینت، آپلود = کلاینت→سرور.
fn relay(client: TcpStream, upstream: TcpStream, rx: Arc<AtomicU64>, tx: Arc<AtomicU64>) {
    // هر مهلت خواندن/نوشتنی که در مرحلهٔ دست‌دادن روی سوکت‌ها مانده پاک شود؛
    // یک دانلود طولانی نباید بعد از ۱۰ ثانیه سکوتِ یک طرف قطع شود.
    let _ = client.set_read_timeout(None);
    let _ = client.set_write_timeout(None);
    let _ = upstream.set_read_timeout(None);
    let _ = upstream.set_write_timeout(None);
    let client2 = match client.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let upstream2 = match upstream.try_clone() {
        Ok(u) => u,
        Err(_) => return,
    };
    let up = std::thread::spawn(move || copy_counted(client, upstream, tx));
    copy_counted(upstream2, client2, rx);
    let _ = up.join();
}

fn copy_counted(mut from: TcpStream, mut to: TcpStream, counter: Arc<AtomicU64>) {
    let mut buf = [0u8; 16_384];
    loop {
        match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if to.write_all(&buf[..n]).is_err() {
                    break;
                }
                counter.fetch_add(n as u64, Ordering::Relaxed);
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    let _ = to.shutdown(Shutdown::Both);
    let _ = from.shutdown(Shutdown::Both);
}

/// معادل `ShareBridge.lanAddress()` — همان ترفند سوکت UDP.
fn detect_lan_ip() -> Option<IpAddr> {
    // UDP-connect trick: no packet is actually sent; the OS only picks the
    // source address it would route through. Probing a public anycast IP
    // first yields the default-route interface address on ANY subnet - the
    // old hard-coded 192.168.1.1 broke on 10.x / 172.16.x / 192.168.0.x
    // networks. Gateway-style fallbacks cover LANs without a default route.
    const PROBES: [(&str, u16); 4] =
        [("1.1.1.1", 80), ("8.8.8.8", 80), ("192.168.1.1", 9), ("10.0.0.1", 9)];
    for (host, port) in PROBES {
        let sock = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if sock.connect((host, port)).is_err() {
            continue;
        }
        if let Ok(addr) = sock.local_addr() {
            let ip = addr.ip();
            if is_lan_address(ip) {
                return Some(ip);
            }
        }
    }
    None
}

/// True only for addresses other LAN devices can actually reach (site-local
/// IPv4) - mirrors the isSiteLocalAddress filter in the mobile ShareBridge.
fn is_lan_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_loopback()
                && !v4.is_unspecified()
                && (v4.is_private() || v4.is_link_local())
        }
        IpAddr::V6(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_none_until_started() {
        let b = ShareBridge::new();
        assert!(b.socks_endpoint().is_none());
        assert!(b.http_endpoint().is_none());
        assert!(!b.is_running());
    }

    #[test]
    fn ports_match_the_android_values() {
        assert_eq!(crate::engine::SHARE_SOCKS_PORT, 10810);
        assert_eq!(crate::engine::SHARE_HTTP_PORT, 10811);
    }

    #[test]
    fn host_port_splitting() {
        assert_eq!(split_host_port("example.com:8443", 443), ("example.com".to_string(), 8443));
        assert_eq!(split_host_port("example.com", 80), ("example.com".to_string(), 80));
    }

    #[test]
    fn only_local_peers_are_allowed() {
        assert!(peer_allowed("127.0.0.1".parse().unwrap()));
        assert!(peer_allowed("192.168.1.20".parse().unwrap()));
        assert!(peer_allowed("10.4.5.6".parse().unwrap()));
        assert!(!peer_allowed("8.8.8.8".parse().unwrap()));
        assert!(!peer_allowed("2001:4860:4860::8888".parse().unwrap()));
    }

    #[test]
    fn restart_reuses_ports_without_10048() {
        // start → stop → start روی همان پورت‌ها نباید با AddrInUse بمیرد.
        let mut b = ShareBridge::new();
        b.start(48810, 48811, false).expect("first start");
        b.start(48810, 48811, false).expect("restart");
        b.stop();
    }
}
