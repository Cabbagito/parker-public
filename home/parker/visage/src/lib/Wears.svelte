<script>
  // WEARS: the one place the expression is named. It exists because the face
  // is entirely parker's choice now — the app infers nothing — so what he
  // picked is worth a caption. Same position and size in every view.
  let { worn = 'idle' } = $props();
</script>

<div class="wears">
  <div class="label">WEARS</div>
  <div class="value">{worn}</div>
  <div class="rule"></div>
</div>

<style>
  .wears {
    position: absolute;
    left: 120px;
    /* Clear of the face: `session` puts parker on the left rail down to
       ~1030px, and at 940 the label sat on his chin. */
    top: 1080px;
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
    margin-top: 10px;
    font-size: 52px;
    line-height: 1.1;
    letter-spacing: -0.01em;
    color: var(--accent-90);
  }
  .rule {
    margin-top: 18px;
    width: 240px;
    height: 1px;
    background: var(--accent-35);
  }
</style>
