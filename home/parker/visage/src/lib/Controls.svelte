<script>
  // ^, — the controls, in the transcript's place (docs/visage/controls/
  // README.md). parker steps to the left rail (App gives the stage the
  // session shape while this is up) and the transcript's column becomes two
  // columns of rows: SOUND, PLAYING, NETWORK, BLUETOOTH on the left; LOOK,
  // QUIET, PARKER, SESSION on the right. The input line stays at the foot
  // and becomes the find or ask line; the key strip says the mode.
  //
  // This file draws; controls.svelte.js holds the state and the keys. The
  // grammar is the palette's and the ledger's: one family, ink by rank,
  // accent spent on state only — the selected row's band, the option that is
  // on, the space you are on. A row: its label, what it is in fg-35, a word
  // about now, then its value drawn — block cells for a level (Peter's pick,
  // over a meter ramp, the face's dots and a hairline) — and as a number or
  // a word.
  //
  // States: selected is the palette's band and rule; editing is outlined in
  // accent with an EDIT tag; in flight is the ledger's sweep under the row,
  // never a spinner; failed is `bad`, always with a word; unavailable is dim
  // and says why; the tower's own rows on the porthole are read-only and say
  // so; a row that moved without this page says so once.
  import { fade } from 'svelte/transition';
  import Message from './Message.svelte';
  import { app } from './state.svelte.js';
  import {
    ctl,
    LEFT,
    RIGHT,
    shown,
    strip,
    replies,
    locked,
    tap,
    tapOption,
    tapLevel,
    typeFind,
    key,
    rowOf,
    byKeys,
  } from './controls.svelte.js';

  const CELLS = 16;

  let lineEl = $state();
  let rowsEl = $state();

  const cols = $derived([shown(LEFT), shown(RIGHT)]);
  const count = $derived.by(() => {
    const all = (ctl.sections ?? []).flatMap((s) => s.rows.flatMap((r) => [r, ...(r.children ?? [])]));
    const real = all.filter((r) => r.kind !== 'info' && r.kind !== 'group').length;
    const showing = cols.flat().reduce((n, s) => n + s.rows.length, 0);
    return ctl.filter ? `${showing} shown` : `${real} settings`;
  });
  const said = $derived(replies());
  const keys = $derived(strip());

  /// App's caret rules while ^, is up: the line, when there is one to type in.
  export function focus() {
    if (ctl.mode === 'find' || ctl.mode === 'ask') lineEl?.focus();
  }

  /// The line at the foot takes the caret in FIND and ASK, and gives it up
  /// otherwise, so nothing typed in NORMAL lands anywhere.
  $effect(() => {
    if (ctl.mode === 'find' || ctl.mode === 'ask') lineEl?.focus();
    else if (document.activeElement === lineEl) lineEl?.blur();
  });

  // Moving keeps the selected row on screen.
  $effect(() => {
    const id = ctl.sel;
    cols;
    rowsEl?.querySelector(`[data-id="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  /// What the line holds is FIND's filter or ASK's words, whichever mode.
  function oninput(e) {
    if (ctl.mode === 'find') typeFind(e.currentTarget.value.replace(/^\//, ''));
    else ctl.ask = e.currentTarget.value;
  }

  /// ⏎, esc and ⌫ on an empty line belong to the mode, not the field.
  function onlinekey(e) {
    if (e.isComposing || e.keyCode === 229) return;
    if (['Enter', 'Escape', 'Backspace'].includes(e.key)) key(e);
  }

  /// A level's cells as drawn: lit up to the value.
  function cells(r) {
    const lo = r.min ?? 0;
    const hi = r.max ?? 100;
    const lit = Math.round(Math.max(0, Math.min(1, (Number(r.value) - lo) / (hi - lo))) * CELLS);
    return Array.from({ length: CELLS }, (_, i) => i < lit);
  }

  /// Choices too many or too long to sit in a row show the one that is on,
  /// with the way to the others while it is being edited.
  const fits = (r) => r.choices.length <= 4 && r.choices.join('').length <= 26;

  function sub(r) {
    if (ctl.failed[r.id]) return { text: ctl.failed[r.id], cls: 'bad' };
    if (ctl.moved[r.id]) {
      return { text: byKeys(r.id) ? 'changed by the volume keys' : 'changed elsewhere', cls: 'moved' };
    }
    if (r.why) return { text: r.why, cls: '' };
    if (r.muted) return { text: 'muted', cls: '' };
    return { text: r.sub ?? '', cls: '' };
  }

  function clickCells(e, r) {
    e.stopPropagation();
    const box = e.currentTarget.getBoundingClientRect();
    tapLevel(r, Math.max(0, Math.min(1, (e.clientX - box.left) / box.width)));
  }

  function clickOpt(e, r, word) {
    e.stopPropagation();
    tapOption(r, word);
  }
</script>

<div class="controls" transition:fade={{ duration: 160 }}>
  <div class="column">
    <div class="head">
      <span class="title">CONTROLS</span>
      <span class="count">{count}</span>
      {#if ctl.filter && ctl.mode !== 'find'}
        <span class="chip">/{ctl.filter} · esc clears</span>
      {/if}
      {#if ctl.error}
        <span class="err">{ctl.error}</span>
      {/if}
    </div>

    <div class="rows" bind:this={rowsEl}>
      {#if !ctl.sections}
        <div class="reading">{ctl.error ? '' : 'reading the machine …'}</div>
      {:else}
        <div class="grid">
          {#each cols as col, ci (ci)}
            <div class="col">
              {#each col as s (s.id)}
                <div class="cs">{s.name}</div>
                {#each s.rows as x (x.r.id)}
                  {@const r = x.r}
                  {@const editing = ctl.mode === 'edit' && ctl.editing === r.id}
                  {@const tower = locked(r)}
                  {@const line = sub(r)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <div
                    class="cr k-{r.kind}"
                    class:sel={ctl.sel === r.id}
                    class:edit={editing}
                    class:child={x.child}
                    class:muted={r.muted}
                    class:off={!!r.why}
                    class:tower
                    data-id={r.id}
                    role="button"
                    tabindex="-1"
                    onclick={() => tap(r.id)}
                  >
                    {#if r.kind === 'group'}<span class="mark">{x.open ? '▾' : '▸'}</span>{/if}
                    <span class="cl"
                      >{r.label}{#if line.text}&nbsp;<span class="csub {line.cls}">{line.text}</span>{/if}</span
                    >
                    {#if editing}<span class="tag">EDIT</span>{/if}
                    {#if tower}
                      <span class="lock">on the tower only</span>
                    {:else if r.hint && r.kind !== 'action'}
                      <span class="ch">{r.hint}</span>
                    {/if}

                    {#if r.kind === 'slider'}
                      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                      <span class="slider" onclick={(e) => !tower && clickCells(e, r)}
                        >{#each cells(r) as lit, i (i)}<i class:lit></i>{/each}</span
                      >
                      <span class="num">{r.value}</span>
                    {:else if r.kind === 'toggle'}
                      <span class="opts">
                        {#each ['on', 'off'] as w (w)}
                          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                          <span class="opt" class:on={(w === 'on') === !!r.value} onclick={(e) => clickOpt(e, r, w)}>{w}</span>
                        {/each}
                      </span>
                    {:else if r.kind === 'choice'}
                      <span class="opts">
                        {#if fits(r)}
                          {#each r.choices as c (c)}
                            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                            <span class="opt" class:on={c === r.value} class:locked={locked(r, c)} onclick={(e) => clickOpt(e, r, c)}
                              >{c}</span
                            >
                          {/each}
                        {:else}
                          <span class="opt on">{#if editing}‹ {/if}{r.value || '—'}{#if editing} ›{/if}</span>
                        {/if}
                      </span>
                    {:else if r.kind === 'action'}
                      <span class="ch">{r.hint ? `${r.value} · ${r.hint}` : r.value}</span>
                      <span class="opts">
                        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                        <span class="opt act" onclick={(e) => clickOpt(e, r, '')}
                          >{r.value === 'connected' ? 'disconnect' : 'connect'}</span
                        >
                      </span>
                    {:else if r.kind === 'transport'}
                      <span class="opts">
                        {#each ['prev', r.value === 'playing' ? 'pause' : 'play', 'next'] as w (w)}
                          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                          <span class="opt act" onclick={(e) => clickOpt(e, r, w)}>{w}</span>
                        {/each}
                      </span>
                    {:else if r.kind === 'session'}
                      <span class="opts">
                        {#each r.choices as c, i (c)}
                          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                          <span
                            class="opt act"
                            class:on={editing && i === ctl.pick && !ctl.armed}
                            class:armed={editing && i === ctl.pick && ctl.armed}
                            class:locked={locked(r, c)}
                            onclick={(e) => clickOpt(e, r, c)}>{c}</span
                          >
                        {/each}
                      </span>
                    {:else if r.kind === 'info'}
                      <span class="txt">{r.value}</span>
                    {/if}

                    {#if ctl.flight[r.id]}
                      <div class="flight"><i></i></div>
                    {/if}
                  </div>
                {/each}
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    {#if said.length}
      <div class="reply" in:fade={{ duration: 200 }}>
        {#each said as m (m.id)}
          {#if m.text || m.blocks?.length}
            <Message message={m} live={app.busy && m.id === said.at(-1)?.id} latest={m.id === said.at(-1)?.id} />
          {/if}
        {/each}
      </div>
    {/if}

    <div class="hairline"></div>

    <div class="input-line">
      <span class="prompt">{ctl.mode === 'find' ? '/' : '›'}</span>
      {#if ctl.mode === 'find' || ctl.mode === 'ask'}
        <input
          bind:this={lineEl}
          value={ctl.mode === 'find' ? ctl.filter : ctl.ask}
          {oninput}
          onkeydown={onlinekey}
          spellcheck="false"
          autocomplete="off"
        />
        {#if ctl.mode === 'ask' && !ctl.ask}
          <span class="hint">talk to parker — from ^,{rowOf(ctl.sel) ? `, about ${rowOf(ctl.sel).label}` : ''}</span>
        {:else if ctl.mode === 'find' && !ctl.filter}
          <span class="hint">type to narrow · volume 30 ⏎ just does it</span>
        {/if}
      {:else}
        <span class="cursor parked"></span>
        <span class="hint rest">/ find · i ask parker · ? keys</span>
      {/if}
    </div>

    <div class="keys">
      {#each keys as k, i (i)}<span class={k.cls}>{k.t}</span>{/each}
    </div>
  </div>

  {#if ctl.help}
    <div class="help" transition:fade={{ duration: 120 }}>
      <div>
        <div class="h">NORMAL</div>
        <div class="k"><b>j k</b><span>move · 3j</span></div>
        <div class="k"><b>h l</b><span>the other column</span></div>
        <div class="k"><b>gg G</b><span>first, last</span></div>
        <div class="k"><b>^N ^P</b><span>move, as in ^K</span></div>
        <div class="k"><b>⏎</b><span>change · open</span></div>
        <div class="k"><b>space</b><span>flip, play</span></div>
        <div class="k"><b>m</b><span>mute a sound</span></div>
        <div class="k"><b>/</b><span>find</span></div>
        <div class="k"><b>i</b><span>ask parker</span></div>
        <div class="k"><b>^,</b><span>put away</span></div>
      </div>
      <div>
        <div class="h">EDIT</div>
        <div class="k"><b>h l</b><span>−1 +1, a choice</span></div>
        <div class="k"><b>H L</b><span>−10 +10</span></div>
        <div class="k"><b>0–9</b><span>type it</span></div>
        <div class="k"><b>m</b><span>mute</span></div>
        <div class="k"><b>⏎</b><span>keep</span></div>
        <div class="k"><b>esc</b><span>put it back</span></div>
        <div class="k"><b>⏎ ⏎</b><span>the session</span></div>
      </div>
      <div>
        <div class="h">FIND</div>
        <div class="k"><b>type</b><span>narrow</span></div>
        <div class="k"><b>⏎</b><span>jump</span></div>
        <div class="k"><b>vol 30⏎</b><span>just do it</span></div>
        <div class="k"><b>esc</b><span>clear</span></div>
      </div>
      <div>
        <div class="h">ASK</div>
        <div class="k"><b>type</b><span>to parker</span></div>
        <div class="k"><b>⏎</b><span>send</span></div>
        <div class="k"><b>esc</b><span>back</span></div>
        <div class="k gap"><b>?</b><span>close this</span></div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* The session view's column, to the pixel (Chat.svelte), so the rows sit
     exactly where the transcript was. */
  .controls {
    position: absolute;
    z-index: 1;
    inset: 0 0 0 870px;
    padding: 0 80px;
  }
  .column {
    width: min(1560px, 100%);
    height: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  /* Clear of the corner, which keeps its place over every view. */
  .head {
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 24px;
    margin-top: 236px;
    padding: 0 36px 14px;
    border-bottom: 1px solid var(--fg-8);
  }
  .title {
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--fg-28);
  }
  .count {
    font-size: 20px;
    color: var(--fg-22);
  }
  .chip {
    margin-left: auto;
    font-size: 20px;
    color: var(--accent-75);
  }
  .err {
    margin-left: auto;
    font-size: 20px;
    color: var(--bad-65);
  }
  .reading {
    padding: 28px 36px;
    font-size: 22px;
    color: var(--fg-28);
  }

  .rows {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 6px 0 20px;
  }
  .rows::-webkit-scrollbar {
    display: none;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    column-gap: 50px;
  }
  .col {
    min-width: 0;
  }

  /* a section label, and a row */
  .cs {
    padding: 20px 36px 8px;
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--fg-28);
  }
  .cr {
    position: relative;
    display: flex;
    align-items: center;
    gap: 24px;
    min-height: 48px;
    padding: 4px 36px 4px 34px;
    border-left: 2px solid transparent;
    box-sizing: border-box;
    font-size: 24px;
    color: var(--fg-85);
  }
  .cr.sel {
    background: var(--accent-14);
    border-left-color: var(--accent);
  }
  .cr.edit {
    background: var(--accent-9);
    box-shadow: inset 0 0 0 1px var(--accent-55);
    border-left-color: var(--accent);
  }
  .cl {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .csub {
    color: var(--fg-35);
  }
  .csub.bad {
    color: var(--bad-65);
  }
  .csub.moved {
    color: var(--fg-55);
    font-style: italic;
  }
  .ch {
    flex: none;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 20px;
    color: var(--fg-32);
  }
  .num {
    flex: none;
    width: 3ch;
    text-align: right;
    font-size: 22px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-72);
  }
  .cr.sel .num,
  .cr.edit .num {
    color: var(--accent);
  }
  .txt {
    flex: none;
    max-width: 62%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 21px;
    color: var(--fg-60);
  }
  .tag {
    flex: none;
    font-size: 15px;
    letter-spacing: 0.16em;
    color: var(--accent-90);
  }
  .lock {
    flex: none;
    font-size: 18px;
    letter-spacing: 0.04em;
    color: var(--fg-35);
  }
  .cr.tower,
  .cr.off {
    color: var(--fg-35);
  }

  /* a level: block cells */
  .slider {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 10px 0;
  }
  .slider i {
    width: 10px;
    height: 22px;
    border-radius: 1px;
    background: var(--fg-10);
  }
  .slider i.lit {
    background: var(--fg-72);
  }
  .cr.sel .slider i.lit {
    background: var(--accent-75);
  }
  .cr.edit .slider i.lit {
    background: var(--accent);
  }

  /* toggles, choices, actions: written as words */
  .opts {
    flex: none;
    display: flex;
    gap: 4px;
  }
  .opt {
    padding: 1px 12px;
    border-radius: 3px;
    font-size: 21px;
    color: var(--fg-32);
  }
  .opt.on {
    color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent-55);
  }
  .opt.act {
    color: var(--fg-60);
  }
  .opt.act.on {
    color: var(--accent);
  }
  .opt.armed {
    color: var(--bg);
    background: var(--accent);
  }
  .opt.locked {
    color: var(--fg-18);
    text-decoration: line-through;
    text-decoration-color: var(--fg-28);
  }

  /* the mixer and the notices: a group opens like a dropdown, its rows
     indented under it */
  .mark {
    flex: none;
    width: 1ch;
    color: var(--fg-45);
  }
  .cr.child {
    padding-left: 82px;
    font-size: 22px;
    min-height: 44px;
  }
  .cr.child .cl::before {
    content: '└ ';
    color: var(--fg-18);
  }
  .cr.muted .cl,
  .cr.muted .num {
    color: var(--fg-28);
  }
  .cr.muted .slider i.lit {
    background: var(--fg-16);
  }

  /* a change in flight: the ledger's sweep, under the row */
  .flight {
    position: absolute;
    left: 34px;
    right: 36px;
    bottom: 2px;
    height: 1px;
    background: var(--fg-8);
    overflow: hidden;
  }
  .flight i {
    display: block;
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

  /* what parker said to an ask, under the rows, in the transcript's grammar */
  .reply {
    flex: none;
    max-height: 32vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 24px;
    padding: 20px 36px 26px;
    mask-image: linear-gradient(to bottom, transparent, #000 60px);
  }

  /* the foot: Chat.svelte's hairline, input line and key strip */
  .hairline {
    flex: none;
    height: 1px;
    background: var(--fg-8);
  }
  .input-line {
    flex: none;
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 32px 0 0;
    font-size: 24px;
    line-height: 1.5;
  }
  .prompt {
    flex: none;
    color: var(--accent-55);
  }
  input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    padding: 0;
    color: var(--fg);
    font: inherit;
    caret-color: var(--accent-80);
  }
  .hint {
    position: absolute;
    left: calc(1ch + 14px);
    top: 32px;
    color: var(--fg-22);
    pointer-events: none;
  }
  .hint.rest {
    position: static;
  }
  .cursor {
    flex: none;
    width: 16px;
    height: 32px;
  }
  .cursor.parked {
    background: var(--accent-35);
  }
  .keys {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 32px;
    padding: 32px 0 30px;
    font-size: 20px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--fg-20);
  }
  .keys .mode {
    color: var(--accent-90);
  }
  .keys .pending {
    color: var(--fg-55);
  }

  /* ? : every key, on the palette's surface, over the rows */
  .help {
    position: absolute;
    z-index: 6;
    left: 80px;
    right: 80px;
    top: 300px;
    max-width: 1560px;
    margin: 0 auto;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    column-gap: 44px;
    padding: 34px 40px;
    background: var(--raised);
    border: 1px solid var(--fg-16);
    border-radius: 24px;
    box-shadow: var(--palette-lift);
  }
  .help .h {
    margin-bottom: 14px;
    font-size: 19px;
    letter-spacing: 0.16em;
    color: var(--accent-90);
  }
  .help .k {
    display: flex;
    gap: 16px;
    font-size: 21px;
    line-height: 1.75;
  }
  .help .k b {
    flex: none;
    width: 110px;
    font-weight: 400;
    color: var(--fg-85);
  }
  .help .k span {
    color: var(--fg-45);
  }
  .help .k.gap {
    margin-top: 20px;
  }

  @media (prefers-reduced-motion: reduce) {
    .flight i {
      animation: none;
    }
  }
</style>
