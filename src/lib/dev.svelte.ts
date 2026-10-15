// Whether this install has a valid Dev access code (shows the Dev tab).

import { api, onSoulReport, type DevStatus, type SoulReport } from "$lib/api";

export const dev = $state({
  unlocked: false,
  name: null as string | null,
  /** The Soul Clicker helper's latest report. */
  report: null as SoulReport | null,
});

export function setDev(s: DevStatus) {
  dev.unlocked = s.unlocked;
  dev.name = s.name;
}

export async function initDev(): Promise<() => void> {
  setDev(await api.devStatus());
  dev.report = await api.soulReport();
  return onSoulReport((r) => (dev.report = r));
}
