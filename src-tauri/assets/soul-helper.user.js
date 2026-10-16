// ==UserScript==
// @name         hesytools · Soul Clicker helper
// @namespace    hesytools
// @version      1.6.0
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
  const VERSION = "1.6.0";

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
    treeBuy: false,
    ritual: { mode: "off", worth: 1.5, shards: 10 },
  };
  const stats = { plan: "", treeNodes: 0, favors: 0, spheres: 0, crystals: 0, leeches: 0, blessings: 0, gens: 0, upgrades: 0, rituals: 0, orbClicks: 0, last: "" };

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

  /** Clicks per second the helper is putting in (0 if it isn't clicking). */
  const clickRate = () => (cfg.enabled && cfg.orb ? Math.min(20, Math.max(1, cfg.orbCps || 8)) : 0);

  /** A "Vow of the Empty Hand" run: every click earns exactly 1 soul, so
   * anything that improves clicking is worthless. The vow checkboxes stay in
   * the page during the run. */
  const clicksLocked = () =>
    $$(".vow-picks label.vow").some((l) => /Empty Hand/i.test(text(l)) && l.querySelector("input")?.checked);

  /** Everything you make per second: production plus what clicking earns. */
  function income() {
    const sps = parseNum(text($(".sps-line b"))) || 0;
    const perClick = parseNum(text($(".sps-line span"))) || 0;
    return sps + clickRate() * perClick;
  }

  /** The numbers every valuation works from. */
  function economy() {
    const gens = generators();
    const production = parseNum(text($(".sps-line b"))) || 0;
    const perClick = parseNum(text($(".sps-line span"))) || 0;
    // "sps each" on a generator only counts its own upgrades, not global
    // boosts (Patron Shards, whole-operation upgrades...). Real production
    // divided by what the labels add up to gives that boost, so gains are
    // measured on the same scale as income.
    const labelled = gens.reduce((sum, o) => sum + o.owned * o.each, 0);
    const boost = labelled > 0 && production > labelled ? production / labelled : 1;
    const locked = clicksLocked();
    const rate = locked ? 0 : clickRate();
    const byName = Object.fromEntries(gens.map((o) => [o.name, o]));
    return { gens, byName, production, boost, locked, clickRate: rate, clickIncome: rate * perClick, earn: production + rate * perClick };
  }

  /** Curiosity Shop items on the shelf: "Name. Effect Costs 1.2 million souls." */
  function shopItems() {
    const out = [];
    for (const c of $$(".shelf button.card:not(.rumor)")) {
      const label = c.getAttribute("aria-label") || "";
      const m = label.match(/^(.+?)\. (.*) Costs (.+) souls\.$/);
      if (!m) continue;
      const cost = parseNum(m[3]);
      if (cost > 0) out.push({ c, name: m[1], effect: m[2], cost, can: isOpen(c) });
    }
    return out;
  }

  /** Souls per second an item adds, read from what it says it does. Click
   * items are worth nothing when nothing is clicking or clicks are locked. */
  function itemGain(effect, e) {
    let g = 0;
    let m;
    const mine = (name) => e.byName[name];
    if ((m = effect.match(/^(.+?) output doubled/))) {
      const o = mine(m[1]);
      if (o) g += o.owned * o.each * e.boost;
    }
    if ((m = effect.match(/^(.+?) gains (?:a further )?\+([\d.]+)% output per (.+?) owned/))) {
      const a = mine(m[1]);
      const b = mine(m[3]);
      if (a && b) g += a.owned * a.each * e.boost * (parseFloat(m[2]) / 100) * b.owned;
    }
    if ((m = effect.match(/total output x([\d.]+)/i))) g += e.production * (parseFloat(m[1]) - 1);
    if (/runs 2% harder/.test(effect)) g += e.production * 0.02;
    if (/twice as effective/i.test(effect)) g += e.clickIncome;
    if ((m = effect.match(/Clicks (?:also earn|earn a further) ([\d.]+)% of your SPS/i))) g += e.clickRate * e.production * (parseFloat(m[1]) / 100);
    if ((m = effect.match(/[Cc]licking[^.]*x([\d.]+)/))) g += e.clickIncome * (parseFloat(m[1]) - 1);
    // Something else that doesn't involve clicking: assume a small lift.
    if (!g && !/[Cc]lick/.test(effect)) g = e.production * 0.01;
    return g;
  }

  /** Pick what pays off soonest against your whole income: a generator, the
   * next milestone of one (and the item that doubles it), or a Curiosity
   * Shop item. payback = time to afford it + time to earn its price back. */
  function buyBest() {
    if (cfg.buyGens && selling()) {
      stats.last = "Shop is in Sell mode, not buying";
      stats.plan = "";
      return false;
    }
    const souls = parseNum(text($(".souls-count"))) || 0;
    const e = economy();
    const earn = e.earn;
    const reserve = Math.max(0, cfg.reserveMinutes || 0) * 60 * earn;
    const bank = Math.max(0, souls - reserve);
    const payback = (cost, gain) => (gain > 0 ? (cost <= bank ? 0 : earn > 0 ? (cost - bank) / earn : Infinity) + cost / gain : Infinity);
    const options = [];
    if (cfg.buyGens && !selling()) {
      for (const o of e.gens) {
        options.push({ el: o.g, can: o.can, cost: o.cost, qty: o.qty, kind: "gen", name: o.name, score: payback(o.cost, o.each * e.boost * o.qty) });
        const goal = MILESTONES.find(([at]) => at > o.owned);
        if (goal) {
          const [at, x] = goal;
          const need = at - o.owned;
          const units = (o.next * (Math.pow(GROWTH, need) - 1)) / (GROWTH - 1);
          // All `at` of them end up making twice as much.
          const gain = (2 * at - o.owned) * o.each * e.boost;
          const cost = units + o.base * x;
          options.push({ el: o.g, can: o.can, cost: o.cost, qty: o.qty, kind: "goal", at, name: o.name, total: cost, score: payback(cost, gain) });
        }
      }
    }
    if (cfg.buyUpgrades) {
      for (const it of shopItems()) {
        options.push({ el: it.c, can: it.can, cost: it.cost, kind: "item", name: it.name, score: payback(it.cost, itemGain(it.effect, e)) });
      }
    }
    const best = options.filter((o) => isFinite(o.score)).sort((a, b) => a.score - b.score)[0];
    if (!best) {
      stats.plan = "";
      return false;
    }
    // Show the reasoning: how soon it pays for itself, and what it costs
    // measured in seconds of everything you make.
    const total = best.total ?? best.cost;
    const why = " · pays for itself in " + span(best.score) + ", costs " + span(earn > 0 ? total / earn : Infinity) + " of income";
    stats.plan =
      (best.kind === "goal"
        ? "Working toward " + best.at + " " + best.name + " (doubles them)"
        : best.kind === "item"
          ? "Shop: " + best.name
          : "Next: " + best.name) +
      why +
      (e.locked ? " · Empty Hand run, skipping click items" : "");
    // Wait (and keep the reserve) rather than spending on something worse.
    if (!best.can || souls - best.cost < reserve) return false;
    press(best.el);
    if (best.kind === "item") {
      stats.upgrades++;
      stats.last = "Bought " + best.name;
    } else {
      stats.gens += best.qty;
      stats.last = "Bought " + (best.qty > 1 ? best.qty + " " : "") + best.name;
    }
    return true;
  }

  /** 0.00004 s, 2.1 s, 4m 10s, 3h 5m... */
  function span(secs) {
    if (!isFinite(secs)) return "never";
    if (secs < 1) return secs.toPrecision(2) + " s";
    if (secs < 60) return secs.toFixed(1) + " s";
    if (secs < 3600) return Math.floor(secs / 60) + "m " + Math.round(secs % 60) + "s";
    if (secs < 86400) return Math.floor(secs / 3600) + "h " + Math.round((secs % 3600) / 60) + "m";
    return Math.round(secs / 86400) + " days";
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
    const run = tip.match(/On this run for ([^.]+)\./);
    const runSecs = run ? duration(run[1]) : 0;
    const bonus = tip.match(/permanent \+([^%]+)%/);
    const pend = tip.match(/bank ([^ ]+(?: [a-z]+)?) shards? \(\+([^%]+)% output\)/);
    if (!pend) return { can: false, pending: 0, bonus: bonus ? parseNum(bonus[1]) : 0, pendingBonus: 0, runSecs };
    return { can: true, pending: parseNum(pend[1]), bonus: bonus ? parseNum(bonus[1]) : 0, pendingBonus: parseNum(pend[2]), runSecs };
  }

  /** "1d 2h 3m" / "4m 5s" / "30s" (the game's run timer) to seconds. */
  function duration(t) {
    const unit = { d: 86400, h: 3600, m: 60, s: 1 };
    return [...t.matchAll(/(\d+)\s*([dhms])/g)].reduce((sum, m) => sum + Number(m[1]) * unit[m[2]], 0);
  }

  /** Best bonus-growth rate seen this run (for the "fastest growth" rule). */
  let peak = { rate: 0, runSecs: 0 };

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
    // "Fastest growth": how fast this run is growing your permanent bonus,
    // per second of the run. It climbs, peaks, then falls as shards get
    // harder to earn; sacrificing just after the peak grows the bonus
    // fastest over many runs.
    let fastest = false;
    if (r.mode === "fastest" && n.runSecs > 0) {
      if (n.runSecs < peak.runSecs) peak = { rate: 0, runSecs: 0 };
      const rate = Math.log(gain) / n.runSecs;
      if (rate > peak.rate) peak = { rate, runSecs: n.runSecs };
      else peak.runSecs = n.runSecs;
      fastest = n.runSecs >= 60 && rate < peak.rate * 0.97;
    }
    const go =
      r.mode === "always" ||
      fastest ||
      (r.mode === "shards" && n.pending >= (r.shards || 1)) ||
      (r.mode === "worth" && gain >= (r.worth || 1.5));
    if (!go) return;
    // Remember what this run was made of, to value the skill tree.
    rememberRun();
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
      // The skill tree only ever opens here. Spend shards on it if asked,
      // then either leave it up for the player or carry on into the next run.
      if (cfg.treeBuy) await spendTree();
      if (!cfg.treeStop) {
        const next = visibleButton("Begin the run") || visibleButton("Return to the run");
        if (next) press(next);
      }
      await sleep(600);
    }
    const after = text($(".souls-count"));
    if (yes && after !== before) {
      stats.rituals++;
      peak = { rate: 0, runSecs: 0 };
      if (cfg.treeStop) stats.last = "Ritual done. Waiting on the skill tree: press Return to the run when you're finished";
    }
    else {
      stats.last = "Ritual didn't go through";
      const cancel = visibleButton("Cancel");
      if (cancel) press(cancel);
    }
    ritualBusy = false;
  }

  // ── The skill tree ────────────────────────────────────────────────────────
  // Node name → [id, ids it needs, kind, cosmetic/vanity]. From the game.
  const TREE = {"The First Wish":["first_wish",[],"root",0],"Call the Roster":["call_the_roster",["first_wish"],"buy",0],"Recruit Wraith":["unlock_wraith",["call_the_roster"],"buy",0],"Recruit Seven":["unlock_seven",["call_the_roster"],"buy",0],"Recruit Ivy":["unlock_ivy",["unlock_wraith"],"buy",0],"Recruit Billy":["unlock_billy",["unlock_seven"],"buy",0],"Recruit Abrams":["unlock_abrams",["call_the_roster"],"buy",0],"Recruit Warden":["unlock_warden",["unlock_abrams"],"buy",0],"Recruit Haze":["unlock_haze",["call_the_roster"],"buy",0],"Recruit Drifter":["unlock_drifter",["call_the_roster"],"buy",0],"Recruit Mina":["unlock_mina",["unlock_drifter"],"buy",0],"Recruit Venator":["unlock_venator",["unlock_drifter"],"buy",0],"Recruit Viscous":["unlock_viscous",["call_the_roster"],"buy",0],"First Coin":["first_coin",["first_wish"],"buy",0],"Beginner's Luck":["beginners_luck",["first_wish"],"buy",0],"A Kindly Omen":["good_omen",["first_wish"],"buy",0],"Seed Money":["seed_money",["first_wish"],"buy",0],"Unsecured Souls":["whispered_ledger",["first_wish"],"buy",0],"The Coven's Nightwork":["midnight_accounting",["whispered_ledger"],"buy",0],"Fattening the Leech":["leech_husbandry",["whispered_ledger"],"buy",0],"The Urn Runs Itself":["perpetual_harvest",["midnight_accounting"],"buy",0],"The Leech's Second Stomach":["second_stomach",["leech_husbandry"],"buy",0],"The Graveyard Shift":["graveyard_shift",["perpetual_harvest"],"buy",0],"Nothing Denied":["books_balance",["graveyard_shift"],"buy",0],"The Patron's Advance":["patrons_advance",["first_wish"],"buy",0],"The Patron's Retainer":["patrons_retainer",["patrons_advance"],"buy",0],"Off the Books":["occult_bookkeeping",["patrons_advance"],"buy",0],"Memory of Ash":["memory_of_ash",["patrons_retainer"],"buy",0],"The Shopkeeper's Tab":["shopkeepers_tab",["occult_bookkeeping"],"buy",0],"Founder's Endowment":["founders_endowment",["memory_of_ash"],"buy",0],"The Curator's Inventory":["curators_inventory",["shopkeepers_tab"],"buy",0],"The Standing Order":["standing_order",["founders_endowment"],"buy",0],"Ashen Dividend":["ashen_dividend",["first_wish"],"buy",0],"The Patron's Tithe":["patrons_tithe",["ashen_dividend"],"tithe",0],"Conscription Papers":["conscription_papers",["patrons_tithe"],"buy",0],"Strength in Numbers":["strength_in_numbers",["conscription_papers"],"buy",0],"The Wide Board":["the_wide_board",["conscription_papers"],"buy",0],"The Rejuvenator":["rejuvenator",["conscription_papers"],"buy",0],"Pact of the Patron":["pact_of_the_patron",["conscription_papers"],"buy",0],"Divine Payroll":["divine_payroll",["pact_of_the_patron"],"buy",0],"Covenant of the Patron":["covenant_of_the_patron",["divine_payroll"],"buy",0],"The Maelstrom Turns Again":["maelstrom_turns",["covenant_of_the_patron"],"buy",0],"Interest on the Wish":["wish_interest",["patrons_tithe"],"buy",0],"The Patron's Arithmetic":["patrons_arithmetic",["wish_interest"],"buy",0],"Shardbound":["shardbound",["patrons_arithmetic"],"buy",0],"The Patron Made Manifest":["patron_manifest",["shardbound"],"buy",0],"The Candle Snuffer":["steady_hand",["first_wish"],"buy",0],"The Mechanical Fist":["calloused_trigger",["steady_hand"],"buy",0],"Secured Souls":["weight_of_wrist",["calloused_trigger"],"buy",0],"Eyes on the Rooftops":["rooftop_eyes",["first_wish"],"buy",0],"Word on the Wire":["contacts_beyond_veil",["rooftop_eyes"],"buy",0],"Gilded Omens":["gilded_omens",["contacts_beyond_veil"],"buy",0],"The Reliquary":["the_reliquary",["gilded_omens"],"buy",0],"The Doorman's Introduction":["doormans_introduction",["weight_of_wrist","gilded_omens"],"buy",0],"Pact of the Hidden King":["pact_king",["doormans_introduction"],"pact",0],"Pact of the Archmother":["pact_mother",["doormans_introduction"],"pact",0],"Twin Idols":["twin_idols",["pact_king"],"buy",0],"The Wrath Made Regular":["wrath_made_regular",["twin_idols"],"buy",0],"The Sapphire Flame Tithe":["sapphire_flame_tithe",["pact_mother"],"buy",0],"The City Mother's Ledger":["city_mothers_ledger",["sapphire_flame_tithe"],"buy",0],"The Cinderforge Commission":["set_infernus",["conscription_papers"],"buy",1],"The Rimewright's Bargain":["set_kelvin",["graveyard_shift"],"buy",1],"The Static Contract":["set_seven",["weight_of_wrist"],"buy",1],"The High Roost Retainer":["set_vindicta",["gilded_omens"],"buy",1],"The High-Roller's Marker":["set_wraith",["pact_king"],"buy",1],"The Sepulchre Rites":["set_geist",["pact_mother"],"buy",1],"The Treeline Contract":["set_grey_talon",["graveyard_shift"],"buy",1],"The Smoke Screen":["set_haze",["gilded_omens"],"buy",1],"The Singularity":["set_dynamo",["conscription_papers"],"buy",1],"The Bluebook":["set_abrams",["weight_of_wrist"],"buy",1],"The Card Trick":["set_wraith_hero",["conscription_papers"],"buy",1],"The Overflowing Urn":["overflowing_urn",["books_balance"],"buy",0],"The Patron's Estate":["patrons_estate",["standing_order"],"buy",0],"The Walkers' Muster":["walkers_muster",["covenant_of_the_patron"],"buy",0],"Usury Everlasting":["usury_eternal",["patron_manifest"],"buy",0],"Souls Secured Forever":["souls_secured_forever",["weight_of_wrist"],"buy",0],"The Threshold Kept":["the_threshold",["doormans_introduction"],"buy",0],"The Patron's Long Vigil":["patrons_long_vigil",["the_threshold"],"buy",0],"Debt Eternal":["debt_eternal",["the_threshold"],"buy",0],"Ledger of the Overworld":["ledger_of_the_overworld",["patrons_long_vigil","patron_manifest"],"buy",0],"The Second Wish":["the_second_wish",["debt_eternal"],"buy",0],"The Patron Ascendant":["the_patron_ascendant",["walkers_muster"],"buy",0],"Throne of Seven Storms":["throne_of_seven_storms",["wrath_made_regular"],"buy",0],"The Crown Eternal":["crown_eternal",["throne_of_seven_storms"],"buy",0],"The Archmother's Brood":["the_archmothers_brood",["city_mothers_ledger"],"buy",0],"Sovereign of the Sapphire Flame":["sovereign_of_sapphire",["the_archmothers_brood"],"buy",0],"The City That Never Sleeps":["sleepless_city",["overflowing_urn"],"buy",0],"The Back Room":["the_back_room",["sleepless_city"],"buy",0],"The Overworld Signed Over":["overworld_signed",["patrons_estate"],"buy",0],"The War Chest":["the_war_chest",["overworld_signed"],"buy",0],"The Hand That Takes All":["hand_takes_all",["souls_secured_forever"],"buy",0],"The Wish Compounded":["wish_compounded",["usury_eternal"],"buy",0],"The Long Ledger":["long_ledger",["wish_compounded"],"buy",0],"The Maelstrom Unbound":["maelstrom_unbound",["the_patron_ascendant"],"buy",0],"The Maelstrom Eternal":["the_maelstrom_eternal",["maelstrom_unbound"],"buy",0],"The Hand Unending":["the_hand_unending",["hand_takes_all"],"buy",0],"The Wish Eternal":["the_wish_eternal",["wish_compounded"],"buy",0],"The Sleepless Overworld":["the_sleepless_overworld",["sleepless_city"],"buy",0],"The Overworld Entire":["the_overworld_entire",["overworld_signed"],"buy",0],"The King's Ransom":["the_kings_ransom",["crown_eternal"],"buy",0],"A Head for Figures":["ind_numfmt",["steady_hand"],"buy",1],"Classic Tide Selection":["ind_tide",["whispered_ledger"],"buy",1],"The Glassblower's Sampler":["ind_core",["ashen_dividend"],"buy",1],"Basic Backdrop Assortment":["ind_backdrop",["patrons_advance"],"buy",1],"Season Switcher":["ind_season",["contacts_beyond_veil"],"buy",1],"The Kept Wish I":["perma_slot_1",["shopkeepers_tab"],"buy",0],"The Kept Wish II":["perma_slot_2",["perma_slot_1"],"buy",0],"The Kept Wish III":["perma_slot_3",["perma_slot_2"],"buy",0],"The Kept Wish IV":["perma_slot_4",["perma_slot_3"],"buy",0],"The Kept Wish V":["perma_slot_5",["perma_slot_4"],"buy",0],"Combined Arms":["combined_arms",["call_the_roster"],"buy",0],"Screen the Line":["screen_the_line",["combined_arms"],"buy",0],"Feed the Boss":["feed_the_boss",["screen_the_line"],"buy",0],"Press the Advantage":["press_the_advantage",["feed_the_boss"],"buy",0],"Crown the Lane":["crown_the_lane",["press_the_advantage"],"buy",0],"The High Lanes":["the_high_lanes",["crown_the_lane"],"buy",0],"The Chain Unbroken":["chain_unbroken",["the_high_lanes"],"buy",0],"The Second Line":["second_line",["strength_in_numbers"],"buy",0],"Deep Ranks":["deep_ranks",["second_line"],"buy",0],"Swell the Ranks":["swell_the_ranks",["deep_ranks"],"buy",0],"The Rejuvenated Ranks":["rejuvenated_ranks",["second_line"],"buy",0],"Cadence":["cadence",["rejuvenated_ranks"],"buy",0],"The Quickening":["the_quickening",["cadence"],"buy",0],"The Numberless Host":["numberless_host",["swell_the_ranks","cadence"],"buy",0],"The Full Roster":["full_roster",["unlock_ivy","unlock_billy","unlock_abrams","unlock_warden"],"buy",0],"The Hands of the Host":["hands_of_the_host",["hand_takes_all"],"buy",0],"The Brood Unnumbered":["the_brood_unnumbered",["sovereign_of_sapphire"],"buy",0]};
  const TREE_ID = Object.fromEntries(Object.entries(TREE).map(([name, [id]]) => [id, name]));

  /** What the last run looked like (the tree shows after it's reset). */
  let lastRun = { shares: {}, counts: {}, clickShare: 0 };
  function rememberRun() {
    const e = economy();
    if (!e.production) return;
    const shares = {};
    const counts = {};
    for (const o of e.gens) {
      shares[o.name] = (o.owned * o.each * e.boost) / e.production;
      counts[o.name] = o.owned;
    }
    lastRun = { shares, counts, clickShare: e.earn > 0 ? e.clickIncome / e.earn : 0, clickRate: e.clickRate };
  }

  const names = (list) =>
    list
      .replace(/ and /g, ", ")
      .replace(/\bthe /gi, "")
      .split(/,\s*/)
      .map((n) => n.trim())
      .filter(Boolean);

  /** A node's worth, in "% more output" (or close to it). */
  function nodeValue(effect, t) {
    let v = 0;
    let m;
    const share = (list) => names(list).reduce((sum, n) => sum + (lastRun.shares[n] || lastRun.shares[n.replace(/s$/, "")] || 0), 0);
    const sources = Object.keys(lastRun.counts);
    if ((m = effect.match(/All output \+(\d+)%/))) v += +m[1];
    if ((m = effect.match(/Every source produces (?:a further )?\+(\d+)%/))) v += +m[1];
    if ((m = effect.match(/Output \+(\d+)% for every Ritual/))) v += +m[1] * t.rituals;
    if ((m = effect.match(/Each Tithe adds \+(\d+)% output/))) v += +m[1];
    if (/^Everything you own produces double/.test(effect)) v += 100;
    if (/^Everything you own produces triple/.test(effect)) v += 200;
    // A better rate for every shard ever earned.
    if ((m = effect.match(/pays (\d)% output/))) v += Math.max(0, t.bonus * (+m[1] / t.perShard - 1));
    // More shards from every future Ritual: worth a share of that.
    if ((m = effect.match(/(\d+)% more (?:Patron )?Shards|further \+?(\d+)% (?:more )?Patron Shards/))) v += +(m[1] || m[2]) * 0.4;
    if ((m = effect.match(/^(.+?) (?:each )?produce (\d+)x/))) v += share(m[1]) * (+m[2] - 1) * 100;
    if ((m = effect.match(/^(.+?) produce (?:a further )?\+(\d+)%/)) && !/^Every/.test(effect)) v += share(m[1]) * +m[2];
    if ((m = effect.match(/Any source you own (\d+) or more of produces (double|triple)/))) {
      const big = sources.filter((n) => lastRun.counts[n] >= +m[1]).reduce((sum, n) => sum + lastRun.shares[n], 0);
      v += big * (m[2] === "double" ? 100 : 200);
    }
    if ((m = effect.match(/\+([\d.]+)% output for each 10 of it you own/))) {
      v += sources.reduce((sum, n) => sum + lastRun.shares[n] * (lastRun.counts[n] / 10) * parseFloat(m[1]), 0);
    }
    if ((m = effect.match(/\+([\d.]+)% to your TOTAL output for every (.+?)(?: you own|\.|$)/))) {
      v += names(m[2]).reduce((sum, n) => sum + (lastRun.counts[n] || lastRun.counts[n.replace(/s$/, "")] || 0), 0) * parseFloat(m[1]);
    }
    if (/^(Begin|Start) (each|every) Ritual/.test(effect)) v += 6;
    if (/^Recruit |Opens the Roster/.test(effect)) v += 5;
    if (/^Opens |Opens the/.test(effect)) v += 1;
    // Nodes that are about clicking / statues / away time are worth only
    // what they say for those; never the "unknown" default below.
    let known = v > 0;
    if (/[Cc]lick/.test(effect) && !/statue/i.test(effect)) {
      known = true;
      // Worthless on an Empty Hand run or when nothing is clicking.
      const cs = t.locked ? 0 : lastRun.clickShare;
      if ((m = effect.match(/(\d+)% more/))) v += cs * +m[1];
      if (/doubled/.test(effect)) v += cs * 100;
      if (/tripled/.test(effect)) v += cs * 200;
      if ((m = effect.match(/x(\d+)/))) v += cs * (+m[1] - 1) * 100;
      if ((m = effect.match(/secures? (\d+)% of your souls per second/)) && !t.locked) v += (lastRun.clickRate || 0) * +m[1];
    }
    if (/[Ss]tatue|blessing|Lucky Souls|Trigger Frenzy/.test(effect) && !v) {
      known = true;
      v += cfg.favors ? 2 : 0;
    }
    if (/Away earnings|Leech/.test(effect) && !v) {
      known = true;
      v += 0.3;
    }
    if (/costs 10% less/.test(effect)) v += 2;
    if (/last run's upgrades|Keep (your|every)/.test(effect)) v += 2;
    if (!v && !known) v = 0.5;
    return v;
  }

  let treeBusy = false;
  /** Spend shards on the best-value nodes. Saves instead when something much
   * better is within reach. */
  async function spendTree() {
    if (treeBusy) return;
    treeBusy = true;
    try {
      for (let round = 0; round < 60; round++) {
        const wrap = $(".asc-wrap.on");
        if (!wrap) break;
        const page = wrap.innerText || "";
        const held = parseNum((page.match(/shards to spend\s*([\d.,]+(?: [a-z]+)?)/i) || [])[1]) || 0;
        const rituals = parseInt((page.match(/rituals completed\s*(\d+)/i) || [])[1] || "0", 10);
        const bonus = parseNum((page.match(/permanent \+([\d.,]+(?: [a-z]+)?)% output/i) || [])[1]) || 0;
        const nodes = $$(".asc-wrap.on button.tnode").map((b) => {
          // "The Patron's Tithe (3 paid). ..." is repeatable; drop the count.
          const raw = b.getAttribute("aria-label") || "";
          const paid = parseInt((raw.match(/ \((\d+) paid\)/) || [])[1] || "0", 10);
          const label = raw.replace(/ \(\d+ paid\)/, "");
          const name = Object.keys(TREE).find((n) => label.startsWith(n + ". ")) || "";
          const cost = parseNum((label.match(/ ([\d.,]+(?: [a-z]+)?) shards?\.$/) || [])[1]);
          const effect = name ? label.slice(name.length + 2).replace(/ [\d.,]+(?: [a-z]+)? shards?\.$/, "") : label;
          const cls = b.className;
          return { b, name, cost, effect, paid, own: /\bown\b/.test(cls), can: /\bcan\b/.test(cls), locked: /\blocked\b/.test(cls) };
        });
        // A Tithe paid at least once counts as done for what it unlocks.
        const owned = new Set(nodes.filter((n) => n.own || n.paid > 0).map((n) => TREE[n.name]?.[0]));
        const perShard = owned.has("ledger_of_the_overworld") ? 5 : owned.has("usury_eternal") ? 4 : owned.has("patron_manifest") ? 3 : 2;
        const t = { rituals, bonus, perShard, locked: clicksLocked() };
        const value = new Map(nodes.map((n) => [n, n.name && !TREE[n.name][3] ? nodeValue(n.effect, t) : 0]));
        // A node that opens something good is worth part of that too.
        for (const n of nodes) {
          if (n.own || !n.name) continue;
          const id = TREE[n.name][0];
          for (const child of nodes) {
            const def = child.name && TREE[child.name];
            // Only while that child is still locked behind this node.
            if (!def || child.own || !child.locked || !def[1].includes(id)) continue;
            if (def[1].every((r) => r === id || owned.has(r))) value.set(n, value.get(n) + 0.3 * value.get(child));
          }
        }
        const open = nodes.filter((n) => !n.own && !n.locked && n.cost > 0 && value.get(n) > 0);
        window.__hesyTree = nodes.filter((n) => !n.own).map((n) => [n.name, n.cost, +value.get(n).toFixed(2), n.locked ? "locked" : n.can ? "can" : "open"]);
        const ratio = (n) => value.get(n) / n.cost;
        open.sort((a, b) => ratio(b) - ratio(a));
        const best = open[0];
        const affordable = open.filter((n) => n.can && n.cost <= held);
        const pick = affordable[0];
        if (!pick) {
          stats.plan = best ? "Tree: saving for " + best.name + " (" + best.cost + " shards)" : "";
          break;
        }
        // Save up when the best node is clearly better and not far off.
        // Shards keep coming every Ritual, so saving for something far better
        // beats spending on weak nodes, unless the weak one is pocket change.
        if (best !== pick && ratio(pick) < 0.5 * ratio(best) && pick.cost > held * 0.02) {
          stats.plan = "Tree: saving for " + best.name + " (" + best.cost + " shards, have " + held + ")";
          break;
        }
        press(pick.b);
        stats.treeNodes = (stats.treeNodes || 0) + 1;
        stats.last = "Tree: bought " + pick.name;
        await sleep(250);
      }
    } finally {
      treeBusy = false;
    }
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
    if (cfg.enabled && cfg.treeBuy && $(".asc-wrap.on") && !treeBusy && tickN++ % 16 === 0) spendTree();
    if (!cfg.enabled || onTree()) return;
    catchPopups();
    tickN++;
    // Shopping a few times a second is plenty.
    if (tickN % 2 === 0) {
      if (cfg.buyGens || cfg.buyUpgrades) buyBest();
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

  window.__hesyHelper = { stats, readStatus, ritualNumbers, parseNum, spendTree, economy, shopItems, itemGain, get cfg() { return cfg; }, get connected() { return connected; } };
  setInterval(tick, 125);
  orbLoop();
  sync();
})();
