// پورت از ui/HomeScreen.kt + ConnectButton.kt + StatusLine.kt + ConnectionMeta.kt + TrafficPanel.kt
//
// رفع ریشه‌ای مشکل ۷: دکمهٔ قبلی فقط یک رنگ عوض می‌کرد؛ حالا مثل
// ConnectButton.kt چهار مود کامل دارد (IDLE / BUSY / CONNECTED / ERROR):
//   * آیکون عوض می‌شود: Power → Autorenew (چرخان) → Bolt
//   * در BUSY یک کمان ۹۰درجهٔ دور دکمه می‌چرخد (۱۱۰۰ms)
//   * در CONNECTED هاله نبض می‌زند (.92→1.06، ۱۶۰۰ms)
//   * گذار رنگ ۶۰۰ms — همان tween اندروید
// به‌علاوه نشان IP + پرچم کشور (Your IP / Server IP) و تایمر «Connected for».
import { app, onChange, toggleConnection, accentFor, formatBytes, formatUptime } from '../main.js'
import { flagHtml } from '../flags.js'
import { t } from '../i18n.js'

const BUSY_STATES = ['STARTING_ENGINE', 'CONNECTING', 'VERIFYING', 'RECONNECTING', 'DISCONNECTING']

// همان آیکون‌های متریال ConnectButton.kt — PowerSettingsNew / Autorenew / Bolt.
const ICON_POWER =
  '<svg viewBox="0 0 24 24" width="56" height="56" aria-hidden="true"><path d="M12 3v9" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/><path d="M6.5 6.8a8 8 0 1 0 11 0" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round"/></svg>'
const ICON_RENEW =
  '<svg viewBox="0 0 24 24" width="56" height="56" aria-hidden="true"><path d="M12 5a7 7 0 0 1 6.3 4" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round"/><path d="M18.6 4.6V9h-4.4" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round" stroke-linejoin="round"/><path d="M12 19a7 7 0 0 1-6.3-4" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round"/><path d="M5.4 19.4V15h4.4" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>'
const ICON_BOLT =
  '<svg viewBox="0 0 24 24" width="56" height="56" aria-hidden="true"><path d="M13 2 4.5 13.5H11L9.8 22 18.5 10.5H12L13 2Z" fill="currentColor"/></svg>'

// پرچم کشور: SVG درون‌ساخت از flags.js — ویندوز فونت ایموجی پرچم ندارد و
// کد کشور به‌صورت دو حرف (مثل «DE») رندر می‌شد؛ رفع ریشه‌ای بخش پرچم مشکل ۴.

// زیرنویس وضعیت — همان رشته‌های strings.xml در StatusLine.kt.
function subtitleFor(snapshot) {
  switch (snapshot.state) {
    case 'DISCONNECTED': return t('Tap to connect securely')
    case 'CONNECTED': return t('Tap to disconnect')
    case 'FAILED': return snapshot.error || t('Something went wrong')
    case 'VERIFYING': return t('Verifying connection…')
    default: return snapshot.detail || ''
  }
}

