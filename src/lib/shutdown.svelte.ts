// The shutdown/restart timer, kept in sync with the backend. Ticks once a
// second only while a timer is set.

import { api, errorText, onShutdownState, type ShutdownTimer } from "$lib/api";
import { toast } from "$lib/toast.svelte";

export const shutdown = $state({
  timer: null as ShutdownTimer | null,
  now: Date.now(),
});

let ticker: ReturnType<typeof setInterval> | undefined;

function set(t: ShutdownTimer | null) {
  shutdown.timer = t;
  shutdown.now = Date.now();
  clearInterval(ticker);
  if (t) ticker = setInterval(() => (shutdown.now = Date.now()), 1000);
}

export async function initShutdown(): Promise<() => void> {
  set(await api.shutdownStatus());
  const un = await onShutdownState(set);
  return () => {
    un();
    clearInterval(ticker);
  };
}

export async function startShutdown(minutes: number, restart: boolean) {
  try {
    set(await api.scheduleShutdown(minutes, restart));
  } catch (e) {
    toast(errorText(e), "error");
  }
}

export async function cancelShutdown() {
  await api.cancelShutdown();
  set(null);
  toast("Shutdown timer cancelled");
}

/** "4:59" or "1:02:30" until the timer goes off. */
export function remaining(t: ShutdownTimer, now: number): string {
  const s = Math.max(0, Math.round((t.endsAt - now) / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}
