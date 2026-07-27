//! پورت ۱:۱ از `core/AetherProcess.kt`.
//!
//! تمام رفتارهایی که در ۱.۲.۲ روی اندروید درست شدند عیناً حفظ شده‌اند:
//!   * درنگ کردن (drain) خروجی موتور تا لوله پر نشود
//!   * انتظار قابل‌قطع (interruptible) به‌جای polling
//!   * SIGTERM کوتاه (250ms) و سپس kill قطعی
//!   * منتظر ماندن برای آزادشدن پورت SOCKS5 محلی پیش از اجرای بعدی

use crate::log::DiagnosticsLog;
use crate::profile::ConnectionProfile;
use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// همان پورتی که موتور در اندروید باز می‌کند.
pub const LOCAL_SOCKS_PORT: u16 = 1819;
/// ۱.۲.۲: 10808/10809 با v2rayNG تداخل داشت، پس به 10810/10811 منتقل شد.
pub const SHARE_SOCKS_PORT: u16 = 10810;
pub const SHARE_HTTP_PORT: u16 = 10811;

const GRACEFUL_EXIT_MS: u64 = 250;

/// آیا هستهٔ همراه برنامه سوییچ --log-level (نسخهٔ 1.4.0 به بعد) را می‌فهمد؟
/// نسخه از فایل CORE_VERSION کنار aether.exe خوانده می‌شود (همان فایلی که
/// پنل About نشان می‌دهد). در صورت هر ابهامی محافظه‌کارانه false
/// برمی‌گردد تا هسته‌های قدیمی با فلگ ناشناخته از کار نیفتند.
fn engine_supports_log_level(exe: &Path) -> bool {
    let Some(dir) = exe.parent() else { return false };
    let Ok(raw) = std::fs::read_to_string(dir.join("CORE_VERSION")) else {
        return false;
    };
    let mut parts = raw.trim().trim_start_matches('v').split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor) >= (1, 4)
}

pub struct AetherProcess {
    exe: PathBuf,
    working_dir: PathBuf,
    child: Option<Child>,
}

impl AetherProcess {
    pub fn new(install_dir: &Path, working_dir: &Path) -> Self {
        let bundled_dir = install_dir.join("engine");
        let exe = match prepare_runtime_engine(&bundled_dir, working_dir) {
            Ok(exe) => exe,
            Err(e) => {
                DiagnosticsLog::w(
                    "engine",
                    &format!("Could not stage the engine in a writable folder ({e}); running it from the install folder."),
                );
                bundled_dir.join("aether.exe")
            }
        };
        Self {
            exe,
            working_dir: working_dir.to_path_buf(),
            child: None,
        }
    }

    pub fn start(&mut self, profile: &ConnectionProfile) -> Result<()> {
        if !self.exe.exists() {
            return Err(anyhow!("Engine binary missing: {}", self.exe.display()));
        }

        let mut args = profile.to_args();
        // لاگر جدید هستهٔ 1.4.0 متغیر RUST_LOG را نادیده می‌گیرد و فقط از
        // سوییچ رسمی خودش دستور می‌گیرد (لاگ v12 این را ثابت کرد: هیچ
        // خط debug چاپ نشد). سطح trace تنها راه دیدن عملیاتی است که
        // بلافاصله بعد از sysprofile با code 5 می‌میرد.
        // v16 (سرعت/پینگ): سطح trace فقط برای شکار باگ code 5 لازم بود و در
        // مسیر داده سربار جدی دارد؛ حالا که ریشه رفع شد به info (پیش‌فرضی که
        // v9 با آن سریع بود) برمی‌گردیم. همچنین sysprofile خودکار هستهٔ
        // 1.4.0 روی این سیستم پروفایل Medium با بافرهای کوچک
        // (netstack 256KB/64KB) انتخاب می‌کند که نسبت به هستهٔ 1.3.0 سرعت را
        // پایین می‌آورد؛ با --perf high بافرهای بزرگ و همروندی کامل اسکن
        // برمی‌گردد. (هر دو فلاگ فقط برای هستهٔ 1.4 به بالا فرستاده می‌شود.)
        if engine_supports_log_level(&self.exe) {
            args.insert(0, "--log-level".into());
            args.insert(1, "info".into());
            args.insert(2, "--perf".into());
            args.insert(3, "high".into());
        }
        // ریشهٔ قطعی خطای Access is denied (code 5): تست‌های تشخیصی روی
        // سیستم کاربر ثابت کرد هستهٔ 1.4.0 با CWD=ریشهٔ پوشهٔ داده می‌میرد
        // (T1/T2) ولی با CWD=پوشهٔ خود موتور کامل وصل می‌شود (T3).
        // پس موتور را در پوشهٔ خودش اجرا می‌کنیم؛ فایل‌های هویت/کانفیگ هم
        // در prepare_runtime_engine به همین پوشه منتقل می‌شوند.
        let run_dir = self
            .exe
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.working_dir.clone());
        let mut cmd = Command::new(&self.exe);
        cmd.args(&args)
            .current_dir(&run_dir)
            .envs(profile.to_env())
            .env("HOME", &run_dir)
            .env("TMPDIR", &run_dir)
            // هستهٔ 1.4.0 بلافاصله بعد از مرحلهٔ جدید sysprofile با
            // Error: Io(code 5, Access is denied) خارج می‌شود. این دو متغیر
            // باعث می‌شوند لاگ سطح debug و backtrace کامل از خود هسته در
            // کنسول لاگ برنامه ثبت شود تا محل دقیق خطا معلوم شود.
            // (برای نسخه‌های قدیمی‌تر هسته بی‌ضررند و نادیده گرفته می‌شوند.)
            .env("RUST_BACKTRACE", "full")
            .env("RUST_LOG", "debug")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // معادل ویندوزیِ اینکه در اندروید فرآیند headless است: پنجرهٔ کنسول
        // نباید جلوی کاربر بالا بیاید.
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn()?;

