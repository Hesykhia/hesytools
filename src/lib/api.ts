// Typed wrappers around the Rust commands in src-tauri/src/lib.rs.
// Shapes mirror the serde types in crates/engine and src-tauri/src/settings.rs.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type MouseButton = "LeftMouse" | "RightMouse" | "MiddleMouse" | "Mouse4" | "Mouse5";

export const MOUSE_BUTTONS: { value: MouseButton; label: string }[] = [
  { value: "LeftMouse", label: "Left" },
  { value: "RightMouse", label: "Right" },
  { value: "MiddleMouse", label: "Middle" },
  { value: "Mouse4", label: "Mouse 4 (back)" },
  { value: "Mouse5", label: "Mouse 5 (forward)" },
];

export type ClickAction =
  | { kind: "mouse"; button: MouseButton }
  | { kind: "key"; key: string }
  /** Click and press a key together every tick. */
  | { kind: "both"; button: MouseButton; key: string };
export type ClickPosition = { kind: "current" } | { kind: "fixed"; x: number; y: number };
export type StopAfter =
  | { kind: "never" }
  | { kind: "clicks"; count: number }
  | { kind: "seconds"; secs: number };

export interface ClickerConfig {
  action: ClickAction;
  intervalMs: number;
  holdMs: number;
  jitterMs: number;
  position: ClickPosition;
  stopAfter: StopAfter;
  /** Double-click each time. */
  double: boolean;
  /** Allow intervals below MIN_INTERVAL_MS. */
  allowFast?: boolean;
}

/** Faster than this, most apps queue clicks up (see clicker.rs). */
export const MIN_INTERVAL_MS = 8;

export type HotkeyMode = "toggle" | "hold";

export interface WebhookSettings {
  url: string;
  mentionUserId: string;
  notifyClickerFinished: boolean;
  notifyMacroFinished: boolean;
}

export interface RecordOptions {
  recordMoves: boolean;
  minWaitMs: number;
}

