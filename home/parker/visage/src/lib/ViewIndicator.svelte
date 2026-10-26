<script>
  // The two views, drawn: both names and a hairline between them, bottom-left,
  // identical in either. The current one takes the accent — which makes it a
  // toggle you can read at a glance, since the other half is right there.
  import { VIEWS } from './views.js';
  let { view = 'default' } = $props();
</script>

<div class="indicator">
  {#each VIEWS as v, i (v)}
    {#if i > 0}<span class="rule"></span>{/if}
    <span class="stop" class:current={v === view}>{v}</span>
  {/each}
</div>

<style>
  .indicator {
    position: absolute;
    left: 44px;
    bottom: 24px;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 20px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--fg-16);
    pointer-events: none;
  }
  .rule {
    width: 40px;
    height: 1px;
    background: var(--fg-16);
  }
  .stop.current {
    color: var(--accent);
  }
</style>
