//! پورت از `core/NetProbe.kt` + `core/PortProbe.kt`.
//!
//! قاعدهٔ ۱.۲.۲ که عیناً حفظ می‌شود: تا وقتی خروج واقعی از داخل تونل
//! تأیید نشده، برنامه حق ندارد بگوید «Connected».
//!
//! جدید (هم‌ترازی کامل با اندروید): جست‌وجوی IP عمومی + کد کشور —
//! معادل NetProbe.fetchIpInfoDirect / fetchIpInfoViaSocks — برای نشانِ
//! «IP + پرچم» صفحهٔ اصلی، با همان فراهم‌کنندگان و همان ترتیب اندروید.

use crate::log::DiagnosticsLog;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// مقاصد راستی‌آزمایی — همان لیست NetProbe.kt.
const PROBE_HOSTS: [(&str, u16); 3] =
    [("cloudflare.com", 80), ("www.gstatic.com", 80), ("1.1.1.1", 80)];

const CONNECT_TIMEOUT: Duration = Duration::from_millis(4_000);
const IO_TIMEOUT: Duration = Duration::from_millis(4_000);

pub struct EgressResult {
    pub endpoint: String,
    pub latency_ms: u64,
}

/// معادل `IpInfo` در NetProbe.kt — IP عمومی + کد کشور دو حرفی.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpInfo {
    pub ip: String,
    pub country_code: Option<String>,
}

/// فراهم‌کنندگان جای‌یابی — همان GEO_PROVIDERS اندروید، منهای مورد TLS
/// (دسکتاپ عمداً وابستگی TLS اضافه نمی‌کند؛ ip-api و 1.1.1.1 هر دو HTTP خام‌اند).
struct GeoProvider {
    host: &'static str,
    port: u16,
    path: &'static str,
    /// آیا این فراهم‌کننده روی TLS/443 است (معادل tls=true در NetProbe.kt).
    tls: bool,
}

// رفع ریشه‌ای مشکل ۲: روی شبکهٔ اپراتورهای ایران، درخواست HTTP خام روی :80
// به ip-api / 1.1.1.1 فیلتر/دستکاری می‌شود (در لاگ: «direct geo ... failed»).
// دقیقاً مانند نسخهٔ موبایل، یک فراهم‌کنندهٔ TLS (cloudflare:443) اضافه شد
// که روی 443 و با دست‌دادن TLS واقعی عبور می‌کند. ترتیب عیناً مانند NetProbe.kt.
// v8 audit: the TLS provider is now FIRST so the default geo path is
// encrypted; plain HTTP (:80) is only a fallback after a TLS failure.
const GEO_PROVIDERS: [GeoProvider; 3] = [
    GeoProvider { host: "www.cloudflare.com", port: 443, path: "/cdn-cgi/trace", tls: true },
    GeoProvider { host: "ip-api.com", port: 80, path: "/json/?fields=status,query,countryCode", tls: false },
    GeoProvider { host: "1.1.1.1", port: 80, path: "/cdn-cgi/trace", tls: false },
];

/// معادل `PortProbe.isReady()` — آیا SOCKS5 محلی موتور بالا آمده؟
pub fn socks_ready(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(s) => {
            let _ = s.shutdown(Shutdown::Both);
            true
        }
        Err(_) => false,
    }
}

/// معادل `NetProbe.checkSocksHandshake` — موتور واقعاً SOCKS5 حرف می‌زند؟
pub fn socks_handshake_ok() -> bool {
    socks5_greeting(CONNECT_TIMEOUT).is_some()
}

