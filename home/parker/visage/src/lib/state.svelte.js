// One store, svelte 5 runes. Views read it; the engine module mutates it.
//
// Message and event shapes are handled defensively on purpose: opencode's
// API is mid-flight to v2 (message.part.delta arrived, deltas left
// message.part.updated), and the engine pin moves monthly. Anything this
// file cannot parse degrades to a refetch, never a crash — every input
// degrades to "I don't know", same as sense does.
//
// Chats live on the engine, not here. Ctrl+N forgets the session id and the
// next message mints a new one; the palette (^K) lists what the engine
// remembers and resuming is just adopting an old id. visage still stores
// nothing of its own: the threads' cards are door's, on parker.

import { engine } from './engine.js';
import { stir, setDance, playBeat } from './motion.svelte.js';
import { applyTheme, savedTheme } from './themes.js';

export const app = $state({
  view: 'default', // 'default' | 'session' — views.js
  engineStatus: 'connecting',
  sessionId: null,
  messages: [],
  busy: false, // a prompt is in flight
  lastEventAt: 0, // engine chatter about our session, for refetch + stir
  error: null,
  worn: null, // the expression parker chose, via `parker reach face`
  travel: 'river', // how a view change moves the face: 'river' | 'linear'
  palette: false, // the ^K palette is up (Palette.svelte) — the one overlay
  controls: false, // ^, is up (Controls.svelte, controls.svelte.js) — in the transcript's place
  notice: null, // a passing line at the bottom — notice()
  brain: null, // { name, model, default, brains } — `parker brain --json`
  theme: savedTheme(), // visage's own look, per screen — themes.js
});

// What the face shows: whatever parker wore, or nothing. No exceptions, and
// this function is the whole rule.
//
// The involuntary track is gone (DECISIONS.md, the 2026-08 redesign). visage
// no longer infers `thinking` or `speaking` from engine chatter — progress
// lives in the transcript (the act ledger and the streaming caret), and the
// face is entirely parker's, worn per beat through `reach face`.
//
// `trouble` on a disconnected engine was the last exception and it is gone
// too (Aug 2026). It was not even wrong about the engine — it was wrong
// about whose face it is. `pump_events` re-emits `disconnected` on every 2s
// reconnect attempt rather than on the transition, so one ~15s outage
// rewrote parker's face seven times while he was saying nothing. An engine
// that is down is a line of text (App.svelte's status strip), not an
// expression: the strip says it once, calmly, and keeps saying it as long as
// it is true. Do not add a branch here. If the face changed, parker changed
// it.
export function expression() {
  return app.worn ?? 'idle';
}

/// How often to re-read the two runtime files. Slower than a frame and
/// faster than a person notices; see the note on `face_state`.
const FACE_POLL_MS = 250;

/// A first line for parker and not for you. `⟦parker:controls⟧ …` rides on
/// what was asked from ^, (controls.svelte.js): where Peter was and on which
/// row. `⟦parker:woken⟧` carried "you just woke him"; nothing writes it
/// anymore — sleep is gone (idle.js) — but old threads resumed from the
/// palette still contain it. If vignettes ever want the same trick ("you
/// caught me reading"), this is the seam that already knows how.
const FOR_PARKER = '⟦parker:';

/// Strip anything meant for the engine rather than for you.
function visible(text) {
  if (!text.startsWith(FOR_PARKER)) return text;
  return text.slice(text.indexOf('\n') + 1).trimStart();
}

function textOf(message) {
  const parts = message?.parts ?? [];
  return visible(
    parts
      .filter((p) => p?.type === 'text' && typeof p.text === 'string')
      .map((p) => p.text)
      .join('\n'),
  );
}

function roleOf(message) {
  return message?.info?.role ?? message?.role ?? 'assistant';
}

/// The turn in reading order: prose, code, acts and gestures interleaved as
/// they happened, because "ran cargo test, then said this about it" and
/// "said this, then ran cargo test" are different sentences. Keys come from
/// the engine (callID / part id) so each block fades in exactly once; parts
/// this cannot parse are dropped, not fatal.
///
/// The grammar (docs/visage/redesign/README.md, "Blocks"):
///   text     prose, typed in behind a caret
///   code     a fenced block lifted out of the prose, landing whole
///   table    a markdown table, lifted out and landing whole the same way
///   act      one line per thing parker did — verb, target, magnitude,
///            never output
///   gesture  what he did with himself (dance). Face wears are dropped
///            entirely: WEARS is the one place the expression is named.
function blocksOf(message) {
  const blocks = [];
  for (const p of message?.parts ?? []) {
    if (p?.type === 'text' && typeof p.text === 'string') {
      const text = visible(p.text);
      if (!text.trim()) continue;
      const key = p.id ?? `t${blocks.length}`;
      blocks.push(...splitFences(text, key));
    } else if (p?.type === 'tool') {
      const b = actOf(p, p.callID ?? p.id ?? `c${blocks.length}`);
      if (b) blocks.push(b);
    }
  }
  return blocks;
}

/// Lift fenced code and tables out of prose so they land whole while the
/// prose types — a table typed cell by cell is a paragraph of pipes until
/// its delimiter row closes. The fence info string becomes the code block's
/// label (`nix path/to/file.nix`).
function splitFences(text, key) {
  const out = [];
  const fence = /^```([^\n]*)\n([\s\S]*?)^```[ \t]*$/gm;
  let last = 0;
  let n = 0;
  let m;
  const prose = (chunk) => {
    for (const piece of splitTables(chunk)) {
      if (piece.table) out.push({ kind: 'table', key: `${key}·${n++}`, text: piece.text });
      else if (piece.text.trim()) out.push({ kind: 'text', key: `${key}·${n++}`, text: piece.text.trim() });
    }
  };
  while ((m = fence.exec(text)) !== null) {
    prose(text.slice(last, m.index));
    const [lang, ...rest] = (m[1] ?? '').trim().split(/\s+/);
    out.push({
      kind: 'code',
      key: `${key}·${n++}`,
      lang: (lang || '').toLowerCase(),
      path: rest.join(' '),
      code: m[2].replace(/\n$/, ''),
    });
    last = m.index + m[0].length;
  }
  prose(text.slice(last));
  return out;
}

