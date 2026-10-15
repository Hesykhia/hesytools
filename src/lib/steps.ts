// Presentation helpers for macro steps: icons, colours, defaults, summaries.

import type { Macro, MouseButton, Step, StepType } from "$lib/api";

export const STEP_TYPES: { type: StepType; label: string; hint: string }[] = [
  { type: "key", label: "Press a key", hint: "Tap, hold or release" },
  { type: "click", label: "Click", hint: "Any mouse button" },
  { type: "wait", label: "Wait", hint: "Pause for a moment" },
  { type: "move", label: "Move mouse", hint: "Jump to a spot" },
  { type: "scroll", label: "Scroll", hint: "Wheel up or down" },
  { type: "text", label: "Type text", hint: "Chat messages, forms" },
  { type: "waitFor", label: "Wait for screen", hint: "Until text, a number or colour shows" },
  { type: "repeat", label: "Repeat", hint: "Loop a group of steps" },
];

// Lucide-style 24×24 stroke paths.
export const ICON: Record<StepType, string> = {
  key: "M5 7h14a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2zm3 7h8",
  click: "M9 3v3M4.5 5.5l2 2M3 10h3m3-1 10 4-4 1.5L13.5 19z",
  wait: "M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
  move: "M5 9l-3 3 3 3M9 5l3-3 3 3M15 19l-3 3-3-3M19 9l3 3-3 3M2 12h20M12 2v20",
  path: "M3 17c3-6 6 2 9-4s6-2 9-8",
  scroll: "M12 3a5 5 0 0 1 5 5v8a5 5 0 0 1-10 0V8a5 5 0 0 1 5-5zm0 4v3",
  text: "M4 7V5h16v2M9 19h6M12 5v14",
  repeat: "M17 2l4 4-4 4M3 11v-1a4 4 0 0 1 4-4h14M7 22l-4-4 4-4M21 13v1a4 4 0 0 1-4 4H3",
  waitFor: "M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12zm10 2.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
};

export const TINT: Record<StepType, string> = {
  key: "violet",
  click: "pink",
  wait: "gray",
  move: "blue",
  path: "blue",
  scroll: "cyan",
  text: "amber",
  repeat: "mint",
  waitFor: "cyan",
};

export const BUTTON_LABEL: Record<MouseButton, string> = {
  LeftMouse: "Left click",
  RightMouse: "Right click",
  MiddleMouse: "Middle click",
  Mouse4: "Mouse 4",
  Mouse5: "Mouse 5",
};

export function defaultStep(type: StepType): Step {
  switch (type) {
    case "key":
      return { type, key: "E", action: "tap", holdMs: 30 };
    case "click":
      return { type, button: "LeftMouse", action: "tap", holdMs: 30, at: null };
    case "wait":
      return { type, ms: 500 };
    case "move":
      return { type, x: 0, y: 0 };
    case "path":
      return { type, points: [] };
    case "scroll":
      return { type, amount: -360, horizontal: false };
    case "text":
      return { type, text: "gg" };
    case "repeat":
      return { type, times: 3, steps: [] };
    case "waitFor":
      return {
        type,
        region: { x: 0, y: 0, w: 0, h: 0 },
        condition: { kind: "textAppears", text: "" },
        timeoutMs: 30000,
        pollMs: 100,
        onTimeout: "continue",
      };
  }
}

export function formatMs(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s < 10 ? s.toFixed(1).replace(/\.0$/, "") : Math.round(s)} s`;
  const m = Math.floor(s / 60);
  return `${m}m ${Math.round(s % 60)}s`;
}

export function durationMs(steps: Step[]): number {
  return steps.reduce((sum, s) => {
    switch (s.type) {
      case "wait":
        return sum + s.ms;
      case "key":
      case "click":
        return sum + (s.action === "tap" ? s.holdMs : 0);
      case "path":
        return sum + (s.points.at(-1)?.t ?? 0);
      case "repeat":
        return sum + durationMs(s.steps) * Math.max(1, s.times);
      default:
        return sum;
    }
  }, 0);
}

/** Steps that do something (waits aren't counted). */
export function actionCount(steps: Step[]): number {
  return steps.reduce((n, s) => n + (s.type === "wait" ? 0 : s.type === "repeat" ? 1 + actionCount(s.steps) : 1), 0);
}

export function macroSummary(m: Macro): string {
  const n = actionCount(m.steps);
  const parts = [`${n} ${n === 1 ? "action" : "actions"}`, formatMs(durationMs(m.steps) / (m.speed || 1))];
  if (m.repeat === 0) parts.push("loops forever");
  else if (m.repeat > 1) parts.push(`×${m.repeat}`);
  return parts.join(" · ");
}

/** The first few things a macro does, for card previews. */
export function preview(steps: Step[], max = 4): { type: StepType; text: string }[] {
  return steps
    .filter((s) => s.type !== "wait")
    .slice(0, max)
    .map((s) => ({ type: s.type, text: shortText(s) }));
}

function shortText(s: Step): string {
  switch (s.type) {
    case "key":
      return s.action === "tap" ? s.key : `${s.action === "down" ? "Hold" : "Release"} ${s.key}`;
    case "click":
      return BUTTON_LABEL[s.button];
    case "move":
      return "Move";
    case "path":
      return "Path";
    case "scroll":
      return s.amount > 0 ? "Scroll up" : "Scroll down";
    case "text":
      return `“${s.text.slice(0, 12)}”`;
    case "repeat":
      return `Repeat ×${s.times || "∞"}`;
    case "waitFor":
      return "Wait for screen";
    default:
      return "";
  }
}