/// دست‌دادن اولیهٔ SOCKS5 (بدون احراز هویت) با موتور.
fn socks5_greeting(timeout: Duration) -> Option<TcpStream> {
    let proxy = SocketAddr::from(([127, 0, 0, 1], crate::engine::LOCAL_SOCKS_PORT));
    let mut s = TcpStream::connect_timeout(&proxy, timeout).ok()?;
    s.set_read_timeout(Some(timeout)).ok()?;
    s.set_write_timeout(Some(timeout)).ok()?;
    s.write_all(&[0x05, 0x01, 0x00]).ok()?;
    let mut reply = [0u8; 2];
    s.read_exact(&mut reply).ok()?;
    if reply != [0x05, 0x00] {
        return None;
    }
    Some(s)
}

/// اتصال TCP به مقصد از «داخل» پروکسی SOCKS5 موتور (RFC 1928).
///
/// اگر مقصد دامنه باشد ATYP=DOMAIN فرستاده می‌شود تا DNS از راه دور و داخل
/// تونل حل شود — دقیقاً همان کاری که NetProbe.socks5Connect در اندروید می‌کند.
pub fn socks5_stream(dest_host: &str, dest_port: u16, timeout: Duration) -> Option<TcpStream> {
    let mut s = socks5_greeting(timeout)?;

    let mut req: Vec<u8> = vec![0x05, 0x01, 0x00];
    if let Ok(v4) = dest_host.parse::<std::net::Ipv4Addr>() {
        req.push(0x01);
        req.extend_from_slice(&v4.octets());
    } else {
        let hb = dest_host.as_bytes();
        if hb.len() > 255 {
            return None;
        }
        req.push(0x03);
        req.push(hb.len() as u8);
        req.extend_from_slice(hb);
    }
    req.extend_from_slice(&dest_port.to_be_bytes());
    s.write_all(&req).ok()?;

    let mut head = [0u8; 4];
    s.read_exact(&mut head).ok()?;
    if head[0] != 0x05 || head[1] != 0x00 {
        return None;
    }
    match head[3] {
        0x01 => {
            let mut skip = [0u8; 4 + 2];
            s.read_exact(&mut skip).ok()?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            s.read_exact(&mut len).ok()?;
            let mut skip = vec![0u8; len[0] as usize + 2];
            s.read_exact(&mut skip).ok()?;
        }
        0x04 => {
            let mut skip = [0u8; 16 + 2];
            s.read_exact(&mut skip).ok()?;
        }
        _ => return None,
    }
    Some(s)
}

/// معادل `NetProbe.checkTcpViaProxy` — CONNECT به IP خام، بدون DNS.
pub fn tcp_via_proxy(dest_ip: &str, dest_port: u16) -> bool {
    socks5_stream(dest_ip, dest_port, CONNECT_TIMEOUT).is_some()
}

/// معادل `NetProbe.verify()` — یک HEAD واقعی از دل تونل.
pub fn verify_egress() -> Option<EgressResult> {
    for (host, port) in PROBE_HOSTS {
        let started = Instant::now();
        if http_head_ok(host, port).is_some() {
            return Some(EgressResult {
                endpoint: format!("{host}:{port}"),
                latency_ms: started.elapsed().as_millis() as u64,
            });
        }
    }
    None
}

