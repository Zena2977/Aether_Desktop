//! پورت از `core/SmartAuto.kt` + منطق نردبان `AetherVpnService.directPlan/buildPlan`.
//!
//! ریشهٔ باگ قبلی دسکتاپ: فقط «یک» پروتکل انتخاب می‌شد و هیچ نردبان
//! تلاشِ چندمرحله‌ای وجود نداشت؛ در اندروید هر اتصال یک «برنامه» چند
//! کاندیدایی است که یکی‌یکی امتحان می‌شوند تا اولینِ قبول‌شده در خودآزما
//! برنده شود. همان منطق این‌جا پیاده شده:
//!
//!  * پروتکل دستی  ← دو پاس (معادل directPlan): اول همان تنظیمات کاربر
//!    (سقف ۷۵ ثانیه)، بعد پاس ضد-DPI سخت‌شده — پروتکل هرگز عوض نمی‌شود.
//!  * Smart Auto ← نردبان MASQUE → MASQUE سخت‌شده → GOOL → WireGuard
//!    (همان ترتیب ترجیح SmartAuto.kt).

use crate::log::DiagnosticsLog;
use crate::profile::{ConnectionProfile, IpVersion, Noize, Protocol};

const TAG: &str = "auto";

/// سقف پاس اول — همان `FIRST_PASS_MAX_MS` اندروید.
const FIRST_PASS_MAX_MS: u64 = 35_000;

/// ترتیب ترجیح — همان ترتیبی که SmartAuto.kt دارد.
const PREFERENCE: [Protocol; 3] = [Protocol::Masque, Protocol::Gool, Protocol::Wireguard];

/// معادل `AutoCandidate` اندروید — یک استراتژی آمادهٔ اجرا.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub profile: ConnectionProfile,
    pub timeout_ms: u64,
    pub label: String,
}

/// ساخت نسخهٔ سخت‌شدهٔ ضد-DPI — معادل پاس دوم `directPlan` اندروید.
fn harden(p: &ConnectionProfile) -> ConnectionProfile {
    let mut h = p.clone();
    if h.noize == Noize::Off {
        h.noize = Noize::Firewall;
    }
    if h.protocol == Protocol::Masque {
        h.masque_http2 = true;
        h.fragment = true;
        h.ech = true;
    }
    h
}

/// معادل `directPlan` — پروتکل دستی، دو پاس، بدون تعویض پروتکل.
fn direct_plan(user: &ConnectionProfile, hostile: bool) -> Vec<Candidate> {
    let full = user.connect_timeout_ms();
    let hardened = harden(user);
    let name = format!("{:?}", user.protocol).to_uppercase();
    if hardened == *user {
        return vec![Candidate {
            profile: user.clone(),
            timeout_ms: full,
            label: format!("{name} · as configured"),
        }];
    }
    let as_configured = Candidate {
        profile: user.clone(),
        timeout_ms: full.min(FIRST_PASS_MAX_MS),
        label: format!("{name} · as configured"),
    };
    let anti_dpi = Candidate {
        profile: hardened,
        timeout_ms: full,
        label: format!("{name} · hardened anti-DPI"),
    };
    if hostile {
        // Filtered network: lead with the hardened pass so the plain pass
        // cannot burn the first-pass window (root cause of the slow connects)
        // or win with a data path that DPI then strangles mid-session.
        vec![anti_dpi, as_configured]
    } else {
        vec![as_configured, anti_dpi]
    }
}

