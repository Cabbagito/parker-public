<script>
  // The transcript, in whichever posture the view asks for.
  //
  // Two postures, one DOM, restyled — never branched. Branching would
  // remount the scroller and the textarea on every toggle, dropping scroll
  // position and focus at exactly the moment you asked to see more of the
  // conversation.
  //
  //   default   the tail of the current exchange, bottom-anchored over the
  //             input, masked so what runs off the top dissolves.
  //   session   the conversation across the right two thirds.
  //
  // Under the messages, identical in every posture: a full-width hairline,
  // the input line — no border, no background, a `›` and a caret sitting in
  // space — and the keybind row. The chrome around it (corner, WEARS, the
  // view indicator) is App.svelte's; this file owns only the column.
  import { fade } from 'svelte/transition';
  import Message from './Message.svelte';
  import { app, send, eventLines, woven } from './state.svelte.js';

  let { view = 'default' } = $props();

  let input = $state('');
  let focused = $state(false);
  let scroller;
  let stack;
  let inputEl;

  /// Whether the transcript is following its tail. Not a guess from how near
  /// the bottom things look after a change — that guess, made once the new
  /// content was already laid out, stranded every opened thread at its top
  /// and let go of the tail whenever a code block landed whole.
  let pinned = true;

  /// Within this of the bottom is at the bottom: fractional scroll offsets
  /// never quite reach it, and a line (LINE) is well past it.
  const SLACK = 24;

  /// One line of prose — 28px at line-height 1.6 (Message.svelte).
  const LINE = 45;

  /// Room left above a turn landed on by Ctrl+Shift+N/P, clear of the
  /// default view's 80px top mask.
  const CLEAR = 80;

  /// Screens a second while Ctrl+D/U is held. Every line crosses the view in
  /// about two thirds of a second: long enough to catch the shape of a
  /// reply, a code block or a word, too short to read it. In screens rather
  /// than pixels, so default's short window keeps the same time on screen.
  const CRUISE = 1.5;

  /// A held key whose repeats stop coming has been let go, even when its
  /// keyup went somewhere else. Repeats come every 29 ms here (hyprland.nix).
  const STALE = 150;

  let cruising = null;

  export function focusInput() {
    inputEl?.focus();
  }

  /// Typing anywhere means talking. The keystroke that summoned the input
  /// was delivered to a body that cannot hold text, so it is put back by
  /// hand — otherwise the first character of every first question is
  /// silently eaten. `append` keeps whatever was already there.
  export function seed(text, append = false) {
    input = append ? input + text : text;
    inputEl?.focus();
    requestAnimationFrame(() => inputEl?.setSelectionRange(input.length, input.length));
  }

  /// The transcript's motions, finest to furthest. App's keymap names them;
  /// this does them.
  ///
  ///   line   Ctrl+Shift+J/K   one line of prose
  ///   half   Ctrl+D/U         half a viewport, context kept either side;
  ///                           held, a steady glide (`cruise`)
  ///   page   PgDn/PgUp        a viewport, less a line of overlap
  ///   turn   Ctrl+Shift+N/P   the next or previous of your turns
  ///   end    Ctrl+Shift+G     the tail; Ctrl+G the top — vim's G and gg
  ///
  /// A line repeats while the key is held, so it moves at once rather than
  /// queueing glides; the ends are too far to glide. The rest glide, so you
  /// can see which way you went.
  export function scroll(unit, dir) {
    if (!scroller) return;
    halt();
    const now = scroller.scrollTop;
    const h = scroller.clientHeight;
    if (unit === 'line') to(now + dir * LINE);
    else if (unit === 'half') to(now + (dir * h) / 2, 'smooth');
    else if (unit === 'page') to(now + dir * (h - LINE), 'smooth');
    else if (unit === 'turn') to(turn(dir), 'smooth');
    else if (unit === 'end') to(dir > 0 ? Infinity : 0);
  }

  /// Ctrl+D/U held. Each repeat used to restart the half-page glide, 35
  /// times a second, so it never got past that glide's slow start — a fifth
  /// of a screen a second, slower than a single tap (measured) — and the
  /// last one coasted on past where you let go. Now the repeats only keep
  /// one steady glide alive, and letting go stops it where it is.
  export function cruise(dir) {
    if (!scroller) return;
    const now = performance.now();
    if (cruising?.dir === dir) {
      cruising.seen = now;
      return;
    }
    halt();
    cruising = { dir, y: scroller.scrollTop, t: now, seen: now };
    cruising.raf = requestAnimationFrame(glide);
  }

  /// Let go of the key: stop dead, on what you stopped for.
  export function halt() {
    if (cruising) cancelAnimationFrame(cruising.raf);
    cruising = null;
  }

  function glide(t) {
    const c = cruising;
    if (!c) return;
    if (t - c.seen > STALE) return halt();
    // The position is kept here, not read back: a frame's step is a few
    // pixels, and a scrollTop rounded to the device pixel would eat them.
    const dt = Math.min(0.05, Math.max(0, (t - c.t) / 1000));
    c.t = t;
    c.y += c.dir * CRUISE * scroller.clientHeight * dt;
    to(c.y);
    if (c.y <= 0 || c.y >= scroller.scrollHeight - scroller.clientHeight) return halt();
    c.raf = requestAnimationFrame(glide);
  }

  /// Every motion lands here, and where it lands decides the pin: a move
  /// that ends on the tail follows it again, and one that leaves it lets go
  /// now rather than at the first scroll event — or a reply growing under a
  /// glide up would yank it straight back down.
  function to(top, behavior = 'auto') {
    const max = scroller.scrollHeight - scroller.clientHeight;
    const y = Math.max(0, Math.min(max, top));
    pinned = y >= max - SLACK;
    scroller.scrollTo({ top: y, behavior });
  }

  /// Where the next or previous of your turns sits, or the end of the
  /// thread past the last one. Your turns are Message's `.msg.user` bands —
  /// in a long agentic reply, the way back to what you asked.
  function turn(dir) {
    const now = scroller.scrollTop;
    const origin = scroller.getBoundingClientRect().top - now;
    const tops = [...stack.querySelectorAll('.msg.user')].map(
      (el) => el.getBoundingClientRect().top - origin - CLEAR,
    );
    const found = dir > 0 ? tops.find((t) => t > now + 1) : tops.findLast((t) => t < now - 1);
    return found ?? (dir > 0 ? Infinity : 0);
  }

  function onScroll() {
    pinned = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < SLACK;
  }

  /// Esc's middle tier. Reports whether there was anything to clear, so the
  /// caller knows whether the keystroke was spent.
  export function clearInput() {
    const had = input.length > 0;
    input = '';
    return had;
  }

  // The transcript as drawn: the engine's messages with this page's event
  // lines woven in by time (state.svelte.js `woven`).
  const items = $derived(woven(app.messages, eventLines, app.sessionId));

  /// An event line's words: the design's Renders board, "all sound 42 → 50
  /// · the keys on the desk, not parker".
  const lineOf = (l) =>
    `${l.label} ${l.from} → ${l.to} · ${l.why === 'elsewhere' ? 'changed elsewhere' : l.why}, not parker`;

  // The last assistant message owns the caret; everything before it is
  // settled and never re-renders.
  const lastAssistant = $derived.by(() => {
    for (let i = app.messages.length - 1; i >= 0; i--) {
      if (app.messages[i].role !== 'user') return app.messages[i].id;
    }
    return null;
  });

  // Between beats — the reply not yet begun, or an act in flight with no
  // prose after it — a bare caret pulses so the wait never reads as a dead
  // line. While prose is typing, its own head caret is the pulse.
  const betweenBeats = $derived.by(() => {
    if (!app.busy) return false;
    const tail = app.messages.at(-1);
    if (!tail || tail.role === 'user') return true;
    return tail.blocks?.at(-1)?.kind !== 'text';
  });

  $effect(() => {
    // Hold the tail while pinned, through anything that changes a height: a
    // turn arriving, prose typing in, a run folding, a view reshaping the
    // column. The stack is the content; the scroller is the window it is
    // seen through. Scrolled away, nothing moves under you.
    const ro = new ResizeObserver(() => {
      if (pinned) scroller.scrollTop = scroller.scrollHeight;
    });
    ro.observe(scroller);
    ro.observe(stack);
    return () => ro.disconnect();
  });

  $effect(() => {
    // An empty transcript is read at its tail by definition, and both ways
    // into a thread start from one — `resume` and `newChat` clear the list
    // before anything arrives. So an opened thread lands on its newest line,
    // wherever the last one was left.
    if (app.messages.length === 0) pinned = true;
  });

  $effect(() => {
    // Grow with the text, to a cap; past the cap the box scrolls rather than
    // eating the screen.
    input;
    if (!inputEl) return;
    inputEl.style.height = 'auto';
    inputEl.style.height = `${inputEl.scrollHeight}px`;
  });

  /// Enter. Exported for App, which also sends when Enter arrives while the
  /// box has lost focus (the caret-reclaiming rules there).
  export async function submit() {
    const text = input;
    input = '';
    // What you just said is what you want to see answered.
    if (text.trim()) pinned = true;
    await send(text);
  }

  function onKey(e) {
    // The palette owns the keyboard while it is up; without this an Enter
    // meant for its list would also fire the box underneath it.
    if (app.palette) return;
    // Enter that commits a composition (App.svelte, onKey) is not a send.
    if (e.isComposing || e.keyCode === 229) return;
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="chat {view}" transition:fade={{ duration: 160 }}>
  <div class="column">
    <div class="messages" bind:this={scroller} onscroll={onScroll}>
      <div class="stack" bind:this={stack}>
        {#each items as { key, m, line } (key)}
          {#if line}
            <div class="event" in:fade={{ duration: 200 }}>{lineOf(line)}</div>
          {:else if m.text || m.blocks?.length}
            <Message message={m} live={app.busy && m.id === lastAssistant} latest={m.id === lastAssistant} />
          {/if}
        {/each}
        {#if betweenBeats}
          <div class="working" in:fade={{ duration: 200 }}><span class="cursor"></span></div>
        {/if}
      </div>
    </div>

    {#if app.error}
      <div class="error">{app.error}</div>
    {/if}

    <div class="hairline"></div>

    <form
      class="input-line"
      onsubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <span class="prompt">›</span>
      {#if !input}
        <span class="cursor" class:parked={!focused}></span>
      {/if}
      <textarea
        bind:this={inputEl}
        bind:value={input}
        onkeydown={onKey}
        onfocus={() => (focused = true)}
        onblur={() => (focused = false)}
        rows="1"
        spellcheck="false"
        class:empty={!input}
      ></textarea>
      {#if !input}
        <span class="hint">talk to parker</span>
      {/if}
    </form>

    <div class="keys">
      <span class="stop">ESC STOP</span>
      <span>^K EARLIER · COMMANDS</span>
      <span>^, CONTROLS</span>
      <span>^SPACE DANCE</span>
      <span>^TAB VIEW</span>
    </div>
  </div>
</div>

<style>
  .chat {
    position: absolute;
    z-index: 1;
  }

  /* The postures. Numbers are the redesign's for the 2560-wide surface
     (docs/visage/redesign/README.md, "The views"). */
  .chat.default {
    inset: auto 0 0 0;
  }
  .chat.default .column {
    width: min(1400px, 90vw);
    margin: 0 auto;
  }
  /* What runs off the top of the tail dissolves rather than being cut. */
  .chat.default .messages {
    max-height: 34vh;
    mask-image: linear-gradient(to bottom, transparent, #000 80px);
  }

  .chat.session {
    inset: 0 0 0 870px;
    padding: 0 80px;
  }
  .chat.session .column {
    width: min(1560px, 100%);
    height: 100%;
    margin: 0 auto;
  }

  .column {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 56px 0 26px;
    scrollbar-width: none;
  }
  .messages::-webkit-scrollbar {
    display: none;
  }
  /* One box around the turns, so a single ResizeObserver sees every change
     in their height. A short transcript sits down by the input rather than
     stranded at the top: `margin-top: auto` rather than `justify-content`,
     which clips the head of the list once it overflows. */
  .stack {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  .working {
    display: flex;
    align-items: center;
  }

  /* An event line (state.svelte.js, "Event lines"): something the machine
     did that nobody said — the keys on the desk moving a level. Quieter than
     any act, as the Renders board draws it, and no glyph: it is not parker's. */
  .event {
    font-size: 19px;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
    color: var(--fg-28);
  }

  .cursor {
    display: inline-block;
    flex: none;
    width: 16px;
    height: 32px;
    background: var(--accent-80);
    animation: caret 1.1s steps(1) infinite;
  }
  .cursor.parked {
    background: var(--accent-35);
    animation: none;
  }
  @keyframes caret {
    50% {
      opacity: 0;
    }
  }

  .error {
    flex: none;
    color: var(--bad);
    padding: 8px 0;
    font-size: 20px;
  }

  .hairline {
    flex: none;
    height: 1px;
    background: var(--fg-8);
  }

  .input-line {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 32px 0 0;
    position: relative;
    font-size: 24px;
    line-height: 1.5;
  }
  .prompt {
    flex: none;
    color: var(--accent-55);
  }
  .input-line .cursor {
    margin-top: 2px;
  }
  .hint {
    position: absolute;
    left: calc(14px + 1ch + 14px + 16px + 14px);
    top: 32px;
    color: var(--fg-22);
    pointer-events: none;
  }
  /* The box is a place in space, not a field: no border, no background.
     Once there is text the native caret (accent) takes over from the drawn
     block — a drawn block cannot follow mid-line edits honestly. */
  textarea {
    flex: 1;
    display: block;
    resize: none;
    overflow-y: auto;
    max-height: 220px;
    background: none;
    border: none;
    color: var(--fg);
    font: inherit;
    line-height: inherit;
    padding: 0;
    outline: none;
    caret-color: var(--accent-80);
    scrollbar-width: none;
  }
  textarea::-webkit-scrollbar {
    display: none;
  }
  /* While the box is empty the drawn block *is* the caret; a second native
     caret blinking beside it reads as a glitch. */
  textarea.empty {
    caret-color: transparent;
  }

  .keys {
    flex: none;
    display: flex;
    gap: 32px;
    padding: 32px 0 30px;
    font-size: 20px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--fg-20);
  }
  .keys .stop {
    color: var(--bad-45);
  }
</style>
