<script>
  // One turn, and the one place a message's *arrival* is described.
  //
  // A turn renders as its blocks, in the order they happened (the grammar is
  // shaped in state.svelte.js): prose that types in behind a caret, code and
  // acts that land whole, gestures in italics. Each block fades in once,
  // keyed on the engine's part ids, and never re-renders.
  //
  //   your turn   a tinted band with an accent left rule
  //   text        28px prose. Types at ~36 chars/s while `live`, counting
  //               the characters you see, not the markdown under them; the
  //               caret rides the head, then parks dim at the end of the
  //               last block and stops blinking. Its markdown is set in
  //               visage's grammar (the rules below; lib/markdown.js).
  //   code        no card — a left rule, an indent, an uppercase label
  //   table       lands whole: chrome labels for its head, one hairline
  //   act         one ledger line: glyph · verb target · magnitude. Never
  //               output. Running acts get a sweeping bar; failed ones turn
  //               bad and say so.
  //   fold        a long run of acts, minus its last three: one row saying
  //               how many and of what kind. Once the turn is done, the
  //               whole run, with a one-line summary. See `folded` below.
  //   gesture     italic, one line (dance only — WEARS names the face)
  //   footer      under a finished turn: how long, how many model calls,
  //               what it cost, which brain answered
  //
  // While a prose block is still typing, the blocks after it wait: a table
  // or a ledger row lands when the sentence before it is finished, so the
  // turn reads in the order it was said.
  //
  // Still a hook, not designed yet: a sources pill row once the engine hands
  // over citations.
  import { fade } from 'svelte/transition';
  import { render, reveal } from './markdown.js';
  import { summaries, summarize, tallyOf, brainOf } from './state.svelte.js';

  let { message, live = false, latest = false } = $props();

  // How far each prose block has typed, by block key. Plain instance state:
  // the keyed {#each} in Chat keeps this component alive across refetches,
  // so finished text does not restart typing when its block is recreated.
  const shown = new Map();
  const CPS = 36;

  let tick = $state(0);
  $effect(() => {
    if (!live) return;
    const t = setInterval(() => (tick = tick + 1), 1000 / 20);
    return () => clearInterval(t);
  });

  const textBlocks = $derived((message.blocks ?? []).filter((b) => b.kind === 'text'));
  const lastTextKey = $derived(textBlocks.at(-1)?.key);

  /// The visible slice of a prose block, in rendered characters, advanced by
  /// the ticker while the turn is live. Blocks that arrived before the tail
  /// are shown whole — only the newest one types.
  function revealed(b) {
    const full = render(b.text).length;
    if (!live || b.key !== lastTextKey) {
      shown.set(b.key, full);
      return full;
    }
    tick;
    const now = shown.get(b.key) ?? 0;
    const next = Math.min(full, now + CPS / 20);
    shown.set(b.key, next);
    return next;
  }

  /// Which caret a prose block carries: the typing head while live, the
  /// parked one on the very last block once the turn is done.
  function caretFor(b, n) {
    if (live && b.key === lastTextKey) return 'live';
    if (!live && latest && b.key === lastTextKey && n >= render(b.text).length) return 'parked';
    return '';
  }

  /// Everything after a prose block that is still typing waits for it.
  /// Only the tail block types, so at most one place in the turn holds.
  const holdFrom = $derived.by(() => {
    if (!live) return Infinity;
    tick;
    const i = folded.findIndex((b) => b.key === lastTextKey);
    if (i < 0) return Infinity;
    const b = folded[i];
    return (shown.get(b.key) ?? 0) < render(b.text).length ? i + 1 : Infinity;
  });

  // A long stretch of tool calls is one paragraph of the turn, not thirty.
  // Twenty reads scrolls the sentence parker was in the middle of off the
  // screen, and reading them was never the point — the ledger exists so you
  // can see *that* he is working and roughly at what.
  //
  // So a run of consecutive acts keeps its last KEEP rows — the ones that
  // say what he is doing now, including whatever is still in flight — and
  // everything before them becomes one row: how many, and a tally of what
  // kind. Only *consecutive* acts fold: prose between two runs means he
  // said something about the first one, and that is a paragraph break.
  //
  // Nothing is hidden that you cannot infer: failures are counted on the
  // fold row and turn it bad, so a run that swallowed an error still says
  // so. There is no expand key, on purpose — the fold summarises acts, and
  // an act never carried its output to begin with.
  //
  // Once the turn is done the last three have nothing left to say about
  // what he is doing *now*, so the whole run folds — the redesign's "old
  // ledger rows fold as the conversation moves past them" — and the row
  // gains the run's wall time and a one-line summary from the engine's
  // ledger agent. Until the summary lands, the tally stands in for it.
  const KEEP = 3;

  /// Runs this long get a written summary; shorter ones read fine as a tally.
  const SUMMARIZE_FROM = 3;

  /// `9 read · 4 ran` — the top three kinds (state.svelte.js `tallyOf`,
  /// which a thread's card in the palette counts with too).
  function tally(acts) {
    return tallyOf(acts)
      .slice(0, 3)
      .map((t) => `${t.count} ${t.noun}`)
      .join(' · ');
  }

  /// `4.2s`, `48s`, `3m 12s` — from epoch milliseconds.
  function span(ms) {
    if (!(ms > 0)) return '';
    const s = ms / 1000;
    if (s < 10) return `${s.toFixed(1)}s`;
    if (s < 60) return `${Math.round(s)}s`;
    const m = Math.floor(s / 60);
    return `${m}m ${Math.round(s - m * 60)}s`;
  }

  /// First act starting to last act ending: the time the run took, model
  /// calls between the acts included, which is the number you feel.
  function runTime(acts) {
    const starts = acts.map((a) => a.start).filter((t) => typeof t === 'number');
    const ends = acts.map((a) => a.end).filter((t) => typeof t === 'number');
    if (!starts.length || !ends.length) return '';
    return span(Math.max(...ends) - Math.min(...starts));
  }

  const done = $derived(!!message.done && !live);

  /// A level's change as cells: lit up to where it is now, and the cells it
  /// left outlined when it went down.
  const LEVEL_CELLS = 20;
  function levelCells(c) {
    const at = (v) => Math.round((Math.max(0, Math.min(100, v)) / 100) * LEVEL_CELLS);
    const to = at(c.to);
    const from = at(c.from);
    return Array.from({ length: LEVEL_CELLS }, (_, i) => (i < to ? 'lit' : i < from ? 'was' : ''));
  }

  /// The spaces a goto is drawn among: 1 to 4, or as far as it went.
  const spaceRun = (c) => Array.from({ length: Math.max(4, c.to, c.from ?? 0) }, (_, i) => i + 1);

  /// The blocks as rendered: the same list, with each act run longer than
  /// KEEP + 1 collapsed — or, once the turn is done, every run of two or
  /// more. Folding a single row into a row saying one row is folded is worse
  /// than not folding, hence the +1 and the two.
  ///
  /// The fold's key is its first hidden act's, so a run that grows while the
  /// turn is live updates the count in place rather than fading a new row in
  /// on every tool call — and the same row becomes the finished fold.
  const folded = $derived.by(() => {
    const out = [];
    let run = [];
    const fold = (hidden, whole) => ({
      kind: 'fold',
      key: `fold·${hidden[0].key}`,
      count: hidden.length,
      failed: hidden.filter((a) => a.status === 'failed').length,
      tally: tally(hidden),
      time: whole ? runTime(hidden) : '',
      whole,
      acts: hidden,
    });
    const flush = () => {
      if (done && run.length >= 2) {
        out.push(fold(run, true));
      } else if (run.length > KEEP + 1) {
        out.push(fold(run.slice(0, run.length - KEEP), false));
        out.push(...run.slice(-KEEP));
      } else {
        out.push(...run);
      }
      run = [];
    };
    for (const b of message.blocks ?? []) {
      // An act that changed the machine stays out of the fold: the change
      // drawn under it is the point of the turn, not a step on the way.
      if (b.kind === 'act' && !b.change) run.push(b);
      else {
        flush();
        out.push(b);
      }
    }
    flush();
    return out;
  });

  // Ask for each finished run's summary. `summarize` is idempotent per key,
  // so this re-running when a summary lands is a no-op.
  $effect(() => {
    for (const b of folded) {
      if (b.kind === 'fold' && b.whole && b.count >= SUMMARIZE_FROM) {
        summarize(b.key, message.ask, b.acts);
      }
    }
  });

  /// `3m 31s · 53 steps · $1.21 · deep thought` under a finished turn. Cost
  /// is what the engine reports per step, summed; a free provider reports
  /// zero and the figure is left off rather than claiming the turn was free.
  /// The brain is the one the engine recorded on the turn's last step, so a
  /// thread read back after a switch still says who answered what.
  const footer = $derived.by(() => {
    if (!done || message.role !== 'assistant' || !message.steps) return '';
    const parts = [];
    if (message.start && message.end) parts.push(span(message.end - message.start));
    parts.push(`${message.steps} ${message.steps === 1 ? 'step' : 'steps'}`);
    if (message.cost > 0) parts.push(message.cost < 0.01 ? '<$0.01' : `$${message.cost.toFixed(2)}`);
    const brain = brainOf(message.model);
    if (brain) parts.push(brain);
    return parts.join(' · ');
  });
