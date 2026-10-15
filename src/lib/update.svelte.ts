// New versions from GitHub Releases: check, then download + install on request.

import { api, errorText, onUpdateProgress, type UpdateInfo } from "$lib/api";
import { toast } from "$lib/toast.svelte";

export const update = $state({
  info: null as UpdateInfo | null,
  /** idle → available → downloading → installing (the app then restarts). */
  phase: "idle" as "idle" | "available" | "downloading" | "installing",
  downloaded: 0,
  total: null as number | null,
  /** "Later" hides the card until the next launch. */
  dismissed: false,
  checking: false,
});

/** `manual` = the user pressed "Check for updates", so always say something. */
export async function checkForUpdates(manual = false) {
  if (update.checking || update.phase === "downloading" || update.phase === "installing") return;
  update.checking = true;
  try {
    const info = await api.checkUpdate();
    update.info = info;
    update.phase = info ? "available" : "idle";
    if (info) update.dismissed = false;
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
