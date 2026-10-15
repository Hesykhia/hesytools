# hesytools

Macros, an autoclicker and screen watching for Windows, built to work in games
(Roblox, Minecraft) as well as normal desktop apps. It's a from-scratch rewrite
of the old Python hesytools (v4).

## What it does

- **Quick Clicker**: click a mouse button or spam a key at any rate, at the
  cursor or a fixed spot, forever or for a count or time. Toggle or hold
  hotkey (F6 by default).
- **Macros**: record (F9) and replay keys, clicks, mouse movement and timing.
  Steps can be edited in place, and you can add loops, typed text and "wait
  until the screen shows…". Each macro can have its own hotkey, repeat count
  and speed. While recording, every step shows up live in the app and in a
  small always-on-top overlay. Old v4 macro files can be imported.
- **Screen Watch**: drag a box on screen, then trigger on text appearing or
  disappearing, a number reaching a value (`1,234`, `1.25e+25`, `12.5K`,
  `3M`), "e25 +1 digit" style matching (e250 to e259), a pixel colour, an
  image (button or icon), or any change. A live viewer shows exactly what's
  being read. Watches can run a macro, stop everything, or just notify you.
- **Discord webhook** alerts, a **panic key** (F8) that stops everything,
  admin mode for elevated apps, and a tray icon.

## Why it's light on CPU

- Text is read with Windows' built-in OCR engine, and only when the watched
  pixels actually changed. A static screen costs almost nothing. Polling a
  region every 100 ms measured about 1% of one core.
- Timing uses high-resolution waitable timers instead of busy loops. A 1 ms
  clicker interval averages exactly 1.000 ms.
- Keys are sent as hardware scancodes, which games reading Raw Input or
  DirectInput accept.
- When nothing is running, nothing animates, so the window doesn't redraw.

## Development

Needs Rust (stable), Node 20+, pnpm and the MSVC build tools.

```sh
pnpm install
pnpm tauri dev      # run the app
pnpm tauri build    # release build + installer in target/release/bundle/nsis
cargo test -p engine
```

**Browser preview mode.** `pnpm dev` and then opening http://localhost:1420
in a normal browser runs the UI against a fake backend
(`src/lib/dev/mock.ts`). That lets you design screens without sending any
real input. `?scene=editor:m2`, `playing:m1`, `recording` and `watch:1` jump
straight to a state.

**Engine tools** (`crates/engine/examples`):

| Example | What it does |
|---|---|
| `probe` | Shows input exactly as a game reading Raw Input sees it |
| `selftest` | Measures timing accuracy and CPU (only presses F24) |
| `ocrtest`, `waitfor`, `pickregion` | Screen Watch checks |
| `recplay` | Records and replays input |
| `importv4` | Converts an old macro file and prints the steps |

## Layout

```
crates/engine/   Everything that touches Windows, independent of the UI:
                 input (SendInput), timer, hooks, hotkeys, clicker, macros
                 (model, recorder, player), capture, ocr, watch, imagematch,
                 picker, import, webhook, elevation
src-tauri/       The app: commands, settings, macro storage, watch runner,
                 tray, recording overlay window
src/             SvelteKit UI (Svelte 5). Motion follows the transitions.dev
                 tokens (.agents/skills)
```

Settings and macros live in `%APPDATA%\com.hesy.hesytools\` as plain JSON
(`settings.json`, `macros\*.json`).
