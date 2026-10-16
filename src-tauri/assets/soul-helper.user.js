// ==UserScript==
// @name         hesytools · Soul Clicker helper
// @namespace    hesytools
// @version      1.5.0
// @description  Lets hesytools play Soul Clicker for you: catches favors, buys, runs the Ritual. Only presses the game's own buttons.
// @match        https://www.deadchaps.gg/soul-clicker*
// @match        https://deadchaps.gg/soul-clicker*
// @grant        GM_xmlhttpRequest
// @connect      127.0.0.1
// @updateURL    http://127.0.0.1:__PORT__/soul-helper.user.js
// @downloadURL  http://127.0.0.1:__PORT__/soul-helper.user.js
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
  const VERSION = "1.5.0";

  /** Off until hesytools says otherwise. */
  let cfg = {
    enabled: false,
    favors: false,
    crystals: false,
    leech: false,
    blessings: false,
    frenzy: false,
    buyGens: false,
    buyUpgrades: false,
    orb: false,
    orbCps: 8,
    reserveMinutes: 0,
    treeStop: true,
    ritual: { mode: "off", worth: 1.5, shards: 10 },
  };
  const stats = { plan: "", favors: 0, spheres: 0, crystals: 0, leeches: 0, blessings: 0, gens: 0, upgrades: 0, rituals: 0, orbClicks: 0, last: "" };

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
    if (cfg.favors) {
      // Every kind of favor (statues, buff favors, the Wrath) is this button.
      for (const b of $$("button.favor")) {
        press(b);
        stats.favors++;
        stats.last = "Caught " + ((b.getAttribute("aria-label") || "a favor").split(",")[0] || "a favor");
      }
      // Chains and storms scatter soul spheres to gather.
      for (const b of $$("button.soul-sphere")) {
        b.click();
        stats.spheres++;
        stats.last = "Gathered soul spheres";
      }
    }
    if (cfg.leech)
      for (const b of $$("button.leech")) {
        b.click();
        stats.leeches++;
        stats.last = "Claimed the Soul Leech";
      }
    if (cfg.blessings)
      for (const b of $$("button")) {
        if (!(b.getAttribute("aria-label") || "").startsWith("Release banked blessing")) continue;
        b.click();
        stats.blessings++;
        stats.last = "Released a banked blessing";
      }
    if (cfg.crystals)
      for (const b of $$("button.crystal-boss")) {
        press(b);
        stats.crystals++;
        stats.last = "Killed a Mid-Boss";
      }
  }

  function buyUpgrade() {
    // "rumor" cards are the ??? ones that can't be bought yet.
    const card = $$(".shelf button.card:not(.rumor)")
      .filter(isOpen)
      .find((c) => canSpend(parseNum(((c.getAttribute("aria-label") || "").match(/Costs (.+) souls/) || [])[1])));
    if (!card) return false;
    press(card);
    stats.upgrades++;
    stats.last = "Bought " + (card.getAttribute("aria-label") || "an upgrade").split(".")[0];
    return true;
  }

  /** The shop is set to Sell: never touch generators then. */
  const selling = () => !!$(".mode-toggle button.on.sell") || !!$(".gens button.gen.selling");

  // Game rules (from Soul Clicker's code): each purchase raises a generator's
  // price by 15%; owning 10/25/50/... of one unlocks a Curiosity Shop item
  // that doubles its output, costing that many times its base price.
  const GROWTH = 1.15;
  const MILESTONES = [
    [10, 10],
    [25, 100],
    [50, 1e3],
    [100, 5e4],
    [150, 5e6],
    [200, 5e8],
    [250, 5e10],
  ];

  /** Each generator as it stands, from its own button:
   * "Buy 10 Trooper for 1,100 souls, owned 5". */
  function generators() {
    const out = [];
    for (const g of $$(".gens button.gen")) {
      if (g.classList.contains("mystery") || g.style.display === "none") continue;
      const m = (g.getAttribute("aria-label") || "").match(/^Buy (\d+) (.+) for (.+) souls, owned (\d+)/);
      if (!m) continue;
      const qty = parseInt(m[1], 10);
      const cost = parseNum(m[3]);
      const owned = parseInt(m[4], 10);
      const each = parseNum(text(g.querySelector(".gen-sps")));
      if (!(qty > 0) || !(cost > 0) || !(each > 0)) continue;
      // Price of the very next one, and the base price it grew from.
      const next = qty === 1 ? cost : (cost * (GROWTH - 1)) / (Math.pow(GROWTH, qty) - 1);
      const base = next / Math.pow(GROWTH, owned);
      out.push({ g, name: m[2], qty, cost, owned, each, next, base, can: isOpen(g) });
    }
    return out;
  }

  /** Everything you make per second: production plus what clicking earns. */
  function income() {
    const sps = parseNum(text($(".sps-line b"))) || 0;
    const perClick = parseNum(text($(".sps-line span"))) || 0;
    const clicks = cfg.enabled && cfg.orb ? Math.min(20, Math.max(1, cfg.orbCps || 8)) : 0;
    return sps + clicks * perClick;
  }

  /** Pick what pays off soonest against your whole income: a plain buy, or
   * working up to a generator's next milestone and its doubling item.
   * payback = time to afford it + time for it to earn its price back. */
  function buyGenerator() {
    if (selling()) {
      stats.last = "Shop is in Sell mode, not buying";
      stats.plan = "";
      return false;
    }
    const souls = parseNum(text($(".souls-count"))) || 0;
    const earn = income();
    const reserve = Math.max(0, cfg.reserveMinutes || 0) * 60 * earn;
    const gens = generators();
    // "sps each" on a generator only counts its own upgrades, not global
    // boosts (Patron Shards, whole-operation upgrades...). Real production
    // divided by what the labels add up to gives that boost, so gains are
    // measured on the same scale as income.
    const labelled = gens.reduce((sum, o) => sum + o.owned * o.each, 0);
    const production = parseNum(text($(".sps-line b"))) || 0;
    const boost = labelled > 0 && production > labelled ? production / labelled : 1;
    const bank = Math.max(0, souls - reserve);
    const payback = (cost, gain) => (cost <= bank ? 0 : earn > 0 ? (cost - bank) / earn : Infinity) + cost / gain;
    let best = null;
    for (const o of gens) {
      const plain = { o, kind: "buy", score: payback(o.cost, o.each * boost * o.qty) };
      if (!best || plain.score < best.score) best = plain;
      const goal = MILESTONES.find(([at]) => at > o.owned);
      if (goal) {
        const [at, x] = goal;
        const need = at - o.owned;
        const units = (o.next * (Math.pow(GROWTH, need) - 1)) / (GROWTH - 1);
        // All `at` of them end up making twice as much.
        const gain = (2 * at - o.owned) * o.each * boost;
        const push = { o, kind: "goal", at, score: payback(units + o.base * x, gain) };
        if (push.score < best.score) best = push;
      }
    }
    if (!best) return false;
    const o = best.o;
    stats.plan =
      best.kind === "goal" ? "Working toward " + best.at + " " + o.name + " (doubles them)" : "Next: " + o.name;
    // Wait (and keep the reserve) rather than spending on something worse.
    if (!o.can || souls - o.cost < reserve) return false;
    press(o.g);
    stats.gens += o.qty;
    stats.last = "Bought " + (o.qty > 1 ? o.qty + " " : "") + o.name;
    return true;
  }

  /** Don't spend into the reserve on upgrades either. */
  function canSpend(cost) {
    const souls = parseNum(text($(".souls-count"))) || 0;
    return souls - cost >= Math.max(0, cfg.reserveMinutes || 0) * 60 * income();
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
      // The skill tree only ever opens here. Either leave it up for the
      // player to spend shards, or carry straight on into the next run.
      if (!cfg.treeStop) {
        const next = visibleButton("Begin the run") || visibleButton("Return to the run");
        if (next) press(next);
      }
      await sleep(600);
    }
    const after = text($(".souls-count"));
    if (yes && after !== before) {
      stats.rituals++;
      if (cfg.treeStop) stats.last = "Ritual done. Waiting on the skill tree: press Return to the run when you're finished";
    }
    else {
      stats.last = "Ritual didn't go through";
      const cancel = visibleButton("Cancel");
      if (cancel) press(cancel);
    }
    ritualBusy = false;
  }

  // ── Orb clicking, with a human-ish rhythm ─────────────────────────────────
  /** A blessing that makes clicks pay more is running: Trigger Frenzy and
   * Kinetic Dash (buff-click) or Spirit Surge (buff-surge). */
  const clickBuff = () => !!$(".buff-stack .buff-click, .buff-stack .buff-surge");
  /** "Hold the orb to draw souls at full speed" (buff-autoclick). */
  let holding = false;
  function holdOrb() {
    const want = cfg.enabled && cfg.frenzy && !!$(".buff-stack .buff-autoclick");
    const orb = $("button.orb");
    if (!orb || want === holding) return;
    holding = want;
    const type = want ? "pointerdown" : "pointerup";
    orb.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerType: "mouse", button: 0, buttons: want ? 1 : 0 }));
    if (want) stats.last = "Holding the orb";
  }
  function orbLoop() {
    const boost = cfg.frenzy && clickBuff();
    const cps = boost ? 20 : Math.min(20, Math.max(1, cfg.orbCps || 8));
    // ±45% jitter keeps the gaps irregular, so the game's rhythm check
    // ("The Concierge Noticed") doesn't flag it.
    const gap = (1000 / cps) * (0.55 + Math.random() * 0.9);
    setTimeout(orbLoop, gap);
    if (!cfg.enabled || !(cfg.orb || boost) || document.hidden || onTree()) return;
    const orb = $("button.orb");
    if (orb) {
      orb.click();
      stats.orbClicks++;
    }
  }

  // ── Main loop ─────────────────────────────────────────────────────────────
  let tickN = 0;
  /** The Ritual / skill tree screen is up: leave everything alone. */
  const onTree = () => !!$(".asc-wrap.on, .asc-confirm.on");
  function tick() {
    holdOrb();
    if (!cfg.enabled || onTree()) return;
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
    const body = JSON.stringify({ version: VERSION, status: readStatus(), stats, ritual: lastRitual });
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
