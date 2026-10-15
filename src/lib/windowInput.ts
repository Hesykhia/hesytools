import { api } from "./api";

/**
 * Windows doesn't always run the global keyboard/mouse hooks for input that
 * goes to our own window, so hotkeys (and setting a hotkey) went dead while
 * hesytools was focused. Pass keys and extra mouse buttons on to the backend;
 * it drops anything the hook already saw.
 */
export function forwardWindowInput(): () => void {
  const key = (down: boolean) => (e: KeyboardEvent) => {
    if (e.repeat || !e.keyCode) return;
    api.windowInput({ vk: e.keyCode, down }).catch(() => {});
  };
  // Left click is how you use the app, so it's never a hotkey.
  const button = (down: boolean) => (e: MouseEvent) => {
    if (e.button === 0) return;
    api.windowInput({ button: e.button, down }).catch(() => {});
  };
  const handlers = [
    ["keydown", key(true)],
    ["keyup", key(false)],
    ["mousedown", button(true)],
    ["mouseup", button(false)],
  ] as const;
  for (const [type, fn] of handlers) addEventListener(type, fn as EventListener, true);
  return () => {
    for (const [type, fn] of handlers) removeEventListener(type, fn as EventListener, true);
  };
}
