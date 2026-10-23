/**
 * Logika eksternal TypeScript untuk NodeUI.
 * Ditranspile otomatis oleh `nui build` (memakai typescript bila terpasang).
 */

type Ctx = {
  state: { get(): Record<string, string>; set(key: string, value: string): void };
  setState(key: string, value: string): void;
};

export async function muatUser(event: Event, { state }: Ctx): Promise<void> {
  // Logika berat (fetch/async) tetap di file TS, bukan di .nui.
  const daftar: string[] = ['Budi Santoso', 'Siti Aminah', 'Andi Wijaya'];
  const nama = daftar[Math.floor(Math.random() * daftar.length)];
  state.set('nama', nama);
}

export function tambahKlik(event: Event, { state }: Ctx): void {
  const n = Number(state.get().jumlah || '0') + 1;
  state.set('jumlah', String(n));
}
