<script>
  // The face view: the dot grid drawn as discs, one rAF loop, transitions as
  // pose lerps — spring (backOut) for voluntary beats, inOut for state
  // changes, exactly the split animate_face() makes in parkerd.
  //
  // The canvas covers the whole viewport and the camera prop (Stage.svelte)
  // says where on it the face is, once per frame. Five things stack, in this
  // order, and each is somebody else's job:
  //
  //   transition   here. Where the face is between two expressions.
  //   beat/dance   motion.svelte.js. A deviation with its own timeline.
  //   lens         lens.js. Per-dot placement — the camera, never a pose.
  //   travel       lens.js + Stage. The river between views, per dot.
  //   echo         here. The last few frames, fading.
  //
  // The blink used to be rolled from `Math.random()` in this file, and parkerd
  // had none at all — so the two faces blinked differently and one of them not
  // at all. It is `blink(t)` in face.js now: deterministic on the clock, so
  // both surfaces agree without a channel between them.
  import { EXPRESSIONS, VOLUNTARY, draw, lerp, ease, Grid, DOTS_W, DOTS_H } from './face.js';
  import { beatAt, beatSecs, motionAt } from './beat.js';
  import catalog from './catalog.json';
  import { motion as beatState, apply, depart } from './motion.svelte.js';
  import {
    lensFor,
    echoFor,
    splitFor,
    surgeFor,
    RIVER,
    RUN,
    TRAIL_STEP,
    riverPath,
    riverSpan,
    riverDot,
    riverSample,
    inRun,
  } from './lens.js';

  let { expression = 'idle', camera, working = false } = $props();

  const LOOP_SECS = 6;
  const TRANSITION_MS = 550;

  let canvas;
  let raf = 0;

  let fromPose = EXPRESSIONS.idle.pose;
  let target = 'idle';
  let transitionStart = 0;

  /// The face parker has on, by name, which is not always one this build
  /// can draw: a visage older than parkerd draws a face it has no pose for
  /// as `idle`. `target` is the drawing; this is the fact. Whether he is
  /// resting is decided on the fact, so an undrawable face still holds the
  /// flair off, as any other face does.
  let worn = 'idle';

  /// How the gesture faces land (parkerd's entrance.rs, carried as data in
  /// catalog.json): a short beat that departs from `fromPose` and ends on the
  /// face's own pose, in place of the spring. An eye-roll is the roll.
  const ENTRANCES = catalog.entrances ?? {};
  let entering = null;

  /// Seconds into the entrance the face is landing with, or null once it has
  /// landed (or never had one).
  function landing(now) {
    if (!entering) return null;
    const into = (now - transitionStart) / 1000;
    return into < beatSecs(entering) ? into : null;
  }

  $effect(() => {
    if (expression !== worn) {
      // A change of face is something happening to parker, as it is in
      // parkerd's animate_face: depart() stops the beat and restarts the
      // settling period, after reading it so the transition leaves from
      // where the face really is. It is often the *only* thing visage hears:
      // a face worn from Claude Code or another thread arrives with no
      // keystroke and no engine event for this one. Without it, the beat the
      // face cut off resumed mid-flight when the face let go, or a fresh one
      // fired the same frame.
      //
      // Keyed on the name, not on the drawing. This used to skip any face
      // missing from EXPRESSIONS, the stir with it: for six days after the
      // faces round (Oct 2026) the tower ran an older build, and thirteen new
      // faces changed WEARS while a vignette played on underneath them.
      fromPose = depart(currentPose(performance.now()), worn === 'idle' && !working);
      worn = expression;
      target = EXPRESSIONS[expression] ? expression : 'idle';
      entering = ENTRANCES[target] ?? null;
      transitionStart = performance.now();
    }
  });

  function currentPose(now) {
    const spec = EXPRESSIONS[target];
    const into = landing(now);
    if (into !== null) return beatAt(entering, spec.pose, into, fromPose);
    if (entering) return spec.pose;
    const t = Math.min(1, (now - transitionStart) / TRANSITION_MS);
    const eased = VOLUNTARY.includes(target) ? ease.backOut(t) : ease.inOut(t);
    return t >= 1 ? spec.pose : lerp(fromPose, spec.pose, eased);
  }

  function inks(style) {
    return [
      'transparent',
      style.getPropertyValue('--face-fg').trim() || '#c8c8c8',
      style.getPropertyValue('--accent').trim() || '#7aa2f7',
      style.getPropertyValue('--good').trim() || '#9ece6a',
      style.getPropertyValue('--bad').trim() || '#f7768e',
    ];
  }

  /// A box (viewport px, centre-anchored) to canvas-pixel grid geometry.
  /// The same fit the sized canvas used to do: the padded grid, whichever
  /// axis binds first, centred in the box — so nothing moved when the canvas
  /// grew to the viewport.
  function geomOf(box, dpr, surge) {
    const pitch = Math.min(box.w / (DOTS_W + 4), box.h / (DOTS_H + 4)) * dpr * surge;
    return {
      pitch,
      ox: box.cx * dpr - (DOTS_W / 2) * pitch,
      oy: box.cy * dpr - (DOTS_H / 2) * pitch,
      r: pitch * 0.34,
    };
  }

  $effect(() => {
    const ctx = canvas.getContext('2d');
    const grid = new Grid();
    // Past frames, newest first. Only filled while dancing; at rest this
    // stays empty and the whole trail machinery costs nothing.
    const trail = [];

    /// One pass over the grid. Everything that draws dots goes through here,
    /// so the lens applies to the echo and to the chromatic split for free.
    const paint = (dots, palette, geom, alpha, offset, forceInk) => {
      const { pitch, ox, oy, r, lens, beats } = geom;
      ctx.globalAlpha = alpha;
      for (let y = 0; y < DOTS_H; y++) {
        for (let x = 0; x < DOTS_W; x++) {
          const ink = dots[y * DOTS_W + x];
          if (!ink) continue;
          const { dx, dy, scale } = lens(x, y, beats);
          const px = ox + (x + 0.5 + dx + offset.dx) * pitch;
          const py = oy + (y + 0.5 + dy + offset.dy) * pitch;
          const rr = r * scale;
          if (rr <= 0.05) continue;
          ctx.fillStyle = palette[forceInk ?? ink];
          ctx.beginPath();
          ctx.arc(px, py, rr, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.globalAlpha = 1;
    };

    /// The river: every lit dot poured from geom A to geom B, three phases,
    /// trails only while running. Nothing here expands, and nothing depends
    /// on the pose — the maths lives in lens.js.
    const pour = (dots, palette, A, B, tt, secs) => {
      const path = riverPath(A, B, DOTS_W, DOTS_H);
      if (!path) return false;
      const span = riverSpan(dots, A, path, DOTS_W, DOTS_H);
      for (let y = 0; y < DOTS_H; y++) {
        for (let x = 0; x < DOTS_W; x++) {
          const ink = dots[y * DOTS_W + x];
          if (!ink) continue;
          const d = riverDot(x, y, A, B, path, span);
          for (let sm = RIVER.trails; sm >= 0; sm--) {
            let w = (tt - d.depart - sm * TRAIL_STEP) / RUN;
            if (sm > 0 && !inRun(w)) continue;
            // Before its turn a dot simply stays where it is: the far edge
            // is still a whole face while the front is already running.
            w = Math.min(1, Math.max(0, w));
            const p = riverSample(d, path, w, secs, A, B, ease.inOut);
            const rr = p.pitch * 0.34 * (1 - p.thin * RIVER.shrink);
            if (rr <= 0.08) continue;
            ctx.globalAlpha = sm === 0 ? 1 : 0.18 * (1 - sm / (RIVER.trails + 1));
            ctx.fillStyle = palette[ink];
            ctx.beginPath();
            ctx.arc(p.px, p.py, rr, 0, Math.PI * 2);
            ctx.fill();
          }
        }
      }
      ctx.globalAlpha = 1;
      return true;
    };

    const frame = (now) => {
      raf = requestAnimationFrame(frame);
      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      if (canvas.width !== rect.width * dpr || canvas.height !== rect.height * dpr) {
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
      }

      const secs = now / 1000;
      const phase = (secs / LOOP_SECS) % 1;

      // The transition, then everything layered on top of it.
      // Flair plays only over the resting face, and never while a turn is in
      // flight: one long step (a rebuild, a slow search) can go quiet for
      // longer than the 45 s settle, and glancing about mid-work would read
      // as parker losing interest in his own task.
      const spec = EXPRESSIONS[target];
      const into = landing(now);
      const stacked = apply(
        currentPose(now),
        into === null ? spec.motion : motionAt(entering, spec.motion, into),
        worn === 'idle' && !working,
        into === null && spec.blinks !== false,
      );
      draw(stacked.pose, stacked.motion, phase, secs, grid);

      const palette = inks(getComputedStyle(canvas));
      const shot = camera(now);
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // A crossing owns the frame outright: the face is a stream, and a
      // dance lens or echo applied to a stream is noise on noise.
      if (shot.A && pour(grid.dots, palette, geomOf(shot.A, dpr, 1), geomOf(shot.B, dpr, 1), shot.t, secs)) {
        return;
      }

      // The surge scales about the box's own centre, so parker grows towards
      // you rather than sliding down and right as he does it. It is locked
      // to the beat and deliberately not smoothed (Stage's easing is).
      const surge = surgeFor(beatState.mood, beatState.beats);
      const geom = {
        ...geomOf(shot.box, dpr, surge),
        lens: lensFor(beatState.mood),
        beats: beatState.beats,
      };

      // Trails first, oldest and faintest at the back.
      const echo = echoFor(beatState.mood);
      if (echo.frames > 0) {
        trail.unshift(new Uint8Array(grid.dots));
        if (trail.length > echo.frames + 1) trail.length = echo.frames + 1;
        for (let i = trail.length - 1; i >= 1; i--) {
          paint(trail[i], palette, geom, echo.falloff ** i, {
            dx: 0,
            dy: -echo.drift * i * 0.15,
          });
        }
      } else if (trail.length) {
        trail.length = 0;
      }

      // RGB glitch, drop only.
      for (const s of splitFor(beatState.mood, beatState.beats) ?? []) {
        paint(grid.dots, palette, geom, s.alpha, { dx: s.dx, dy: s.dy }, s.ink);
      }

      paint(grid.dots, palette, geom, 1, { dx: 0, dy: 0 });
    };

    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  });
</script>

<canvas bind:this={canvas}></canvas>

<style>
  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
