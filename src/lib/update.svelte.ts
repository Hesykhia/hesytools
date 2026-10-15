// New versions from GitHub Releases: check, then download + install on request.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, errorText, onUpdateProgress, type UpdateInfo } from "$lib/api";
import { toast } from "$lib/toast.svelte";

export const update = $state({
  info: null as UpdateInfo | null,
  /** idle → available → downloading → installing (the app then restarts). */
  phase: "idle" as "idle" | "available" | "downloading" | "installing",
  downloaded: 0,
  total: null as number | null,
  /** "Later" hides the card until a newer version than this one shows up. */
  dismissed: false,
  checking: false,
});

/** `manual` = the user pressed "Check for updates", so always say something. */
export async function checkForUpdates(manual = false) {
  if (update.checking || update.phase === "downloading" || update.phase === "installing") return;
  update.checking = true;
  try {
    const info = await api.checkUpdate();
    const isNew = info?.version !== update.info?.version;
    update.info = info;
    update.phase = info ? "available" : "idle";
    // Checking again in the background shouldn't bring back a card they
    // already said "Later" to, unless it's for an even newer version.
    if (info && (isNew || manual)) update.dismissed = false;
    else if (manual) toast("You're on the latest version");
  } catch (e) {
    // Offline or GitHub unreachable: only worth mentioning if they asked.
    if (manual) toast(errorText(e), "error");
  } finally {
    update.checking = false;
  }
}

export async function installUpdate() {
  update.phase = "downloading";
  update.downloaded = 0;
  update.total = null;
  const unlisten = await onUpdateProgress((p) => {
    if (p.done) update.phase = "installing";
    else {
      update.downloaded = p.downloaded;
      update.total = p.total;
    }
  });
  try {
    await api.installUpdate();
    // On Windows the installer closes hesytools before we get here.
    update.phase = "installing";
  } catch (e) {
    toast(errorText(e), "error", 7000);
    update.phase = "available";
  } finally {
    unlisten();
  }
}

const RECHECK_EVERY = 30 * 60 * 1000;
/** Coming back to the window rechecks, at most this often. */
const RECHECK_ON_FOCUS = 2 * 60 * 1000;

/**
 * Check shortly after launch, then every few hours and when the window comes
 * back into view. hesytools mostly lives in the tray, so a launch-only check
 * could miss updates for days.
 */
export function startAutoCheck(enabled: () => boolean): () => void {
  let last = 0;
  const check = () => {
    if (!enabled()) return;
    last = Date.now();
    checkForUpdates();
  };
  const onFocus = () => {
    if (Date.now() - last > RECHECK_ON_FOCUS) check();
  };
  // A quiet check a few seconds after launch, so startup stays snappy.
  const first = setTimeout(check, 4000);
  const timer = setInterval(check, RECHECK_EVERY);
  // The window's own focus event: opening it from the tray or alt-tabbing
  // back doesn't always reach the page as a DOM focus.
  let unlisten: (() => void) | undefined;
  let stopped = false;
  try {
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => focused && onFocus())
      .then((u) => (stopped ? u() : (unlisten = u)))
      .catch(() => {});
  } catch {
    // Browser preview: no Tauri window.
  }
  addEventListener("focus", onFocus);
  return () => {
    stopped = true;
    clearTimeout(first);
    clearInterval(timer);
    unlisten?.();
    removeEventListener("focus", onFocus);
  };
}
