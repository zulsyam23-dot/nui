/**
 * Logika aplikasi Piano NodeUI.
 *
 * Semua pekerjaan berat (sintesis suara, pemetaan keyboard, posisi tombol
 * hitam, pedal sustain) berada di sini. Berkas .nui hanya berisi struktur,
 * state, dan event.
 *
 * Signature handler: (event, ctx)
 *   ctx.state  -> { get(), set(key, value) }
 *   ctx.refs   -> { piano, volume, ... } (node ber-id)
 */

// Urutan nada kromatik dalam satu oktaf.
const URUTAN = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

// Nada putih per oktaf (tombol putih pada keyboard).
const PUTIH = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];

// Pemetaan keyboard laptop -> urutan tombol.
const TOMBOL_PUTIH = { a: 0, s: 1, d: 2, f: 3, g: 4, h: 5, j: 6, k: 7, l: 8, ';': 9 };
const TOMBOL_HITAM = { w: 0, e: 1, t: 2, y: 3, u: 4, o: 5, p: 6 };

// Rentang oktaf yang boleh dipakai.
const OKTAV_MIN = 1;
const OKTAV_MAKS = 7;

// Nada yang sedang berbunyi: "C4" -> { voice, ditahan, kunci }.
// `voice` boleh null (Web Audio tidak tersedia) agar state tetap sinkron.
const aktif = new Map();
// Daftar tombol piano yang sudah dibaca (diisi pada hook `siap`).
let tombol = [];
let sustainAktif = false;

/* ------------------------------------------------------------------ */
/* Utilitas nada                                                       */
/* ------------------------------------------------------------------ */