/// A table is a header row, a delimiter row with as many cells, and the rows
/// under it, each line starting with `|` — the shape every model writes. One
/// indented into a list or a quote stays in the prose, where marked still
/// sets it. Until its delimiter row arrives a header is only a line of text.
function splitTables(text) {
  const lines = text.split('\n');
  const piped = (line) => /^ {0,3}\|/.test(line ?? '');
  const out = [];
  let prose = [];
  for (let i = 0; i < lines.length; ) {
    if (piped(lines[i]) && piped(lines[i + 1]) && isDelimiter(lines[i], lines[i + 1])) {
      let j = i + 2;
      while (j < lines.length && piped(lines[j])) j++;
      if (prose.length) out.push({ text: prose.join('\n') });
      out.push({ table: true, text: lines.slice(i, j).join('\n') });
      prose = [];
      i = j;
    } else {
      prose.push(lines[i++]);
    }
  }
  if (prose.length) out.push({ text: prose.join('\n') });
  return out;
}

function cellsOf(line) {
  return line.trim().replace(/^\|/, '').replace(/\|$/, '').split('|');
}

function isDelimiter(head, line) {
  const cells = cellsOf(line);
  return cells.length === cellsOf(head).length && cells.every((c) => /^\s*:?-+:?\s*$/.test(c));
}

/// One engine tool part -> one ledger row (or a gesture, or nothing).
/// Everything here is best-effort: a magnitude that cannot be derived is
/// omitted, never guessed.
function actOf(p, key) {
  const name = (p.tool ?? 'tool').replace(/^parker_/, '');
  const i = p.state?.input ?? {};
  const out = typeof p.state?.output === 'string' ? p.state.output : '';
  const status = p.state?.status ?? 'completed';

  // The expressive verbs. The face is named by WEARS, never restated here;
  // dance is half the sentence and stays.
  if (name === 'face') return null;
  if (name === 'dance') {
    const r = i.routine ?? 'auto';
    return {
      kind: 'gesture',
      key,
      text: r === 'stop' ? 'stops dancing' : `dances (${r})`,
    };
  }

  const secs = duration(p.state?.time);
  const failed = status === 'error';
  const running = status !== 'completed' && !failed;
  const row = (glyph, verb, target, magnitude = '') => ({
    kind: 'act',
    key,
    glyph,
    verb,
    target,
    magnitude,
    status: failed ? 'failed' : running ? 'running' : 'ok',
    // Wall-clock bounds, for the fold row's run duration.
    start: p.state?.time?.start ?? null,
    end: p.state?.time?.end ?? null,
  });

  switch (name) {
    case 'bash': {
      let mag = secs;
      if (failed) mag = '';
      else if (!running && secs) mag += ' · ok';
      return row('$', 'ran', firstLine(i.command ?? i.cmd ?? ''), mag);
    }
    case 'read':
      return row('▤', 'read', pathOf(i), running || failed ? '' : lineCount(out));
    case 'edit':
      return row('±', 'edited', pathOf(i), diffCount(p.state));
    case 'write':
      return row('±', 'wrote', pathOf(i), diffCount(p.state));
    case 'grep':
    case 'glob':
      return row('⌕', 'searched for', firstLine(i.pattern ?? i.query ?? ''), hitCount(out));
    case 'search':
      return row('⌕', 'searched the web for', firstLine(i.query ?? ''), hitCount(out));
    case 'webfetch':
    case 'fetch':
      return row('↗', 'fetched', domainOf(i.url ?? ''), '');
    case 'sense':
      return row('⊙', 'sense', '', '');
    case 'look':
      return row('⊙', 'look', i.target ? `(${i.target})` : '', '');
    // A change to the machine is drawn, not only named (docs/visage/controls/
    // README.md, "parker's changes, drawn"): the verbs say what they moved
    // from and to — `spotify 45 → 20`, `volume 42% → 50%`, `moved to
    // workspace 2 (from 3)` — and the row carries it as `change`, which
    // Message.svelte draws under it as cells, a dot or the space numbers.
    case 'controls': {
      if (i.action !== 'set') return row('⊙', 'reads', 'the controls', '');
      const c = !running && !failed ? changeOf(out) : null;
      return { ...row('⊙', 'set', c?.label ?? i.id ?? '', secs), change: c };
    }
    case 'media': {
      const c = !running && !failed ? changeOf(out.replace(/^volume /, 'all sound ').replace(/%| \(muted\)/g, '')) : null;
      if (c) return { ...row('⊙', 'set', 'all sound', secs), change: c };
      return row('⊙', 'media:', i.action ?? '', '');
    }
    case 'notify':
      return row('⊙', 'notifies', firstLine(i.text ?? i.message ?? ''), '');
    case 'goto': {
      const m = /workspace (\d+)(?: \(from (\d+)\))?/.exec(out);
      const to = Number(m?.[1] ?? i.space);
      const change = m && !failed ? { kind: 'space', to, from: m[2] ? Number(m[2]) : null } : null;
      return { ...row('⊙', 'goes to', `space ${i.space ?? ''}`, ''), change };
    }
    case 'focus':
      return row('⊙', 'focuses', i.app ?? '', '');
    case 'show':
      return row('⊙', 'shows', firstLine(i.what ?? i.url ?? i.path ?? ''), '');
    default:
      return row('⊙', name, firstLine(firstString(i)), '');
  }
}

