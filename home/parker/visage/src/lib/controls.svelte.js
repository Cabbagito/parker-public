// ^, — the controls, in visage (docs/visage/controls/README.md).
//
// The list is parkerd's (`parker controls`, door's `controls`): every
// setting as a row — what it is, what it is now, how it changes. This file
// is the overlay's state and its keys; Controls.svelte draws it and App
// opens it. The keys are vim's, in four modes:
//
//   NORMAL  where you land. j k move (3j), h l the other column, gg G the
//           ends, ^N ^P as in ^K, ⏎ change the row or open a group, space
//           flip a toggle or play/pause, m mute a sound row, / find, i ask,
//           ? the keys
//   EDIT    ⏎ on a row: h l −1 +1 or the next choice, H L −10 +10, digits
//           type a level, m mute, ⏎ keep, esc put it back. The session is
//           ⏎ ⏎: the second press is parker asking first.
//   FIND    / : typing narrows the rows; ⏎ jumps to the first, or runs a
//           whole command — `volume 30`, `volume spotify 20`, `wifi off`
//   ASK     i : typing goes to parker, sent marked as from ^, with the row
//           you are on; ⏎ sends, esc comes back
//
// Calls the design made (the README has why): `i` to talk, because in
// NORMAL most letters are keys; a toggle flips on ⏎ without EDIT, since two
// states need one press; esc never closes ^, — only ^, does (App); arrow
// keys are not bound.
//
// A level is set as it moves, so you hear it; everything else is set when
// the edit is kept. While ^, is up the list is read again every two
// seconds, and a row that moved without this page moving it says so. A
// level the keys on the desk moved is heard at once, without waiting for
// the poll (door's `watch_levels`, `heard`), and its row says it was them.

import { app, notice, send, pickTheme, previewTheme, pickBrain } from './state.svelte.js';
import { engine } from './engine.js';
import { inTauri } from './shell.js';
import { THEMES } from './themes.js';

export const LEFT = ['sound', 'playing', 'network', 'bluetooth'];
export const RIGHT = ['look', 'quiet', 'parker', 'session'];

/// What the second ⏎ on the session does, said before it does it.
const SESSION_SAY = {
  lock: '⏎ again locks the screen.',
  sleep: '⏎ again and I sleep.',
  reboot: '⏎ again and I reboot. back in a minute.',
  off: '⏎ again turns me off. esc and I stay.',
};

const POLL_MS = 2000;
/// A row this page changed is not "changed elsewhere" for this long: the
/// list read straight after a change can still be the old one.
const MINE_MS = 4000;
const MOVED_MS = 5000;

/// The porthole is not at the tower: tower-only rows are read-only there,
/// and parkerd refuses them anyway (door's `set_control`, `away`).
export const away = !inTauri;

export const ctl = $state({
  sections: null, // parkerd's list plus this screen's own row; null until read
  error: '',
  mode: 'normal', // normal | edit | find | ask
  sel: 'output',
  opened: {}, // a group's id → open
  filter: '',
  ask: '',
  editing: null, // the row in EDIT
  before: null, // its value when the edit began
  mutedBefore: null,
  pick: 0, // the session's lock, sleep, reboot or off
  armed: false,
  count: '',
  pendingG: false,
  help: false,
  flight: {}, // id → a change on its way
  failed: {}, // id → why it did not take
  moved: {}, // id → changed without this page
  keys: {}, // id → when door heard the desk's volume keys move it
  after: undefined, // ASK: the message the ask came after; its reply shows under the rows
});

// ── the list ────────────────────────────────────────────────────────────────

/// "this screen": visage's own look, per screen and the page's alone, so it
/// is added here rather than read from parkerd.
function withScreen(sections) {
  const row = {
    id: 'screen',
    label: 'this screen',
    kind: 'choice',
    value: app.theme,
    choices: THEMES.map((t) => t.name),
    hint: `${THEMES.length} looks`,
    local: true,
  };
  const look = sections.find((s) => s.id === 'look');
  if (look) look.rows.push(row);
  else sections.splice(4, 0, { id: 'look', name: 'LOOK', rows: [row] });
  return sections;
}

