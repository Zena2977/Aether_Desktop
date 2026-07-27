// =============================================================================
//  Aether Desktop — پوستهٔ کاربری
//  پورت یک‌به‌یک از ui/ مخزن اندروید (Jetpack Compose → DOM).
// =============================================================================
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { renderHome } from './views/home.js'
import { renderAdvanced } from './views/advanced.js'
import { renderDiagnostics } from './views/diagnostics.js'
import { renderShare } from './views/share.js'
import { renderAbout } from './views/about.js'

import { t, applyLang } from './i18n.js'

// --- وضعیت سراسری ------------------------------------------------------
export const app = {
  snapshot: {
    state: 'DISCONNECTED',
    detail: '',
    error: null,
    endpoint: null,
    protocol: null,
    latencyMs: null,
    uptimeSecs: 0,
    rxBytes: 0,
    txBytes: 0,
    shareSocks: null,
    shareHttp: null,
    ipInfo: null,
    ipLoading: true,
  },
  profile: null,
  tab: 'home',
  listeners: new Set(),
}

export function onChange(fn) {
  app.listeners.add(fn)
  return () => app.listeners.delete(fn)
}

function emit() {
  for (const fn of app.listeners) fn(app)
}

export async function saveProfile(patch) {
  app.profile = { ...app.profile, ...patch }
  await invoke('set_profile', { profile: app.profile })
  emit()
}

export async function toggleConnection() {
  try {
    await invoke('toggle_connection')
  } catch (e) {
    app.snapshot.error = String(e)
    emit()
  }
}

// --- رنگ حالت — دقیقاً همان قانون ConnectButton.kt --------------------
export function accentFor(state) {
  if (state === 'CONNECTED') return '#32E0C4'
  if (state === 'FAILED') return '#FF5C7A'
  return '#4C8DFF'
}

// --- متن حالت — همان رشته‌های strings.xml -----------------------------
export const STATE_LABEL = {
  DISCONNECTED: 'Disconnected',
  STARTING_ENGINE: 'Starting engine…',
  CONNECTING: 'Connecting…',
  VERIFYING: 'Verifying…',
  CONNECTED: 'Connected',
  RECONNECTING: 'Reconnecting…',
  DISCONNECTING: 'Disconnecting…',
  FAILED: 'Connection failed',
}

export function formatBytes(n) {
  if (!n) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), units.length - 1)
  return `${(n / 1024 ** i).toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

export function formatUptime(secs) {
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  const pad = (x) => String(x).padStart(2, '0')
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`
}

// --- نوار عنوان سفارشی (decorations: false) ---------------------------
// رفع ریشه‌ای آیکون‌های خراب (مربع): آیکون‌های قبلی گلیف فونت
// «Segoe MDL2 Assets» در index.html بودند که بدون آن فونت به‌صورت مربع
// رندر می‌شدند. حالا هر سه آیکون SVG داخلی هستند و همین‌جا در زمان
// اجرا داخل دکمه‌ها تزریق می‌شوند؛ یعنی منبع آیکون‌ها فقط همین باندل
// جاوااسکریپت است و حتی با index.html قدیمی هم درست رندر می‌شوند.
const ICON_MINIMIZE =
  '<svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M0 5h10" stroke="currentColor" stroke-width="1" fill="none"/></svg>'
const ICON_MAXIMIZE =
  '<svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/></svg>'
const ICON_RESTORE =
  '<svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="currentColor" stroke-width="1"/><rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1"/></svg>'
const ICON_CLOSE =
  '<svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1.1" fill="none"/></svg>'

// v9: Material-style outline icons for the permanent navigation rail.
const NAV_ICONS = {
  home: '<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 10.5 12 3l9 7.5"/><path d="M5.5 9.5V20h13V9.5"/></svg>',
  advanced: '<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M4 7h8M18 7h2M4 17h2M10 17h10"/><circle cx="15" cy="7" r="2.4"/><circle cx="7" cy="17" r="2.4"/></svg>',
  diagnostics: '<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12h4l2.5-6 5 12 2.5-6h4"/></svg>',
  share: '<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M4.5 12a10.5 10.5 0 0 1 15 0"/><path d="M7.8 15.2a6 6 0 0 1 8.4 0"/><circle cx="12" cy="18.6" r="1.5" fill="currentColor" stroke="none"/></svg>',
  about: '<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="8.5"/><path d="M12 11v5"/><circle cx="12" cy="7.8" r="1" fill="currentColor" stroke="none"/></svg>',
}

