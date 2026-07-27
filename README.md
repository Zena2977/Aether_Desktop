<div align="center">

<img src="src-tauri/icons/128x128.png" width="96" alt="Aether" />

# Aether Desktop

**Freedom, in one tap**

The Windows edition of Aether Mobile — same interface, same icon, same core engine.

[فارسی](README.fa.md) · [Releases](../../releases) · [Setup guide](SETUP.md)

</div>

---

## 🎉 Version 1.0.0 — first desktop release

This is the **first** release of Aether for Windows. It is version `1.0.0` because the
desktop edition starts its own version line, independent of the Android app.

Every feature of the Android edition is implemented here, with nothing left out.

### Interface

- Identical design, colour palette and icon to the mobile edition (always dark theme)
- Adapted for large desktop displays: two-column layout with a persistent side rail
- Custom Windows 11-style title bar (minimise / maximise / close) — double-clicking it
  maximises/restores, like any native Windows window
- Full right-to-left support
- Fully bilingual interface — English and فارسی — switchable live from the Advanced tab;
  the choice is stored outside the profile, so "Reset to defaults" never changes your language

### Connection

- Connect button with the same **8 states** as Android:
  Disconnected · Starting engine · Connecting · Verifying · Connected · Reconnecting · Disconnecting · Connection failed
- Protocols: `Smart` (automatic pick — same name as the mobile edition) · `MASQUE` · `WireGuard` · `WARP×2`
- Scan modes: Turbo · Balanced · Thorough · Stealth · Ironclad
- IPv4, IPv6 or dual-stack
- Smart automatic protocol fallback when a protocol fails
- Automatic reconnect (capped at 5 attempts, exactly as on Android)
- Live connection details: protocol, endpoint, latency, uptime
- Traffic panel: live per-second download/upload rates plus session totals
- IP badge with a country flag — "Your IP" while disconnected, the tunnel's "Server IP"
  once connected. Flags are built-in SVGs (75+ countries, neutral globe fallback), because
  Windows has no emoji flag font

### Advanced settings

| Setting | Values |
|---|---|
| Noize | Off · Light · Firewall · Balanced · GFW · Aggressive |
| Endpoint | Auto · Manual peer · Manual range |
| MTU | presets plus a free numeric field (default 1280) |
| Keepalive | presets plus a free numeric field |
| Fragment | on / off |
| ECH | on / off |
| MASQUE HTTP/2 | on / off |
| Quick reconnect | on / off |
| Split tunnelling | Off · Include · Exclude |
| Language | English · فارسی |

Split tunnelling selects applications by executable name instead of Android package name —
the only place where the desktop edition differs, because Windows has no package names.

A **Reset to defaults** button restores every setting above to its factory value —
the UI language is deliberately left untouched.

### Network sharing

- Share the tunnel with other devices on your local network
- SOCKS5 on port `10810`, HTTP on port `10811` (same ports as the Android edition)
- Binds only to the detected LAN address, never to every interface
- Endpoint addresses shown in the app with one-click copy
- Both ports auto-detect the protocol — each one accepts HTTP **and** SOCKS5,
  so either port works in either field

### Diagnostics

- **Run test** — the same live self-test as Android: the checks update in place
  (PENDING → RUNNING → PASS / FAIL) under a colour-coded overall verdict
- **Environment check** — a six-point report, identical in meaning to the Android panel:
  1. Engine binary present
  2. Wintun driver present
  3. Administrator rights
  4. Local SOCKS5 port reachable
  5. Real tunnel egress verified (SOCKS5 handshake plus an outbound request)
  6. Profile sanity
- Live, colour-coded log console (E / W / I / D levels) that refreshes every second
- **Copy logs** (with a confirmation toast) and **Clear** actions

### Windows-specific behaviour

- Real system-wide tunnel using the official, Microsoft-signed **Wintun** driver —
  the desktop equivalent of Android's `VpnService`
- Windows Firewall rule configured automatically during installation, removed on uninstall
- Single-instance launch: reopening focuses the existing window
- Statically linked — no Visual C++ Redistributable required
- WebView2 installed automatically if missing
- On connect, the Windows system proxy (WinINET — what Edge/Chrome/Firefox call the
  "system proxy") is pointed at a local HTTP↔SOCKS5 bridge so system traffic really
  flows through the tunnel; it is reverted on disconnect, on errors and on exit

### About panel

- App version, core engine version and CPU architecture at a glance
- Expandable credits card — links to the upstream Cluvex Studio project (GitHub · Telegram)
  and to the Windows edition, with the upstream feature list and what this build adds
- Links open in your default browser

### Deliberately absent

- **Proxy Mode** — not meaningful on Windows, so it was left out by design.
  The build pipeline actively fails if anyone reintroduces it.
- **In-app updater** — removed. The About panel links to the GitHub Releases page instead.

---

## Downloads

All files are produced automatically by GitHub Actions and published to
[Releases](../../releases). No manual step is involved at any point.

| File | Description |
|---|---|
| `Aether-Setup-1.0.0-x64.exe` | Windows 64-bit — graphical installer with uninstaller (recommended) |
| `Aether-Setup-1.0.0-x86.exe` | Windows 32-bit — graphical installer with uninstaller |
| `Aether-Portable-1.0.0-x64.zip` | Portable, no installation, 64-bit |
| `Aether-Portable-1.0.0-x86.zip` | Portable, no installation, 32-bit |
| `SHA256SUMS.txt` | Checksums for verifying file integrity |

**Requirements:** Windows 10 build 1809 (October 2018 Update) or newer.
Establishing the tunnel requires Administrator rights, because a network adapter must be
created — the direct equivalent of Android's VPN permission prompt.

The portable edition keeps its settings and logs next to the executable, so it leaves no
trace elsewhere on the machine.

---

## Technology

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2** | Uses the system WebView2, so the installer stays a few megabytes instead of ~100 MB with Electron |
| Logic | **Rust** | The Aether core engine is already Rust, so the app and the engine share one toolchain |
| Interface | HTML / CSS / vanilla JS, bundled by **Vite** | Reproduces the Compose design exactly, with no framework overhead |
| Tunnel | **Wintun** | The official WireGuard driver for Windows — signed by Microsoft |
| Installer | **Inno Setup 6** | Modern resizable wizard, bilingual, real uninstaller |
| Automation | **GitHub Actions** | Every build, test, package and publish step runs without human involvement |

### Module mapping from Android

| Android (Kotlin) | Windows (Rust) |
|---|---|
| `MainActivity` / `AetherApp` | `main.rs` |
| `AetherController` | `state.rs` |
| `AetherVpnService` | `tun.rs` |
| `AetherProcess` | `engine.rs` |
| `Profile` | `profile.rs` |
| `ProfileStore` | `store.rs` |
| `NetProbe` / `PortProbe` | `probe.rs` |
| `SmartAuto` | `smart_auto.rs` |
| `ShareBridge` | `share.rs` |
| `Diagnostics` | `diagnostics.rs` |
| `DiagnosticsLog` | `log.rs` |

---

## Repository layout

```
.github/workflows/build.yml   the entire automated pipeline
installer/aether.iss          professional installation wizard
scripts/                      build, packaging and verification scripts
src/                          interface (HTML / CSS / JS)
src-tauri/                    application logic (Rust)
native/aether/                the Aether core engine, synced automatically
```

## Building it yourself

You do not need to. Pushing to `main` produces every artifact automatically.
See [SETUP.md](SETUP.md) for the step-by-step repository setup guide.

## Licence

MIT — see [LICENSE](LICENSE).
Bundles the Wintun driver under its own licence.
