/**
 * Logika eksternal untuk contoh NodeUI (JS biasa, diimpor lewat `use`).
 * Semua logika berat (fetch, loop, kalkulasi) hidup di sini, bukan di .nui.
 */

export function muatUser(event, { state }) {
  // Simulasi pemuatan data pengguna.
  state.set('nama', 'Budi Santoso');
  state.set('jumlah', String(Number(state.get().jumlah || 0) + 1));
}

export function tampilkanModal(event, { overlay }) {
  // Overlay berada di layer terpisah (bukan child dari node induknya).
  if (!overlay) return;
  overlay.style.display = overlay.style.display === 'flex' ? 'none' : 'flex';
}