export function renderHome() {
  const root = document.createElement('div')
  root.className = 'view view--home'
  root.innerHTML = `
    <div class="hero">
      <h1 class="hero__title">${t('Aether')}</h1>
      <p class="hero__tagline">${t('Freedom, in one tap')}</p>
    </div>

    <div class="connect-wrap">
      <span class="connect__halo" id="halo"></span>
      <button class="connect" id="connect" type="button" aria-live="polite">
        <svg class="connect__arc" viewBox="0 0 100 100" aria-hidden="true">
          <circle cx="50" cy="50" r="47" fill="none" stroke="currentColor" stroke-width="2.5"
                  stroke-linecap="round" stroke-dasharray="73.8 221.4"/>
        </svg>
        <span class="connect__icon" id="cicon">${ICON_POWER}</span>
      </button>
    </div>

    <p class="status" id="status">${t('Disconnected')}</p>
    <p class="status__detail" id="detail">${t('Tap to connect securely')}</p>

    <div class="ipbadge ltr" dir="ltr" id="ipbadge">
      <span class="ipbadge__flag" id="ip-flag">${flagHtml(null)}</span>
      <span class="ipbadge__label" id="ip-label">${t('Your IP')}</span>
      <span class="ipbadge__value" id="ip-value">${t('Checking IP…')}</span>
    </div>

    <p class="uptime" id="uptime" hidden><span class="uptime__k">${t('Connected for')}</span> <span class="uptime__v ltr" dir="ltr" id="uptime-v">00:00</span></p>

    <div class="meta" id="meta">
      <div class="meta__cell"><span class="meta__k">${t('Protocol')}</span><span class="meta__v" id="m-proto">—</span></div>
      <div class="meta__cell"><span class="meta__k">${t('Endpoint')}</span><span class="meta__v ltr" dir="ltr" id="m-endpoint">—</span></div>
      <div class="meta__cell"><span class="meta__k">${t('Latency')}</span><span class="meta__v" id="m-latency">—</span></div>
    </div>

    <div class="traffic" id="traffic" hidden>
      <div class="traffic__cell">
        <span class="traffic__badge traffic__badge--down" aria-hidden="true">↓</span>
        <span class="traffic__text">
          <span class="traffic__k">${t('Download')}</span>
          <span class="traffic__rate ltr" dir="ltr" id="t-rx-rate">0 B/s</span>
          <span class="traffic__total ltr" dir="ltr" id="t-rx">0 B</span>
        </span>
      </div>
      <span class="traffic__divider" aria-hidden="true"></span>
      <div class="traffic__cell">
        <span class="traffic__badge traffic__badge--up" aria-hidden="true">↑</span>
        <span class="traffic__text">
          <span class="traffic__k">${t('Upload')}</span>
          <span class="traffic__rate ltr" dir="ltr" id="t-tx-rate">0 B/s</span>
          <span class="traffic__total ltr" dir="ltr" id="t-tx">0 B</span>
        </span>
      </div>
    </div>
  `

  root.querySelector('#connect').addEventListener('click', toggleConnection)

  // محاسبهٔ نرخ لحظه‌ای — همان کاری که TrafficPanel.kt با دلتای بایت‌ها می‌کرد.
  let lastRx = 0, lastTx = 0, lastAt = 0
  const spinAnims = []

  const paint = ({ snapshot }) => {
    const accent = accentFor(snapshot.state)
    const busy = BUSY_STATES.includes(snapshot.state)
    const connected = snapshot.state === 'CONNECTED'

    const btn = root.querySelector('#connect')
    btn.style.setProperty('--accent', accent)
    btn.classList.toggle('is-busy', busy)
    btn.classList.toggle('is-on', connected)
    btn.classList.toggle('is-error', snapshot.state === 'FAILED')
    root.querySelector('#halo').classList.toggle('is-on', connected)

    // تعویض آیکون — فقط وقتی مود واقعاً عوض شده (تا انیمیشن ریست نشود).
    const mode = busy ? 'busy' : connected ? 'on' : 'idle'
    const iconEl = root.querySelector('#cicon')
    if (iconEl.dataset.mode !== mode) {
      iconEl.dataset.mode = mode
      iconEl.innerHTML = busy ? ICON_RENEW : connected ? ICON_BOLT : ICON_POWER
      // Root fix (recurring problem 2): drive the busy spin with the Web
      // Animations API. Plain CSS animations get globally neutralised when
      // Windows reports prefers-reduced-motion (animation effects off /
      // VM / RDP), which is exactly why the arc + arrows looked frozen.
      // WAAPI animations run on the compositor and win over stylesheets.
      for (const a of spinAnims.splice(0)) a.cancel()
      if (busy) {
        const spin = [
          { transform: 'translateZ(0) rotate(0deg)' },
          { transform: 'translateZ(0) rotate(360deg)' },
        ]
        const arcEl = root.querySelector('.connect__arc')
        if (arcEl && arcEl.animate) {
          spinAnims.push(arcEl.animate(spin, { duration: 1100, iterations: Infinity }))
        }
        if (iconEl.animate) {
          spinAnims.push(iconEl.animate(spin, { duration: 1400, iterations: Infinity }))
        }
      }
    }

    const title = {
      DISCONNECTED: t('Disconnected'),
      STARTING_ENGINE: t('Starting engine…'),
      CONNECTING: t('Connecting…'),
      VERIFYING: t('Verifying connection…'),
      CONNECTED: t('Connected'),
      RECONNECTING: t('Reconnecting…'),
      DISCONNECTING: t('Disconnecting…'),
      FAILED: t('Connection failed'),
    }[snapshot.state] ?? snapshot.state
    const statusEl = root.querySelector('#status')
    statusEl.textContent = title
    statusEl.style.color = accent
    root.querySelector('#detail').textContent = subtitleFor(snapshot)

    // نشان IP + پرچم — همان رفتار MainActivity: در حالت‌های گذار مخفی.
    const badge = root.querySelector('#ipbadge')
    badge.hidden = busy
    if (!busy) {
      const info = snapshot.ipInfo
      root.querySelector('#ip-label').textContent = info && info.viaTunnel ? t('Server IP') : t('Your IP')
      badge.classList.toggle('is-tunnel', !!(info && info.viaTunnel))
      if (info) {
        root.querySelector('#ip-flag').innerHTML = flagHtml(info.countryCode)
        root.querySelector('#ip-value').textContent = info.countryCode ? `${info.ip} · ${info.countryCode}` : info.ip
      } else {
        root.querySelector('#ip-flag').innerHTML = flagHtml(null)
        root.querySelector('#ip-value').textContent = snapshot.ipLoading ? t('Checking IP…') : t('IP unavailable')
      }
    }

    // تایمر «Connected for»
    const up = root.querySelector('#uptime')
    up.hidden = !connected
    if (connected) root.querySelector('#uptime-v').textContent = formatUptime(snapshot.uptimeSecs)

    root.querySelector('#m-proto').textContent = snapshot.protocol ?? '—'
    root.querySelector('#m-endpoint').textContent = snapshot.endpoint ?? '—'
    root.querySelector('#m-latency').textContent = snapshot.latencyMs != null ? `${snapshot.latencyMs} ms` : '—'

    // پنل ترافیک — فقط در حالت متصل (مثل TrafficPanel موبایل).
    const traffic = root.querySelector('#traffic')
    traffic.hidden = !connected
    if (connected) {
      const now = Date.now()
      if (lastAt && now > lastAt) {
        const dt = (now - lastAt) / 1000
        const rxRate = Math.max(0, (snapshot.rxBytes - lastRx) / dt)
        const txRate = Math.max(0, (snapshot.txBytes - lastTx) / dt)
        root.querySelector('#t-rx-rate').textContent = `${formatBytes(rxRate)}/s`
        root.querySelector('#t-tx-rate').textContent = `${formatBytes(txRate)}/s`
      }
      root.querySelector('#t-rx').textContent = formatBytes(snapshot.rxBytes)
      root.querySelector('#t-tx').textContent = formatBytes(snapshot.txBytes)
      lastRx = snapshot.rxBytes
      lastTx = snapshot.txBytes
      lastAt = now
    } else {
      lastRx = 0
      lastTx = 0
      lastAt = 0
    }
  }

  paint(app)
  onChange(paint)
  return root
}