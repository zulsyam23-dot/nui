/**
 * Stub DOM minimal untuk menguji hasil generator NodeUI tanpa browser.
 * Cukup untuk: elemen, teks, atribut, event, shadow root.
 */

class StubNode {
  constructor(tag) {
    this.tagName = tag;
    this.childNodes = [];
    this.attributes = {};
    this.listeners = {};
    this.parentNode = null;
    this.style = {};
    this.shadowRoot = null;
    this._text = '';

    // Properti elemen media (video/audio).
    if (tag === 'video' || tag === 'audio') {
      this.currentTime = 0;
      this.duration = NaN;
      this.volume = 1;
      this.paused = true;
      this.load = () => { this.loaded = true; };
      this.play = () => { this.paused = false; return Promise.resolve(); };
      this.pause = () => { this.paused = true; };
    }
  }

  appendChild(child) {
    child.parentNode = this;
    this.childNodes.push(child);
    return child;
  }

  append(...nodes) {
    for (const n of nodes) this.appendChild(n);
  }

  setAttribute(key, value) { this.attributes[key] = String(value); }
  getAttribute(key) { return Object.prototype.hasOwnProperty.call(this.attributes, key) ? this.attributes[key] : null; }
  removeAttribute(key) { delete this.attributes[key]; }
  hasAttribute(key) { return Object.prototype.hasOwnProperty.call(this.attributes, key); }

  // Daftar class sederhana (add/remove/contains/toggle).
  get classList() {
    const node = this;
    const read = () => String(node.getAttribute('class') || '').split(/\s+/).filter(Boolean);
    const write = (list) => node.setAttribute('class', list.join(' '));
    return {
      contains: (name) => read().includes(name),
      add: (...names) => { const list = read(); for (const n of names) if (!list.includes(n)) list.push(n); write(list); },
      remove: (...names) => write(read().filter((n) => !names.includes(n))),
      toggle: (name) => { const list = read(); if (list.includes(name)) { write(list.filter((n) => n !== name)); return false; } list.push(name); write(list); return true; },
      get value() { return read().join(' '); },
    };
  }

  addEventListener(type, fn) {
    (this.listeners[type] = this.listeners[type] || []).push(fn);
  }

  attachShadow() {
    const shadow = new StubNode('#shadow-root');
    shadow.host = this;
    this.shadowRoot = shadow;
    return shadow;
  }

  set textContent(value) { this._text = String(value); this.childNodes = []; }
  get textContent() {
    if (this.tagName === '#text') return this._text;
    return this.childNodes.map((c) => c.textContent).join('');
  }

  // Trigger event (untuk simulasi klik).
  dispatch(type, event = {}) {
    for (const fn of this.listeners[type] || []) fn(event);
  }

  // Cari elemen pertama yang cocok (DFS, termasuk shadow root).
  find(predicate) {
    for (const child of this.childNodes) {
      if (predicate(child)) return child;
      const found = child.find && child.find(predicate);
      if (found) return found;
    }
    return null;
  }

  // Selector sederhana: nama tag ("source"), kelas (".key"), atau atribut ("[note]").
  matches(selector) {
    const sel = String(selector).trim();
    if (sel.startsWith('.')) {
      const cls = sel.slice(1);
      const list = (this.getAttribute('class') || '').split(/\s+/);
      return list.includes(cls);
    }
    if (sel.startsWith('[') && sel.endsWith(']')) {
      const name = sel.slice(1, -1).split('=')[0];
      return this.hasAttribute(name);
    }
    return this.tagName.toLowerCase() === sel.toLowerCase();
  }

  querySelector(selector) {
    return this.find((n) => n.matches(selector));
  }

  // Mengembalikan NodeList sungguhan: punya length/item/forEach, TIDAK punya map/filter.
  querySelectorAll(selector) {
    const list = this.findAll((n) => n.matches(selector));
    const nodelist = { length: list.length, item: (i) => list[i] || null };
    list.forEach((n, i) => { nodelist[i] = n; });
    nodelist.forEach = (fn, thisArg) => list.forEach(fn, thisArg);
    return nodelist;
  }

  // Semua node yang cocok.
  findAll(predicate, out = []) {
    for (const child of this.childNodes) {
      if (predicate(child)) out.push(child);
      if (child.findAll) child.findAll(predicate, out);
    }
    return out;
  }
}

// Stub window: mendukung addEventListener + dispatch untuk simulasi keyboard.
function createWindow() {
  const win = new StubNode('window');
  win.listeners = {};
  return win;
}

function createDocument() {
  const body = new StubNode('body');
  return {
    body,
    createElement: (tag) => new StubNode(tag),
    createTextNode: (text) => {
      const n = new StubNode('#text');
      n.textContent = text;
      return n;
    },
    addEventListener() {},
  };
}

module.exports = { StubNode, createDocument, createWindow };