export interface Region {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Cmp = ">=" | "<=" | "==";

/** A small picture to find on screen: base64 RGBA pixels. */
export interface Template {
  w: number;
  h: number;
  data: string;
}

export type Condition =
  /** digitsAfter: null = plain contains, 0 = exactly, N = exactly N more digits (e25 +1 = e250..e259). */
  | { kind: "textAppears"; text: string; digitsAfter?: number | null }
  | { kind: "textGone"; text: string; digitsAfter?: number | null }
  | { kind: "number"; op: Cmp; value: number }
  | { kind: "colorAppears"; rgb: [number, number, number]; tolerance: number }
  | { kind: "colorAt"; dx: number; dy: number; rgb: [number, number, number]; tolerance: number }
  | { kind: "imageAppears"; template: Template; threshold: number }
  | { kind: "changes" };

export interface Check {
  matched: boolean;
  text: string | null;
  number: number | null;
  numbers: number[];
  found: { x: number; y: number; score: number } | null;
  ocrRan: boolean;
  captureMs: number;
  ocrMs: number;
}

export interface ViewerCheck extends Check {
  width: number;
  height: number;
}

export type WatchAction =
  | { kind: "notify" }
  | { kind: "runMacro"; macroId: string }
  | { kind: "stopEverything" }
  | { kind: "click"; button: MouseButton; times: number; at: { kind: "found" } | { kind: "point"; x: number; y: number } }
  | { kind: "pressKey"; key: string; times: number }
  /** Pauses playing macros while it matches; they carry on once it's gone. */
  | { kind: "pauseMacros" }
  /** Shuts down / restarts after a 1 minute countdown. */
  | { kind: "shutdown"; restart: boolean };

export interface WatchRule {
  id: string;
  name: string;
  enabled: boolean;
  region: Region;
  condition: Condition;
  pollMs: number;
  action: WatchAction;
  discord: boolean;
  cooldownMs: number;
}

export interface WatchStatus {
  id: string;
  matched: boolean;
  text: string | null;
  error: string | null;
}

export type OnTimeout = "continue" | "stop";

export type PressAction = "tap" | "down" | "up";
export interface PathPoint {
  t: number;
  x: number;
  y: number;
}

export type Step =
  | { type: "key"; key: string; action: PressAction; holdMs: number }
  | { type: "click"; button: MouseButton; action: PressAction; holdMs: number; at: [number, number] | null }
  | { type: "move"; x: number; y: number }
  | { type: "path"; points: PathPoint[] }
  | { type: "scroll"; amount: number; horizontal: boolean }
  | { type: "wait"; ms: number }
  | { type: "text"; text: string }
  | { type: "repeat"; times: number; steps: Step[] }
  | {
      type: "waitFor";
      region: Region;
      condition: Condition;
      timeoutMs: number;
      pollMs: number;
      onTimeout: OnTimeout;
    };

export type StepType = Step["type"];

export interface Macro {
  id: string;
  name: string;
  steps: Step[];
  speed: number;
  /** 0 = until stopped */
  repeat: number;
  hotkey: string | null;
  updatedAt: number;
}

export interface ActivityStatus {
  playing: string | null;
  recording: boolean;
}

export interface MacroStepEvent {
  id: string;
  index: number;
  iteration: number;
}

export interface Settings {
  clicker: ClickerConfig;
  clickerHotkey: string | null;
  clickerMode: HotkeyMode;
  panicHotkey: string | null;
  recordHotkey: string | null;
  record: RecordOptions;
  /** Red click-through outline around every monitor while recording. */
  recordingOverlay: boolean;
  startupMacro: string | null;
  closeToTray: boolean;
  autoCheckUpdates: boolean;
  webhook: WebhookSettings;
  watches: WatchRule[];
}

export interface ClickerStatus {
  running: boolean;
  clicks: number;
  blocked: number;
}

export interface SystemInfo {
  elevated: boolean;
  version: string;
}

export interface ShutdownTimer {
  /** Unix ms when the PC shuts down / restarts. */
  endsAt: number;
  restart: boolean;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  done: boolean;
}

export type HotkeyId = "clicker" | "panic" | "record";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setClickerConfig: (config: ClickerConfig) => invoke<void>("set_clicker_config", { config }),
  setClickerMode: (mode: HotkeyMode) => invoke<void>("set_clicker_mode", { mode }),
  setHotkey: (id: HotkeyId, hotkey: string | null) => invoke<Settings>("set_hotkey", { id, hotkey }),
  captureHotkey: () => invoke<string | null>("capture_hotkey"),
  cancelCapture: () => invoke<void>("cancel_capture"),
  windowInput: (input: { vk?: number; button?: number; down: boolean }) => invoke<void>("window_input", input),
  pickPoint: () => invoke<[number, number] | null>("pick_point"),
  cancelPick: () => invoke<void>("cancel_pick"),
  clickerToggle: () => invoke<boolean>("clicker_toggle"),
  clickerStatus: () => invoke<ClickerStatus>("clicker_status"),
  startupEnabled: () => invoke<boolean>("startup_enabled"),
  setStartupEnabled: (on: boolean) => invoke<void>("set_startup_enabled", { on }),
  setStartupMacro: (id: string | null) => invoke<void>("set_startup_macro", { id }),
  /** Eyedropper: next click on screen returns that pixel's colour. */
  pickColor: () => invoke<[number, number, number] | null>("pick_color"),
  scheduleShutdown: (minutes: number, restart: boolean) => invoke<ShutdownTimer>("schedule_shutdown", { minutes, restart }),
  cancelShutdown: () => invoke<void>("cancel_shutdown"),
  shutdownStatus: () => invoke<ShutdownTimer | null>("shutdown_status"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  setAutoCheckUpdates: (on: boolean) => invoke<void>("set_auto_check_updates", { on }),
  setCloseToTray: (on: boolean) => invoke<void>("set_close_to_tray", { on }),
  quitApp: () => invoke<void>("quit_app"),
  setRecordingOverlay: (on: boolean) => invoke<void>("set_recording_overlay", { on }),
  screenLayout: () => invoke<Region[]>("screen_layout"),
  setRecordOptions: (options: RecordOptions) => invoke<void>("set_record_options", { options }),
  listMacros: () => invoke<Macro[]>("list_macros"),
  saveMacro: (entry: Macro) => invoke<Macro>("save_macro", { entry }),
  deleteMacro: (id: string) => invoke<void>("delete_macro", { id }),
  /** Old hesytools v4 macro files as [file name, JSON text] pairs. */
  importMacros: (files: [string, string][]) => invoke<Macro[]>("import_macros", { files }),
  playMacro: (id: string) => invoke<void>("play_macro", { id }),
  stopMacro: () => invoke<void>("stop_macro"),
  activityStatus: () => invoke<ActivityStatus>("activity_status"),
  recordingStart: () => invoke<void>("recording_start"),
  recordingStop: () => invoke<void>("recording_stop"),
  pickRegion: () => invoke<Region | null>("pick_region"),
  captureTemplate: () => invoke<Template | null>("capture_template"),
  /** `id` identifies one live viewer; several can run at once. */
  viewerCheck: (id: string, region: Region, condition: Condition) =>
    invoke<ViewerCheck>("viewer_check", { id, region, condition }),
  /** Raw bytes: u32 width, u32 height (little-endian), then RGBA pixels. */
  viewerFrame: (id: string, maxWidth: number) => invoke<ArrayBuffer>("viewer_frame", { id, maxWidth }),
  viewerStop: (id: string) => invoke<void>("viewer_stop", { id }),
  saveWatches: (watches: WatchRule[]) => invoke<WatchRule[]>("save_watches", { watches }),
  setWebhook: (webhook: WebhookSettings) => invoke<void>("set_webhook", { webhook }),
  testWebhook: (webhook: WebhookSettings) => invoke<void>("test_webhook", { webhook }),
  systemInfo: () => invoke<SystemInfo>("system_info"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
};

export function onClickerState(cb: (running: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>("clicker-state", (e) => cb(e.payload));
}

export function onToast(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("toast", (e) => cb(e.payload));
}

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}

export function onMacroState(cb: (playingId: string | null) => void): Promise<UnlistenFn> {
  return listen<string | null>("macro-state", (e) => cb(e.payload));
}

export function onMacroStep(cb: (e: MacroStepEvent) => void): Promise<UnlistenFn> {
  return listen<MacroStepEvent>("macro-step", (e) => cb(e.payload));
}

export function onRecordingState(cb: (recording: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>("recording-state", (e) => cb(e.payload));
}

export function onRecordingFinished(cb: (steps: Step[]) => void): Promise<UnlistenFn> {
  return listen<Step[]>("recording-finished", (e) => cb(e.payload));
}

export function onWatchStatus(cb: (s: WatchStatus) => void): Promise<UnlistenFn> {
  return listen<WatchStatus>("watch-status", (e) => cb(e.payload));
}

export function onWatchHit(cb: (id: string) => void): Promise<UnlistenFn> {
  return listen<string>("watch-hit", (e) => cb(e.payload));
}

export function onSettingsChanged(cb: (s: Settings) => void): Promise<UnlistenFn> {
  return listen<Settings>("settings-changed", (e) => cb(e.payload));
}

/** The macro-so-far while recording (about 6 times a second). */
export function onRecordingProgress(cb: (steps: Step[]) => void): Promise<UnlistenFn> {
  return listen<Step[]>("recording-progress", (e) => cb(e.payload));
}

/** The macro list changed in the backend (e.g. a hotkey moved off a macro). */
export function onMacrosChanged(cb: (macros: Macro[]) => void): Promise<UnlistenFn> {
  return listen<Macro[]>("macros-changed", (e) => cb(e.payload));
}

const TYPING_NAMES = new Set([
  "Space", "Enter", "Tab", "Backspace", "Delete", "Semicolon", "Equals", "Comma", "Minus", "Period",
  "Slash", "Backtick", "LBracket", "Backslash", "RBracket", "Quote", "NumAdd", "NumSubtract",
  "NumMultiply", "NumDivide", "NumDecimal",
]);

/**
 * Hotkeys take over their key system-wide. Warn when that costs something
 * you'd normally use: a typing key (without Ctrl/Alt/Win) or a mouse button.
 */
export function hotkeyWarning(combo: string | null): string | null {
  if (!combo) return null;
  const parts = combo.split("+");
  const key = parts.pop()!;
  if (parts.some((m) => m === "Ctrl" || m === "Alt" || m === "Win")) return null;
  if (key === "RightMouse") return "Right-click won't work normally while this hotkey is set.";
  if (key === "MiddleMouse") return "Middle-click won't work normally while this hotkey is set.";
  const typing = key.length === 1 || /^Num\d$/.test(key) || TYPING_NAMES.has(key);
  return typing ? `“${key}” won't type anywhere while hesytools is running. It'll only trigger this hotkey.` : null;
}

export function onUpdateProgress(cb: (p: UpdateProgress) => void): Promise<UnlistenFn> {
  return listen<UpdateProgress>("update-progress", (e) => cb(e.payload));
}

export function onShutdownState(cb: (t: ShutdownTimer | null) => void): Promise<UnlistenFn> {
  return listen<ShutdownTimer | null>("shutdown-state", (e) => cb(e.payload));
}
