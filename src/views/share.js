// پورت از ui/SharePanel.kt (معادل core/ShareBridge.kt)
import { app, onChange, saveProfile } from '../main.js'
import { t } from '../i18n.js'

export function renderShare() {
  const root = document.createElement('div')
  root.className = 'view view--share'
  root.innerHTML = `
    <h2 class="view__title">${t('Share over LAN')}</h2>
    <p class="view__lead">${t('Other devices on the same Wi‑Fi can route their traffic through this computer. Point them at one of the addresses below.')}</p>

    <section class="field field--row">
      <div>
        <span class="field__label">${t('Enable sharing')}</span>
        <span class="field__hint">${t('Only listens on your local network address.')}</span>
      </div>
      <button type="button" class="switch" id="share-toggle" role="switch"><span class="switch__knob"></span></button>
    </section>

    <div class="cards">
      <div class="card">
        <span class="card__k">SOCKS5</span>
        <span class="card__v ltr" dir="ltr" id="share-socks">—</span>
        <button class="btn btn--ghost" data-copy="share-socks">${t('Copy')}</button>
      </div>
      <div class="card">
        <span class="card__k">HTTP</span>
        <span class="card__v ltr" dir="ltr" id="share-http">—</span>
        <button class="btn btn--ghost" data-copy="share-http">${t('Copy')}</button>
      </div>
    </div>

    <p class="note">${t('Sharing only works while Aether is connected.')}</p>
    <p class="note">${t('Both ports accept HTTP and SOCKS5 automatically — either port works in either field.')}</p>
    <p class="note">${t('Apps like Telegram ignore the system proxy; set a SOCKS5 proxy inside the app instead.')}</p>
  `

  const sw = root.querySelector('#share-toggle')
  sw.addEventListener('click', async () => {
    const on = !sw.classList.contains('is-on')
    sw.classList.toggle('is-on', on)
    sw.setAttribute('aria-checked', String(on))
    await saveProfile({ lanShare: on })
  })

  root.querySelectorAll('[data-copy]').forEach((b) => {
    b.addEventListener('click', () => {
      const text = root.querySelector(`#${b.dataset.copy}`).textContent
      if (text && text !== '—') navigator.clipboard.writeText(text)
    })
  })

  const paint = ({ snapshot, profile }) => {
    sw.classList.toggle('is-on', !!profile?.lanShare)
    sw.setAttribute('aria-checked', String(!!profile?.lanShare))
    root.querySelector('#share-socks').textContent = snapshot.shareSocks ?? '—'
    root.querySelector('#share-http').textContent = snapshot.shareHttp ?? '—'
  }

  paint(app)
  onChange(paint)
  return root
}
