// App-wide live state: the macro library, what's playing, and recording.
// Backend events keep it current; components just read it.

import {
  api,
  errorText,
  onMacroState,
  onMacroStep,
  onRecordingFinished,
  onRecordingState,
  type Macro,
  type Step,
} from "$lib/api";
import { toast } from "$lib/toast.svelte";

export const activity = $state({
  macros: [] as Macro[],
  playingId: null as string | null,
  /** Top-level step index currently playing, and which run it's on. */
  step: -1,
  iteration: 0,
  recording: false,
  recordingSince: 0,
  /** Macro the editor should open (set after a recording finishes). */
  openId: null as string | null,
});

export async function initActivity(): Promise<() => void> {
  const [macros, status] = await Promise.all([api.listMacros(), api.activityStatus()]);
  activity.macros = macros;
  activity.playingId = status.playing;
  activity.recording = status.recording;
  if (status.recording) activity.recordingSince = Date.now();

  const unlisteners = await Promise.all([
    onMacroState((id) => {
      activity.playingId = id;
      activity.step = -1;
      activity.iteration = 0;
    }),
    onMacroStep((e) => {
      if (e.id !== activity.playingId) return;
      activity.step = e.index;
      activity.iteration = e.iteration;
    }),
    onRecordingState((r) => {
      activity.recording = r;
      if (r) activity.recordingSince = Date.now();
    }),
    onRecordingFinished(saveRecording),
  ]);
  return () => unlisteners.forEach((u) => u());
}

async function saveRecording(steps: Step[]) {
  if (steps.length === 0) {
    toast("Nothing was recorded. Actions inside hesytools itself are skipped.");
    return;
  }
  const when = new Date().toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  try {
    const saved = await api.saveMacro({
      id: "",
      name: `Recording ${when}`,
      steps,
      speed: 1,
      repeat: 1,
      hotkey: null,
      updatedAt: 0,
    });
    upsertMacro(saved);
    activity.openId = saved.id;
    toast(`Recorded ${steps.length} steps`);
  } catch (e) {
    toast(errorText(e), "error");
  }
}

export function upsertMacro(m: Macro) {
  const i = activity.macros.findIndex((x) => x.id === m.id);
  if (i >= 0) activity.macros[i] = m;
  else activity.macros.unshift(m);
}

export function removeMacro(id: string) {
  activity.macros = activity.macros.filter((m) => m.id !== id);
}
