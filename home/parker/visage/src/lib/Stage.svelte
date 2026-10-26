<script>
  // The camera. One Face for the life of the app; the view only moves it.
  //
  // The canvas is the whole viewport now — the redesign's one structural
  // change. A sized, translated box cannot pour the face across the screen
  // dot by dot, so placement is computed *inside* the draw loop instead:
  // this component owns where the face is (the placement half of the split,
  // with views.js) and hands Face a `camera(now)` to call once per frame.
  //
  // Two travels, one config flag (state.svelte.js `app.travel`):
  //
  //   linear  the original: the box eases a fraction of the remaining
  //           distance per second, integrated per frame, interruptible for
  //           free — change the view mid-flight and it curves from wherever
  //           parker actually is.
  //   river   the redesign's default: the dots pour (lens.js). This state
  //           machine still tracks a lerped box underneath it, so an
  //           interruption mid-river starts the next crossing from where
  //           parker visibly is, and the dance surge has a box to scale.
  import Face from './Face.svelte';
  import { placement } from './views.js';
  import { ease } from './face.js';
  import { RIVER } from './lens.js';

  let { view = 'default', expression = 'idle', travel = 'river', working = false } = $props();

  // Fraction of the remaining distance closed per second, as a rate rather
  // than a duration so the motion is frame-rate independent.
  const RATE = 9;
  const SETTLED = 0.4; // px — below this, snap to the target

  let cur = null; // where the face is, always (a box: cx, cy, w, h)
  let river = null; // { A, B, start } while a crossing runs
  let lastView = null; // set on the first frame, so opening never travels
  let last = 0;

  function target() {
    return placement(view, window.innerWidth, window.innerHeight);
  }

  const lerpBox = (a, b, t) => ({
    cx: a.cx + (b.cx - a.cx) * t,
    cy: a.cy + (b.cy - a.cy) * t,
    w: a.w + (b.w - a.w) * t,
    h: a.h + (b.h - a.h) * t,
  });

  /// Called by Face exactly once per frame. Returns either
  /// `{ box }` (draw the face there) or `{ box, A, B, t }` (a river
  /// crossing in flight — draw the swarm between A and B).
  function camera(now) {
    const to = target();
    const dt = Math.min(0.1, (now - (last || now)) / 1000);
    last = now;

    // Land in place on the first frame: parker is already here when the app
    // opens, he does not fly in from nowhere.
    if (!cur) {
      cur = to;
      lastView = view;
    }

    if (view !== lastView) {
      lastView = view;
      // A crossing interrupted by another view change starts from where
      // parker visibly is — `cur` tracks the lerp below, so this is free.
      if (travel === 'river') river = { A: { ...cur }, B: to, start: now };
    }

    if (river) {
      river.B = to; // a resize mid-crossing lands at the right place
      const t = (now - river.start) / RIVER.ms;
      if (t >= 1) {
        river = null;
        cur = to;
      } else {
        cur = lerpBox(river.A, river.B, ease.inOut(t));
        return { box: cur, A: river.A, B: river.B, t };
      }
    }

    const k = 1 - Math.exp(-RATE * dt);
    const next = lerpBox(cur, to, k);
    const far =
      Math.abs(to.cx - next.cx) +
      Math.abs(to.cy - next.cy) +
      Math.abs(to.w - next.w) +
      Math.abs(to.h - next.h);
    cur = far < SETTLED ? to : next;
    return { box: cur };
  }
</script>

<div class="stage">
  <Face {expression} {camera} {working} />
</div>

<style>
  .stage {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 2;
  }
</style>