const firstLine = (s) => String(s).split('\n')[0].slice(0, 100);

/// `titanfall 2 100 → 30` → what changed and how to draw it: a level as
/// cells, a device's link as a dot, anything else as two words with the new
/// one lit. The label is everything before the last two words, so it may
/// have spaces of its own. Null when the line is not a change.
export function changeOf(line) {
  const m = /^(.+) (\S+) → (\S+)$/.exec(String(line).split('\n')[0].trim());
  if (!m) return null;
  const [, label, from, to] = m;
  const kind = /^\d+$/.test(from) && /^\d+$/.test(to)
    ? 'level'
    : to === 'connected' || from === 'connected'
      ? 'link'
      : 'word';
  return { kind, label, from: kind === 'level' ? Number(from) : from, to: kind === 'level' ? Number(to) : to };
}

function firstString(input) {
  for (const v of Object.values(input ?? {})) if (typeof v === 'string') return v;
  return '';
}

function pathOf(i) {
  const p = i.filePath ?? i.file_path ?? i.path ?? '';
  // Long absolute paths read as noise at ledger size; the tail carries it.
  return String(p).replace(/^\/home\/[^/]+\//, '~/');
}

function domainOf(url) {
  try {
    return new URL(url).hostname;
  } catch {
    return firstLine(url);
  }
}

function duration(time) {
  const start = time?.start;
  const end = time?.end;
  if (typeof start !== 'number' || typeof end !== 'number' || end <= start) return '';
  const s = (end - start) / 1000;
  return s < 10 ? `${s.toFixed(1)}s` : `${Math.round(s)}s`;
}

function lineCount(out) {
  if (!out) return '';
  const n = out.split('\n').length;
  return `${n} lines`;
}

function hitCount(out) {
  if (!out) return '';
  // Both grep output and web results are line-shaped enough to count.
  const m = out.match(/(\d+)\s+(results?|hits?|matches?|files?)/i);
  if (m) return `${m[1]} hits`;
  const n = out.split('\n').filter((l) => l.trim()).length;
  return n ? `${n} hits` : '';
}

/// `+n −m` from an edit/write, when the engine hands over enough to say.
function diffCount(state) {
  const meta = state?.metadata ?? {};
  const added = meta.additions ?? meta.added ?? meta.linesAdded;
  const removed = meta.deletions ?? meta.removed ?? meta.linesRemoved;
  const parts = [];
  if (typeof added === 'number' && added > 0) parts.push(`+${added}`);
  if (typeof removed === 'number' && removed > 0) parts.push(`−${removed}`);
  return parts.join(' ');
}

/// glyph -> the word for a pile of them: the fold row's tally (Message.svelte)
/// and the work on a thread's card.
export const NOUN = {
  $: 'ran',
  '▤': 'read',
  '±': 'edited',
  '⌕': 'searched',
  '↗': 'fetched',
  '⊙': 'reached',
};

/// Acts counted by kind, most first: `[{ glyph, noun, count }]`.
export function tallyOf(acts) {
  const n = new Map();
  for (const a of acts) n.set(a.glyph, (n.get(a.glyph) ?? 0) + 1);
  return [...n]
    .map(([glyph, count]) => ({ glyph, noun: NOUN[glyph] ?? 'did', count }))
    .sort((a, b) => b.count - a.count);
}

export function shaped(message) {
  const info = message?.info ?? {};
  return {
    id: info.id ?? message?.id ?? crypto.randomUUID(),
    role: roleOf(message),
    text: textOf(message),
    blocks: blocksOf(message),
    // One model call's bookkeeping, summed per turn by `turns`.
    finish: info.finish ?? null,
    cost: typeof info.cost === 'number' ? info.cost : 0,
    model: info.providerID && info.modelID ? `${info.providerID}/${info.modelID}` : null,
    created: info.time?.created ?? null,
    completed: info.time?.completed ?? null,
    raw: message,
  };
}

/// opencode writes one assistant message per model call, so a turn with
/// fifty tool calls arrives as fifty messages of one act each — and the fold
/// in Message.svelte, which works within a message, never saw a run longer
/// than one. Measured on the "disable the song-change popup" thread (Oct
/// 2026): 53 messages, one bash each, every one rendered as its own row.
///
/// So consecutive assistant messages become one turn here, before anything
/// renders. The turn keeps the *first* message's id: Chat keys its {#each}
/// on it, so the Message component — and the typing state it holds — stays
/// alive as steps arrive instead of remounting on every tool call.
///
/// A turn is `done` when its last step finished for a reason other than
/// more tool calls, or when anything follows it — a message you sent
/// mid-turn splits the work in two (opencode folds it in at the next step
/// boundary), and the half before it is finished by definition.
function turns(list) {
  const out = [];
  let ask = '';
  for (const m of list) {
    if (m.role !== 'assistant') {
      out.push(m);
      if (m.role === 'user') ask = m.text;
      continue;
    }
    const prev = out.at(-1);
    if (prev?.role === 'assistant') {
      prev.blocks.push(...m.blocks);
      prev.text = [prev.text, m.text].filter(Boolean).join('\n');
      prev.steps += 1;
      prev.cost += m.cost;
      prev.model = m.model ?? prev.model;
      prev.end = m.completed;
      prev.finish = m.finish;
      continue;
    }
    out.push({
      ...m,
      blocks: [...m.blocks],
      ask,
      steps: 1,
      start: m.created,
      end: m.completed,
    });
  }
  out.forEach((m, i) => {
    if (m.role !== 'assistant') return;
    m.done = i < out.length - 1 || (m.end != null && m.finish !== 'tool-calls');
  });
  return out;
}

let refetchTimer = null;
function refetchSoon() {
  clearTimeout(refetchTimer);
  refetchTimer = setTimeout(refreshMessages, 250);
}

export async function refreshMessages() {
  if (!app.sessionId) return;
  try {
    const res = await engine.getMessages(app.sessionId);
    const list = res?.data ?? (Array.isArray(res) ? res : []);
    app.messages = turns(list.map(shaped));
  } catch (e) {
    app.error = String(e);
  }
}

// Event lines: what the machine did that nobody said in the conversation.
// Today one kind — a level moved by something other than parker or ^, (the
// keys on the desk, an app's own slider), said once as a quiet line: "all
// sound 42 → 50 · the keys on the desk, not parker" (door's `watch_levels`,
// docs/visage/controls/README.md). parker's own changes are already drawn
// under their ledger rows, and door leaves them out.
//
// The lines are this page's, not the engine's: nothing is sent and nothing
// is kept past a reload. They live apart from `app.messages`, which every
// refetch replaces, and are woven in by time when the transcript is drawn
// (`woven`): a line sits after every message that began before it. Each
// belongs to the thread that was on screen when it happened — a new chat's
// go with the thread its first message mints — so leaving a thread and
// coming back finds them where they were.
//
// A held key is one line: a change to the same row, of the same kind, within
// FOLD_MS of that line's last one moves the line's `to` instead of adding a
// line (door already settles a burst into a few events). A line that comes
// back to where it started has nothing left to say and goes.
export const eventLines = $state([]);
const FOLD_MS = 4000;
const LINES_KEPT = 60;

/// door's `levels` event: `{ changes: [{ id, label, kind, from, to }], keys }`.
export function heardLevels(v) {
  const now = Date.now();
  for (const c of v?.changes ?? []) {
    if (!c?.id || c.from == null || c.to == null) continue;
    const why = v.keys ? 'the keys on the desk' : 'elsewhere';
    const held = eventLines.findLast(
      (l) => l.id === c.id && l.kind === c.kind && l.thread === app.sessionId && now - l.last < FOLD_MS,
    );
    if (held) {
      Object.assign(held, { to: c.to, last: now, why: held.why === why ? why : 'elsewhere' });
      if (held.to === held.from) eventLines.splice(eventLines.indexOf(held), 1);
      continue;
    }
    eventLines.push({
      key: `line:${c.id}:${c.kind}:${now}`,
      thread: app.sessionId,
      at: now,
      last: now,
      id: c.id,
      kind: c.kind,
      label: c.label ?? c.id,
      from: c.from,
      to: c.to,
      why,
    });
  }
  if (eventLines.length > LINES_KEPT) eventLines.splice(0, eventLines.length - LINES_KEPT);
}

/// The transcript as drawn: `app.messages` with this thread's event lines
/// woven in, each after every message that began before it. A message not
/// yet on the engine (the one you just sent) began after all of them.
export function woven(messages, lines, thread) {
  const mine = lines.filter((l) => l.thread === thread);
  if (!mine.length) return messages.map((m) => ({ key: m.id, m }));
  const out = [];
  let i = 0;
  for (const m of messages) {
    const began = m.start ?? m.created ?? Infinity;
    while (i < mine.length && mine[i].at < began) out.push({ key: mine[i].key, line: mine[i++] });
    out.push({ key: m.id, m });
  }
  while (i < mine.length) out.push({ key: mine[i].key, line: mine[i++] });
  return out;
}

/// Lines seen on a new chat's empty screen belong to nothing once you leave
/// it for another thread, and to the thread its first message mints.
function adoptLines(id) {
  for (let i = eventLines.length - 1; i >= 0; i--) {
    if (eventLines[i].thread !== null) continue;
    if (id) eventLines[i].thread = id;
    else eventLines.splice(i, 1);
  }
}

// Earlier threads — what the palette (^K, Palette.svelte) lists with nothing
// typed. The engine keeps every session; this is a window onto the last
// fifty, grouped by day, with a card for the selected one. Presented as
// parker's threads of attention, never a roster of parkers (PARKER.md
// invariant 2). Resuming is the only verb, and Ctrl+N still forgets rather
// than deletes. Whether the palette is up is `app.palette` and nothing
// else: this holds what it shows.
export const earlier = $state({ threads: [], loading: false });

/// How many threads the palette lists.
const EARLIER_LIMIT = 50;

/// opencode's name for a session nobody has named yet. A thread still
/// wearing it never got as far as a first answer (the July test sessions)
/// and has nothing to list.
const UNNAMED = /^New session - \d{4}-\d\d-\d\dT/;

/// parker's threads, newest first. The `agent` filter is what keeps the
/// engine's other sessions out — ledger and digest sessions are deleted as
/// they answer, but a non-parker session never belongs here.
async function parkerThreads(limit) {
  const res = await engine.listSessions();
  const list = Array.isArray(res) ? res : (res?.data ?? []);
  return list
    .filter((s) => s?.agent === 'parker' && s?.title && !UNNAMED.test(s.title))
    .sort((a, b) => (b?.time?.updated ?? 0) - (a?.time?.updated ?? 0))
    .slice(0, limit)
    .map((s) => ({
      id: s.id,
      title: s.title,
      created: s?.time?.created ?? 0,
      updated: s?.time?.updated ?? 0,
      cost: typeof s?.cost === 'number' ? s.cost : null,
      model: s?.model?.id ?? '',
    }));
}

/// ^K. The box is up at once, wearing the list it had last time; the threads
/// refresh when the engine answers, then the cards.
export async function openPalette() {
  app.palette = true;
  earlier.loading = true;
  let items;
  try {
    items = await parkerThreads(EARLIER_LIMIT);
  } catch (e) {
    app.error = String(e);
    return;
  } finally {
    earlier.loading = false;
  }
  earlier.threads = items;
  if (!app.palette) return;
  const ids = items.map((t) => t.id);
  const [kept, marked] = await Promise.all([
    engine.digests(ids).catch(() => null), // no cards is a list without its second column
    engine.marks(ids).catch(() => null), // no marks is every thread on its card's word
  ]);
  for (const [id, card] of Object.entries(kept ?? {})) {
    if (!cards[id]?.v || (card?.updated ?? 0) >= (cards[id].updated ?? 0)) cards[id] = card;
  }
  // Every listed thread takes door's word, so a mark cleared in the other
  // frame goes here too — except one this page is still writing.
  if (marked) for (const id of ids) if (!marking[id]) marks[id] = marked[id] ?? null;
  // What can be counted is counted at once, for every thread without a
  // fresh card — a local read each, the palette's search does the same.
  // What needs writing waits in line behind it; the palette puts the
  // selected one first (`selectThread`).
  await Promise.all(items.filter((t) => !fresh(t)).map((t) => read(t).then((r) => counted(t, r)).catch(() => {})));
  for (const t of items) want(t);
}

/// ^K again, Esc, or a pick: down, the card line emptied, and the look
/// /visage was showing taken back off.
export function closePalette() {
  app.palette = false;
  queue.length = 0;
  previewTheme(null);
}

/// The selected thread's card jumps the line.
export function selectThread(t) {
  want(t, true);
}

/// The look this screen wears while /visage (or ^,'s "this screen") is
/// choosing: `name` on, unsaved, or (null) back to the one it wears.
/// Nothing previews once both are down, whoever asks — the palette's effect
/// can outlive its fade.
let previewing = null;
export function previewTheme(name) {
  const wear = app.palette || app.controls ? name : null;
  if (wear === previewing) return;
  previewing = wear;
  applyTheme(wear ?? app.theme, { save: false });
}

// Each thread read once per page and cached until its `updated` moves:
// what was said (the palette's search), what can be counted, and the
// text the digest agent reads. The engine has no route that searches
// transcripts (its /find routes search the empty working directory), so
// search reads the threads themselves too. Prose only — what you and parker
// *said*. Tool output and reasoning are never searched or sent, the same
// line the ledger draws.
const reads = new Map(); // session id -> { updated, lines, facts, text, done }

async function read(thread) {
  const hit = reads.get(thread.id);
  if (hit && hit.updated === thread.updated) return hit;
  const res = await engine.getMessages(thread.id);
  const list = res?.data ?? (Array.isArray(res) ? res : []);
  const r = { updated: thread.updated, ...readThread(list, thread) };
  reads.set(thread.id, r);
  return r;
}

async function linesOf(thread) {
  return (await read(thread)).lines;
}

/// The text parts of one message that were said, not attached.
function proseOf(message) {
  return visible(
    (message?.parts ?? [])
      .filter((p) => p?.type === 'text' && typeof p.text === 'string' && !p.synthetic)
      .map((p) => p.text)
      .join('\n'),
  ).trim();
}

/// A thread, read: lines for search, facts for the card, the digest's text,
/// and whether it is finished. Facts are counted, never written: turns are
/// your messages, steps are model calls, work is the time those calls took,
/// and the tally is the ledger's own.
function readThread(list, thread) {
  const lines = list.flatMap((m) => {
    const who = roleOf(m) === 'user' ? 'YOU' : 'PARKER';
    return (m?.parts ?? [])
      .filter((p) => p?.type === 'text' && typeof p.text === 'string' && !p.synthetic)
      .map((p) => ({ who, text: visible(p.text).replace(/\s+/g, ' ').trim() }))
      .filter((l) => l.text);
  });
  const msgs = list.map(shaped);
  const steps = msgs.filter((m) => m.role === 'assistant');
  const acts = steps.flatMap((m) => m.blocks.filter((b) => b.kind === 'act'));
  const touched = new Map();
  for (const a of acts) if (a.glyph === '±' && a.target) touched.set(a.target, a.verb);
  const facts = {
    turns: msgs.filter((m) => m.role === 'user').length,
    steps: steps.length,
    work: steps.reduce(
      (sum, m) => sum + (m.created && m.completed ? Math.max(0, m.completed - m.created) : 0),
      0,
    ),
    tally: tallyOf(acts),
    edited: [...touched].slice(0, 8).map(([target, verb]) => ({ verb, target })),
    where: placeOf(list),
  };
  return { lines, facts, text: transcript(list), done: finished(list, thread) };
}

/// Whether a thread is between turns, and so worth a card. One still in
/// flight would be digested half-told and paid for again in a minute.
function finished(list, thread) {
  if (thread.id === app.sessionId && app.busy) return false;
  const last = list.at(-1)?.info;
  if (!last) return false;
  const recent = Date.now() - (thread.updated ?? 0) < 120_000;
  if (last.role === 'assistant') {
    if (last.time?.completed == null && !last.error) return false;
    if (last.finish === 'tool-calls' && recent) return false;
    return true;
  }
  return !recent;
}

/// Where Peter was, from the sentence door puts on each message
/// (`at_the_tower`, `away`): the device when away, `tower` at the tower, and
/// nothing for a message older than the sentence.
function placeOf(list) {
  for (let i = list.length - 1; i >= 0; i--) {
    const s = list[i]?.info?.system;
    if (typeof s !== 'string') continue;
    const away = s.match(/away from the tower: this message came through the porthole, on (.+?), over the tailnet/);
    if (away) return away[1].replace(/\s*\(.*\)$/, '');
    if (s.includes('Peter is at the tower')) return 'tower';
  }
  return '';
}

/// The thread as the digest agent reads it: who said what, and one line per
/// act. Long threads keep their opening and as much of the end as fits —
/// the end is where a thread says what it became.
const DIGEST_HEAD = 4_000;
const DIGEST_TOTAL = 24_000;

function transcript(list) {
  const clip = (s, n) => (s.length > n ? `${s.slice(0, n)}…` : s);
  const out = [];
  for (const m of list) {
    const said = proseOf(m);
    if (roleOf(m) === 'user') {
      if (said) out.push(`Peter: ${clip(said, 2_000)}`);
      continue;
    }
    if (said) out.push(`parker: ${clip(said, 2_000)}`);
    for (const b of blocksOf(m)) {
      if (b.kind === 'act') out.push(`  · ${`${b.verb} ${b.target}`.trim()}${b.status === 'failed' ? ' (failed)' : ''}`);
      else if (b.kind === 'gesture') out.push(`  · ${b.text}`);
    }
  }
  const all = out.join('\n');
  if (all.length <= DIGEST_TOTAL) return all;
  return `${all.slice(0, DIGEST_HEAD)}\n\n[… the middle of the thread is left out …]\n\n${all.slice(-(DIGEST_TOTAL - DIGEST_HEAD))}`;
}

// The cards. door keeps them on parker (`digest`), so the tower and the
// porthole share one and each thread is paid for once; this is the page's
// view of them. A card is fresh while its `updated` is the thread's; a stale
// one keeps showing what it said while a new one is written. Until a thread
// has a card at all, its entry holds the counted facts alone, with `pending`
// set while a digest is on its way.
export const cards = $state({});

/// A card is fresh while its `updated` is the thread's — except a version-1
/// card left open, which predates `next` and is read once more so the steps
/// it left waiting get written.
const fresh = (t) => {
  const c = cards[t.id];
  return !!c?.v && c.updated === t.updated && (c.v >= 2 || c.state !== 'open');
};
const stamp = (t) => `${t.id}@${t.updated}`;

// The marks: someone's word on where a thread stands — yours from ^X, or
// parker's from his `recall mark` — kept on parker beside the cards and
// written only by `parker recall mark`. A mark beats the card while it holds,
// and it holds until the thread moves: a new message is the thread saying
// more, and the digest has the next word, told about the mark (door's
// `told`). Null where door has none.
export const marks = $state({});

/// The thread's mark, if it still holds.
export function markOf(t) {
  const m = t ? marks[t.id] : null;
  return m && m.updated === t.updated ? m : null;
}

/// Where a thread stands: 'open', 'done', 'chat', or '' when nobody has
/// read it yet. A holding mark's word first, then the card's — a stale card
/// included, since it says what it said until the new one lands.
export function stateOf(t) {
  return markOf(t)?.state ?? cards[t?.id]?.state ?? '';
}

// ^X. The palette shows the flip at once and door writes it after, in
// order: two quick presses must land as two, not race (`chain`). A reply
// that is no longer the newest press for its thread changes nothing, and
// `marking` keeps a palette reopened mid-write from taking door's older
// word over the press.
const marking = {}; // thread id -> presses not yet answered
const pressed = {}; // thread id -> the newest press
let chain = Promise.resolve();

/// Flip the thread: open → done, anything else → open. A flip that lands on
/// what its fresh card already says is no mark at all — it is cleared, so the
/// card speaks again and nobody gets credit for agreeing with it.
export function toggleMark(t) {
  if (!t) return;
  const want = stateOf(t) === 'open' ? 'done' : 'open';
  const card = cards[t.id];
  const verdict = fresh(t) && card?.state ? (card.state === 'open' ? 'open' : 'done') : null;
  const word = verdict === want ? 'clear' : want;
  const before = marks[t.id] ?? null;
  marks[t.id] =
    word === 'clear'
      ? null
      : { state: want, by: 'peter', at: Date.now(), updated: t.updated, turns: card?.facts?.turns ?? 0 };
  const press = (pressed[t.id] ?? 0) + 1;
  pressed[t.id] = press;
  marking[t.id] = (marking[t.id] ?? 0) + 1;
  chain = chain
    .then(() => engine.mark(t.id, word))
    .then(
      (res) => {
        if (pressed[t.id] !== press) return;
        marks[t.id] = res?.mark ?? null;
        // The engine's `updated` is the truth. A list read before the
        // thread's last turn learns it here, and its card is stale with it.
        const item = earlier.threads.find((x) => x.id === t.id);
        if (item && typeof res?.updated === 'number' && res.updated > item.updated) {
          item.updated = res.updated;
          want(item);
        }
      },
      (e) => {
        if (pressed[t.id] !== press) return;
        marks[t.id] = before;
        notice(`could not mark “${t.title}”: ${e}`, 12000);
      },
    )
    // Nothing a press does may break the chain for the next one.
    .catch(() => {})
    .finally(() => {
      marking[t.id] -= 1;
      if (!marking[t.id]) delete marking[t.id];
    });
}

/// Counted facts land before the words do.
function counted(t, r) {
  if (fresh(t)) return;
  cards[t.id] = { ...(cards[t.id] ?? {}), facts: r.facts, pending: r.done };
}

// One digest at a time, the selected thread first. Not concurrent for the
// same reason the ledger's summaries are chained: fifty sessions at once is
// a stampede for no gain. The line empties when the palette closes, and a
// thread that failed is not asked again until it moves, or the page reloads.
const queue = [];
const tried = new Set();
let filling = false;

function want(t, first = false) {
  if (!t || fresh(t) || tried.has(stamp(t))) return;
  const at = queue.indexOf(t.id);
  if (at >= 0) queue.splice(at, 1);
  if (first) queue.unshift(t.id);
  else queue.push(t.id);
  fill();
}

async function fill() {
  if (filling) return;
  filling = true;
  while (app.palette && queue.length) {
    const t = earlier.threads.find((x) => x.id === queue[0]);
    queue.shift();
    if (!t || fresh(t) || tried.has(stamp(t))) continue;
    tried.add(stamp(t));
    await cardFor(t);
  }
  filling = false;
}

async function cardFor(t) {
  try {
    const r = await read(t);
    counted(t, r);
    // A thread caught mid-turn stays tried: its card is asked for again when
    // it moves, which a finished turn always does. Untrying it here loops —
    // `counted` rewrites the entry, the palette reselects, `want` asks again.
    if (!r.done) return;
    const card = await engine.digest(t.id, r.text, r.facts);
    if (!card || typeof card !== 'object') throw new Error('no card');
    cards[t.id] = card;
    // The title went back to the engine too; this is the list catching up.
    const item = earlier.threads.find((x) => x.id === t.id);
    if (item && card.title) item.title = card.title;
  } catch {
    if (cards[t.id]) cards[t.id] = { ...cards[t.id], pending: false };
  }
}

/// What was said in each of `threads` that holds `needle` (three characters
/// or more: under that, everything matches and nothing is found). Per thread
/// id, how many lines hold it and the first three, each split around the
/// match so the palette can light it, and who said it.
export async function searchSaid(needle, threads) {
  if (needle.length < 3) return {};
  const all = await Promise.all(threads.map((t) => linesOf(t).catch(() => [])));
  const by = {};
  threads.forEach((t, i) => {
    const hits = [];
    let n = 0;
    for (const { who, text } of all[i]) {
      const at = text.toLowerCase().indexOf(needle);
      if (at < 0) continue;
      n += 1;
      if (hits.length === 3) continue;
      const from = Math.max(0, at - 40);
      const to = Math.min(text.length, at + needle.length + 90);
      hits.push({
        who,
        before: (from > 0 ? '…' : '') + text.slice(from, at),
        match: text.slice(at, at + needle.length),
        after: text.slice(at + needle.length, to) + (to < text.length ? '…' : ''),
      });
    }
    if (n) by[t.id] = { n, hits };
  });
  return by;
}

/// Leaving a thread — ^N, or another one resumed — is when it is done for
/// now, so it is when its card is written, if it has no fresh one: the
/// palette is not the only reader (`parker recall list --open` is parker's),
/// and a thread nobody browsed would otherwise have no card to say it was
/// left open. Never mid-turn — a thread in flight would be digested
/// half-told — and best-effort: the palette writes it next time otherwise.
/// Its synchronous start reads `app.busy` before the caller resets it.
async function leave(id) {
  if (!id || app.busy) return;
  try {
    const t = (await parkerThreads(EARLIER_LIMIT)).find((x) => x.id === id);
    if (!t) return;
    const card = (await engine.digests([id]))?.[id];
    if (card && (!cards[id]?.v || (card.updated ?? 0) >= (cards[id].updated ?? 0))) cards[id] = card;
    if (fresh(t) || tried.has(stamp(t))) return;
    tried.add(stamp(t));
    await cardFor(t);
  } catch {
    /* the palette writes it next time */
  }
}

/// Adopt an engine thread as the current conversation — a thread picked in
/// the palette.
export async function resume(id) {
  if (app.sessionId !== id) leave(app.sessionId);
  closePalette();
  adoptLines(null);
  app.sessionId = id;
  app.messages = [];
  app.error = null;
  app.busy = false;
  await refreshMessages();
}

/// One line at the bottom of the screen that says a thing happened, then
/// goes — the palette's `/theme` result, mostly. Not an error banner.
let noticeTimer = null;
export function notice(text, ms = 6000) {
  app.notice = text;
  clearTimeout(noticeTimer);
  if (ms) noticeTimer = setTimeout(() => (app.notice = null), ms);
}

/// How often to re-ask which brain parker is on. A switch from here lands at
/// once (`pickBrain`); this only catches one made elsewhere — `parker brain`
/// over SSH, or the other frame — and a brain is not a thing that flickers.
const BRAIN_POLL_MS = 5000;

async function loadBrain() {
  try {
    app.brain = (await engine.brain()) ?? null;
  } catch {
    /* keep the last known brain; the next poll asks again */
  }
}

/// The palette's `/brain <name>`. Door switches only to a brain its set
/// lists, so a name that is not one comes back as a refusal, said quietly.
export async function pickBrain(name) {
  try {
    app.brain = (await engine.brain(name)) ?? app.brain;
    notice(`brain: ${app.brain?.name ?? name}, from the next message`);
  } catch (e) {
    notice(`brain ${name} failed: ${e}`, 12000);
  }
}

/// The palette's `/visage <name>`. visage's own theme, not the machine's
/// `/theme`: nothing to ask door, so it lands at once, and this screen
/// remembers it.
export function pickTheme(name) {
  app.theme = applyTheme(name);
  notice(`visage: ${app.theme}`);
}

/// A brain's name for a `provider/model`, or the bare model when no brain
/// names it — an old thread answered by something parker has since stopped
/// using still says what it was.
export function brainOf(model) {
  if (!model) return '';
  const known = app.brain?.brains?.find((b) => b.model === model);
  return known?.name ?? model.split('/').slice(1).join('/');
}

/// Ctrl+N. Nothing is created here: dropping the id is enough, and it means
/// a reflex press cannot leave a trail of empty sessions on the engine.
export function newChat() {
  leave(app.sessionId);
  adoptLines(null);
  app.sessionId = null;
  app.messages = [];
  app.error = null;
  app.busy = false;
}

async function ensureSession() {
  if (app.sessionId) return app.sessionId;
  const s = await engine.createSession();
  const id = s?.id ?? s?.info?.id;
  if (!id) throw new Error('engine returned a session without an id');
  app.sessionId = id;
  adoptLines(id);
  return id;
}

// One line per finished run of acts, written by the engine's `ledger` agent
// (haiku, no tools — engine.nix) from the acts' verbs and targets alone:
// the fold summarises what parker did, never what came back, the same rule
// the ledger rows keep. Keyed by the fold's key, which is its first act's
// call id, so a summary is asked for once and survives refetches.
//
// Kept in localStorage so a resumed thread does not pay for its summaries
// twice. Bounded, because it is a cache and not a record — the engine holds
// the record.
const SUMMARY_KEY = 'visage.summaries';
const SUMMARY_CAP = 400;
export const summaries = $state(loadSummaries());
const asked = new Set();
let summarizing = Promise.resolve();

function loadSummaries() {
  try {
    const raw = JSON.parse(localStorage.getItem(SUMMARY_KEY) ?? '{}');
    return raw && typeof raw === 'object' ? raw : {};
  } catch {
    return {};
  }
}

function keepSummaries() {
  try {
    const entries = Object.entries(summaries).slice(-SUMMARY_CAP);
    localStorage.setItem(SUMMARY_KEY, JSON.stringify(Object.fromEntries(entries)));
  } catch {
    /* a cache that cannot be written is only a slower cache */
  }
}

/// Ask for a run's summary, once. Calls are chained rather than fired
/// together: resuming a long thread folds a dozen runs in one render, and a
/// dozen concurrent sessions on the engine is a stampede for no gain.
/// Failure is silent — the fold row already carries a tally, which is what
/// it showed before summaries existed.
export function summarize(key, ask, acts) {
  if (summaries[key] || asked.has(key)) return;
  asked.add(key);
  const lines = acts
    .map((a) => `${a.verb} ${a.target}${a.status === 'failed' ? ' (failed)' : ''}`.trim())
    .join('\n');
  const prompt = `Request: ${ask || '(none)'}\n\nActs, in order:\n${lines}`;
  summarizing = summarizing
    .then(() => engine.summarize(prompt))
    .then((line) => {
      // The ledger speaks lowercase ("ran", "read"); haiku sometimes
      // capitalises anyway. Leave acronyms alone: only "Found", not "OSD".
      const tidy = typeof line === 'string'
        ? line.trim().replace(/\.$/, '').replace(/^[A-Z](?=[a-z])/, (c) => c.toLowerCase())
        : '';
      if (tidy) {
        summaries[key] = tidy;
        keepSummaries();
      }
    })
    .catch(() => {});
}

/// Prompts the engine has not finished answering. More than one is normal:
/// a message sent mid-turn is not refused here, because opencode already
/// does the right thing with it — it lands in the session at once and the
/// running loop picks it up at the next step boundary, so it steers the
/// work in flight rather than waiting behind it (measured against 1.18.35,
/// Oct 2026). Each POST resolves when the loop it joined ends.
let inflight = 0;

/// `context`, when given, goes ahead of the text as a line for parker
/// alone (`visible` strips it): what ^, sends with an ask.
export async function send(text, context = '') {
  text = text.trim();
  if (!text) return;
  app.error = null;
  let id;
  try {
    id = await ensureSession();
  } catch (e) {
    app.error = String(e);
    return;
  }

  const outbound = context ? `${context}\n${text}` : text;

  app.messages = [...app.messages, { id: `local-${Date.now()}`, role: 'user', text }];
  inflight += 1;
  app.busy = true;
  try {
    await engine.sendMessage(id, outbound);
    await refreshMessages();
  } catch (e) {
    app.error = String(e);
  } finally {
    inflight -= 1;
    app.busy = inflight > 0;
  }
}

/// Esc's first tier. Best-effort: a stop that fails is not worth an error
/// banner, and `send` resolves or rejects on its own either way.
export async function interrupt() {
  if (!app.sessionId || !app.busy) return false;
  try {
    await engine.interrupt(app.sessionId);
  } catch {
    /* the stream ends when it ends */
  }
  return true;
}

/// Esc's last tier: the same gesture as SUPER+P, spoken through the one
/// summon path so there is still only one implementation of it.
export function dismiss() {
  engine.dismiss().catch(() => {});
}

export async function init() {
  engine.onStatus((s) => (app.engineStatus = s));
  engine.onEvent((ev) => {
    // Any engine chatter that names our session keeps the face speaking and
    // the transcript fresh; exact event taxonomy stays the server's problem.
    const flat = JSON.stringify(ev ?? {});
    if (app.sessionId && flat.includes(app.sessionId)) {
      app.lastEventAt = Date.now();
      // Parker doing something counts as something happening to parker: no
      // idle beat should fire while he is mid-answer.
      stir();
      refetchSoon();
    }
  });

  // The voluntary track and the dance mode, both written by parkerd's verbs.
  const poll = async () => {
    try {
      const s = await engine.faceState();
      app.worn = s?.expression ?? null;
      setDance(s?.dance ?? null, currentTrack());
      playBeat(s?.beat ?? null);
    } catch {
      // No shell, or no runtime dir yet. Parker is never in the critical
      // path, including of himself: keep the last known state and retry.
    }
  };
  poll();
  setInterval(poll, FACE_POLL_MS);

  loadBrain();
  setInterval(loadBrain, BRAIN_POLL_MS);
}

/// What is playing, for seeding the routine so the same track dances the same
/// way twice. visage cannot see MPRIS; parkerd can, and when it starts
/// passing the title through the dance slot this reads it. Until then `auto`
/// falls back to a per-session seed, which is a different routine each time
/// rather than a wrong one.
function currentTrack() {
  return null;
}
