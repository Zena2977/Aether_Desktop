//! پروکسی سیستمی ویندوز — معادل کاربردیِ `VpnService.establish()` اندروید.
//!
//! ریشهٔ باگ «کانکت می‌شود ولی هیچ سایتی باز نمی‌شود»: نسخهٔ قبلی دسکتاپ
//! هیچ مسیر داده‌ای بین سیستم و SOCKS5 موتور برقرار نمی‌کرد (Wintun فقط
//! ساخته می‌شد؛ نه مسیریابی داشت و نه رله). حالا هنگام اتصال:
//!   ۱. پل محلی HTTP↔SOCKS5 (`share.rs`) روی 127.0.0.1:10811 بالا می‌آید،
//!   ۲. پروکسی سیستمی ویندوز (WinINET — همان که Edge/Chrome/Firefox
//!      «system proxy» می‌خوانند) به آن پل تنظیم می‌شود،
//!   ۳. هنگام قطع، تنظیم برگردانده می‌شود.
//! این کار به دسترسی Administrator نیاز ندارد (HKCU) و همان نقش «کل
//! ترافیک از تونل برود» اندروید را روی ویندوز بازی می‌کند.

use crate::log::DiagnosticsLog;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
/// مقصدهایی که هرگز نباید از پروکسی بروند (شبکهٔ محلی و خود لوپ‌بک).
const BYPASS: &str = "localhost;127.*;10.*;172.16.*;192.168.*;<local>";

/// فعال‌سازی پروکسی سیستمی روی پل HTTP محلی. `true` یعنی ثبت شد.
pub fn enable(http_port: u16) -> bool {
    let server = format!("127.0.0.1:{http_port}");
    let ok = reg(&["add", KEY, "/v", "ProxyServer", "/t", "REG_SZ", "/d", &server, "/f"])
        && reg(&["add", KEY, "/v", "ProxyOverride", "/t", "REG_SZ", "/d", BYPASS, "/f"])
        && reg(&["add", KEY, "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "1", "/f"]);
    broadcast_change();
    if ok {
        DiagnosticsLog::i("sysproxy", &format!("System proxy enabled -> {server} (bypass: {BYPASS})"));
    } else {
        DiagnosticsLog::e("sysproxy", "Could not write the system proxy registry values.");
    }
    ok
}

/// غیرفعال‌سازی پروکسی سیستمی — در قطع اتصال، خطا و خروج برنامه صدا زده می‌شود.
pub fn disable() -> bool {
    let ok = reg(&["add", KEY, "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "0", "/f"]);
    broadcast_change();
    if ok {
        DiagnosticsLog::i("sysproxy", "System proxy disabled.");
    }
    ok
}

fn reg(args: &[&str]) -> bool {
    let mut cmd = Command::new("reg");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// به WinINET اعلام می‌کند تنظیمات پروکسی عوض شد تا برنامه‌های باز (مرورگرها)
/// بدون ری‌استارت آن را بردارند — بدون این فراخوان، تغییر تا اجرای بعدی
/// برنامه‌ها دیده نمی‌شد.
#[cfg(windows)]
fn broadcast_change() {
    const INTERNET_OPTION_REFRESH: u32 = 37;
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    #[link(name = "wininet")]
    extern "system" {
        fn InternetSetOptionW(
            hinternet: *mut core::ffi::c_void,
            dwoption: u32,
            lpbuffer: *mut core::ffi::c_void,
            dwbufferlength: u32,
        ) -> i32;
    }
    unsafe {
        InternetSetOptionW(std::ptr::null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, std::ptr::null_mut(), 0);
        InternetSetOptionW(std::ptr::null_mut(), INTERNET_OPTION_REFRESH, std::ptr::null_mut(), 0);
    }
}

#[cfg(not(windows))]
fn broadcast_change() {}