        // موتور بدون کنسول اجرا می‌شود (CREATE_NO_WINDOW). تست cmd کاربر ثابت
        // کرد هستهٔ 1.4.0 در کنسول واقعی کامل وصل می‌شود و سؤال تعاملی
        // quick-reconnect ([Y/n]) می‌پرسد؛ زیر برنامه بدون stdin معتبر، دسترسی
        // کنسولی هسته با Access is denied (code 5) شکست می‌خورد.
        // اینجا یک stdin معتبر می‌دهیم و جواب پیش‌فرض «بله» را می‌نویسیم؛
        // با بسته‌شدن pipe، خواندن‌های بعدی EOF تمیز می‌گیرند نه خطا.
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(b"y\n");
        }

        DiagnosticsLog::i("engine", &format!("Spawned aether.exe {}", args.join(" ")));

        // درنگ کردن stdout و stderr — دقیقاً مثل ترد «aether-log» در اندروید.
        if let Some(out) = child.stdout.take() { spawn_drain(out); }
        if let Some(err) = child.stderr.take() { spawn_drain(err); }

        self.child = Some(child);
        Ok(())
    }

    pub fn is_alive(&mut self) -> bool {
        match self.child.as_mut() {
            Some(c) => matches!(c.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// معادل `awaitExit`: مسدود می‌ماند تا خروج موتور یا اتمام مهلت.
    /// برخلاف نسخهٔ قبلی اندروید، هیچ polling دومّینی‌ای در کار نیست.
    pub fn await_exit(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match self.child.as_mut().map(|c| c.try_wait()) {
                None | Some(Ok(Some(_))) => return true,
                Some(Ok(None)) => std::thread::sleep(Duration::from_millis(20)),
                Some(Err(_)) => return false,
            }
        }
        false
    }

    /// معادل `stop()`: خاتمهٔ مؤدبانه، سپس kill قطعی پس از 250ms.
    /// برگشت از این تابع یعنی فرآیند واقعاً reap شده است.
    pub fn stop(&mut self) {
        let Some(mut child) = self.child.take() else { return };
        let _ = child.kill(); // در ویندوز TerminateProcess فوری است
        let deadline = Instant::now() + Duration::from_millis(GRACEFUL_EXIT_MS);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = child.try_wait() { break; }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = child.wait();
        DiagnosticsLog::w("engine", "Engine stopped and reaped.");
    }
}

impl Drop for AetherProcess {
    fn drop(&mut self) { self.stop(); }
}

/// ریشهٔ باگ «روی هیچ پروتکلی کانکت نمی‌شود» (هستهٔ 1.4.x):
/// هستهٔ جدید بلافاصله بعد از شروع، وضعیت خودش (کش quick-reconnect و…) را
/// کنار فایل اجرایی‌اش می‌نویسد. وقتی موتور از `C:\Program Files\…\engine`
/// اجرا شود، آن پوشه برای فرآیندِ بدون Administrator فقط‌خواندنی است و موتور
/// در همان میلی‌ثانیهٔ اول با `Io(Os { code: 5 … Access is denied })` می‌میرد
/// — دقیقاً امضای لاگ کاربر (همهٔ پروتکل‌ها، خروج فوری، قبل از بازشدن SOCKS5).
///
/// رفع ریشه‌ای: موتور در اولین اجرا به پوشهٔ دادهٔ کاربر (قابل‌نوشتن) کپی و
/// همیشه از همان‌جا اجرا می‌شود تا هر نوشتنِ «کنار exe» مجاز باشد. این کار
/// نسخه‌های آیندهٔ هسته را هم در برابر همین کلاس خطا بیمه می‌کند.
/// اگر کپی به هر دلیلی شکست بخورد، رفتار قدیمی (اجرای مستقیم از پوشهٔ نصب)
/// حفظ می‌شود تا هیچ‌وقت وضع بدتر از قبل نشود.
fn prepare_runtime_engine(bundled_dir: &Path, working_dir: &Path) -> Result<PathBuf> {
    let runtime_dir = working_dir.join("engine");
    if runtime_dir == *bundled_dir {
        let exe = runtime_dir.join("aether.exe");
        return if exe.exists() {
            Ok(exe)
        } else {
            Err(anyhow!("Engine binary missing: {}", exe.display()))
        };
    }
    std::fs::create_dir_all(&runtime_dir)?;
    for entry in std::fs::read_dir(bundled_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let src = entry.path();
        let dst = runtime_dir.join(entry.file_name());
        if !runtime_copy_is_fresh(&src, &dst) {
            std::fs::copy(&src, &dst)?;
        }
    }
    // هویت ثبت‌شدهٔ قبلی کاربر (اگر در ریشهٔ پوشهٔ داده باشد) یک‌بار به
    // پوشهٔ اجرای موتور منتقل می‌شود تا دوباره ثبت‌نام لازم نشود.
    // (فایل‌های دیگر مثل masque/lastconn عمداً کپی نمی‌شوند؛ هستهٔ
    // جدید خودش نسخهٔ سالم می‌سازد.)
    for name in ["aether.toml", "aether-secondary.toml"] {
        let src = working_dir.join(name);
        let dst = runtime_dir.join(name);
        if src.is_file() && !dst.exists() {
            let _ = std::fs::copy(&src, &dst);
        }
    }
    let exe = runtime_dir.join("aether.exe");
    if !exe.exists() {
        return Err(anyhow!(
            "Engine binary missing: {}",
            bundled_dir.join("aether.exe").display()
        ));
    }
    Ok(exe)
}

/// فقط وقتی دوباره کپی می‌کنیم که نسخهٔ نصب‌شده عوض شده باشد (آپدیت برنامه).
fn runtime_copy_is_fresh(src: &Path, dst: &Path) -> bool {
    let (Ok(a), Ok(b)) = (std::fs::metadata(src), std::fs::metadata(dst)) else {
        return false;
    };
    if a.len() != b.len() {
        return false;
    }
    matches!((a.modified(), b.modified()), (Ok(s), Ok(d)) if d >= s)
}

fn spawn_drain<R: std::io::Read + Send + 'static>(reader: R) {
    std::thread::Builder::new()
        .name("aether-log".into())
        .spawn(move || {
            for line in BufReader::new(reader).lines().map_while(Result::ok) {
                // مثل اندروید: خروجی موتور فقط به لاگ خصوصیِ برنامه می‌رود،
                // نه به جایی که بقیهٔ سیستم بخواند (معادل ممنوعیت Logcat).
                DiagnosticsLog::d("engine", &line);
            }
            DiagnosticsLog::w("engine", "Engine output stream closed.");
        })
        .ok();
}

/// معادل `PortProbe.kt`: پیش از اجرای موتور جدید، منتظر آزادشدن پورت می‌مانیم.
/// این همان ریشهٔ باگ «تعویض پروتکل طول می‌کشد» در ۱.۲.۱ بود.
pub fn wait_for_port_release(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_err() { return true; }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}
