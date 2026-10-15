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

export type ClickAction = { kind: "mouse"; button: MouseButton } | { kind: "key"; key: string };
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
}

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

export type Condition =
  /** digitsAfter: null = plain contains, 0 = exactly, N = exactly N more digits (e25 +1 = e250..e259). */
  | { kind: "textAppears"; text: string; digitsAfter?: number | null }
  | { kind: "textGone"; text: string; digitsAfter?: number | null }
  | { kind: "number"; op: Cmp; value: number }
  | { kind: "colorAt"; dx: number; dy: number; rgb: [number, number, number]; tolerance: number }
  | { kind: "changes" };

export interface Check {
  matched: boolean;
  text: string | null;
  number: number | null;
  numbers: number[];
  ocrRan: boolean;
  captureMs: number;
  ocrMs: number;
}

export interface ViewerCheck extends Check {
  width: number;
  height: number;
}

export type WatchAction = { kind: "notify" } | { kind: "runMacro"; macroId: string } | { kind: "stopEverything" };

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

export type HotkeyId = "clicker" | "panic" | "record";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setClickerConfig: (config: ClickerConfig) => invoke<void>("set_clicker_config", { config }),
  setClickerMode: (mode: HotkeyMode) => invoke<void>("set_clicker_mode", { mode }),
  setHotkey: (id: HotkeyId, hotkey: string | null) => invoke<Settings>("set_hotkey", { id, hotkey }),
  captureHotkey: () => invoke<string | null>("capture_hotkey"),
  cancelCapture: () => invoke<void>("cancel_capture"),
  pickPoint: () => invoke<[number, number] | null>("pick_point"),
  cancelPick: () => invoke<void>("cancel_pick"),
  clickerToggle: () => invoke<boolean>("clicker_toggle"),
  clickerStatus: () => invoke<ClickerStatus>("clicker_status"),
  setRecordOptions: (options: RecordOptions) => invoke<void>("set_record_options", { options }),
  listMacros: () => invoke<Macro[]>("list_macros"),
  saveMacro: (entry: Macro) => invoke<Macro>("save_macro", { entry }),
  deleteMacro: (id: string) => invoke<void>("delete_macro", { id }),
  playMacro: (id: string) => invoke<void>("play_macro", { id }),
  stopMacro: () => invoke<void>("stop_macro"),
  activityStatus: () => invoke<ActivityStatus>("activity_status"),
  recordingStart: () => invoke<void>("recording_start"),
  recordingStop: () => invoke<void>("recording_stop"),
  pickRegion: () => invoke<Region | null>("pick_region"),
  viewerCheck: (region: Region, condition: Condition) => invoke<ViewerCheck>("viewer_check", { region, condition }),
  /** Raw bytes: u32 width, u32 height (little-endian), then RGBA pixels. */
  viewerFrame: (maxWidth: number) => invoke<ArrayBuffer>("viewer_frame", { maxWidth }),
  viewerStop: () => invoke<void>("viewer_stop"),
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