async function refreshMaximizeButton(win, btn) {
  if (!btn) return
  try {
    const maximized = await win.isMaximized()
    btn.innerHTML = maximized ? ICON_RESTORE : ICON_MAXIMIZE
    btn.setAttribute('aria-label', maximized ? 'Restore' : 'Maximize')
    btn.title = maximized ? 'Restore' : 'Maximize'
  } catch {
    btn.innerHTML = ICON_MAXIMIZE
  }
}

function wireTitlebar() {
  const win = getCurrentWindow()
  const minBtn = document.querySelector('.twin--min')
  const maxBtn = document.querySelector('.twin--max')
  const closeBtn = document.querySelector('.twin--close')

  // تزریق آیکون‌ها — جایگزین هر محتوای قبلی (از جمله گلیف فونت خراب).
  if (minBtn) minBtn.innerHTML = ICON_MINIMIZE
  if (closeBtn) closeBtn.innerHTML = ICON_CLOSE
  refreshMaximizeButton(win, maxBtn)

  document.querySelector('.titlebar')?.addEventListener('mousedown', (e) => {
    if (e.target.closest('.twin')) return
    win.startDragging()
  })
  // دقیقاً مثل ویندوز: دابل‌کلیک روی نوار عنوان هم بزرگ/کوچک می‌کند.
  document.querySelector('.titlebar')?.addEventListener('dblclick', async (e) => {
    if (e.target.closest('.twin')) return
    await win.toggleMaximize()
    refreshMaximizeButton(win, maxBtn)
  })
  minBtn?.addEventListener('click', () => win.minimize())
  maxBtn?.addEventListener('click', async () => {
    await win.toggleMaximize()
    refreshMaximizeButton(win, maxBtn)
  })
  closeBtn?.addEventListener('click', () => win.close())
  win.onResized(() => refreshMaximizeButton(win, maxBtn))
}

// --- مسیریابی تب‌ها (در اندروید باتم‌شیت بود، در دسکتاپ ریل کناری) --
const VIEWS = {
  home: renderHome,
  advanced: renderAdvanced,
  diagnostics: renderDiagnostics,
  share: renderShare,
  about: renderAbout,
}

function renderTab() {
  const host = document.getElementById('view')
  // رفع ریشه‌ای یکی از علل گیرکردن انیمیشن: لیسنرهای paint ویوهای قبلی
  // پاک می‌شوند؛ وگرنه با هر تعویض تب، paint روی DOM جداشده هر ۲۰۰ms اجرا می‌ماند.
  app.listeners.clear()
  host.innerHTML = ''
  host.appendChild(VIEWS[app.tab](app))
  for (const b of document.querySelectorAll('.rail__item')) {
    b.classList.toggle('is-active', b.dataset.tab === app.tab)
  }
}

function wireRail() {
  for (const b of document.querySelectorAll('.rail__item')) {
    b.addEventListener('click', () => {
      app.tab = b.dataset.tab
      renderTab()
    })
  }
}

// --- راه‌اندازی ---------------------------------------------------------
const NAV_LABELS = { home: 'Home', advanced: 'Advanced', diagnostics: 'Diagnostics', share: 'Share over LAN', about: 'About' }

// v9: retranslate the chrome (nav rail icons + labels + window title) for the
// active language. The rail is a permanent Material-style navigation rail.
function translateChrome() {
  for (const b of document.querySelectorAll('.rail__item')) {
    const label = t(NAV_LABELS[b.dataset.tab] ?? b.dataset.tab)
    b.innerHTML =
      '<span class="rail__icon">' + (NAV_ICONS[b.dataset.tab] ?? '') + '</span>' +
      '<span class="rail__label"></span>'
    b.querySelector('.rail__label').textContent = label
    b.title = label
  }
  const title = document.querySelector('.titlebar__title')
  if (title) title.textContent = t('Aether')
}

// v8: re-render chrome + current tab after a language change.
export function rerender() {
  translateChrome()
  renderTab()
}

async function boot() {
  applyLang()
  wireTitlebar()
  wireRail()
  translateChrome()

  app.profile = await invoke('get_profile')
  app.snapshot = await invoke('get_snapshot')

  // جریان زندهٔ وضعیت — معادل StateFlow در اندروید (هر ۲۰۰ میلی‌ثانیه).
  let lastAccent = null
  await listen('aether://state', (event) => {
    app.snapshot = event.payload
    // فقط وقتی رنگ واقعاً عوض شده بنویس — نوشتن مداوم متغیر CSS روی <html>
    // هر ۲۰۰ms باعث style-recalc کل صفحه و گیرکردن انیمیشن کمان می‌شد.
    const accent = accentFor(app.snapshot.state)
    if (accent !== lastAccent) {
      lastAccent = accent
      document.documentElement.style.setProperty('--accent', accent)
    }
    emit()
  })

  renderTab()
  emit()
}

boot()
