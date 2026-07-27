// Inline SVG country flags (no font, no network) - root fix for the flag part
// of problem 4: Windows has no emoji-flag font, so regional-indicator emoji
// render as bare letters like "DE". These are self-contained SVG bodies
// (viewBox 60x40) covering common WARP egress countries; unknown or missing
// codes fall back to a neutral globe mark, matching the mobile behaviour.

const W = 60
const H = 40

function bg(c) { return `<rect width="${W}" height="${H}" fill="${c}"/>` }

function hBands(...cols) {
  const bh = H / cols.length
  return cols.map((c, i) => `<rect x="0" y="${(i * bh).toFixed(2)}" width="${W}" height="${bh.toFixed(2)}" fill="${c}"/>`).join('')
}

function vBands(...cols) {
  const bw = W / cols.length
  return cols.map((c, i) => `<rect x="${(i * bw).toFixed(2)}" y="0" width="${bw.toFixed(2)}" height="${H}" fill="${c}"/>`).join('')
}

function disc(c, r = 8, cx = W / 2, cy = H / 2) { return `<circle cx="${cx}" cy="${cy}" r="${r}" fill="${c}"/>` }

function nordic(field, cross, inner) {
  let s = bg(field)
  s += `<rect x="18" y="0" width="10" height="${H}" fill="${cross}"/><rect x="0" y="15" width="${W}" height="10" fill="${cross}"/>`
  if (inner) s += `<rect x="20.5" y="0" width="5" height="${H}" fill="${inner}"/><rect x="0" y="17.5" width="${W}" height="5" fill="${inner}"/>`
  return s
}

function star(cx, cy, r, c) {
  const pts = []
  for (let i = 0; i < 5; i++) {
    const a = -Math.PI / 2 + (i * 4 * Math.PI) / 5
    pts.push(`${(cx + r * Math.cos(a)).toFixed(1)},${(cy + r * Math.sin(a)).toFixed(1)}`)
  }
  return `<polygon points="${pts.join(' ')}" fill="${c}"/>`
}

function unionJack() {
  return bg('#012169')
    + '<path d="M0,0 60,40 M60,0 0,40" stroke="#fff" stroke-width="8"/>'
    + '<path d="M0,0 60,40 M60,0 0,40" stroke="#C8102E" stroke-width="4"/>'
    + '<rect x="24" y="0" width="12" height="40" fill="#fff"/><rect x="0" y="14" width="60" height="12" fill="#fff"/>'
    + '<rect x="27" y="0" width="6" height="40" fill="#C8102E"/><rect x="0" y="17" width="60" height="6" fill="#C8102E"/>'
}

function usFlag() {
  let s = ''
  for (let i = 0; i < 13; i++) s += `<rect x="0" y="${((i * H) / 13).toFixed(2)}" width="${W}" height="${(H / 13).toFixed(2)}" fill="${i % 2 ? '#fff' : '#B22234'}"/>`
  s += `<rect width="26" height="${((7 * H) / 13).toFixed(2)}" fill="#3C3B6E"/>`
  return s
}

function grFlag() {
  let s = ''
  for (let i = 0; i < 9; i++) s += `<rect x="0" y="${((i * H) / 9).toFixed(2)}" width="${W}" height="${(H / 9).toFixed(2)}" fill="${i % 2 ? '#fff' : '#004C98'}"/>`
  s += '<rect width="22" height="22" fill="#004C98"/><rect x="9" width="4" height="22" fill="#fff"/><rect y="9" width="22" height="4" fill="#fff"/>'
  return s
}

function myFlag() {
  let s = ''
  for (let i = 0; i < 14; i++) s += `<rect x="0" y="${((i * H) / 14).toFixed(2)}" width="${W}" height="${(H / 14).toFixed(2)}" fill="${i % 2 ? '#fff' : '#CC0001'}"/>`
  s += '<rect width="30" height="20" fill="#010066"/><circle cx="12" cy="10" r="6" fill="#FFCC00"/><circle cx="14.5" cy="10" r="5.2" fill="#010066"/>' + star(21, 10, 4, '#FFCC00')
  return s
}

