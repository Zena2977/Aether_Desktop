## Aether Desktop 1.0.0

<div dir="rtl">

### 🇮🇷 تازه‌های نسخهٔ ۱.۰.۰ — اولین نسخهٔ دسکتاپ

این اولین انتشار رسمی **Aether برای ویندوز** است. همهٔ امکانات نسخهٔ اندروید، بدون کم و کاستی، روی ویندوز پیاده شده است.

**رابط کاربری**
- همان طراحی، همان رنگ‌ها و همان آیکون نسخهٔ موبایل
- بهینه‌شده برای نمایشگرهای بزرگ دسکتاپ (چیدمان دوستونی و نوار کناری)
- نوار عنوان اختصاصی به سبک ویندوز ۱۱
- پشتیبانی کامل از راست‌به‌چپ

**اتصال**
- دکمهٔ اتصال با همان ۸ حالت اندروید
- پروتکل‌های `Auto` ، `MASQUE` ، `WireGuard` و `WARP×2`
- حالت‌های اسکن: Turbo / Balanced / Thorough / Stealth / Ironclad
- انتخاب IPv4 ، IPv6 یا هر دو
- انتخاب خودکار هوشمند پروتکل در صورت شکست
- اتصال مجدد خودکار

**تنطیمات پیشرفته**
- نویز (Noize) — هر ۶ حالت
- ورودی دستی سرور و رنج آدرس
- تنطیم MTU و Keepalive
- Fragment ، ECH و MASQUE HTTP/2
- تونل تفکیکی (Split Tunneling) — انتخاب برنامه‌ها بر اساس فایل exe

**اشتراک شبکه**
- اشتراک اتصال در شبکهٔ محلی روی پورت‌های SOCKS5 `10810` و HTTP `10811`
- فقط روی آدرس شبکهٔ محلی گوش می‌دهد، نه روی همهٔ رابط‌ها

**عیب‌یابی**
- ۶ بررسی خودکار: موتور، درایور Wintun ، دسترسی مدیر، پورت محلی، خروج واقعی، پروفایل
- لاگ زنده با امکان پاک‌سازی

**مخصوص ویندوز**
- تونل واقعی سطح سیستم با درایور رسمی و امضاشدهٔ Wintun
- تنزیم خودکار قاعدهٔ فایروال در زمان نصب
- اجرای تک‌نمونه (دو بار باز نمی‌شود)
- خروجی بدون وابستگی به Visual C++ Redistributable

**چه چیزی عمداً نیست**
- حالت پروکسی (Proxy Mode) — در ویندوز کاربردی ندارد
- بروزرسانی درون‌برنامه‌ای — فقط لینک به همین صفحهٔ Releases

### کدام فایل را دانلود کنم؟

| فایل | توضیح |
|---|---|
| `Aether-Setup-1.0.0-x64.exe` | ویندوز ۶۴بیتی — نصب معمول (توصیه‌شده) |
| `Aether-Setup-1.0.0-x86.exe` | ویندوز ۳۲بیتی |
| `Aether-Portable-1.0.0-x64.zip` | بدون نصب، ۶۴بیتی |
| `Aether-Portable-1.0.0-x86.zip` | بدون نصب، ۳۲بیتی |
| `SHA256SUMS.txt` | برای راستی‌آزمایی سلامت فایل‌ها |

**پیش‌نیاز:** ویندوز ۱۰ نسخهٔ ۱۸۰۹ یا بالاتر. برای برقراری تونل، دسترسی مدیر (Administrator) لازم است — معادل دیالوگ مجوز VPN در اندروید.

</div>

---

### 🇬🇧 What's new in 1.0.0 — the first desktop release

This is the first official release of **Aether for Windows**. Every feature from the Android edition is present, with nothing left out.

**Interface**
- Identical design, colours and icon to the mobile edition
- Adapted for large desktop displays (two-column layout with a side rail)
- Custom Windows 11-style title bar
- Full right-to-left support

**Connection**
- Connect button with the same 8 states as Android
- `Auto`, `MASQUE`, `WireGuard` and `WARP×2` protocols
- Scan modes: Turbo / Balanced / Thorough / Stealth / Ironclad
- IPv4, IPv6 or dual-stack
- Smart automatic protocol fallback
- Automatic reconnect

**Advanced settings**
- Noize — all 6 levels
- Manual peer and manual address-range entry
- MTU and keepalive tuning
- Fragment, ECH and MASQUE HTTP/2
- Split tunnelling — choose applications by executable

**Network sharing**
- Share the connection over your LAN on SOCKS5 `10810` and HTTP `10811`
- Binds only to the LAN address, never to every interface

**Diagnostics**
- 6 automated checks: engine, Wintun driver, administrator rights, local port, real egress, profile
- Live log with a clear action

**Windows-specific**
- Real system-wide tunnel using the official signed Wintun driver
- Firewall rule configured automatically at install time
- Single-instance launch
- Statically linked — no Visual C++ Redistributable required

