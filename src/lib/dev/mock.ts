// Browser preview mode: when the UI runs in a normal browser during
// development (no Tauri backend), fake the backend with fixture data so
// every screen can be designed and checked without sending real input.
// Never loaded in the app itself.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type { Macro, Settings } from "$lib/api";

const settings: Settings = {
  clicker: {
    action: { kind: "mouse", button: "LeftMouse" },
    intervalMs: 100,
    holdMs: 10,
    jitterMs: 0,
    position: { kind: "current" },
    stopAfter: { kind: "never" },
  },
  clickerHotkey: "F6",
  clickerMode: "toggle",
  panicHotkey: "F8",
  recordHotkey: "F9",
  record: { recordMoves: false, minWaitMs: 1 },
  webhook: { url: "", mentionUserId: "", notifyClickerFinished: true, notifyMacroFinished: true },
};

let macros: Macro[] = [
  {
    id: "m1",
    name: "Auto rebirth",
    speed: 1,
    repeat: 0,
    hotkey: "F7",
    updatedAt: 3,
    steps: [
      { type: "key", key: "L", action: "tap", holdMs: 30 },
      { type: "wait", ms: 800 },
      { type: "click", button: "LeftMouse", action: "tap", holdMs: 20, at: [1249, 464] },
      { type: "wait", ms: 200 },
      { type: "key", key: "M", action: "tap", holdMs: 30 },
      { type: "wait", ms: 400 },
      { type: "click", button: "LeftMouse", action: "tap", holdMs: 20, at: [940, 494] },
    ],
  },
  {
    id: "m2",
    name: "Every step type",
    speed: 1,
    repeat: 3,
    hotkey: null,
    updatedAt: 2,
    steps: [
      { type: "key", key: "E", action: "tap", holdMs: 30 },
      { type: "wait", ms: 250 },
      { type: "key", key: "W", action: "down", holdMs: 0 },
      { type: "wait", ms: 1200 },
      { type: "click", button: "LeftMouse", action: "tap", holdMs: 20, at: [940, 494] },
      { type: "wait", ms: 80 },
      { type: "key", key: "W", action: "up", holdMs: 0 },
      { type: "scroll", amount: -360, horizontal: false },
      { type: "text", text: "gg wp" },
      {
        type: "repeat",
        times: 5,
        steps: [
          { type: "click", button: "RightMouse", action: "tap", holdMs: 30, at: null },
          { type: "wait", ms: 100 },
        ],
      },
      { type: "path", points: [{ t: 0, x: 10, y: 10 }, { t: 400, x: 300, y: 200 }] },
    ],
  },
  {
    id: "m3",
    name: "Fishing loop",
    speed: 1.5,
    repeat: 10,
    hotkey: "Ctrl+F",
    updatedAt: 1,
    steps: [
      { type: "click", button: "RightMouse", action: "tap", holdMs: 30, at: null },
      { type: "wait", ms: 2500 },
      { type: "click", button: "RightMouse", action: "tap", holdMs: 30, at: null },
    ],
  },
];

let clicker = { running: false, clicks: 0, blocked: 0, started: 0, intervalMs: 100 };
let playing: string | null = null;
let recording = false;

function clickerStatus() {
  if (clicker.running) clicker.clicks = Math.floor((Date.now() - clicker.started) / clicker.intervalMs);
  return { running: clicker.running, clicks: clicker.clicks, blocked: 0 };
}

/**
 * `?scene=` jumps straight to a state, for screenshots:
 * editor:<id>, playing:<id> (editor with live playback), recording.
 */
async function applyScene() {
  const scene = new URLSearchParams(location.search).get("scene");
  if (!scene) return;
  const [kind, id] = scene.split(":");
  const { activity } = await import("$lib/activity.svelte");
  await new Promise((r) => setTimeout(r, 400)); // let the page load its data
  if (kind === "editor" || kind === "playing") activity.openId = id;
  if (kind === "playing") {
    playing = id;
    await emit("macro-state", id);
    simulatePlayback(id);
  }
  if (kind === "recording") {
    recording = true;
    await emit("recording-state", true);
  }
}

export function install() {
  setTimeout(applyScene);
  mockWindows("main");
  mockIPC(
    async (cmd, args) => {
      const a = (args ?? {}) as Record<string, any>;
      switch (cmd) {
        case "get_settings":
          return structuredClone(settings);
        case "system_info":
          return { elevated: false, version: "0.1.0 (browser preview)" };
        case "set_clicker_config":
          settings.clicker = a.config;
          clicker.intervalMs = a.config.intervalMs;
          return;
        case "set_clicker_mode":
          settings.clickerMode = a.mode;
          return;
        case "set_record_options":
          settings.record = a.options;
          return;
        case "set_hotkey":
          if (a.id === "clicker") settings.clickerHotkey = a.hotkey;
          if (a.id === "panic") settings.panicHotkey = a.hotkey;
          if (a.id === "record") settings.recordHotkey = a.hotkey;
          return structuredClone(settings);
        case "capture_hotkey":
          await new Promise((r) => setTimeout(r, 900));
          return "F10";
        case "pick_point":
          await new Promise((r) => setTimeout(r, 900));
          return [1280, 720];
        case "clicker_toggle":
          clicker.running = !clicker.running;
          if (clicker.running) clicker.started = Date.now();
          await emit("clicker-state", clicker.running);
          return clicker.running;
        case "clicker_status":
          return clickerStatus();
        case "list_macros":
          return structuredClone(macros);
        case "save_macro": {
          const m: Macro = { ...a.entry, id: a.entry.id || `m${Date.now()}`, updatedAt: Date.now() };
          macros = [m, ...macros.filter((x) => x.id !== m.id)];
          return structuredClone(m);
        }
        case "delete_macro":
          macros = macros.filter((x) => x.id !== a.id);
          return;
        case "play_macro":
          playing = a.id;
          await emit("macro-state", playing);
          simulatePlayback(a.id);
          return;
        case "stop_macro":
          playing = null;
          await emit("macro-state", null);
          return;
        case "activity_status":
          return { playing, recording };
        case "recording_start":
          recording = true;
          await emit("recording-state", true);
          return;
        case "recording_stop":
          recording = false;
          await emit("recording-state", false);
          await emit("recording-finished", [
            { type: "key", key: "E", action: "tap", holdMs: 40 },
            { type: "wait", ms: 520 },
            { type: "click", button: "LeftMouse", action: "tap", holdMs: 25, at: [812, 433] },
          ]);
          return;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}

/** Walk the step highlight through a macro so the editor can be previewed live. */
async function simulatePlayback(id: string) {
  const m = macros.find((x) => x.id === id);
  if (!m) return;
  for (let run = 0; playing === id && (m.repeat === 0 || run < m.repeat); run++) {
    for (let i = 0; i < m.steps.length && playing === id; i++) {
      await emit("macro-step", { id, index: i, iteration: run });
      const s = m.steps[i];
      await new Promise((r) => setTimeout(r, s.type === "wait" ? Math.min(s.ms, 900) : 350));
    }
  }
  if (playing === id) {
    playing = null;
    await emit("macro-state", null);
  }
}
