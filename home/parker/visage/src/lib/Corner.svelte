<script>
  // The corner: his body and his room, and where you are. Never a count
  // of threads, never disk usage or fan speed. Same position in every view.
  //
  //   19:31 · thursday
  //   cpu ▇▅▃▂▁▁▁▁ 22% · gpu ▇▇▇▅▃▂▁▁ 38%
  //   51°C in
  //   space 1 2 3 4
  //
  // The last line is the space under visage, the current one in accent —
  // there is no bar any more (docs/visage/controls/README.md), and visage
  // sits on a special workspace, so the space can change while it is up.
  // It follows Hyprland's events (door's `watch_spaces`), not a poll.
  //
  // Data is one `parker sense --json` every few seconds; the sparklines are
  // the last eight readings, kept here. Two lines are deliberately absent
  // and have their hooks: his sentence (italic, between the clock and the
  // meters — no channel for parker to write one yet) and the outdoor
  // temperature (`4°C out` — no honest sensor for it yet).
  import { onMount } from 'svelte';
  import { engine } from './engine.js';

  const POLL_MS = 2500;
  const CELLS = 8;
  const BARS = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

  let clock = $state('');
  let day = $state('');
  let cpu = $state(null); // { pct, hist: number[] }
  let gpu = $state(null);
  let tempIn = $state(null);
  let spaces = $state(null); // { spaces: [{ id, windows }], active }

  /// The numbers to draw: every space Hyprland has, and the one in front
  /// even when it is empty and so not yet one of them.
  const ids = $derived.by(() => {
    const list = (spaces?.spaces ?? []).map((s) => s.id);
    const on = spaces?.active;
    if (on > 0 && !list.includes(on)) list.push(on);
    return list.sort((a, b) => a - b);
  });

  function bar(pct) {
    const i = Math.min(BARS.length - 1, Math.floor((pct / 100) * BARS.length));
    return BARS[i];
  }

  /// The last eight readings as cells, padded left with rest.
  function cells(hist) {
    const padded = Array(Math.max(0, CELLS - hist.length))
      .fill(0)
      .concat(hist.slice(-CELLS));
    return padded.map((p) => ({ ch: bar(p), lit: p >= 8 }));
  }

  function tickClock() {
    const now = new Date();
    clock = now.toTimeString().slice(0, 5);
    day = now.toLocaleDateString('en-GB', { weekday: 'long' }).toLowerCase();
  }

  function push(prev, pct) {
    const hist = [...(prev?.hist ?? []), pct].slice(-CELLS);
    return { pct, hist };
  }

  async function poll() {
    try {
      const s = await engine.sense();
      const m = s?.machine;
      if (!m) return;
      if (typeof m.cpu?.util_pct === 'number') cpu = push(cpu, m.cpu.util_pct);
      if (typeof m.gpu?.util_pct === 'number') gpu = push(gpu, m.gpu.util_pct);
      // "in" is the box itself: the CPU die where measured, the GPU's
      // otherwise. Whichever the machine can honestly say.
      tempIn = m.cpu?.temp_c ?? m.gpu?.temp_c ?? null;
    } catch {
      // No shell, or no parker on PATH. The corner shows what it knows.
    }
  }

  onMount(() => {
    tickClock();
    poll();
    engine
      .spaces()
      .then((v) => (spaces = v ?? spaces))
      .catch(() => {});
    let unlisten = null;
    engine.onSpaces((v) => (spaces = v)).then((u) => (unlisten = u));
    const c = setInterval(tickClock, 10_000);
    const p = setInterval(poll, POLL_MS);
    return () => {
      clearInterval(c);
      clearInterval(p);
      unlisten?.();
    };
  });
</script>

<div class="corner">
  <div>{clock} · {day}</div>
  {#if cpu || gpu}
    <div class="meters">
      {#if cpu}
        <span
          >cpu
          {#each cells(cpu.hist) as c, i (i)}<span class="cell" class:lit={c.lit}>{c.ch}</span
            >{/each}
          {cpu.pct}%</span
        >
      {/if}
      {#if cpu && gpu}<span class="dot">·</span>{/if}
      {#if gpu}
        <span
          >gpu
          {#each cells(gpu.hist) as c, i (i)}<span class="cell" class:lit={c.lit}>{c.ch}</span
            >{/each}
          {gpu.pct}%</span
        >
      {/if}
    </div>
  {/if}
  {#if tempIn !== null}
    <div><span class="in">{tempIn}°C in</span></div>
  {/if}
  {#if ids.length}
    <div class="spaces">
      space{#each ids as id (id)}<span class="n" class:on={id === spaces.active}>{id}</span>{/each}
    </div>
  {/if}
</div>

<style>
  .corner {
    position: absolute;
    top: 52px;
    right: 72px;
    z-index: 1;
    text-align: right;
    font-size: 20px;
    line-height: 1.9;
    color: var(--fg-28);
    pointer-events: none;
  }
  .meters .cell {
    letter-spacing: 1px;
    color: var(--accent-20);
  }
  .meters .cell.lit {
    color: var(--accent-75);
  }
  .dot {
    margin: 0 6px;
  }
  .in {
    color: var(--good-75);
  }
  .spaces .n {
    margin-left: 14px;
  }
  .spaces .n.on {
    color: var(--accent);
  }
</style>