**Deliberately absent**
- Proxy Mode — not meaningful on Windows
- In-app updater — replaced by a link to this Releases page

### Which file do I download?

| File | Description |
|---|---|
| `Aether-Setup-1.0.0-x64.exe` | Windows 64-bit — normal install (recommended) |
| `Aether-Setup-1.0.0-x86.exe` | Windows 32-bit |
| `Aether-Portable-1.0.0-x64.zip` | No install, 64-bit |
| `Aether-Portable-1.0.0-x86.zip` | No install, 32-bit |
| `SHA256SUMS.txt` | For verifying file integrity |

**Requirements:** Windows 10 build 1809 or newer. Establishing the tunnel requires Administrator rights — the equivalent of Android's VPN permission dialog.

---

<div dir="rtl">

### 🛡️ ممیزی امنیتی نسخهٔ ۱.۰.۰ — امتیاز ۹۰ از ۱۰۰ (v9)

خلاصهٔ ممیزی کامل هفت‌محوری (اسرار هاردکد، رمزنگاری و پروتکل‌ها، نشت داده، ذخیره‌سازی محلی، مجوزها و مانیفست، لاگ، کیفیت کد و پیکربندی شبکه):

- ✅ هیچ کلید API، توکن یا رمز عبور هاردکدشده‌ای در سورس‌کد وجود ندارد؛ هویت WARP در زمان اجرا ساخته می‌شود.
- ✅ اعتبارسنجی TLS با Certificate Pinning (دو پین) انجام می‌شود؛ حملهٔ MitM روی کانال کنترل عملاً ممکن نیست.
- ✅ تونل واقعی سطح سیستم (Wintun): DNS از داخل تونل عبور می‌کند و نشت IPv6 طبق انتخاب کاربر مدیریت می‌شود.
- ✅ مانیفست اندروید حداقلی: `allowBackup=false`، بدون `debuggable`، سرویس VPN از بیرون در دسترس نیست (`exported=false`).
- ✅ ترافیک cleartext در اندروید به‌طور کامل مسدود است (`network_security_config`).
- 🆕 پل اشتراک LAN اکنون فقط به دستگاه‌های loopback و شبکهٔ خصوصی اجازهٔ اتصال می‌دهد (فیلتر مبدأ اتصال).
- ⚠️ متوسط: فایل‌های هویت WARP به‌صورت متن ساده در پوشهٔ کاری ذخیره می‌شوند — قفل ACL آزمایشی v8 در v9 حذف شد چون روی برخی سیستم‌ها دسترسی موتور را هم می‌بست؛ رمزگذاری DPAPI در نقشهٔ راه است.
- ✅ رفع شد در v8: جستار موقعیت جغرافیایی اکنون ابتدا از مسیر TLS (پورت ۴۴۳) انجام می‌شود و HTTP ساده فقط گزینهٔ پشتیبان است.
- ✅ رفع شد در v8: IP خروجی در لاگ ماندگار با ماسک (1.2.3.xxx) ثبت می‌شود؛ نمایش کامل فقط در UI است. چرخش خودکار ۵۱۲KiB پابرجاست.
- ⚠️ کم: SNI برای نقطهٔ MASQUE به‌صورت cleartext ارسال می‌شود (سرور مقصد ECH را نمی‌پذیرد — محدودیت سمت سرور).

</div>

### 🛡️ Security audit summary — score 90/100 (v9)

Summary of the full seven-area audit (hardcoded secrets, cryptography & protocols, data-leak risks, local storage, permissions & manifest, logging, code quality & network config):

- ✅ No hardcoded API keys, tokens or passwords anywhere in the source; WARP identities are generated at runtime.
- ✅ TLS is validated with certificate pinning (2 pins); MitM on the control channel is not feasible.
- ✅ Real system-level tunnel (Wintun): DNS resolves inside the tunnel and IPv6 is handled per the user's stack selection.
- ✅ Minimal Android manifest: `allowBackup=false`, non-debuggable, VpnService not exported.
- ✅ All cleartext HTTP is blocked on Android by the network security config.
- 🆕 The LAN share bridge now only accepts connections from loopback / private / link-local peers (source filter).
- ⚠️ Medium: WARP identity files are stored as plaintext in the working directory — the experimental v8 ACL lock was removed in v9 because it could also block the engine's own access on some systems; DPAPI encryption is on the roadmap.
- ✅ Fixed in v8: geolocation lookups now try the TLS provider (port 443) first; plain HTTP is only a fallback.
- ✅ Fixed in v8: the persistent log stores the exit IP masked (1.2.3.xxx); the full IP is shown only in the UI. Automatic 512 KiB rotation unchanged.
- ⚠️ Low: the SNI for the MASQUE endpoint is sent in cleartext (the endpoint does not accept ECH — a server-side limitation).