</script>

<div class="msg {message.role}" in:fade={{ duration: 220 }}>
  {#if message.role === 'user'}
    <div class="turn">{message.text}</div>
  {:else if message.blocks?.length}
    {#each folded as b, i (b.key)}
      {#if i >= holdFrom}
        <!-- waits for the sentence before it -->
      {:else if b.kind === 'text'}
        {@const n = revealed(b)}
        <div class="prose">
          {@html reveal(render(b.text).html, Math.ceil(n), caretFor(b, n))}
        </div>
      {:else if b.kind === 'table'}
        <div class="prose table" in:fade={{ duration: 200 }}>{@html render(b.text).html}</div>
      {:else if b.kind === 'code'}
        <div class="code" in:fade={{ duration: 200 }}>
          {#if b.lang || b.path}
            <div class="label">{[b.lang, b.path].filter(Boolean).join(' · ')}</div>
          {/if}
          <pre>{b.code}</pre>
        </div>
      {:else if b.kind === 'gesture'}
        <div class="gesture" in:fade={{ duration: 200 }}>({b.text})</div>
      {:else if b.kind === 'act'}
        <div class="act" class:failed={b.status === 'failed'} in:fade={{ duration: 200 }}>
          <div class="row">
            <span class="glyph" class:bash={b.glyph === '$'} class:files={b.glyph === '±'}
              >{b.glyph}</span
            >
            <span class="line"
              >{b.verb} <span class="target">{b.target}</span>{#if b.status === 'failed'}&nbsp;— failed{/if}</span
            >
            {#if b.magnitude}
              <span class="magnitude">{b.magnitude}</span>
            {/if}
          </div>
          {#if b.status === 'running'}
            <div class="flight"><div class="sweep"></div></div>
          {/if}
          {#if b.change && b.status === 'ok'}
            {@const c = b.change}
            <div class="vis">
              {#if c.kind === 'level'}
                <span class="rich">{#each levelCells(c) as cell, i (i)}<i class={cell}></i>{/each}</span>
                <span><span class="from">{c.from}</span> → <span class="to">{c.to}</span></span>
              {:else if c.kind === 'link'}
                <span class="from">{c.from}</span><span>→</span>
                <span class="dot" class:lit={c.to === 'connected'}></span><span class="now">{c.to}</span>
              {:else if c.kind === 'space'}
                <span>space</span>
                <span class="spaces"
                  >{#each spaceRun(c) as n (n)}<span class:to={n === c.to} class:was={n === c.from}>{n}</span>{/each}</span
                >
              {:else}
                <span class="from">{c.from}</span><span>→</span><span class="to">{c.to}</span>
              {/if}
            </div>
          {/if}
        </div>
      {:else if b.kind === 'fold'}
        {@const said = b.whole ? summaries[b.key] : ''}
        {@const noun = b.count === 1 ? 'act' : 'acts'}
        <div class="act fold" class:failed={b.failed > 0} in:fade={{ duration: 200 }}>
          <div class="row">
            <span class="glyph">⋯</span>
            <span class="line"
              >{#if said}<span class="said">{said}</span>{:else}{b.count}{b.whole ? '' : ' earlier'} {noun}{/if}{#if b.failed}&nbsp;— {b.failed} failed{/if}</span
            >
            <span class="magnitude"
              >{[said ? `${b.count} ${noun}` : b.tally, b.time].filter(Boolean).join(' · ')}</span
            >
          </div>
        </div>
      {/if}
    {/each}
  {:else if message.text}
    <div class="prose">{@html render(message.text).html}</div>
  {/if}
  {#if footer}
    <div class="footer" in:fade={{ duration: 300 }}>{footer}</div>
  {/if}
</div>

<style>
  .msg {
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  /* Your turn: a band, not a bubble. Full column width, an accent rule. */
  .turn {
    background: var(--accent-9);
    border-left: 2px solid var(--accent-55);
    border-radius: 3px;
    padding: 14px 24px;
    font-size: 24px;
    line-height: 1.55;
    white-space: pre-wrap;
  }

  /* Prose, and the markdown in it, set in visage's grammar rather than the
     browser's: nothing louder than prose, accent only on what you can act
     on, every colour a rung. */
  .prose {
    font-size: 28px;
    line-height: 1.6;
  }

  /* Every block a breath after the one before. A quote or a break gets more;
     a heading more above than below, so it belongs to what follows it. */
  .prose > :global(* + *) {
    margin-top: 0.6em;
  }
  .prose > :global(* + blockquote),
  .prose > :global(blockquote + *),
  .prose > :global(* + hr),
  .prose > :global(hr + *) {
    margin-top: 28px;
  }
  .prose > :global(* + :is(h1, h2, h3, h4, h5, h6)) {
    margin-top: 48px;
  }
  .prose > :global(:is(h1, h2, h3, h4, h5, h6) + *) {
    margin-top: 10px;
  }

  /* A heading is a chrome label — uppercase, tracked, quiet — and the top
     two carry the palette's day-header rule out to the edge. A section is
     named, never shouted. */
  .prose :global(:is(h1, h2, h3, h4, h5, h6)) {
    font-size: 19px;
    line-height: 1.6;
    font-weight: 400;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--fg-35);
  }
  .prose :global(h1) {
    color: var(--fg-55);
  }
  .prose :global(h2) {
    color: var(--fg-45);
  }
  .prose :global(:is(h1, h2)) {
    overflow: hidden;
  }
  .prose :global(:is(h1, h2)::after) {
    content: '';
    display: inline-block;
    width: 100%;
    height: 1px;
    margin: 0 -100% 0 18px;
    vertical-align: middle;
    background: var(--fg-8);
  }

  /* Bold only where parker asked for it — the one place visage sets it. */
  .prose :global(strong) {
    font-weight: 700;
    color: var(--fg);
  }
  .prose :global(em) {
    color: var(--fg-55);
  }
  .prose :global(del) {
    color: var(--fg-35);
    text-decoration-thickness: 2px;
  }
  .prose :global(code) {
    font-size: 0.9em;
    background: var(--fg-8);
    padding: 1px 8px;
    border-radius: 3px;
  }

  /* A link is the sentence's own words with an accent underline: the colour
     of the thing you can act on, not of the text. */
  .prose :global(a) {
    color: inherit;
    text-decoration: underline;
    text-decoration-color: var(--accent-50);
    text-decoration-thickness: 2px;
    text-underline-offset: 0.24em;
  }

  /* Lists: the separator glyph as the marker; numbers hung quietly, and
     native, so a list that starts at 3 still says 3. */
  .prose :global(ul) {
    list-style: none;
  }
  .prose :global(ol) {
    padding-left: 3ch;
  }
  .prose :global(ol > li::marker) {
    color: var(--fg-35);
    font-variant-numeric: tabular-nums;
  }
  .prose :global(ul > li) {
    position: relative;
    padding-left: 2ch;
  }
  .prose :global(ul > li::before) {
    content: '·';
    position: absolute;
    left: 0.45ch;
    color: var(--fg-45);
  }
  .prose :global(ul ul > li::before) {
    color: var(--fg-28);
  }
  .prose :global(li + li),
  .prose :global(li > :is(ul, ol)) {
    margin-top: 0.15em;
  }

  /* Tasks: hollow while waiting, a tick once done — and done recedes, dim
     and never struck, the way SET ASIDE does in ^K. */
  .prose :global(li input[type='checkbox']) {
    display: none;
  }
  .prose :global(ul > li:has(> input[type='checkbox'], > p > input[type='checkbox'])::before) {
    content: '○';
    left: 0;
  }
  .prose :global(ul > li:has(> input:checked, > p > input:checked)) {
    color: var(--fg-45);
  }
  .prose :global(ul > li:has(> input:checked, > p > input:checked)::before) {
    content: '✓';
    color: var(--fg-35);
  }

  /* A quote: the code block's indent with a quiet rule — someone else's
     words, not parker's and not code. */
  .prose :global(blockquote) {
    padding-left: 28px;
    border-left: 1px solid var(--fg-16);
    color: var(--fg-60);
  }
  .prose :global(blockquote > * + *) {
    margin-top: 0.6em;
  }

  /* Callouts (`> [!NOTE]`, markdown.js): a label over a quote. Warning and
     caution borrow bad, which means stop; important, accent for its label. */
  .prose :global(blockquote.alert) {
    color: var(--fg-72);
  }
  .prose :global(.alert-label) {
    font-size: 17px;
    line-height: 1.9;
    letter-spacing: 0.16em;
    color: var(--fg-35);
  }
  .prose :global(blockquote > .alert-label + *) {
    margin-top: 0;
  }
  .prose :global(.alert.important .alert-label) {
    color: var(--accent-80);
  }
  .prose :global(blockquote:is(.warning, .caution)) {
    border-left-color: var(--bad-45);
    color: var(--fg);
  }
  .prose :global(:is(.warning, .caution) .alert-label) {
    color: var(--bad-65);
  }

  /* A table: no grid. Chrome labels for the head, one hairline under it,
     figures in tabular numerals, the first column at full ink. */
  .prose :global(table) {
    border-collapse: collapse;
    font-size: 24px;
    line-height: 1.6;
    font-variant-numeric: tabular-nums;
  }
  .prose :global(th) {
    padding: 0 56px 12px 0;
    border-bottom: 1px solid var(--fg-10);
    font-size: 17px;
    font-weight: 400;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    text-align: left;
    vertical-align: bottom;
    color: var(--fg-35);
  }
  .prose :global(td) {
    padding: 10px 56px 0 0;
    vertical-align: baseline;
    color: var(--fg-72);
  }
  .prose :global(tbody tr:first-child td) {
    padding-top: 16px;
  }
  .prose :global(td:first-child) {
    color: var(--fg);
  }
  .prose :global(:is(th, td)[align='right']) {
    text-align: right;
  }
  .prose :global(:is(th, td)[align='center']) {
    text-align: center;
  }
  .prose :global(:is(th, td):last-child) {
    padding-right: 0;
  }
  .prose.table {
    overflow-x: auto;
  }

  /* A break: three dots, the separator, spaced. */
  .prose :global(hr) {
    border: 0;
    color: var(--fg-28);
  }
  .prose :global(hr::before) {
    content: '·   ·   ·';
    white-space: pre;
  }
  .prose :global(.cursor) {
    display: inline-block;
    width: 16px;
    height: 32px;
    vertical-align: -4px;
    margin-left: 6px;
  }
  .prose :global(.cursor.live) {
    background: var(--accent-80);
  }
  .prose :global(.cursor.parked) {
    background: var(--accent-35);
  }

  /* Code: no card. A rule, an indent, a label. */
  .code {
    border-left: 1px solid var(--accent-35);
    padding: 4px 0 4px 28px;
    overflow-x: auto;
  }
  .code .label {
    font-size: 19px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--fg-28);
    margin-bottom: 10px;
  }
  .code pre {
    font-size: 24px;
    line-height: 1.8;
    white-space: pre;
  }

  .gesture {
    font-style: italic;
    font-size: 23px;
    color: var(--fg-42);
  }

  /* The ledger: one line per act, no output, ever. */
  .act {
    font-size: 23px;
    line-height: 1.55;
    color: var(--fg-45);
  }
  .act .row {
    display: flex;
    align-items: baseline;
    gap: 0;
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
    padding-right: 24px;
  }
  .target {
    color: var(--fg-72);
  }
  .magnitude {
    flex: none;
    font-size: 20px;
    color: var(--fg-28);
  }
  /* The fold sits under the rows it stands for: dimmer than an act, and no
     target — there is no one thing it points at. */
  .act.fold {
    color: var(--fg-30);
  }
  /* A written summary is a sentence about the work, so it reads at the
     brightness of an act's target rather than the fold's grey. */
  .act.fold .said {
    color: var(--fg-58);
  }

  /* Under a finished turn: the bill, quietly. Dimmer than any act. */
  .footer {
    margin-top: -12px;
    font-size: 19px;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
    color: var(--fg-22);
  }

  .act.failed,
  .act.failed .glyph,
  .act.failed .target,
  .act.failed .said,
  .act.failed .magnitude {
    color: var(--bad-65);
  }

  /* What a change did, drawn under its row (state.svelte.js `changeOf`):
     cells for a level, a dot for a link, the spaces for a goto. */
  .vis {
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 8px 0 4px 24px;
    font-size: 21px;
    color: var(--fg-45);
  }
  .rich {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .rich i {
    width: 10px;
    height: 22px;
    border-radius: 1px;
    background: var(--fg-10);
  }
  .rich i.lit {
    background: var(--accent-75);
  }
  .rich i.was {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--fg-28);
  }
  .vis .from {
    color: var(--fg-35);
  }
  .vis .to {
    color: var(--accent);
  }
  .vis .now {
    color: var(--fg-72);
  }
  .vis .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--fg-28);
  }
  .vis .dot.lit {
    background: var(--accent);
  }
  .vis .spaces {
    display: flex;
    gap: 14px;
  }
  .vis .spaces .was {
    color: var(--fg-35);
    text-decoration: line-through;
  }

  /* While an act is in flight, an indeterminate bar sweeps under the row. */
  .flight {
    margin-top: 8px;
    height: 1px;
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
</style>
