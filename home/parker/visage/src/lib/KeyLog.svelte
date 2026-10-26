<script>
  // A key log, for the porthole on a screen this repo cannot reach: open it
  // with `?keys` on the address and it lists the last keys the page heard,
  // as the page heard them — before any handler, in the capture phase. The
  // two-Esc palette on giskard was the reason (AGENTS.md, The porthole):
  // headless Chrome injects keys below the browser and never reproduced it,
  // so the only witness is the Mac's own browser. Off unless asked for, and
  // never in the Tauri window, which has no address to ask with.
  import { onMount } from 'svelte';

  const KEEP = 14;
  let lines = $state([]);

  onMount(() => {
    const t0 = performance.now();
    const log = (e) => {
      const mods = ['ctrl', 'alt', 'shift', 'meta'].filter((m) => e[`${m}Key`]).join('+');
      const at = document.activeElement;
      lines = [
        {
          id: `${e.timeStamp}${e.type}`,
          t: ((e.timeStamp - t0) / 1000).toFixed(2),
          type: e.type === 'keydown' ? 'down' : 'up',
          key: JSON.stringify(e.key),
          code: e.code,
          keyCode: e.keyCode,
          composing: e.isComposing,
          repeat: e.repeat,
          mods,
          focus: at ? at.tagName.toLowerCase() + (at.className ? `.${String(at.className).split(' ')[0]}` : '') : '-',
        },
        ...lines,
      ].slice(0, KEEP);
    };
    window.addEventListener('keydown', log, true);
    window.addEventListener('keyup', log, true);
    return () => {
      window.removeEventListener('keydown', log, true);
      window.removeEventListener('keyup', log, true);
    };
  });
</script>

<div class="log">
  <div class="head">KEYS · AS THE PAGE HEARD THEM · NEWEST FIRST</div>
  {#each lines as l (l.id)}
    <div class="line" class:up={l.type === 'up'}>
      {l.t}s {l.type} {l.key} {l.code} {l.keyCode}{l.mods ? ` ${l.mods}` : ''}{l.composing ? ' COMPOSING' : ''}{l.repeat
        ? ' repeat'
        : ''} → {l.focus}
    </div>
  {:else}
    <div class="line up">press something</div>
  {/each}
</div>

<style>
  .log {
    position: absolute;
    top: 24px;
    left: 24px;
    z-index: 20;
    padding: 16px 20px;
    background: var(--raised);
    border: 1px solid var(--fg-16);
    border-radius: 12px;
    font-size: 17px;
    line-height: 1.6;
    font-variant-numeric: tabular-nums;
    color: var(--fg-72);
    pointer-events: none;
  }
  .head {
    margin-bottom: 8px;
    font-size: 15px;
    letter-spacing: 0.14em;
    color: var(--fg-35);
  }
  .line.up {
    color: var(--fg-35);
  }
</style>