/** Pecah "C#4" -> { nada: "C#", oktaf: 4 }. */
function pecahNada(nama) {
  const m = /^([A-G]#?)(\d)$/.exec(String(nama || ''));
  if (!m) return null;
  return { nada: m[1], oktav: Number(m[2]) };
}

/** Susun nama nada dari nada + oktaf. */
function susunNada(nada, oktav) {
  return `${nada}${oktav}`;
}

/** Geser nada sejumlah oktaf ("C#4" + 1 -> "C#5"). */
function geserNada(nama, delta) {
  const p = pecahNada(nama);
  if (!p) return nama;
  return susunNada(p.nada, p.oktav + delta);
}

/** Frekuensi Hz dari nama nada (A4 = 440 Hz). */
export function frekuensi(nama) {
  const p = pecahNada(nama);
  if (!p) return 0;
  const midi = (p.oktav + 1) * 12 + URUTAN.indexOf(p.nada);
  return 440 * Math.pow(2, (midi - 69) / 12);
}

/** Urutan nada kromatik dari nada awal sepanjang jumlah tertentu. */
function rentangNada(awal, jumlah) {
  const p = pecahNada(awal);
  if (!p) return [];
  const out = [];
  let index = URUTAN.indexOf(p.nada);
  let oktaf = p.oktav;
  for (let i = 0; i < jumlah; i += 1) {
    out.push(susunNada(URUTAN[index], oktaf));
    index += 1;
    if (index >= URUTAN.length) { index = 0; oktaf += 1; }
  }
  return out;
}

/* ------------------------------------------------------------------ */
/* Audio (Web Audio API)                                               */
/* ------------------------------------------------------------------ */

let audioCtx = null;
let masterGain = null;

/** Siapkan AudioContext + master gain (hanya sekali). */
function siapkanAudio(volume) {
  if (audioCtx) return audioCtx;
  const scope = typeof window !== 'undefined' ? window : null;
  const AudioCtx = scope && (scope.AudioContext || scope.webkitAudioContext);
  if (!AudioCtx) return null;
  try {
    audioCtx = new AudioCtx();
  } catch (_) {
    return null;
  }
  masterGain = audioCtx.createGain();
  masterGain.gain.value = Number(volume) / 100;
  masterGain.connect(audioCtx.destination);
  return audioCtx;
}

/** Mulai Mastercard nada: osilator + amplop (attack/decay). */
function bunyiMulai(nama, volume) {
  const ctx = siapkanAudio(volume);
  if (!ctx) return null;
  if (ctx.state === 'suspended' && ctx.resume) ctx.resume();

  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const sekarang = ctx.currentTime;

  osc.type = 'triangle';
  osc.frequency.value = frekuensi(nama);
  gain.gain.setValueAtTime(0.0001, sekarang);
  gain.gain.exponentialRampToValueAtTime(0.28, sekarang + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.16, sekarang + 0.25);

  osc.connect(gain);
  gain.connect(masterGain);
  osc.start(sekarang);
  return { osc, gain };
}

/** Hentikan nada dengan release halus. */
function bunyiStop(suaraSatu) {
  if (!suaraSatu || !audioCtx) return;
  const sekarang = audioCtx.currentTime;
  try {
    suaraSatu.gain.gain.cancelScheduledValues(sekarang);
    suaraSatu.gain.gain.setValueAtTime(Math.max(0.0001, suaraSatu.gain.gain.value), sekarang);
    suaraSatu.gain.gain.exponentialRampToValueAtTime(0.0001, sekarang + 0.18);
    suaraSatu.osc.stop(sekarang + 0.2);
  } catch (_) { /* sudah berhenti */ }
}

/* ------------------------------------------------------------------ */
/* State UI                                                            */
/* ------------------------------------------------------------------ */

function perbaruiDitekan(ctx) {
  const daftar = [...aktif.keys()].sort();
  ctx.state.set('ditekan', daftar.length ? daftar.join(', ') : '-');
}

/* ------------------------------------------------------------------ */
/* Hook siklus hidup                                                   */
/* ------------------------------------------------------------------ */

/** Dipanggil sekali setelah halaman terpasang. */
export function siap(event, ctx) {
  const root = ctx.refs.piano;
  if (!root || !root.querySelectorAll) return;

  // NodeList di browser tidak punya .map -> wajib Array.from.
  tombol = Array.from(root.querySelectorAll('.key')).map((el) => ({
    el,
    dasar: el.getAttribute('note'),
    hitam: String(el.getAttribute('class') || '').split(/\s+/).includes('black'),
  }));

  siapkanAudio(ctx.state.get().volume);
  terapkanOktav(ctx);
}

/* ------------------------------------------------------------------ */
/* Posisi tombol hitam                                                */
/* ------------------------------------------------------------------ */

/** Jumlah tombol putih sebelum nada hitam tertentu. */
function putihSebelum(namaHitam) {
  const putih = tombol.filter((t) => !t.hitam);
  if (!putih.length) return 0;
  const awal = putih[0].dasar;
  const total = rentangNada(awal, 48);
  const posisi = total.indexOf(namaHitam);
  if (posisi <= 0) return 0;
  let n = 0;
  for (let i = 0; i < posisi; i += 1) {
    if (!total[i].includes('#')) n += 1;
  }
  return n;
}

/** Posisikan tombol hitam di atas batas antara dua tombol putih. */
function posisiTombolHitam() {
  const totalPutih = tombol.filter((t) => !t.hitam).length;
  if (!totalPutih) return;
  for (const t of tombol) {
    if (!t.hitam) continue;
    const n = putihSebelum(t.dasar);
    // Batas setelah n tombol putih = n/total dari lebar baris tombol.
    const persen = (n / totalPutih) * 100;
    t.el.style.left = `calc(${persen.toFixed(4)}% - 23px)`;
  }
}

/** Terapkan oktaf saat ini: geser label nada + posisi tombol hitam. */
function terapkanOktav(ctx) {
  const oktav = Number(ctx.state.get().octav) || 4;
  const delta = oktav - 4;
  for (const t of tombol) {
    t.el.setAttribute('note', geserNada(t.dasar, delta));
  }
  posisiTombolHitam();
}

/* ------------------------------------------------------------------ */
/* Handler keyboard                                                    */
/* ------------------------------------------------------------------ */

/** Tombol yang sedang ditekan di keyboard laptop -> nada. */
function nadaDariTombol(kunci, ctx) {
  const oktav = Number(ctx.state.get().octav) || 4;
  const k = String(kunci || '').toLowerCase();

  if (Object.prototype.hasOwnProperty.call(TOMBOL_PUTIH, k)) {
    const index = TOMBOL_PUTIH[k];
    const putih = tombol.filter((t) => !t.hitam);
    if (index >= putih.length) return null;
    return putih[index].el.getAttribute('note');
  }
  if (Object.prototype.hasOwnProperty.call(TOMBOL_HITAM, k)) {
    const index = TOMBOL_HITAM[k];
    const hitam = tombol.filter((t) => t.hitam);
    if (index >= hitam.length) return null;
    return hitam[index].el.getAttribute('note');
  }
  return null;
}

/** Tekan tombol keyboard laptop. */
export function tekanTombol(event, ctx) {
  if (!event || event.repeat) return;
  const target = event.target;
  if (target && target.tagName === 'INPUT') return; // jangan saat menggeser slider

  const kunci = String(event.key || '').toLowerCase();

  if (kunci === ' ' || event.code === 'Space') {
    if (event.preventDefault) event.preventDefault(); // jangan scroll halaman
    setSustain(true, ctx);
    return;
  }
  if (kunci === 'z' || kunci === 'arrowleft') { turunOktav(event, ctx); return; }
  if (kunci === 'x' || kunci === 'arrowright') { naikOktav(event, ctx); return; }

  const nada = nadaDariTombol(kunci, ctx);
  if (!nada) return;
  mulaiNada(nada, ctx, kunci);
}

/** Lepas tombol keyboard laptop. */
export function lepasTombol(event, ctx) {
  if (!event) return;
  const kunci = String(event.key || '').toLowerCase();
  if (kunci === ' ' || event.code === 'Space') {
    setSustain(false, ctx);
    return;
  }
  const nada = nadaDariTombol(kunci, ctx);
  if (!nada) return;
  stopNada(nada, ctx);
}

/* ------------------------------------------------------------------ */
/* Handler mouse / sentuh                                              */
/* ------------------------------------------------------------------ */

/** Menekan tombol piano dengan mouse/sentuh. */
export function tekan(event, ctx) {
  const el = event && event.currentTarget;
  if (!el) return;
  if (event && event.preventDefault) event.preventDefault(); // jangan geser fokus
  const nada = el.getAttribute('note');
  if (!nada) return;
  mulaiNada(nada, ctx, null);
}

/** Melepas tombol piano. */
export function lepas(event, ctx) {
  const el = event && event.currentTarget;
  if (!el) return;
  const nada = el.getAttribute('note');
  if (!nada) return;
  stopNada(nada, ctx);
}

/* ------------------------------------------------------------------ */
/* Inti playback                                                       */
/* ------------------------------------------------------------------ */

/** Mulai nada: sembunyikan + bunyikan. */
function mulaiNada(nada, ctx, kunci) {
  if (!nada) return;
  if (aktif.has(nada)) return; // tidak ada suara ganda

  const volume = ctx.state.get().volume;
  const voice = bunyiMulai(nada, volume);
  aktif.set(nada, { voice, ditahan: false, kunci: kunci || null });

  const el = tombol.find((t) => t.el.getAttribute('note') === nada);
  if (el && el.el.classList) el.el.classList.add('tekan');

  ctx.state.set('nada', nada);
  perbaruiDitekan(ctx);
}

/** Hentikan nada (dengan atau tanpa sustain). */
function stopNada(nada, ctx) {
  const el = tombol.find((t) => t.el.getAttribute('note') === nada);
  if (sustainAktif) {
    // Pedal sustain ditekan: nada terus berbunyi sampai pedal dilepas.
    const s = aktif.get(nada);
    if (s) s.ditahan = true;
    if (el && el.el.classList) el.el.classList.remove('tekan');
    perbaruiDitekan(ctx);
    return;
  }

  const s = aktif.get(nada);
  if (s) {
    bunyiStop(s.voice);
    aktif.delete(nada);
  }
  if (el && el.el.classList) el.el.classList.remove('tekan');
  perbaruiDitekan(ctx);
}

/* ------------------------------------------------------------------ */
/* Kontrol                                                             */
/* ------------------------------------------------------------------ */

/** Nyalakan / matikan pedal sustain. */
export function setSustain(nilai, ctx) {
  const nyala = nilai === true || nilai === 'on' || nilai === 'nyala';
  if (nyala === sustainAktif) return;
  sustainAktif = nyala;
  ctx.state.set('sustain', nyala ? 'nyala' : 'mati');

  if (!nyala) {
    // Lepaskan semua nada yang ditahan pedal.
    for (const [nada, s] of [...aktif.entries()]) {
      if (!s.ditahan) continue;
      bunyiStop(s.voice);
      aktif.delete(nada);
      const el = tombol.find((t) => t.el.getAttribute('note') === nada);
      if (el && el.el.classList) el.el.classList.remove('tekan');
    }
    perbaruiDitekan(ctx);
  }
}

/** Toggle sustain lewat tombol. */
export function toggleSustain(event, ctx) {
  setSustain(!sustainAktif, ctx);
}

/** Naik satu oktaf. */
export function naikOktav(event, ctx) {
  const oktav = Number(ctx.state.get().octav) || 4;
  if (oktav >= OKTAV_MAKS) return;
  ctx.state.set('octav', String(oktav + 1));
  terapkanOktav(ctx);
}

/** Turun satu oktaf. */
export function turunOktav(event, ctx) {
  const oktav = Number(ctx.state.get().octav) || 4;
  if (oktav <= OKTAV_MIN) return;
  ctx.state.set('octav', String(oktav - 1));
  terapkanOktav(ctx);
}

/** Atur volume dari slider. */
export function aturVolume(event, ctx) {
  const slider = event && event.currentTarget;
  const nilai = slider ? Number(slider.value) : Number(ctx.state.get().volume) || 0;
  const clamped = Math.max(0, Math.min(100, nilai || 0));
  ctx.state.set('volume', String(clamped));
  if (masterGain && audioCtx) {
    masterGain.gain.setValueAtTime(clamped / 100, audioCtx.currentTime);
  }
}
