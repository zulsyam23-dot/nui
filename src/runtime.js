/**
 * Runtime NodeUI (disertakan ke hasil build).
 *
 * - Reaktivitas sederhana: state + subscribe.
 * - Isolasi DOM: setiap page dirender di dalam Shadow Root sehingga DOM/CSS
 *   internal tidak bocor keluar dan tidak bisa menimpa node luar.
 * - Overlay: pengecualian resmi, dirender pada layer terpisah (root sendiri).
 */

// State reaktif sederhana dengan subscriber.
export function createState(initial = {}) {
  const data = { ...initial };
  const subs = new Map();
  let seq = 0;
  return {
    get() { return data; },
    set(key, value) {
      if (Object.is(data[key], value)) return;
      data[key] = value;
      for (const cb of subs.values()) cb(data, key);
    },
    subscribe(cb) {
      const id = `__sub${seq++}`;
      subs.set(id, cb);
      return id;
    },
    unsubscribe(id) { subs.delete(id); },
  };
}

// Atribut yang isinya URL: hanya skema aman yang boleh dipasang.
const URL_ATTRS = new Set(['href', 'src', 'poster', 'action', 'formaction', 'data-src', 'xlink:href']);
const DANGEROUS_URL = /^\s*(?:javascript|vbscript|data\s*:\s*text\/html)/i;
const BLOCKED_ATTRS = new Set(['srcdoc']);

function safeUrl(name, value) {
  if (!URL_ATTRS.has(name)) return value;
  return DANGEROUS_URL.test(value) ? '#' : value;
}

// Pembuat elemen sederhana.
export function h(tag, props = {}, children = []) {
  if (tag === '#text') {
    return document.createTextNode(String(props.value ?? ''));
  }
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v === null || v === undefined) continue;
    const name = String(k);
    const lower = name.toLowerCase();
    if (lower === 'style' && typeof v === 'object') {
      Object.assign(el.style, v);
    } else if (lower.startsWith('on')) {
      if (typeof v === 'function') el.addEventListener(lower.slice(2), v);
    } else if (!BLOCKED_ATTRS.has(lower)) {
      el.setAttribute(name, safeUrl(lower, String(v)));
    }
  }
  for (const c of children || []) {
    if (c === null || c === undefined) continue;
    el.appendChild(typeof c === 'string' ? document.createTextNode(c) : c);
  }
  return el;
}

// CSS dasar per page (terisolasi di shadow root, tidak bocor keluar).
export const BASE_CSS = [
  ':host { display: block; }',
  '*, *::before, *::after { box-sizing: border-box; }',
  'body { margin: 0; }',
].join('\n');

// Set atribut dan sinkronkan properti untuk kontrol form
// (atribut "value" saja tidak mengubah nilai yang sedang tampil).
export function setAttr(el, name, value) {
  const attr = String(name);
  const lower = attr.toLowerCase();
  if (lower.startsWith('on') || BLOCKED_ATTRS.has(lower)) return el;

  const text = safeUrl(lower, String(value));
  el.setAttribute(attr, text);
  if (lower === 'value' && 'value' in el) el.value = text;
  if (lower === 'checked' && 'checked' in el) el.checked = value !== '' && value !== 'false';
  return el;
}

// Root halaman: host + Shadow Root + slot style.
export function createRoot() {
  const host = document.createElement('div');
  host.setAttribute('data-nui-root', '');
  const shadow = host.attachShadow({ mode: 'open' });
  const base = document.createElement('style');
  base.textContent = BASE_CSS;
  const style = document.createElement('style');
  const body = document.createElement('div');
  body.setAttribute('data-nui-body', '');
  shadow.append(base, style, body);
  return { host, shadow, body, style };
}

// Layer terpisah untuk node Overlay (modal/tooltip/dropdown).
// Overlay TIDAK boleh menjadi child dari node induknya: ia hidup di root
// dokumen sendiri, sehingga DOM & CSS-nya tidak bocor ke halaman.
export function createOverlayLayer(name = 'overlay') {
  // Idempoten: layer dengan nama sama dipakai ulang, tidak digandakan.
  // Dicocokkan lewat atribut (bukan selector) agar nama tidak bisa menjadi CSS injection.
  const nodes = document.body.querySelectorAll('[data-nui-overlay]');
  let existing = null;
  for (let index = 0; index < nodes.length; index += 1) {
    const node = nodes.item ? nodes.item(index) : nodes[index];
    if (node && node.getAttribute('data-nui-overlay') === name) {
      existing = node;
      break;
    }
  }
  if (existing && existing.shadowRoot) {
    return { host: existing, shadow: existing.shadowRoot, body: existing.shadowRoot, style: existing.shadowRoot.querySelector('style') };
  }

  const host = document.createElement('div');
  host.setAttribute('data-nui-overlay', name);
  Object.assign(host.style, {
    position: 'fixed', inset: '0', display: 'none', padding: '24px',
    alignItems: 'center', justifyContent: 'center', zIndex: '9999',
    overflow: 'auto', background: 'rgba(3, 5, 10, 0.62)',
  });
  const shadow = host.attachShadow({ mode: 'open' });
  const base = document.createElement('style');
  base.textContent = BASE_CSS;
  const style = document.createElement('style');
  shadow.append(base, style);
  document.body.appendChild(host);
  return { host, shadow, body: shadow, style };
}

// Pasang host ke dokumen.
export function mount(host, target = document.body) {
  target.appendChild(host);
  return host;
}