fn http_head_ok(host: &str, port: u16) -> Option<()> {
    let mut s = socks5_stream(host, port, CONNECT_TIMEOUT)?;
    s.set_read_timeout(Some(IO_TIMEOUT)).ok()?;
    let request = format!("HEAD / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    s.write_all(request.as_bytes()).ok()?;
    let mut buf = [0u8; 12];
    let n = s.read(&mut buf).ok()?;
    let _ = s.shutdown(Shutdown::Both);
    if n >= 5 && buf.starts_with(b"HTTP/") {
        Some(())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
//  جای‌یابی IP — معادل بخش geolocation در NetProbe.kt
// ---------------------------------------------------------------------------

/// IP واقعی/اپراتور (مستقیم، خارج از تونل) — معادل `fetchIpInfoDirect`.
pub fn fetch_ip_direct(timeout_ms: u64) -> Option<IpInfo> {
    for p in &GEO_PROVIDERS {
        let got = if p.tls {
            connect_direct_ipv4(p.host, p.port, timeout_ms)
                .and_then(|s| tls_get(s, p.host, p.path))
                .and_then(|body| parse_ip_info(&body))
        } else {
            connect_direct_ipv4(p.host, p.port, timeout_ms)
                .and_then(|mut s| http_get(&mut s, p.host, p.path))
                .and_then(|body| parse_ip_info(&body))
        };
        match got {
            Some(info) => return Some(refine_country(info, p.host, false, timeout_ms)),
            None => DiagnosticsLog::d("netprobe", &format!("direct geo {} failed", p.host)),
        }
    }
    None
}

/// IP خروجی/سرور (از داخل پروکسی SOCKS5) — معادل `fetchIpInfoViaSocks`.
pub fn fetch_ip_via_socks(timeout_ms: u64) -> Option<IpInfo> {
    let timeout = Duration::from_millis(timeout_ms);
    for p in &GEO_PROVIDERS {
        let got = if p.tls {
            socks5_stream(p.host, p.port, timeout)
                .and_then(|s| tls_get(s, p.host, p.path))
                .and_then(|body| parse_ip_info(&body))
        } else {
            socks5_stream(p.host, p.port, timeout)
                .and_then(|mut s| http_get(&mut s, p.host, p.path))
                .and_then(|body| parse_ip_info(&body))
        };
        match got {
            Some(info) => return Some(refine_country(info, p.host, true, timeout_ms)),
            None => DiagnosticsLog::d("netprobe", &format!("proxied geo {} failed", p.host)),
        }
    }
    None
}

/// معادل `fetchIpInfoDirectWithRetry` — شبکهٔ اپراتور بدقلق را دور می‌زند.
pub fn fetch_ip_direct_retry(attempts: u32, delay_ms: u64, timeout_ms: u64) -> Option<IpInfo> {
    for i in 0..attempts {
        if let Some(info) = fetch_ip_direct(timeout_ms) {
            return Some(info);
        }
        if i + 1 < attempts {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    None
}

/// معادل `fetchIpInfoViaSocksWithRetry` — پنجرهٔ سردِ warp-in-warp را رد می‌کند.
pub fn fetch_ip_via_socks_retry(attempts: u32, delay_ms: u64, timeout_ms: u64) -> Option<IpInfo> {
    for i in 0..attempts {
        if let Some(info) = fetch_ip_via_socks(timeout_ms) {
            return Some(info);
        }
        if i + 1 < attempts {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    None
}

/// معادل `refineCountry` در اندروید: پرچم همیشه از یک پایگاه دادهٔ واحد
/// (ip-api) بیاید تا بین فراهم‌کنندگان فرق نکند و «پرچم‌پرش» رخ ندهد.
fn refine_country(info: IpInfo, provider_host: &str, via_socks: bool, timeout_ms: u64) -> IpInfo {
    if provider_host == "ip-api.com" {
        return info;
    }
    let path = format!("/json/{}?fields=status,countryCode", info.ip);
    let body = if via_socks {
        socks5_stream("ip-api.com", 80, Duration::from_millis(timeout_ms))
            .and_then(|mut s| http_get(&mut s, "ip-api.com", &path))
    } else {
        connect_direct_ipv4("ip-api.com", 80, timeout_ms)
            .and_then(|mut s| http_get(&mut s, "ip-api.com", &path))
    };
    if let Some(cc) = body.and_then(|b| json_str(&b, "countryCode")) {
        return IpInfo { ip: info.ip, country_code: Some(cc) };
    }
    info
}

/// اتصال مستقیم با اجبار IPv4 — همان رفع باگ «IPv6 موقت روی شبکهٔ دو‌پشته».
fn connect_direct_ipv4(host: &str, port: u16, timeout_ms: u64) -> Option<TcpStream> {
    let timeout = Duration::from_millis(timeout_ms);
    let addrs: Vec<SocketAddr> = (host, port).to_socket_addrs().ok()?.collect();
    let addr = addrs.iter().find(|a| a.is_ipv4()).or_else(|| addrs.first()).copied()?;
    let s = TcpStream::connect_timeout(&addr, timeout).ok()?;
    s.set_read_timeout(Some(timeout)).ok()?;
    s.set_write_timeout(Some(timeout)).ok()?;
    Some(s)
}

/// Lightweight network fingerprint - a scaled-down port of the DPI probe in
/// the mobile SmartAuto.kt. IP-literal hosts only (no DNS), so a poisoned
/// resolver cannot skew the verdict. If direct TCP:80 egress is dead, the
/// network is treated as filtered and the connection ladder leads with the
/// hardened anti-DPI candidate instead of wasting the first pass.
pub fn network_looks_filtered() -> bool {
    const FINGERPRINT_PROBES: [(&str, u16); 2] = [("1.1.1.1", 80), ("8.8.8.8", 80)];
    let reachable = FINGERPRINT_PROBES
        .iter()
        .any(|(host, port)| connect_direct_ipv4(host, *port, 1_200).is_some());
    if !reachable {
        DiagnosticsLog::w("netprobe", "fingerprint: direct :80 egress is blocked on this network");
    }
    !reachable
}

/// HTTP/1.1 GET مینیمال — همان `httpGet` اندروید.
fn http_get(stream: &mut TcpStream, host: &str, path: &str) -> Option<String> {
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: Aether/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > 65_536 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// GET روی TLS — معادل tlsWrap در NetProbe.kt.
/// در ویندوز native-tls از SChannel بومی ویندوز استفاده می‌کند (بدون OpenSSL / بدون unsafe).
/// تایم‌اوت‌های ساکت قبل از ورود به این تابع روی TcpStream تنظیم شده‌اند.
fn tls_get(stream: TcpStream, host: &str, path: &str) -> Option<String> {
    let connector = native_tls::TlsConnector::new().ok()?;
    let mut tls = connector.connect(host, stream).ok()?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: Aether/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    tls.write_all(request.as_bytes()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        match tls.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > 65_536 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// معادل `parseIpInfo` — هر دو قالب ip-api (JSON) و cloudflare trace.
fn parse_ip_info(response: &str) -> Option<IpInfo> {
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("");
    // قالب ۱: JSONِ ip-api  {"query":"1.2.3.4","countryCode":"DE"}
    if let Some(ip) = json_str(body, "query") {
        return Some(IpInfo { ip, country_code: json_str(body, "countryCode") });
    }
    // قالب ۲: خطوط key=value در /cdn-cgi/trace
    let mut ip: Option<String> = None;
    let mut loc: Option<String> = None;
    for line in body.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("ip=") {
            ip = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("loc=") {
            let v = v.trim();
            if v.len() == 2 {
                loc = Some(v.to_uppercase());
            }
        }
    }
    ip.map(|ip| IpInfo { ip, country_code: loc })
}

/// استخراج مقدار رشته‌ای یک کلید JSON بدون وابستگی regex.
fn json_str(body: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let idx = body.find(&pat)? + pat.len();
    let rest = &body[idx..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socks_ready_is_false_on_a_closed_port() {
        assert!(!socks_ready(1));
    }

    #[test]
    fn parses_ip_api_json() {
        let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"query\":\"1.2.3.4\",\"countryCode\":\"DE\"}";
        let info = parse_ip_info(resp).unwrap();
        assert_eq!(info.ip, "1.2.3.4");
        assert_eq!(info.country_code.as_deref(), Some("DE"));
    }

    #[test]
    fn parses_cloudflare_trace() {
        let resp = "HTTP/1.1 200 OK\r\n\r\nfl=1\nip=5.6.7.8\nloc=nl\n";
        let info = parse_ip_info(resp).unwrap();
        assert_eq!(info.ip, "5.6.7.8");
        assert_eq!(info.country_code.as_deref(), Some("NL"));
    }
}
