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
}

export interface Settings {
  clicker: ClickerConfig;
  clickerHotkey: string | null;
  clickerMode: HotkeyMode;
  panicHotkey: string | null;
  webhook: WebhookSettings;
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

export type HotkeyId = "clicker" | "panic";

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