const FLAGS = {
  AE: bg('#00732F') + '<rect y="13.33" width="60" height="13.34" fill="#fff"/><rect y="26.67" width="60" height="13.33" fill="#000"/><rect width="16" height="40" fill="#FF0000"/>',
  AM: hBands('#D90012', '#0033A0', '#F2A800'),
  AR: hBands('#74ACDF', '#fff', '#74ACDF') + disc('#F6B40E', 5),
  AT: hBands('#ED2939', '#fff', '#ED2939'),
  AU: bg('#012169') + star(45, 24, 7, '#fff') + star(15, 10, 4, '#fff'),
  AZ: hBands('#0092BC', '#E4002B', '#00AF66') + disc('#fff', 4),
  BE: vBands('#000', '#FDDA24', '#EF3340'),
  BG: hBands('#fff', '#00966E', '#D62612'),
  BR: bg('#009C3B') + '<polygon points="30,5 55,20 30,35 5,20" fill="#FFDF00"/>' + disc('#002776', 7),
  CA: vBands('#D80621', '#fff', '#D80621') + '<polygon points="30,10 33,17 40,16 35,22 37,29 30,25 23,29 25,22 20,16 27,17" fill="#D80621"/>',
  CH: bg('#DA291C') + '<rect x="26" y="12" width="8" height="16" fill="#fff"/><rect x="22" y="16" width="16" height="8" fill="#fff"/>',
  CL: bg('#fff') + '<rect y="20" width="60" height="20" fill="#D52B1E"/><rect width="20" height="20" fill="#0039A6"/>' + star(10, 10, 5, '#fff'),
  CN: bg('#DE2910') + star(12, 12, 7, '#FFDE00'),
  CO: '<rect width="60" height="20" fill="#FCD116"/><rect y="20" width="60" height="10" fill="#003893"/><rect y="30" width="60" height="10" fill="#CE1126"/>',
  CY: bg('#fff') + disc('#D57800', 6),
  CZ: hBands('#fff', '#D7141A') + '<polygon points="0,0 30,20 0,40" fill="#11457E"/>',
  DE: hBands('#000', '#DD0000', '#FFCE00'),
  DK: nordic('#C8102E', '#fff'),
  EE: hBands('#0072CE', '#000', '#fff'),
  EG: hBands('#CE1126', '#fff', '#000') + disc('#C09300', 4),
  ES: '<rect width="60" height="10" fill="#AA151B"/><rect y="10" width="60" height="20" fill="#F1BF00"/><rect y="30" width="60" height="10" fill="#AA151B"/>',
  FI: nordic('#fff', '#002F6C'),
  FR: vBands('#002395', '#fff', '#ED2939'),
  GB: unionJack(),
  GE: bg('#fff') + '<rect x="26" width="8" height="40" fill="#FF0000"/><rect y="16" width="60" height="8" fill="#FF0000"/>',
  GR: grFlag(),
  HK: bg('#DE2910') + disc('#fff', 6),
  HR: hBands('#FF0000', '#fff', '#171796') + disc('#FF0000', 5),
  HU: hBands('#CE2939', '#fff', '#477050'),
  ID: hBands('#CE1126', '#fff'),
  IE: vBands('#009A44', '#fff', '#FF8200'),
  IL: bg('#fff') + '<rect y="4" width="60" height="5" fill="#0038B8"/><rect y="31" width="60" height="5" fill="#0038B8"/><path d="M30 12 37 24 23 24Z M30 28 23 16 37 16Z" fill="none" stroke="#0038B8" stroke-width="1.8"/>',
  IN: hBands('#FF9933', '#fff', '#138808') + '<circle cx="30" cy="20" r="4.5" fill="none" stroke="#000080" stroke-width="1.5"/>',
  IQ: hBands('#CE1126', '#fff', '#000') + '<circle cx="30" cy="20" r="3" fill="#007A3D"/>',
  IR: hBands('#239F40', '#fff', '#DA0000') + '<circle cx="30" cy="20" r="4" fill="none" stroke="#DA0000" stroke-width="1.6"/>',
  IS: nordic('#02529C', '#fff', '#DC1E35'),
  IT: vBands('#009246', '#fff', '#CE2B37'),
  JP: bg('#fff') + disc('#BC002D', 8),
  KE: hBands('#000', '#B22222', '#006600'),
  KR: bg('#fff') + disc('#CD2E3A', 7) + '<path d="M23 20a7 7 0 0 1 14 0a3.5 3.5 0 0 1-7 0a3.5 3.5 0 0 0-7 0Z" fill="#0047A0"/>',
  KW: hBands('#007A3D', '#fff', '#CE1126') + '<polygon points="0,0 15,13.3 15,26.7 0,40" fill="#000"/>',
  KZ: bg('#00AFCA') + disc('#FEC50C', 6),
  LT: hBands('#FDB913', '#006A44', '#C1272D'),
  LU: hBands('#EF3340', '#fff', '#00A3E0'),
  LV: bg('#9E3039') + '<rect y="16" width="60" height="8" fill="#fff"/>',
  MA: bg('#C1272D') + star(30, 20, 7, '#006233'),
  MT: vBands('#fff', '#CF142B'),
  MX: vBands('#006847', '#fff', '#CE1126') + disc('#8C6239', 4),
  MY: myFlag(),
  NG: vBands('#008751', '#fff', '#008751'),
  NL: hBands('#AE1C28', '#fff', '#21468B'),
  NO: nordic('#BA0C2F', '#fff', '#00205B'),
  NZ: bg('#012169') + star(44, 22, 5, '#C8102E'),
  OM: hBands('#fff', '#DB161B', '#008000') + '<rect width="15" height="40" fill="#DB161B"/>',
  PH: hBands('#0038A8', '#CE1126') + '<polygon points="0,0 26,20 0,40" fill="#fff"/><circle cx="9" cy="20" r="4" fill="#FCD116"/>',
  PK: bg('#01411C') + '<rect width="15" height="40" fill="#fff"/><circle cx="38" cy="20" r="8" fill="#fff"/><circle cx="41" cy="18" r="7" fill="#01411C"/>' + star(45, 13, 3, '#fff'),
  PL: hBands('#fff', '#DC143C'),
  PT: '<rect width="24" height="40" fill="#006600"/><rect x="24" width="36" height="40" fill="#FF0000"/><circle cx="24" cy="20" r="6" fill="#FFFF00"/>',
  QA: bg('#8A1538') + '<rect width="18" height="40" fill="#fff"/>',
  RO: vBands('#002B7F', '#FCD116', '#CE1126'),
  RS: hBands('#C6363C', '#0C4076', '#fff'),
  RU: hBands('#fff', '#0039A6', '#D52B1E'),
  SA: bg('#006C35') + '<rect x="14" y="26" width="32" height="3" fill="#fff"/>',
  SE: nordic('#006AA7', '#FECC02'),
  SG: hBands('#EF3340', '#fff') + '<circle cx="12" cy="10" r="6" fill="#fff"/><circle cx="14.5" cy="10" r="5" fill="#EF3340"/>',
  SI: hBands('#fff', '#005DA4', '#ED1C24'),
  SK: hBands('#fff', '#0B4EA2', '#EE1C25'),
  TH: bg('#A51931') + '<rect y="6.67" width="60" height="26.66" fill="#F4F5F8"/><rect y="13.33" width="60" height="13.34" fill="#2D2A4A"/>',
  TR: bg('#E30A17') + '<circle cx="24" cy="20" r="8" fill="#fff"/><circle cx="26" cy="20" r="6.4" fill="#E30A17"/>' + star(35, 20, 3.5, '#fff'),
  TW: bg('#FE0000') + '<rect width="30" height="20" fill="#000095"/><circle cx="15" cy="10" r="5" fill="#fff"/>',
  UA: hBands('#0057B7', '#FFD700'),
  US: usFlag(),
  VN: bg('#DA251D') + star(30, 20, 8, '#FFFF00'),
  ZA: hBands('#E03C31', '#fff', '#001489') + '<polygon points="0,0 20,20 0,40" fill="#007749"/>',
}

const GLOBE = '<svg class="flag flag--globe" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.6"/><ellipse cx="12" cy="12" rx="4" ry="9" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M3.5 9h17M3.5 15h17" stroke="currentColor" stroke-width="1.2" fill="none"/></svg>'

// Returns a ready-to-insert <svg> string for a 2-letter country code, or a
// neutral globe when the code is unknown/absent (same fallback as mobile).
export function flagHtml(cc) {
  const key = typeof cc === 'string' ? cc.trim().toUpperCase() : ''
  const body = FLAGS[key]
  if (!body) return GLOBE
  return `<svg class="flag" viewBox="0 0 60 40" preserveAspectRatio="xMidYMid slice" aria-hidden="true">${body}</svg>`
}
