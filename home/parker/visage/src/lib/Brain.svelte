<script>
  // BRAIN: the one place visage names the brain parker is on (brains.nix,
  // `parker brain`). Under WEARS and quieter than it — the face changes per
  // beat and is the thing to look at; the brain changes when Peter says so.
  // Not accent: accent is spent on state, and which model is in is a
  // setting. Same position and size in every view.
  //
  // The value is sized so the longest name, "just read the instructions",
  // fits WEARS's 380px column on one line: 26 characters of JetBrains Mono
  // at 24px is ~374px.
  let { brain = null } = $props();
</script>

{#if brain}
  <div class="brain">
    <div class="label">BRAIN</div>
    <div class="value">{brain}</div>
  </div>
{/if}

<style>
  .brain {
    position: absolute;
    left: 120px;
    /* WEARS ends near 1190 (label, value, rule — Wears.svelte). */
    top: 1214px;
    width: 380px;
    z-index: 1;
    pointer-events: none;
  }
  .label {
    font-size: 19px;
    letter-spacing: 0.18em;
    color: var(--fg-25);
  }
  .value {
    margin-top: 6px;
    font-size: 24px;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--fg-55);
  }
</style>
