// ==UserScript==
// @name         hesytools · Soul Clicker helper
// @namespace    hesytools
// @version      1.0.0
// @description  Lets hesytools play Soul Clicker for you: catches favors, buys, runs the Ritual. Only presses the game's own buttons.
// @match        https://www.deadchaps.gg/soul-clicker*
// @match        https://deadchaps.gg/soul-clicker*
// @grant        GM_xmlhttpRequest
// @connect      127.0.0.1
// @run-at       document-idle
// @noframes
// ==/UserScript==

// Everything here works the way a player does: it reads what's on the page
// and presses the game's buttons. It never touches the game's memory, your
// save or the console. Settings come from the Dev tab in hesytools.

(function () {
  "use strict";

  const BRIDGE = "http://127.0.0.1:__PORT__/soul";
  const TOKEN = "__TOKEN__";

  /** Off until hesytools says otherwise. */
  let cfg = {
    enabled: false,
    favors: false,
    crystals: false,
    buyGens: false,
    buyUpgrades: false,
    orb: false,
    orbCps: 8,
    ritual: { mode: "off", worth: 1.5, shards: 10 },
  };
  const stats = { favors: 0, crystals: 0, gens: 0, upgrades: 0, rituals: 0, orbClicks: 0, last: "" };

  // ── Reading the page ──────────────────────────────────────────────────────
  const WORDS = ["thousand", "million", "billion", "trillion", "quadrillion", "quintillion", "sextillion", "septillion", "octillion", "nonillion"];
  const UNITS = ["", "un", "duo", "tre", "quattuor", "quin", "sex", "septen", "octo", "novem"];
  const TENS = ["", "decillion", "vigintillion", "trigintillion", "quadragintillion", "quinquagintillion", "sexagintillion", "septuagintillion", "octogintillion", "nonagintillion"];
  /** log10 of a big-number word ("million" → 6, "duodecillion" → 39). */
  function wordExp(w) {
    const i = WORDS.indexOf(w);
    if (i >= 0) return 3 * (i + 1);
    for (let t = 1; t < TENS.length; t++)
      for (let u = 0; u < UNITS.length; u++) if (UNITS[u] + TENS[t] === w) return 3 * (t * 10 + u + 1);
    return null;
  }
  /** "1.25 trillion" / "12,345" / "1e45" → a number (Infinity past 1e308). */
  function parseNum(text) {
    if (!text) return NaN;
    const t = text.toLowerCase().replace(/,/g, "").trim();
    const m = t.match(/(-?[\d.]+(?:e[+-]?\d+)?)\s*([a-z]+)?/);
    if (!m) return NaN;
    let n = parseFloat(m[1]);
    const exp = m[2] ? wordExp(m[2]) : 0;
    if (exp === null) return n;
    return n * Math.pow(10, exp);
  }
  const $ = (sel) => document.querySelector(sel);
  const $$ = (sel) => [...document.querySelectorAll(sel)];
  const text = (el) => (el ? el.textContent.replace(/\s+/g, " ").trim() : "");

  function readStatus() {
    const sps = text($(".sps-line"));
    return {
      souls: text($(".souls-count")),
      sps: sps.split("·")[0].replace("souls per second", "").trim(),
      perClick: (sps.split("·")[1] || "").replace("per click", "").trim(),
      pending: text($(".ritual-btn .rb-pend")),
    };
  }

  // ── Doing things ──────────────────────────────────────────────────────────
  const press = (el) => {
    // Some of the game's buttons act on pointer-down, others on click.
    el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "mouse", button: 0 }));
    el.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerType: "mouse", button: 0 }));
    el.click();
  };
  const isOpen = (el) => el && !el.classList.contains("cant") && !el.disabled;

  function catchPopups() {
    if (cfg.favors)
      for (const b of $$("button.favor")) {
        press(b);
        stats.favors++;
        stats.last = "Caught a favor";
      }
    if (cfg.crystals)
      for (const b of $$("button.crystal-boss")) {
        press(b);
        stats.crystals++;
        stats.last = "Killed a Mid-Boss";
      }
  }

  /** Make sure the shop buys one at a time (not sells, not x100). */
  function shopToOne() {
    const ctl = $$(".shop-controls button");
    const buy = ctl.find((b) => text(b) === "BUY");
    const one = ctl.find((b) => text(b) === "x1");
    if (buy && !buy.classList.contains("on")) buy.click();
    if (one && !one.classList.contains("on")) one.click();
  }

  function buyUpgrade() {
    // "rumor" cards are the ??? ones that can't be bought yet.
    const card = $$(".shelf button.card:not(.rumor)").find(isOpen);
    if (!card) return false;
    press(card);
    stats.upgrades++;
    stats.last = "Bought " + (card.getAttribute("aria-label") || "an upgrade").split(".")[0];
    return true;
  }

  /** The generator with the most souls per second per soul spent. */
  function buyGenerator() {
    let best = null;
    for (const g of $$(".gens button.gen")) {
      if (g.classList.contains("mystery")) continue;
      const cost = parseNum(text(g.querySelector(".gen-cost")));
      const sps = parseNum(text(g.querySelector(".gen-sps")));
      if (!(cost > 0) || !(sps > 0)) continue;
      const value = sps / cost;
      if (!best || value > best.value) best = { g, value };
    }
    // Wait for the best one rather than wasting souls on worse ones.
    if (!best || !isOpen(best.g)) return false;
    shopToOne();
    press(best.g);
    stats.gens++;
    stats.last = "Bought " + text(best.g.querySelector(".gen-name"));
    return true;
  }

  // ── The Ritual ────────────────────────────────────────────────────────────
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  /** Hover the Ritual button and read its tooltip: current and pending bonus. */
  async function ritualNumbers() {
    const btn = $(".ritual-btn");
    if (!btn) return null;
    btn.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body }));
    // The tooltip draws a moment after the hover.
    let tip = "";
    for (let i = 0; i < 5 && !tip; i++) {
      await sleep(80);
      tip = $$(".souls-achtip, .souls-tipbox").map(text).find((t) => t.includes("Patron Shards")) || "";
    }
    btn.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: document.body }));
    const bonus = tip.match(/permanent \+([^%]+)%/);
    const pend = tip.match(/bank ([^ ]+(?: [a-z]+)?) shards? \(\+([^%]+)% output\)/);
    if (!pend) return { can: false, pending: 0, bonus: bonus ? parseNum(bonus[1]) : 0, pendingBonus: 0 };
    return { can: true, pending: parseNum(pend[1]), bonus: bonus ? parseNum(bonus[1]) : 0, pendingBonus: parseNum(pend[2]) };
  }

  let ritualBusy = false;
  /** Last tooltip reading; re-read only when the pending count on the button
   * changes (and at most every 20 s), so the tooltip doesn't keep flashing. */
  let lastRitual = null;
  let lastPeek = { pend: null, at: 0 };
  async function peekRitual() {
    const pend = text($(".ritual-btn .rb-pend"));
    if (pend === lastPeek.pend && lastRitual) return lastRitual;
    if (Date.now() - lastPeek.at < 20000 && lastRitual) return lastRitual;
    lastPeek = { pend, at: Date.now() };
    lastRitual = await ritualNumbers();
    return lastRitual;
  }
  async function maybeRitual() {
    const r = cfg.ritual || {};
    if (r.mode === "off" || ritualBusy) return;
    ritualBusy = true;
    const n = await peekRitual();
    ritualBusy = false;
    if (!n || !n.can) return;
    const gain = (100 + n.bonus + n.pendingBonus) / (100 + n.bonus);
    const go =
      r.mode === "always" || (r.mode === "shards" && n.pending >= (r.shards || 1)) || (r.mode === "worth" && gain >= (r.worth || 1.5));
    if (!go) return;
    ritualBusy = true;
    lastRitual = null;
    stats.last = "Ritual for " + n.pending + " shards (×" + gain.toFixed(2) + " output)";
    // Ritual button → "Sacrifice the run" → the tree screen → back to a
    // fresh run ("Begin the run" instead when vows are on offer).
    const visibleButton = (label) => $$("button").find((b) => b.offsetParent !== null && text(b) === label);
    const before = text($(".souls-count"));
    press($(".ritual-btn"));
    await sleep(700);
    const yes = visibleButton("Sacrifice the run");
    if (yes) {
      press(yes);
      await sleep(1200);
      const next = visibleButton("Begin the run") || visibleButton("Return to the run");
      if (next) press(next);
      await sleep(600);
    }
    const after = text($(".souls-count"));
    if (yes && after !== before) stats.rituals++;
    else {
      stats.last = "Ritual didn't go through";
      const cancel = visibleButton("Cancel");
      if (cancel) press(cancel);
    }
    ritualBusy = false;
  }

  // ── Orb clicking, with a human-ish rhythm ─────────────────────────────────
  function orbLoop() {
    const cps = Math.min(20, Math.max(1, cfg.orbCps || 8));
    // ±45% jitter keeps the gaps irregular, so the game's rhythm check
    // ("The Concierge Noticed") doesn't flag it.
    const gap = (1000 / cps) * (0.55 + Math.random() * 0.9);
    setTimeout(orbLoop, gap);
    if (!cfg.enabled || !cfg.orb || document.hidden) return;
    const orb = $("button.orb");
    if (orb) {
      orb.click();
      stats.orbClicks++;
    }
  }

  // ── Main loop ─────────────────────────────────────────────────────────────
  let tickN = 0;
  function tick() {
    if (!cfg.enabled) return;
    catchPopups();
    tickN++;
    // Shopping a few times a second is plenty.
    if (tickN % 2 === 0) {
      if (cfg.buyUpgrades && buyUpgrade()) return;
      if (cfg.buyGens) buyGenerator();
    }
    if (tickN % 8 === 0) maybeRitual();
  }

  // ── Talking to hesytools (only on this PC) ────────────────────────────────
  let connected = false;
  function sync() {
    const body = JSON.stringify({ status: readStatus(), stats, ritual: lastRitual });
    const done = (ok, resText) => {
      connected = ok;
      if (ok) {
        try {
          cfg = Object.assign(cfg, JSON.parse(resText));
        } catch {}
      } else cfg.enabled = false;
      setTimeout(sync, 1000);
    };
    if (typeof GM_xmlhttpRequest === "function") {
      GM_xmlhttpRequest({
        method: "POST",
        url: BRIDGE,
        headers: { "Content-Type": "application/json", "X-Hesy-Token": TOKEN },
        data: body,
        timeout: 3000,
        onload: (r) => done(r.status === 200, r.responseText),
        onerror: () => done(false),
        ontimeout: () => done(false),
      });
    } else if (window.__hesyTestConfig) {
      // Test harness: settings injected directly.
      done(true, JSON.stringify(window.__hesyTestConfig));
    } else done(false);
  }

  window.__hesyHelper = { stats, readStatus, ritualNumbers, parseNum, get cfg() { return cfg; }, get connected() { return connected; } };
  setInterval(tick, 125);
  orbLoop();
  sync();
})();