/// نردبان Smart Auto — معادل `SmartAuto.buildPlan`.
fn auto_plan(user: &ConnectionProfile, hostile: bool) -> Vec<Candidate> {
    let full = user.connect_timeout_ms();
    let mut plan = Vec::new();

    // فقط IPv6: WireGuard پایدارتر است — همان قاعدهٔ اندروید.
    let order: Vec<Protocol> = if user.ip_version == IpVersion::V6 {
        vec![Protocol::Wireguard, Protocol::Masque, Protocol::Gool]
    } else {
        PREFERENCE.to_vec()
    };

    for (i, proto) in order.iter().enumerate() {
        let mut base = user.clone();
        base.protocol = *proto;
        let name = format!("{proto:?}").to_uppercase();
        if i == 0 {
            let as_configured = Candidate {
                profile: base.clone(),
                timeout_ms: full.min(FIRST_PASS_MAX_MS),
                label: format!("{name} · as configured"),
            };
            let anti_dpi = Candidate {
                profile: harden(&base),
                timeout_ms: full.min(120_000),
                label: format!("{name} · hardened anti-DPI"),
            };
            if hostile {
                plan.push(anti_dpi);
                plan.push(as_configured);
            } else {
                plan.push(as_configured);
                plan.push(anti_dpi);
            }
        } else {
            plan.push(Candidate {
                profile: harden(&base),
                timeout_ms: if i + 1 == order.len() { full } else { full.min(120_000) },
                label: format!("{name} · hardened anti-DPI"),
            });
        }
    }
    plan
}

/// نقطهٔ ورود: برنامهٔ کامل اتصال برای پروفایل کاربر.
pub fn build_plan(user: &ConnectionProfile, hostile: bool) -> Vec<Candidate> {
    if hostile {
        DiagnosticsLog::w(TAG, "Network fingerprint: this network looks filtered - anti-DPI attempts run first.");
    }
    let plan = if user.protocol == Protocol::Smart {
        DiagnosticsLog::i(TAG, "Smart Auto: building the strategy ladder…");
        auto_plan(user, hostile)
    } else {
        direct_plan(user, hostile)
    };
    let summary: Vec<String> = plan.iter().map(|c| c.label.clone()).collect();
    DiagnosticsLog::i(TAG, &format!("Plan ready ({} attempt(s)): {}", plan.len(), summary.join(" → ")));
    plan
}

/// معادل `SmartAuto.choose()` — برای سازگاری با کد/تست‌های قبلی حفظ شده.
pub fn pick(profile: &ConnectionProfile) -> Protocol {
    if profile.protocol != Protocol::Smart {
        return profile.protocol;
    }
    if profile.has_manual_peer() {
        return Protocol::Masque;
    }
    if profile.ip_version == IpVersion::V6 {
        return Protocol::Wireguard;
    }
    PREFERENCE[0]
}

/// ترتیب تلاش مجدد پس از شکست — معادل `nextCandidate()`.
pub fn next_after(failed: Protocol) -> Option<Protocol> {
    let idx = PREFERENCE.iter().position(|p| *p == failed)?;
    PREFERENCE.get(idx + 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_never_reaches_the_engine() {
        let p = ConnectionProfile::default();
        assert_ne!(pick(&p), Protocol::Smart);
        for c in build_plan(&p, false) {
            assert_ne!(c.profile.protocol, Protocol::Smart);
        }
    }

    #[test]
    fn explicit_protocol_is_respected() {
        let p = ConnectionProfile { protocol: Protocol::Wireguard, ..Default::default() };
        assert_eq!(pick(&p), Protocol::Wireguard);
        for c in build_plan(&p, false) {
            assert_eq!(c.profile.protocol, Protocol::Wireguard);
        }
    }

    #[test]
    fn fallback_order_matches_android() {
        assert_eq!(next_after(Protocol::Masque), Some(Protocol::Gool));
        assert_eq!(next_after(Protocol::Gool), Some(Protocol::Wireguard));
        assert_eq!(next_after(Protocol::Wireguard), None);
    }

    #[test]
    fn direct_plan_has_a_hardened_second_pass() {
        let p = ConnectionProfile { protocol: Protocol::Gool, ..Default::default() };
        let plan = build_plan(&p, false);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].profile.noize, Noize::Off);
        assert_eq!(plan[1].profile.noize, Noize::Firewall);
        assert_eq!(plan[0].profile.protocol, plan[1].profile.protocol);
    }
}
