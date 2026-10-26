<script>
  // ^K — the palette, visage's one overlay (Oct 2026; DECISIONS.md). What
  // you type is its only state:
  //
  //   nothing    parker's earlier threads: what is left open pinned on top,
  //              each with its first step under it, then the rest by day,
  //              each with its card
  //   words      the same list, narrowed: by title and by card from the first
  //              letter, by what was said from the third; under each row,
  //              where it matched
  //   /          the commands, by what they touch (commands.js), each with
  //              a card of its own
  //   /visage …  a command's choices; a look is worn while it is selected
  //
  // Ctrl+N / Ctrl+P or the arrows move, ⏎ picks, ⇥ opens a command's
  // choices, ^X closes the selected thread or opens it again. Esc and ^K
  // close it, and both are caught here, on the box, before anything else
  // can hear them — a key the text system is still composing with
  // included — so one press is always enough.
  //
  // It lists; App acts (`onpick`), because App owns the view, the chat and
  // the dance, and a palette that reached into them would be a second copy
  // of each. A click selects a row and a click on the selected row picks it
  // — the pointer is hidden on the tower, but not in the porthole.
  //
  // Nothing here is {@html}: titles, cards and what was said all come from
  // transcripts that can quote the open web.
  import { fade } from 'svelte/transition';
  import { onDestroy, untrack } from 'svelte';
  import {
    app,
    earlier,
    cards,
    selectThread,
    searchSaid,
    previewTheme,
    stateOf,
    markOf,
    toggleMark,
  } from './state.svelte.js';
  import { commands, DOMAINS, shortModel } from './commands.js';
  import { resolve, polarity } from './themes.js';

  let { onpick, onclose } = $props();

  let query = $state('');
  let sel = $state(0);
  /// What was said, per thread, for one term — kept with its term so a
  /// newer query never wears an older answer.
  let found = $state({ term: '', by: {} });
  let reading = $state(false);
  let inputEl;
  let listEl = $state();

  /// App's caret rules, while the palette is up: the box is where it goes.
  export function focus() {
    inputEl?.focus();
  }

  /// A letter typed while the box had lost focus — on the porthole, after
  /// a click somewhere — is the next letter of the query, not a lost one.
  export function type(text) {
    query += text;
    inputEl?.focus();
  }

  /// Any other key that arrived while the box had lost focus: handled the
  /// way the box handles it, so Esc, ^K, the moves and ⏎ never depend on
  /// where focus is. Whether it was the palette's to handle.
  export function key(e) {
    const mine = onkey(e);
    if (mine) inputEl?.focus();
    return mine;
  }

  $effect(() => {
    inputEl?.focus();
  });

  // ── when ─────────────────────────────────────────────────────────────────
  const WD = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'];
  const MO = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec'];
  const DAY = 86_400_000;

  const midnight = (ms) => new Date(ms).setHours(0, 0, 0, 0);
  const daysAgo = (ms) => Math.round((midnight(Date.now()) - midnight(ms)) / DAY);
  const pad = (n) => String(n).padStart(2, '0');
  const hhmm = (ms) => {
    const d = new Date(ms);
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  };
  const date = (ms) => {
    const d = new Date(ms);
    return `${WD[d.getDay()]} ${d.getDate()} ${MO[d.getMonth()]}`;
  };
  const plural = (n, one) => `${n} ${n === 1 ? one : `${one}s`}`;

  /// TODAY · SAT 10 OCT, or SUN 27 SEP · 13 DAYS AGO — the day, and how far.
  function dayOf(ms) {
    const n = daysAgo(ms);
    const label = n === 0 ? 'TODAY' : n === 1 ? 'YESTERDAY' : date(ms).toUpperCase();
    const sub =
      n <= 1
        ? date(ms).toUpperCase()
        : n < 14
          ? `${n} DAYS AGO`
          : n < 60
            ? `${Math.round(n / 7)} WEEKS AGO`
            : `${Math.round(n / 30)} MONTHS AGO`;
    return { label, sub };
  }

  /// A pinned row's time: the hour today, the weekday this week, and how
  /// many days before that — it sits above the days, so it says which.
  function when(ms) {
    const n = daysAgo(ms);
    return n === 0 ? hhmm(ms) : n < 7 ? WD[new Date(ms).getDay()] : `${n}d`;
  }

  /// `TODAY`, `YESTERDAY`, `3 DAYS` — how long the oldest open thread has
  /// waited.
  const waited = (n) => (n === 0 ? 'TODAY' : n === 1 ? 'YESTERDAY' : `${n} DAYS`);

  /// `9s`, `48s`, `4m 37s`, `1h 05m`.
  function span(ms) {
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ${pad(s % 60)}s`;
    return `${Math.floor(m / 60)}h ${pad(m % 60)}m`;
  }

  /// claude-sonnet-5-5 → sonnet 5.5; anything else as the engine names it.
  function modelName(id) {
    const m = /^claude-([a-z]+)-(\d+)(?:-(\d+))?$/.exec(id ?? '');
    return m ? `${m[1]} ${m[2]}${m[3] ? `.${m[3]}` : ''}` : (id ?? '');
  }

  /// The ledger's glyph colours (Message.svelte): commands in accent, edits
  /// in good, everything else plain.
  const kind = (glyph) => (glyph === '$' ? 'bash' : glyph === '±' ? 'files' : '');

  /// `text` split around the first `needle`, or null.
  function cut(text, needle) {
    const at = (text ?? '').toLowerCase().indexOf(needle);
    if (at < 0) return null;
    return { before: text.slice(0, at), match: text.slice(at, at + needle.length), after: text.slice(at + needle.length) };
  }

  /// A window of `text` around `needle`, for the line under a row.
  function around(text, needle) {
    const at = text.toLowerCase().indexOf(needle);
    const from = Math.max(0, at - 30);
    const to = Math.min(text.length, at + needle.length + 90);
    return {
      before: (from > 0 ? '…' : '') + text.slice(from, at),
      match: text.slice(at, at + needle.length),
      after: text.slice(at + needle.length, to) + (to < text.length ? '…' : ''),
    };
  }

  // ── what the query asks for ──────────────────────────────────────────────
  const lead = $derived(query.replace(/^\s+/, '').toLowerCase());
  const slash = $derived(lead.startsWith('/'));
  const q = $derived(slash ? '' : query.trim().toLowerCase());
  /// `/visage gla` → the verb typed, whether a space followed it, and the rest.
  const parsed = $derived.by(() => {
    const m = /^\/(\S*)(\s+([\s\S]*))?$/.exec(lead);
    return { tok: m?.[1] ?? '', spaced: m?.[2] !== undefined, rest: (m?.[3] ?? '').trim() };
  });
  const verbs = $derived(commands());
  /// The command whose choices are showing, if any.
  const opened = $derived(
    slash && parsed.spaced ? (verbs.find((v) => v.id === parsed.tok && v.choices?.length) ?? null) : null,
  );
  const scope = $derived(!slash ? 'threads' : opened ? 'choices' : 'commands');

  // ── the rows, in groups ─────────────────────────────────────────────────
  const groups = $derived.by(() => {
    if (scope === 'threads') {
      const out = [];
      // What is left open, on top, with nothing typed — each thread once,
      // so the days below hold only what landed. A thread ^X closes drops
      // into its day, and the cursor stays on the same row number, so the
      // next open thread comes up under it.
      const pin = !q;
      if (pin) {
        const open = earlier.threads.filter((t) => stateOf(t) === 'open');
        if (open.length) {
          out.push({
            key: 'open',
            label: 'LEFT OPEN',
            sub: `${open.length} · OLDEST ${waited(Math.max(...open.map((t) => daysAgo(t.updated))))}`,
            pinned: true,
            rows: open.map((t) => {
              const next = cards[t.id]?.next ?? [];
              const line = next.length
                ? {
                    label: next[0].who === 'you' ? 'YOU' : 'PARKER',
                    plain: next[0].step,
                  }
                : null;
              return { kind: 'thread', key: `t:${t.id}`, t, pinned: true, line, more: next.length > 1 ? `+${next.length - 1}` : '' };
            }),
          });
        }
      }
      for (const t of earlier.threads) {
        if (pin && stateOf(t) === 'open') continue;
        let row = { kind: 'thread', key: `t:${t.id}`, t };
        if (q) {
          const card = cards[t.id];
          const steps = (card?.next ?? []).map((s) => s.step).join(' · ');
          const title = cut(t.title, q);
          const about = card?.about ? cut(card.about, q) : null;
          const landed = card?.landed ? cut(card.landed, q) : null;
          const next = steps ? cut(steps, q) : null;
          const said = found.term === q ? found.by[t.id] : null;
          if (!title && !about && !landed && !next && !said) continue;
          // Where it matched, what was said first: a title or a card is
          // already on screen, and the line is for what is not.
          const line = said
            ? { label: 'SAID', ...said.hits[0] }
            : landed
              ? { label: stateOf(t) === 'open' ? 'LEFT OPEN' : 'LANDED', ...around(card.landed, q) }
              : next
                ? { label: 'NEXT', ...around(steps, q) }
                : about
                  ? { label: 'ABOUT', ...around(card.about, q) }
                  : null;
          const total = (said?.n ?? 0) + (landed ? 1 : 0) + (next ? 1 : 0) + (about ? 1 : 0);
          row = { ...row, title, about, landed, said, line, more: line && total > 1 ? `+${total - 1}` : '' };
        }
        const key = midnight(t.updated);
        if (out.at(-1)?.key !== key) out.push({ key, ...dayOf(t.updated), rows: [] });
        out.at(-1).rows.push(row);
      }
      return out;
    }
    if (scope === 'choices') {
      const v = opened;
      const fits = (c) => !parsed.rest || `${c.arg} ${c.hint}`.toLowerCase().includes(parsed.rest);
      const row = (c) => ({ kind: 'choice', key: `c:${v.id}:${c.arg}`, v, c });
      const sets = v.groups ?? [[undefined, v.choicesLabel ?? `/${v.id.toUpperCase()}`]];
      return sets
        .map(([id, label]) => ({
          key: id ?? v.id,
          label,
          sub: '',
          rows: v.choices.filter((c) => c.group === id && fits(c)).map(row),
        }))
        .filter((g) => g.rows.length);
    }
    const tok = parsed.tok;
    const fits = (v) =>
      !tok || `/${v.id} ${v.what} ${(v.choices ?? []).map((c) => c.arg).join(' ')}`.toLowerCase().includes(tok);
    return DOMAINS.map((d) => ({
      key: d,
      label: d,
      sub: '',
      rows: verbs.filter((v) => v.domain === d && fits(v)).map((v) => ({ kind: 'verb', key: `v:${v.id}`, v })),
    })).filter((g) => g.rows.length);
  });

  /// Every pickable row, in screen order — what Ctrl+N/P walk.
  const rows = $derived(groups.flatMap((g) => g.rows));
  const indexOf = $derived(new Map(rows.map((r, i) => [r.key, i])));
  const current = $derived(rows[Math.min(sel, rows.length - 1)] ?? null);

  /// The row a change of query should land on, by key; else the top.
  let prefer = null;
  $effect(() => {
    query;
    untrack(() => {
      const i = prefer ? (indexOf.get(prefer) ?? -1) : -1;
      prefer = null;
      sel = Math.max(0, i);
    });
  });

  // Moving with the keys keeps the selected row on screen.
  $effect(() => {
    const i = sel;
    rows;
    listEl?.querySelector(`[data-i="${i}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  // The selected thread's card is written first.
  $effect(() => {
    const t = current?.kind === 'thread' ? current.t : null;
    if (t) untrack(() => selectThread(t));
  });

  // What was said waits for a pause in the typing and for three characters.
  $effect(() => {
    const term = q;
    const list = earlier.threads;
    if (term.length < 3) {
      reading = false;
      return;
    }
    reading = true;
    const timer = setTimeout(async () => {
      const by = await searchSaid(term, list).catch(() => ({}));
      if (term === untrack(() => q)) {
        found = { term, by };
        reading = false;
      }
    }, 180);
    return () => clearTimeout(timer);
  });

  // /visage: the screen wears the look that is selected; anything else, or
  // the palette going down, puts back the one it wore.
  const look = $derived(current?.kind === 'choice' && current.v.id === 'visage' ? current.c.arg : null);
  $effect(() => {
    previewTheme(look);
  });
  onDestroy(() => previewTheme(null));

  // ── acting ───────────────────────────────────────────────────────────────
  function move(d) {
    if (!rows.length) return;
    sel = (Math.min(sel, rows.length - 1) + d + rows.length) % rows.length;
  }

  /// A command's choices, landing on the one `want` names (`/glacier` found
  /// /visage), else the one that is on now.
  function open(v, want) {
    const c =
      (want ? v.choices.find((x) => x.arg.toLowerCase().includes(want)) : null) ??
      v.choices.find((x) => x.now) ??
      v.choices[0];
    prefer = c ? `c:${v.id}:${c.arg}` : null;
    query = `/${v.id} `;
    inputEl?.focus();
  }

  function pick(row) {
    if (!row) return;
    if (row.kind === 'thread') return onpick({ kind: 'thread', id: row.t.id });
    if (row.kind === 'verb' && row.v.choices?.length) return open(row.v, parsed.tok);
    onpick({ kind: 'command', id: row.v.id, arg: row.c?.arg ?? '', machine: !!row.v.machine });
  }

  function click(i, row) {
    if (i === sel) pick(row);
    else sel = i;
    inputEl?.focus();
  }

  /// The box's keys. Returns whether the key was the palette's.
  function onkey(e) {
    const k = e.key;
    const ctrl = e.ctrlKey && !e.altKey && !e.metaKey;
    // First, and whatever else is going on: the two ways out. Stopped here,
    // so App never sees them and nothing between can swallow the first one.
    if (k === 'Escape' || (ctrl && !e.shiftKey && k.toLowerCase() === 'k')) {
      e.preventDefault();
      e.stopPropagation();
      onclose();
      return true;
    }
    // The text system's while it composes (App.svelte, onKey).
    if (e.isComposing || e.keyCode === 229) return false;
    // ^X crosses the selected thread off, or opens it again. Cut still wins
    // when there is something in the box to cut. Nothing moves the cursor:
    // the row number stays, so a closed thread leaves the next one under it.
    if (ctrl && !e.shiftKey && k.toLowerCase() === 'x' && current?.kind === 'thread') {
      if (document.activeElement === inputEl && inputEl.selectionStart !== inputEl.selectionEnd) return false;
      e.preventDefault();
      e.stopPropagation();
      toggleMark(current.t);
      return true;
    }
    const down = (ctrl && !e.shiftKey && k === 'n') || k === 'ArrowDown';
    const up = (ctrl && !e.shiftKey && k === 'p') || k === 'ArrowUp';
    if (down || up) {
      // Stopped here so App's own Ctrl+N (new chat) never fires under it.
      e.preventDefault();
      e.stopPropagation();
      move(down ? 1 : -1);
      return true;
    }
    if (k === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      pick(current);
      return true;
    }
    // Plain Tab opens a command's choices and otherwise does nothing: its
    // default would walk the caret out of the box. Ctrl+Tab is App's.
    if (k === 'Tab' && !e.ctrlKey) {
      e.preventDefault();
      if (current?.kind === 'verb' && current.v.choices?.length) open(current.v, parsed.tok);
      return true;
    }
    return false;
  }

  /// A click on the dim around the box closes it, where there is a pointer.
  function outside(e) {
    if (e.target === e.currentTarget) onclose();
  }

  // ── the card ─────────────────────────────────────────────────────────────
  const thread = $derived(current?.kind === 'thread' ? current : null);
  const card = $derived(thread ? (cards[thread.t.id] ?? null) : null);
  const facts = $derived(card?.facts ?? null);
  const fresh = $derived(!!card?.v && card.updated === thread?.t.updated);
  const away = $derived(facts?.where && facts.where !== 'tower' ? facts.where : '');
  /// Where the thread stands, and whose word that is when it is not the
  /// card's: `YOU CLOSED IT, 14:31`, `PARKER KEPT IT OPEN`.
  const mark = $derived(thread ? markOf(thread.t) : null);
  const standing = $derived(thread ? stateOf(thread.t) : '');
  const by = $derived.by(() => {
    if (!mark) return '';
    const who = mark.by === 'parker' ? 'PARKER' : 'YOU';
    if (standing === 'open') return `${who} KEPT IT OPEN`;
    const at = daysAgo(mark.at) === 0 ? hhmm(mark.at) : date(mark.at).toUpperCase();
    return `${who} CLOSED IT, ${at}`;
  });
  /// The steps it left: NEXT while it is open, and SET ASIDE once a mark
  /// closed it with steps still on the card — kept, quiet, never struck,
  /// because nobody can tell from here whether they were done or dropped.
  const steps = $derived(card?.next?.length && (standing === 'open' || mark) ? card.next : []);
  const kicker = $derived(
    thread
      ? `${daysAgo(thread.t.updated) === 0 ? 'TODAY' : date(thread.t.updated).toUpperCase()} · ${hhmm(thread.t.updated)}`
      : '',
  );
  const length = $derived(
    thread
      ? [
          facts && plural(facts.turns, 'turn'),
          facts && plural(facts.steps, 'step'),
          facts?.work > 0 && `${span(facts.work)} of work`,
          thread.t.cost != null && `$${thread.t.cost.toFixed(2)}`,
        ]
          .filter(Boolean)
          .join(' · ')
      : '',
  );
  const lineage = $derived(
    thread
      ? [
          modelName(thread.t.model),
          thread.t.created &&
            midnight(thread.t.created) !== midnight(thread.t.updated) &&
            `started ${date(thread.t.created)}`,
        ]
          .filter(Boolean)
          .join(' · ')
      : '',
  );

  /// The work, as ledger rows: a count per kind, except edits, which name
  /// their files — the one kind where *which* is the point.
  const work = $derived.by(() => {
    if (!facts) return [];
    const edited = facts.edited ?? [];
    const out = (facts.tally ?? [])
      .filter((a) => !(a.glyph === '±' && edited.length))
      .map((a) => ({ key: a.glyph, glyph: a.glyph, count: a.count, noun: a.noun }));
    for (const e of edited.slice(0, 4)) out.push({ key: `±${e.target}`, glyph: '±', noun: e.verb, target: e.target });
    if (edited.length > 4) out.push({ key: '±more', glyph: '', noun: `and ${plural(edited.length - 4, 'more file')}` });
    return out;
  });

  /// A look's six colours as this screen would wear them: accent, good and
  /// bad held at 4.5:1 (themes.js), so the card says when one moved.
  const inks = $derived.by(() => {
    const t = current?.kind === 'choice' ? current.c.theme : null;
    if (!t) return [];
    const { vars } = resolve(t);
    const held = (k) => (vars[`--${k}`].toLowerCase() !== t[k].toLowerCase() ? `held at 4.5:1, was ${t[k]}` : '');
    return [
      { role: 'ground', hex: t.bg, note: '' },
      { role: 'ink', hex: t.fg, note: '' },
      { role: 'accent', hex: vars['--accent'], note: held('accent') },
      { role: 'good', hex: vars['--good'], note: held('good') },
      { role: 'bad', hex: vars['--bad'], note: held('bad') },
      { role: 'raised', hex: vars['--raised'], note: "the palette's surface" },
    ];
  });

  // ── the frame ────────────────────────────────────────────────────────────
  const count = $derived.by(() => {
    if (scope === 'threads') {
      const n = earlier.threads.length;
      if (q) return rows.length ? `${rows.length} of ${n}` : '';
      return n ? `${plural(n, 'thread')} · since ${date(earlier.threads.at(-1).updated)}` : '';
    }
    if (scope === 'choices') {
      const noun = { visage: 'looks', brain: 'brains' }[opened.id] ?? 'choices';
      return `${opened.choices.length} ${noun}${opened.now ? ` · ${opened.now} now` : ''}`;
    }
    return plural(rows.length, 'command');
  });

  const foot = $derived(
    scope === 'threads'
      ? ['^N ^P MOVE', '⏎ RESUME', ...(thread ? [standing === 'open' ? '^X DONE' : '^X REOPEN'] : []), '/ COMMANDS']
      : scope === 'commands'
        ? ['^N ^P MOVE', '⏎ RUN · › OPENS', '⌫ THREADS']
        : ['^N ^P MOVE', opened.id === 'visage' ? '⏎ WEAR IT' : '⏎ PICK', '⌫ COMMANDS'],
  );
  const footEnd = $derived(
    scope === 'choices' && opened.id === 'visage'
      ? `ESC · ^K CLOSE — ${app.theme.toUpperCase()} STAYS`
      : 'ESC · ^K CLOSE',
  );

  const empty = $derived(
    rows.length || reading || (scope === 'threads' && earlier.loading && !earlier.threads.length)
      ? ''
      : scope === 'threads' && !q
        ? 'nothing yet — say something first'
        : 'nothing matches',
  );
</script>

<!-- An overlay, not a third view: the views are two shapes of one
     conversation, and jumping between conversations must not live on
     that key. -->
<!-- The keyboard's way out is Esc or ^K, on the box; this is the pointer's. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="scrim" transition:fade={{ duration: 120 }} onmousedown={outside}>
  <div class="box">
    <div class="query">
      <span class="prompt">&gt;</span>
      <input
        bind:this={inputEl}
        bind:value={query}
        onkeydown={onkey}
        spellcheck="false"
        autocomplete="off"
        aria-label="search threads, or / for commands"
        placeholder="earlier — type to search, / for commands"
      />
      <span class="count">{count}{#if reading}<span class="busy"> · reading</span>{/if}</span>
    </div>

    <div class="split">
      <div class="list" bind:this={listEl}>
        {#each groups as g (g.key)}
          <div class="section" class:pinned={g.pinned}>
            <span>{g.label}</span>{#if g.sub}<span class="sub">{g.sub}</span>{/if}<span class="rule"></span>
          </div>
          {#each g.rows as r (r.key)}
            {@const i = indexOf.get(r.key)}
            <!-- mousedown is refused so a click never takes focus from the
                 box: the keyboard stays the palette's, and Enter never
                 presses a button. -->
            {#if r.kind === 'thread'}
              <button
                type="button"
                tabindex="-1"
                class="row thread"
                class:sel={i === sel}
                data-i={i}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => click(i, r)}
              >
                <span class="time">{r.pinned ? when(r.t.updated) : hhmm(r.t.updated)}</span>
                <span class="name">
                  {#if stateOf(r.t) === 'open'}<span class="dot" aria-label="left open"></span>{/if}{#if r.title}{r.title.before}<mark>{r.title.match}</mark>{r.title.after}{:else}{r.t.title}{/if}
                </span>
                <span class="mag">{cards[r.t.id]?.facts ? plural(cards[r.t.id].facts.turns, 'turn') : ''}</span>
                {#if r.line}
                  <span class="line">
                    <span class="where">{r.line.label}</span>
                    {#if r.line.plain != null}
                      <span class="text">{r.line.plain}</span>
                    {:else}
                      <span class="text">{r.line.before}<mark>{r.line.match}</mark>{r.line.after}</span>
                    {/if}
                    {#if r.more}<span class="more">{r.more}</span>{/if}
                  </span>
                {/if}
              </button>
            {:else if r.kind === 'verb'}
              <button
                type="button"
                tabindex="-1"
                class="row verb"
                class:sel={i === sel}
                data-i={i}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => click(i, r)}
              >
                <span class="cmd">/{r.v.id}</span>
                <span class="what">{r.v.what}</span>
                <span class="now">{r.v.now ?? ''}</span>
                <span class="key">{r.v.key ?? ''}</span>
                <span class="opens">{r.v.choices?.length ? '›' : ''}</span>
              </button>
            {:else}
              <button
                type="button"
                tabindex="-1"
                class="row choice"
                class:sel={i === sel}
                data-i={i}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => click(i, r)}
              >
                <span class="lead">
                  {#if r.c.theme}
                    <span class="chip" style:background={r.c.theme.bg}
                      ><i style:background={r.c.theme.fg}></i><i style:background={r.c.theme.accent}></i><i
                        style:background={r.c.theme.good}
                      ></i></span
                    >
                  {/if}
                </span>
                <span class="name">{#if r.c.now}<span class="dot" aria-label="now"></span>{/if}{r.c.arg}</span>
                <span class="hint">{r.c.hint}{r.c.now ? ' · now' : ''}</span>
              </button>
            {/if}
          {/each}
        {/each}
        {#if empty}<div class="empty">{empty}</div>{/if}
      </div>

      <div class="card">
        {#if thread}
          <div class="kicker">
            <span>{kicker}</span>
            {#if away}<span class="right">ON {away.toUpperCase()}</span>{/if}
          </div>
          <div class="title">{thread.t.title}</div>
          <div class="meta">
            {#if length}<div>{length}</div>{/if}
            {#if lineage}<div class="quiet">{lineage}</div>{/if}
          </div>

          {#if thread.said}
            <section>
              <div class="label">SAID · {plural(thread.said.n, 'LINE').toUpperCase()}</div>
              {#each thread.said.hits as h, n (n)}
                <div class="hit">
                  <span class="who">{h.who}</span>
                  <span>{h.before}<mark>{h.match}</mark>{h.after}</span>
                </div>
              {/each}
            </section>
          {/if}

          {#if card?.about || (card?.pending && !fresh)}
            <section>
              <div class="label">ABOUT</div>
              {#if card?.about}
                <div class="said" class:stale={!fresh}>
                  {#if thread.about}{thread.about.before}<mark>{thread.about.match}</mark>{thread.about.after}{:else}{card.about}{/if}
                </div>
              {/if}
              {#if card?.pending && !fresh}<div class="flight"><div class="sweep"></div></div>{/if}
            </section>
          {/if}

          {#if card?.landed || mark}
            <section>
              <div class="label" class:waiting={standing === 'open'}>
                {standing === 'open' ? 'LEFT OPEN' : 'LANDED'}{#if by}<span class="by">{` · ${by}`}</span>{/if}
              </div>
              {#if card?.landed}
                <div class="said" class:stale={!fresh}>
                  {#if thread.landed}{thread.landed.before}<mark>{thread.landed.match}</mark>{thread.landed.after}{:else}{card.landed}{/if}
                </div>
              {/if}
              {#if mark?.note}<div class="plain note">{mark.note}</div>{/if}
            </section>
          {/if}

          {#if steps.length}
            <section>
              <div class="label">{standing === 'open' ? 'NEXT' : 'SET ASIDE'}</div>
              {#each steps as s, n (n)}
                <div
                  class="step"
                  class:you={standing === 'open' && s.who === 'you'}
                  class:aside={standing !== 'open'}
                  class:stale={!fresh}
                >
                  <span class="who">{s.who === 'you' ? 'YOU' : 'PARKER'}</span>
                  <span>{s.step}</span>
                </div>
              {/each}
            </section>
          {/if}

          {#if work.length}
            <section>
              <div class="label">WORK</div>
              {#each work as a (a.key)}
                <div class="act">
                  <span class="glyph {kind(a.glyph)}">{a.glyph}</span>
                  <span class="line">
                    {#if a.count != null}
                      <span class="target">{a.count}</span> {a.noun}
                    {:else}
                      {a.noun}
                      {#if a.target}<span class="target">{a.target}</span>{/if}
                    {/if}
                  </span>
                </div>
              {/each}
            </section>
          {/if}
        {:else if current?.kind === 'verb'}
          {@const v = current.v}
          <div class="kicker">
            <span>{v.domain}</span>
            {#if v.key}<span class="right">{v.key}</span>{/if}
          </div>
          <div class="title">/{v.id}</div>
          <div class="meta">
            <div>{v.what}</div>
            {#if v.now}<div class="quiet">now {v.now}</div>{/if}
          </div>
          {#if v.choices?.length}
            <section>
              <div class="label">{v.choicesLabel ?? 'CHOICES'}</div>
              {#each v.card ?? v.choices as c (c.arg)}
                <div class="opt">
                  {#if c.theme}
                    <span class="chip" style:background={c.theme.bg}
                      ><i style:background={c.theme.fg}></i><i style:background={c.theme.accent}></i><i
                        style:background={c.theme.good}
                      ></i></span
                    >
                  {/if}
                  <span class="name" class:now={c.now}>{c.arg}</span>
                  {#if c.now}<span class="tag">NOW</span>{/if}
                </div>
              {/each}
            </section>
          {/if}
          <section>
            <div class="label">WHERE IT LANDS</div>
            {#each v.lands as l (l)}<div class="plain">{l}</div>{/each}
          </section>
        {:else if current?.kind === 'choice'}
          {@const v = current.v}
          {@const c = current.c}
          <div class="kicker">
            <span>{v.domain} · /{v.id}</span>
            {#if c.now}<span class="right state">NOW</span>{/if}
          </div>
          <div class="title">{c.arg}</div>
          <div class="meta">
            {#if c.theme}
              <div>{c.theme.world ? `the ${c.theme.world}` : "visage's own"} · {polarity(c.theme)}</div>
            {:else if c.model}
              <div>{shortModel(c.model)}</div>
              <div class="quiet">{c.model}</div>
            {:else}
              <div>{c.hint}</div>
            {/if}
          </div>
          {#if v.body === 'inks' && inks.length}
            <section>
              <div class="label">INKS</div>
              {#each inks as k (k.role)}
                <div class="ink">
                  <span class="swatch" style:background={k.hex}></span>
                  <span>{k.role}</span>
                  <span class="hex">{k.hex}{#if k.note}<span class="note">{k.note}</span>{/if}</span>
                </div>
              {/each}
            </section>
          {/if}
          {#if v.body === 'lore' && c.lore}
            <section>
              <div class="label">NAMED FOR</div>
              <div class="plain">{c.lore}</div>
            </section>
          {/if}
          <section>
            <div class="label">WHERE IT LANDS</div>
            {#each v.lands as l (l)}<div class="plain">{l}</div>{/each}
          </section>
        {/if}
      </div>
    </div>

    <div class="foot">
      {#each foot as f (f)}<span>{f}</span>{/each}
      <span class="end">{footEnd}</span>
    </div>
  </div>
</div>

<style>
  /* px for the 2560-wide screen: History's box, the palette's query line,
     one radius, one lift. Every colour is a rung (themes.js) — the swatches
     and chips alone carry a look's own colours, because they are data. */
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 5;
    background: var(--bg-55);
  }
  .box {
    position: absolute;
    left: 50%;
    top: 120px;
    transform: translateX(-50%);
    width: min(2000px, 92vw);
    height: min(1180px, calc(100vh - 200px));
    display: flex;
    flex-direction: column;
    background: var(--raised);
    border: 1px solid var(--fg-16);
    border-radius: 24px;
    box-shadow: var(--palette-lift);
    overflow: hidden;
  }

  /* ── the query line, where ^Y's heading was ─────────────────────────── */
  .query {
    flex: none;
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 28px 36px;
    border-bottom: 1px solid var(--fg-10);
    font-size: 26px;
  }
  .prompt {
    flex: none;
    color: var(--accent-90);
  }
  input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--fg);
    font: inherit;
    caret-color: var(--accent-70);
  }
  input::placeholder {
    color: var(--fg-22);
  }
  .count {
    flex: none;
    font-size: 20px;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
    color: var(--fg-28);
  }
  .busy {
    color: var(--accent-50);
  }

  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) min(900px, 45%);
  }

  /* ── the list ───────────────────────────────────────────────────────── */
  .list {
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 4px 0 16px;
    border-right: 1px solid var(--fg-10);
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .section {
    display: flex;
    align-items: baseline;
    gap: 18px;
    padding: 26px 36px 8px;
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--fg-28);
  }
  .section .sub {
    letter-spacing: 0.1em;
    color: var(--fg-18);
  }
  .section .rule {
    flex: 1;
    height: 1px;
    align-self: center;
    background: var(--fg-8);
  }
  /* The one section that names a state: what is left open, on top. */
  .section.pinned > span:first-child {
    color: var(--accent-80);
  }

  /* One grid for every kind of row, so a thread, a command and a choice
     line up the same way under one another. */
  .row {
    display: grid;
    grid-template-columns: 5ch minmax(0, 1fr) auto;
    column-gap: 24px;
    align-items: baseline;
    width: 100%;
    margin: 0;
    padding: 9px 36px 9px 34px;
    border: 0;
    border-left: 2px solid transparent;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: 24px;
    line-height: 1.4;
    text-align: left;
  }
  .row.sel {
    background: var(--accent-14);
    border-left-color: var(--accent);
  }
  .time {
    font-size: 21px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-32);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 14px;
    vertical-align: 3px;
    border-radius: 50%;
    background: var(--accent);
  }
  .mag {
    font-size: 20px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    color: var(--fg-32);
  }
  .row.sel .time,
  .row.sel .mag {
    color: var(--fg-45);
  }
  mark {
    background: var(--accent-20);
    color: var(--fg);
    border-radius: 3px;
  }

  /* Where a thread matched, under its title: one line, the match marked. */
  .row > .line {
    grid-column: 2 / 4;
    display: flex;
    align-items: baseline;
    gap: 16px;
    min-width: 0;
    margin-top: 2px;
    font-size: 21px;
    line-height: 1.5;
    color: var(--fg-45);
  }
  .row.sel > .line {
    color: var(--fg-60);
  }
  .where {
    flex: none;
    font-size: 17px;
    letter-spacing: 0.16em;
    color: var(--fg-25);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    flex: none;
    font-size: 19px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-28);
  }

  /* A command: its name, what it does, what is on now, its key, and a ›
     when it has choices behind it. */
  .row.verb {
    grid-template-columns: 12ch minmax(0, 1fr) auto 9ch 1ch;
  }
  .cmd {
    white-space: nowrap;
    color: var(--fg-85);
  }
  .what {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 22px;
    color: var(--fg-45);
  }
  .now {
    font-size: 22px;
    white-space: nowrap;
    color: var(--fg-72);
  }
  .key {
    font-size: 20px;
    letter-spacing: 0.08em;
    text-align: right;
    white-space: nowrap;
    color: var(--fg-32);
  }
  .opens {
    font-size: 22px;
    color: var(--fg-28);
  }
  .row.sel .what {
    color: var(--fg-60);
  }
  .row.sel .key,
  .row.sel .opens {
    color: var(--fg-45);
  }

  .lead {
    display: flex;
    align-items: center;
  }
  .hint {
    font-size: 21px;
    white-space: nowrap;
    color: var(--fg-32);
  }
  .row.sel .hint {
    color: var(--fg-45);
  }

  /* A look's chip: its own ground, and its ink, accent and good on it, as
     dots — parker is dots. Data, not an icon. */
  .chip {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 5px;
    width: 52px;
    height: 30px;
    box-sizing: border-box;
    border: 1px solid var(--fg-16);
    border-radius: 6px;
  }
  .chip i {
    display: block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .empty {
    padding: 26px 40px;
    font-size: 22px;
    font-style: italic;
    color: var(--fg-35);
  }

  /* ── the card ───────────────────────────────────────────────────────── */
  .card {
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
    gap: 34px;
    padding: 44px 56px;
  }
  .card::-webkit-scrollbar {
    display: none;
  }
  .kicker {
    display: flex;
    gap: 18px;
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--fg-28);
  }
  .kicker .right {
    margin-left: auto;
    color: var(--fg-45);
  }
  .kicker .right.state {
    color: var(--accent-80);
  }
  .title {
    margin-top: -14px;
    font-size: 38px;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }
  .meta {
    margin-top: -18px;
    font-size: 21px;
    line-height: 1.7;
    font-variant-numeric: tabular-nums;
    color: var(--fg-45);
  }
  .meta .quiet {
    color: var(--fg-28);
  }
  .label {
    margin-bottom: 14px;
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--fg-25);
  }
  /* Not `.open`-ish names: the list's dot and this label must never share
     a class, or its box lands here. */
  .label.waiting {
    color: var(--accent-80);
  }
  /* Whose word it is, after the state: quieter than the state itself. */
  .label .by {
    letter-spacing: 0.12em;
    color: var(--fg-45);
  }
  .note {
    margin-top: 10px;
  }

  /* A step the thread left: who, then what. Yours are the brightest thing
     on the card after its title, because you are the one who acts on them;
     parker's sit at secondary prose; set-aside steps drop to quiet chrome. */
  .step {
    display: grid;
    grid-template-columns: 9ch minmax(0, 1fr);
    column-gap: 12px;
    align-items: baseline;
    font-size: 26px;
    line-height: 1.5;
    color: var(--fg-55);
  }
  .step + .step {
    margin-top: 12px;
  }
  .step.you {
    color: var(--fg-85);
  }
  .step.aside,
  .step.stale {
    color: var(--fg-32);
  }
  .step .who {
    font-size: 17px;
    letter-spacing: 0.16em;
    color: var(--fg-25);
  }
  .said {
    font-size: 26px;
    line-height: 1.6;
    color: var(--fg-72);
  }
  .said.stale {
    color: var(--fg-45);
  }
  .plain {
    font-size: 23px;
    line-height: 1.6;
    color: var(--fg-45);
  }

  /* What was said, as hits: who said it, the line, the match marked. */
  .hit {
    display: grid;
    grid-template-columns: 8ch minmax(0, 1fr);
    column-gap: 12px;
    font-size: 23px;
    line-height: 1.55;
    color: var(--fg-55);
  }
  .hit + .hit {
    margin-top: 10px;
  }
  .who {
    padding-top: 4px;
    font-size: 17px;
    letter-spacing: 0.16em;
    color: var(--fg-25);
  }

  .act {
    display: flex;
    align-items: baseline;
    font-size: 23px;
    line-height: 1.55;
    color: var(--fg-45);
  }
  .glyph {
    flex: none;
    width: 24px;
    color: var(--fg-30);
  }
  .glyph.bash {
    color: var(--accent-60);
  }
  .glyph.files {
    color: var(--good-55);
  }
  .act .line {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .target {
    color: var(--fg-72);
  }

  /* A command's choices on its card. */
  .opt {
    display: flex;
    align-items: center;
    gap: 18px;
    font-size: 23px;
    line-height: 1.75;
    color: var(--fg-60);
  }
  .opt .name.now {
    color: var(--fg);
  }
  .tag {
    font-size: 19px;
    letter-spacing: 0.12em;
    color: var(--accent-80);
  }

  /* A look's colours, resolved: a swatch, the role, the hex. */
  .ink {
    display: grid;
    grid-template-columns: 34px 9ch minmax(0, 1fr);
    column-gap: 18px;
    align-items: center;
    font-size: 22px;
    line-height: 1.9;
    color: var(--fg-45);
  }
  .swatch {
    width: 26px;
    height: 26px;
    box-sizing: border-box;
    border: 1px solid var(--fg-16);
    border-radius: 3px;
  }
  .hex {
    font-variant-numeric: tabular-nums;
    color: var(--fg-72);
  }
  .note {
    margin-left: 16px;
    color: var(--fg-32);
  }

  /* While a card is being written: the running act's sweep (Message.svelte). */
  .flight {
    height: 1px;
    margin-top: 8px;
    background: var(--fg-8);
    overflow: hidden;
  }
  .sweep {
    width: 36%;
    height: 1px;
    background: var(--accent-70);
    animation: sweep 1.4s ease-in-out infinite;
  }
  @keyframes sweep {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(280%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .sweep {
      animation: none;
    }
  }

  .foot {
    flex: none;
    display: flex;
    gap: 32px;
    padding: 18px 36px;
    border-top: 1px solid var(--fg-10);
    font-size: 20px;
    letter-spacing: 0.12em;
    color: var(--fg-25);
  }
  .foot .end {
    margin-left: auto;
  }
</style>