/// Every row, children included, each knowing its section and parent.
export function allRows() {
  const out = [];
  for (const s of ctl.sections ?? []) {
    for (const r of s.rows) {
      out.push({ r, section: s, parent: null });
      for (const c of r.children ?? []) out.push({ r: c, section: s, parent: r });
    }
  }
  return out;
}

export const rowOf = (id) => allRows().find((x) => x.r.id === id)?.r ?? null;
const parentOf = (id) => allRows().find((x) => x.r.id === id)?.parent ?? null;

/// The words a row answers to in FIND.
function hay(r, s) {
  return [r.label, r.sub, s.name, r.slash, ...(r.choices ?? []), typeof r.value === 'string' ? r.value : '']
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
}

/// The words of a find that narrow: a level (`20`, `+5`) is what to set,
/// not something to look for.
const terms = (filter) =>
  filter
    .toLowerCase()
    .replace(/^\//, '')
    .split(/\s+/)
    .filter((w) => w && !/^[+-]?\d+%?$/.test(w));

/// Sections as drawn: an open group's rows under it, and with a filter only
/// the rows it matches — a matching child shows even while its group is
/// shut, under its group.
export function shown(ids) {
  const words = terms(ctl.filter);
  const fits = (r, s) => words.every((w) => hay(r, s).includes(w));
  return (ctl.sections ?? [])
    .filter((s) => !ids || ids.includes(s.id))
    .map((s) => {
      const rows = [];
      for (const r of s.rows) {
        const kids = (r.children ?? []).filter((c) => !words.length || fits(c, s));
        const open = !!ctl.opened[r.id] || (words.length > 0 && kids.length > 0);
        if (!words.length || fits(r, s) || kids.length) rows.push({ r, child: false, open });
        if (r.children && open) for (const c of kids) rows.push({ r: c, child: true, parent: r });
      }
      return { id: s.id, name: s.name, rows };
    })
    .filter((s) => s.rows.length);
}

/// The two columns as lists of ids, in screen order: what j k walk.
function columns() {
  return [LEFT, RIGHT].map((ids) => shown(ids).flatMap((s) => s.rows.map((x) => x.r.id)));
}

function where() {
  const cols = columns();
  for (let c = 0; c < cols.length; c++) {
    const i = cols[c].indexOf(ctl.sel);
    if (i >= 0) return { cols, c, i };
  }
  return { cols, c: 0, i: -1 };
}

/// Keep the cursor on a row that is on screen.
function settle() {
  const { cols, i } = where();
  if (i < 0) ctl.sel = cols[0][0] ?? cols[1][0] ?? ctl.sel;
}

function move(d) {
  const { cols, c, i } = where();
  const col = cols[c];
  if (col.length) ctl.sel = col[Math.max(0, Math.min(col.length - 1, Math.max(0, i) + d))];
}

function side(d) {
  const { cols, c, i } = where();
  const t = Math.max(0, Math.min(cols.length - 1, c + d));
  if (t !== c && cols[t].length) ctl.sel = cols[t][Math.min(Math.max(0, i), cols[t].length - 1)];
}

function edge(d) {
  const { cols, c } = where();
  const col = cols[c];
  if (col.length) ctl.sel = d > 0 ? col.at(-1) : col[0];
}

// ── reading ─────────────────────────────────────────────────────────────────

let timer = null;
const mine = {}; // id → when this page last changed it

export async function refresh() {
  let got;
  try {
    got = await engine.controls();
  } catch (e) {
    ctl.error = String(e);
    return;
  }
  if (!got?.sections) {
    ctl.error = 'parker controls did not answer';
    return;
  }
  const next = withScreen(got.sections);
  const old = new Map(allRows().map((x) => [x.r.id, x.r]));
  const now = Date.now();
  for (const s of next) {
    for (const r of [...s.rows, ...s.rows.flatMap((x) => x.children ?? [])]) {
      const was = old.get(r.id);
      if (!was) continue;
      // The row in EDIT is the edit's until it is kept or put back.
      if (r.id === ctl.editing) {
        r.value = was.value;
        r.muted = was.muted;
        continue;
      }
      const moved =
        ['slider', 'toggle', 'choice', 'action'].includes(r.kind) &&
        !ctl.flight[r.id] &&
        now - (mine[r.id] ?? 0) > MINE_MS &&
        (was.value !== r.value || !!was.muted !== !!r.muted);
      if (moved) {
        ctl.moved[r.id] = now;
        setTimeout(() => {
          if (ctl.moved[r.id] === now) delete ctl.moved[r.id];
        }, MOVED_MS);
      }
    }
  }
  ctl.sections = next;
  ctl.error = '';
  settle();
}

/// door's `levels` event: a level moved by something other than parker or
/// ^,. While ^, is up the list is read now rather than at the next poll,
/// and a row the keys moved says so (Controls.svelte).
export function heard(v) {
  if (!app.controls) return;
  if (v?.keys) for (const c of v.changes ?? []) ctl.keys[c.id] = Date.now();
  refresh();
}

/// Whether the keys on the desk are what last moved this row.
export const byKeys = (id) => Date.now() - (ctl.keys[id] ?? 0) < MOVED_MS;

export function openControls() {
  app.controls = true;
  Object.assign(ctl, { mode: 'normal', help: false, count: '', pendingG: false, after: undefined });
  refresh();
  clearInterval(timer);
  timer = setInterval(refresh, POLL_MS);
}

export function closeControls() {
  if (ctl.mode === 'edit') undo(true);
  previewTheme(null);
  Object.assign(ctl, { mode: 'normal', help: false, filter: '', ask: '' });
  app.controls = false;
  clearInterval(timer);
  timer = null;
}

// ── changing ────────────────────────────────────────────────────────────────

/// One change through door. The row shows it in flight, says what changed
/// in the notice line, and on a refusal puts the list back as parkerd has it.
async function change(r, value, { quiet = false } = {}) {
  mine[r.id] = Date.now();
  ctl.flight[r.id] = true;
  delete ctl.failed[r.id];
  try {
    const c = await engine.setControl(r.id, String(value));
    mine[r.id] = Date.now();
    if (!quiet && c?.said) notice(c.said);
    return c;
  } catch (e) {
    ctl.failed[r.id] = String(e);
    notice(`${r.label}: ${e}`, 9000);
    await refresh();
    return null;
  } finally {
    delete ctl.flight[r.id];
  }
}

// A level being edited is set as it moves, one call at a time: the latest
// value waits while one is on its way, and the ones in between are skipped.
const pending = {};
const running = {};
function queue(r) {
  pending[r.id] = r.value;
  mine[r.id] = Date.now();
  if (running[r.id]) return;
  running[r.id] = (async () => {
    while (pending[r.id] != null) {
      const v = pending[r.id];
      pending[r.id] = null;
      try {
        await engine.setControl(r.id, String(v));
        mine[r.id] = Date.now();
      } catch (e) {
        ctl.failed[r.id] = String(e);
        notice(`${r.label}: ${e}`, 9000);
        break;
      }
    }
    running[r.id] = null;
  })();
}

const isSound = (r) => r?.kind === 'slider' && (r.id === 'output' || r.id === 'input' || r.id.startsWith('app:'));
const lo = (r) => r.min ?? 0;
const hi = (r) => r.max ?? 100;
const clamp = (r, v) => Math.max(lo(r), Math.min(hi(r), v));

/// Whether this row, or this one of its choices, is the tower's alone and
/// the hand is not at the tower.
export function locked(r, choice) {
  if (!away || !r?.tower) return false;
  if (r.tower === true) return true;
  return Array.isArray(r.tower) && choice != null && r.tower.includes(choice);
}

function refuse(r) {
  notice(`${r.label} is the tower's: from here it would cut the line you are on`);
}

function flip(r) {
  if (locked(r)) return refuse(r);
  if (r.why) return notice(r.why);
  const on = !r.value;
  r.value = on;
  change(r, on ? 'on' : 'off').then((c) => {
    if (!c) r.value = !on;
  });
}

function mute(r) {
  if (r?.id === 'mute') return flip(r);
  if (!isSound(r)) return;
  const on = !r.muted;
  r.muted = on;
  change(r, on ? 'mute' : 'unmute').then((c) => {
    if (!c) r.muted = !on;
  });
}

function transport(action) {
  const r = rowOf('track');
  if (!r || r.why) return notice(r?.why ?? 'nothing is playing');
  if (action === 'toggle') r.value = r.value === 'playing' ? 'paused' : 'playing';
  change(r, action, { quiet: true }).then((c) => c && setTimeout(refresh, 400));
}

function act(r) {
  if (r.why) return notice(r.why);
  const to = r.value === 'connected' ? 'disconnect' : 'connect';
  notice(`${to === 'connect' ? 'connecting' : 'letting go of'} the ${r.label} …`, 0);
  change(r, to).then((c) => {
    if (c) r.value = c.to;
  });
}

function enter() {
  const r = rowOf(ctl.sel);
  if (!r) return;
  if (r.kind === 'group') {
    ctl.opened[r.id] = !ctl.opened[r.id];
    return;
  }
  if (r.kind === 'toggle') return flip(r);
  if (r.kind === 'action') return act(r);
  if (r.kind === 'info') return r.value && notice(String(r.value), 8000);
  if (locked(r)) return refuse(r);
  if (r.why && r.kind !== 'session') return notice(r.why);
  ctl.mode = 'edit';
  ctl.editing = r.id;
  ctl.before = r.value;
  ctl.mutedBefore = r.muted ?? null;
  ctl.pick = 0;
  ctl.armed = false;
  typed = '';
}

function quick() {
  const r = rowOf(ctl.sel);
  if (r?.kind === 'toggle') return flip(r);
  if (r?.kind === 'transport') return transport('toggle');
  enter();
}

/// The choices h and l walk: the tower's own skipped from the porthole.
function walk(r) {
  return (r.choices ?? []).filter((c) => !locked(r, c));
}

function adjust(r, d) {
  if (r.kind === 'slider') {
    r.value = clamp(r, (Number(r.value) || 0) + d);
    if (r.live) queue(r);
  } else if (r.kind === 'session') {
    ctl.pick = Math.max(0, Math.min(r.choices.length - 1, ctl.pick + Math.sign(d)));
    while (locked(r, r.choices[ctl.pick]) && ctl.pick > 0) ctl.pick -= 1;
    ctl.armed = false;
  } else if (r.kind === 'choice') {
    const list = walk(r);
    if (!list.length) return;
    const i = list.indexOf(r.value);
    r.value = list[(Math.max(0, i) + Math.sign(d) + list.length) % list.length];
    // A look is worn while it is chosen, as /visage does in ^K.
    if (r.id === 'screen') previewTheme(r.value);
  } else if (r.kind === 'transport') {
    transport(d > 0 ? 'next' : 'previous');
  }
}

async function keep() {
  const r = rowOf(ctl.editing);
  if (!r) return leave();
  if (r.kind === 'session') {
    const what = r.choices[ctl.pick];
    if (locked(r, what)) return refuse(r);
    if (!ctl.armed) {
      ctl.armed = true;
      notice(SESSION_SAY[what] ?? '⏎ again.', 0);
      return;
    }
    leave();
    change(r, what);
    return;
  }
  const was = ctl.before;
  leave();
  if (r.kind === 'transport' || r.value === was) {
    if (r.kind === 'slider' && r.live) notice(`${r.label} ${r.value}`);
    return;
  }
  if (r.id === 'screen') {
    pickTheme(r.value);
    previewTheme(null);
    return;
  }
  if (r.id === 'brain') {
    pickBrain(r.value).then(refresh);
    return;
  }
  if (r.kind === 'slider' && r.live) {
    notice(`${r.label} ${was} → ${r.value}`);
    return;
  }
  const slow = r.id === 'family' || r.id === 'glass';
  if (slow) notice(`${r.label} ${was} → ${r.value} · a rebuild, about half a minute`, 0);
  const c = await change(r, r.value);
  if (!c) r.value = was;
}

/// esc in EDIT: everything back as it was when the edit began.
function undo(silent = false) {
  const r = rowOf(ctl.editing);
  const was = ctl.before;
  const mutedWas = ctl.mutedBefore;
  leave();
  if (!r) return;
  if (r.kind === 'session') {
    if (!silent) notice('staying up');
    return;
  }
  if (r.id === 'screen') previewTheme(null);
  if (r.value !== was) {
    r.value = was;
    if (r.kind === 'slider' && r.live) queue(r);
  }
  if (mutedWas != null && !!r.muted !== !!mutedWas) mute(r);
  if (!silent && r.kind !== 'transport') notice(`${r.label} kept at ${r.kind === 'toggle' ? (was ? 'on' : 'off') : was}`);
}

function leave() {
  Object.assign(ctl, { mode: 'normal', editing: null, armed: false });
}

// ── finding ─────────────────────────────────────────────────────────────────

function refilter(f) {
  ctl.filter = f;
  settle();
}

/// A whole command, as words: the longest slash name or label they start
/// with is the row, and the rest is the value — `volume spotify 20`,
/// `volume 30`, `wifi off`, `spotify mute`, `connect wireless
/// controller`. A toggle, a device or the track with no value flips.
function command(text) {
  const words = text.trim().replace(/^\//, '').split(/\s+/).filter(Boolean);
  const lower = words.map((w) => w.toLowerCase());
  let best = null;
  for (const { r } of allRows()) {
    if (r.kind === 'info' || r.kind === 'group') continue;
    const flips = ['toggle', 'action', 'transport'].includes(r.kind);
    for (const name of [r.slash, r.label]) {
      if (!name) continue;
      const n = name.toLowerCase().split(/\s+/);
      const fits = (n.length < lower.length || (flips && n.length === lower.length)) && n.every((w, i) => lower[i] === w);
      if (fits && (!best || n.length > best.n)) best = { r, n: n.length };
    }
  }
  if (!best) return null;
  const value = words.slice(best.n).join(' ') || 'toggle';
  return { r: best.r, value };
}

/// Run a command made by `command`, as ^, would: the row's own path.
function run({ r, value }) {
  if (r.local || r.id === 'brain') {
    const hit = (r.choices ?? []).find((c) => c.toLowerCase().startsWith(value.toLowerCase()));
    if (!hit) return notice(`${r.label} has no ${value}`);
    if (r.id === 'screen') pickTheme(hit);
    else pickBrain(hit).then(refresh);
    return;
  }
  if (locked(r, value) || (r.tower === true && away)) return refuse(r);
  const group = parentOf(r.id);
  if (group) ctl.opened[group.id] = true;
  change(r, value).then(refresh);
}

/// ^K's machine commands (commands.js): `/volume spotify 20` picked there
/// runs here, so a command and its row can never disagree.
export async function runSlash(verb, arg) {
  if (!ctl.sections) await refresh();
  const c = command(`${verb} ${arg ?? ''}`);
  if (!c) return notice(`no control answers /${verb} ${arg ?? ''}`.trim());
  run(c);
}

/// ⏎ in FIND: a whole command runs (`command`); anything else jumps to the
/// first row it matches.
function runFind() {
  const c = command(ctl.filter);
  if (c) {
    Object.assign(ctl, { mode: 'normal', filter: '', sel: c.r.id });
    run(c);
    return;
  }
  const ids = columns().flat();
  ctl.mode = 'normal';
  if (ids.length) {
    if (!ids.includes(ctl.sel)) ctl.sel = ids[0];
  } else {
    notice('nothing matches — i asks parker');
  }
}

// ── asking ──────────────────────────────────────────────────────────────────

/// ⏎ in ASK. It goes to parker in the thread on screen, with a first line
/// he reads and you do not (state.svelte.js `visible` strips it): that it
/// came from ^, and which row you were on. His answer lands under the rows,
/// in the transcript's own grammar, renders and all.
function sendAsk() {
  const text = ctl.ask.trim();
  ctl.mode = 'normal';
  ctl.ask = '';
  if (!text) return;
  const r = rowOf(ctl.sel);
  const on = r ? ` on the row "${r.label}" (${r.id}${valueOf(r)})` : '';
  const context = `⟦parker:controls⟧ Peter is asking from ^, — the controls in visage —${on}. Your controls tool reaches every row he can see.`;
  ctl.after = app.messages.at(-1)?.id ?? null;
  send(text, context);
}

function valueOf(r) {
  if (r.kind === 'slider') return `, at ${r.value}${r.muted ? ', muted' : ''}`;
  if (r.kind === 'toggle') return `, ${r.value ? 'on' : 'off'}`;
  if (r.kind === 'choice' || r.kind === 'action' || r.kind === 'transport') return `, ${r.value}`;
  return '';
}

/// The turn the last ask started, for under the rows.
export function replies() {
  if (ctl.after === undefined) return [];
  const i = ctl.after === null ? -1 : app.messages.findIndex((m) => m.id === ctl.after);
  return i < 0 && ctl.after !== null ? app.messages.slice(-2) : app.messages.slice(i + 1);
}

// ── keys ────────────────────────────────────────────────────────────────────

let typed = '';
let typedAt = 0;

/// The overlay's keys, in whatever mode it is in. Returns whether the key
/// was its: App lets everything else through (^SPACE, ^K, ^,).
export function key(e) {
  const k = e.key;
  const stop = () => {
    e.preventDefault();
    e.stopPropagation();
    return true;
  };
  if (e.metaKey || e.altKey) return false;

  if (ctl.help) {
    if (k === '?' || k === 'Escape' || k === 'q') {
      ctl.help = false;
      return stop();
    }
    return !e.ctrlKey && stop();
  }

  if (ctl.mode === 'edit') {
    const r = rowOf(ctl.editing);
    if (!r) {
      leave();
      return stop();
    }
    if (e.ctrlKey) return false;
    const fn = { Escape: undo, Enter: keep, m: () => mute(r), '?': () => (ctl.help = true) }[k];
    if (fn) {
      fn();
      return stop();
    }
    if (k === ' ' && r.kind === 'transport') {
      transport('toggle');
      return stop();
    }
    if (k === 'h' || k === 'l' || k === 'H' || k === 'L') {
      const step = k === 'H' || k === 'L' ? 10 : 1;
      adjust(r, (k.toLowerCase() === 'l' ? 1 : -1) * step);
      return stop();
    }
    if (/^[0-9]$/.test(k) && r.kind === 'slider') {
      const now = Date.now();
      typed = now - typedAt < 1200 ? (typed + k).slice(-3) : k;
      typedAt = now;
      r.value = clamp(r, parseInt(typed, 10));
      if (r.live) queue(r);
      return stop();
    }
    return stop();
  }

  // FIND and ASK type into the line at the foot (Controls.svelte), which
  // hands its ⏎, esc and empty ⌫ here.
  if (ctl.mode === 'find') {
    if (k === 'Escape') {
      Object.assign(ctl, { mode: 'normal', filter: '' });
      settle();
      return stop();
    }
    if (k === 'Enter') {
      runFind();
      return stop();
    }
    if (k === 'Backspace' && !ctl.filter) {
      ctl.mode = 'normal';
      return stop();
    }
    return false;
  }
  if (ctl.mode === 'ask') {
    if (k === 'Escape') {
      Object.assign(ctl, { mode: 'normal', ask: '' });
      return stop();
    }
    if (k === 'Enter' && !e.shiftKey) {
      sendAsk();
      return stop();
    }
    return false;
  }

  // NORMAL
  if (e.ctrlKey) {
    if (k !== 'n' && k !== 'p') return false;
    move(k === 'n' ? 1 : -1);
    return stop();
  }
  if (/^[1-9]$/.test(k) || (k === '0' && ctl.count)) {
    ctl.count = (ctl.count + k).slice(-2);
    ctl.pendingG = false;
    return stop();
  }
  const n = Math.max(1, parseInt(ctl.count || '1', 10));
  const done = (fn) => {
    ctl.count = '';
    ctl.pendingG = false;
    fn();
    return stop();
  };
  switch (k) {
    case 'j':
      return done(() => move(n));
    case 'k':
      return done(() => move(-n));
    case 'h':
      return done(() => side(-1));
    case 'l':
      return done(() => side(1));
    case 'G':
      return done(() => edge(1));
    case 'g':
      if (ctl.pendingG) return done(() => edge(-1));
      ctl.pendingG = true;
      return stop();
    case 'Enter':
      return done(enter);
    case ' ':
      return done(quick);
    case 'm':
      return done(() => mute(rowOf(ctl.sel)));
    case '?':
      return done(() => (ctl.help = true));
    case '/':
      return done(() => Object.assign(ctl, { mode: 'find', filter: '' }));
    case 'i':
      return done(() => Object.assign(ctl, { mode: 'ask', ask: '' }));
    case 'Escape':
      return done(() => {
        if (ctl.filter) refilter('');
        else if (ctl.after !== undefined) ctl.after = undefined;
        else notice('^, puts the controls away');
      });
  }
  // Any other key is not a key here: swallowed, so it never types into the
  // conversation hidden underneath.
  return k.length === 1 ? stop() : false;
}

/// FIND's line changed.
export function typeFind(text) {
  refilter(text);
}

/// A click or a tap on a row: the first selects it, the second is ⏎.
export function tap(id) {
  if (ctl.mode === 'edit' && ctl.editing !== id) undo(true);
  if (ctl.sel !== id) {
    ctl.sel = id;
    return;
  }
  if (ctl.mode === 'edit') keep();
  else enter();
}

/// A tap on one of a row's words — a choice, on or off, prev or next.
export function tapOption(r, word) {
  ctl.sel = r.id;
  if (r.kind === 'transport') return transport(word === 'prev' ? 'previous' : word === 'next' ? 'next' : 'toggle');
  if (locked(r, word) || locked(r)) return refuse(r);
  if (r.kind === 'toggle') {
    if ((word === 'on') !== !!r.value) flip(r);
    return;
  }
  if (r.kind === 'action') return act(r);
  if (r.kind === 'session') {
    if (ctl.mode !== 'edit' || ctl.editing !== r.id) enter();
    ctl.pick = r.choices.indexOf(word);
    keep();
    return;
  }
  if (r.kind === 'choice' && word !== r.value) {
    Object.assign(ctl, { mode: 'edit', editing: r.id, before: r.value });
    r.value = word;
    keep();
  }
}

/// A tap on a level's cells: that cell's level, kept.
export function tapLevel(r, frac) {
  ctl.sel = r.id;
  const to = clamp(r, Math.round(lo(r) + frac * (hi(r) - lo(r))));
  const was = r.value;
  r.value = to;
  change(r, to).then((c) => {
    if (!c) r.value = was;
  });
}

/// What the strip at the foot says: only what you need now; ? has the rest.
export function strip() {
  const K = (t, cls = '') => ({ t, cls });
  if (ctl.mode === 'edit') {
    const r = rowOf(ctl.editing);
    if (r?.kind === 'session') return [K('EDIT · SESSION', 'mode'), K('H L PICK'), K(ctl.armed ? '⏎ DO IT' : '⏎ ⏎ DO IT'), K('ESC NOT NOW')];
    if (r?.kind === 'transport') return [K('EDIT · TRACK', 'mode'), K('H PREV'), K('L NEXT'), K('SPACE PLAY'), K('⏎ DONE')];
    return [K(`EDIT · ${(r?.label ?? '').toUpperCase()}`, 'mode'), K('H L CHANGE'), K('⏎ KEEP'), K('ESC UNDO'), K('? KEYS')];
  }
  if (ctl.mode === 'find') return [K('FIND', 'mode'), K('⏎ JUMP · OR DO IT'), K('ESC CLEAR')];
  if (ctl.mode === 'ask') return [K('ASK', 'mode'), K('⏎ SEND'), K('ESC BACK')];
  const pend = ctl.count || (ctl.pendingG ? 'g' : '');
  return [
    K('NORMAL', 'mode'),
    ...(pend ? [K(pend, 'pending')] : []),
    K('J K MOVE'),
    K('⏎ CHANGE'),
    K('/ FIND'),
    K('I ASK'),
    K('? KEYS'),
  ];
}
