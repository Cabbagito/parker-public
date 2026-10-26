// The camera, per dot.
//
// `Face.svelte` draws each lit dot at `(ox + x*pitch, oy + y*pitch)`. A lens
// sits in that one expression: a pure function of `(dot position, time) ->
// screen position`, which is *precisely* the definition of placement in
// views.js — "a transition that carries parker to the rail along a helix is
// placement — same lit dots, a different camera".
//
// So none of this is a pose. parkerd never learns that the face scattered
// into loose dots and reformed; it only ever said the word "drop". That is
// the whole seam, and it is what lets the camera be as elaborate as it likes.
//
// Everything here is driven by `beats` rather than by seconds, because a
// camera that is not locked to the bar reads as a screensaver running behind
// a dance rather than as part of it.

import { hash01 } from './face.js';

const TAU = Math.PI * 2;

/// A dot's own stable randomness — its scatter direction, its shear phase.
/// Keyed on grid position so a dot behaves the same way every bar, which is
/// what makes a scatter look like the face coming apart rather than like
/// static.
function dotRand(x, y, salt) {
  return hash01(y * 128 + x, salt);
}

/// Sharp on the downbeat, gone by the next one. The camera's own `hit`.
function hit(beats, div = 1) {
  const into = (beats / div) % 1;
  return (1 - into) ** 3;
}

/// 0 at the start of a bar, 1 in the middle, 0 at the end. Anything that has
/// to *return* by the bar line uses this, so the face is always whole when
/// the next bar starts.
function arch(beats, bars = 1) {
  const into = (beats / (4 * bars)) % 1;
  return Math.sin(into * Math.PI) ** 2;
}

/// Out and back inside the *first beat* of each bar, and nothing for the
/// other three.
///
/// This replaced `arch` for the scatter, and the difference is the whole
/// character of the drop. An arch over a full bar leaves the face dispersed
/// for most of it, and a face that is never whole is not a face coming apart
/// — it is confetti. Rendered and looked at: parker was unrecognisable for
/// the entire drop, with no moment of reassembly to make the dispersal read
/// as dispersal.
///
/// One beat out, three beats whole and hitting. The burst punctuates the
/// choreography instead of erasing it.
function burst(beats) {
  const into = beats % 4;
  if (into > 1) return 0;
  return Math.sin(into * Math.PI);
}

const IDENTITY = () => ({ dx: 0, dy: 0, scale: 1 });

/// Ripple — a wave crossing the face, one crest per bar.
function ripple(x, y, beats) {
  const dy = Math.sin(x * 0.32 - beats * TAU * 0.5) * 1.6;
  return { dx: 0, dy, scale: 1 };
}

/// Shear — alternating bands of rows slide opposite ways, snapping on the
/// beat. Split-flap. Stepped rather than eased, same reason the pose moves
/// are: interpolated, it is a wobble.
function shear(x, y, beats) {
  const band = Math.floor(y / 3);
  const dir = band % 2 === 0 ? 1 : -1;
  return { dx: dir * hit(beats, 0.5) * 3.4, dy: 0, scale: 1 };
}

/// Vortex — angular displacement proportional to radius, so the face twists
/// into a whirlpool and untwists by the bar line.
function vortex(x, y, beats, strength) {
  const cx = x - 48;
  const cy = (y - 20) * 2.4; // dots are ~2.4x taller than wide on this grid
  const r = Math.hypot(cx, cy);
  if (r < 0.001) return { dx: 0, dy: 0, scale: 1 };
  const a = Math.atan2(cy, cx) + (r / 40) * strength;
  return { dx: Math.cos(a) * r - cx, dy: (Math.sin(a) * r - cy) / 2.4, scale: 1 };
}

/// Scatter — every dot flies out along its own vector and snaps back. The
/// single most impressive thing available, and `Stage.svelte` already
/// anticipated it in prose ("a dissolve-and-reform").
///
/// The distance is the number that matters and it wants to be *small*. The
/// grid is 96 dots across, so a throw of 26 puts every dot most of a
/// screen-width from where it belongs and the face stops existing. Ten reads
/// as parker coming apart at the seams and going back together, which is the
/// effect actually wanted — you have to be able to see the thing that is
/// being disrupted.
function scatter(x, y, beats) {
  const k = burst(beats);
  if (k < 0.001) return { dx: 0, dy: 0, scale: 1 };
  const a = dotRand(x, y, 0x5ca7) * TAU;
  const d = (0.35 + dotRand(x, y, 0x11e5) * 0.65) * k * 10;
  return {
    dx: Math.cos(a) * d,
    dy: Math.sin(a) * d * 0.42,
    // Dots shrink as they fly, so a scattered face reads as dispersing
    // rather than as the same face drawn wrong.
    scale: 1 - k * 0.3,
  };
}

/// The camera for a mood. `null` outside a dance: parker at rest gets no
/// lens at all, and the ordinary idle face costs nothing for this to exist.
export function lensFor(mood) {
  switch (mood) {
    case 'intro':
      return (x, y, beats) => ripple(x, y, beats);
    case 'build':
      return (x, y, beats) => {
        const s = shear(x, y, beats);
        const v = vortex(x, y, beats, Math.sin(beats * TAU * 0.125) * 0.6);
        return { dx: s.dx + v.dx, dy: s.dy + v.dy, scale: 1 };
      };
    case 'drop':
      // The vortex rides the same burst rather than its own slow arch: two
      // displacements on independent clocks read as noise, and stacking a
      // bar-long twist under a bar-long scatter was most of why the first
      // version never resolved into a face.
      return (x, y, beats) => {
        const k = burst(beats);
        const s = scatter(x, y, beats);
        const v = vortex(x, y, beats, k * 0.45);
        return { dx: s.dx + v.dx, dy: s.dy + v.dy, scale: s.scale };
      };
    case 'outro':
      return (x, y, beats) => {
        const r = ripple(x, y, beats);
        return { dx: 0, dy: r.dy * 0.4, scale: 1 };
      };
    default:
      return IDENTITY;
  }
}

/// How many past frames to draw behind the current one, and how fast they
/// fade. Trails are cheap and they make every other move look more expensive
/// than it is.
export function echoFor(mood) {
  switch (mood) {
    case 'drop':
      return { frames: 5, falloff: 0.55, drift: 2.5 };
    case 'build':
      return { frames: 3, falloff: 0.45, drift: 1.5 };
    case 'intro':
    case 'outro':
      return { frames: 2, falloff: 0.32, drift: 1.0 };
    default:
      return { frames: 0, falloff: 0, drift: 0 };
  }
}

/// RGB glitch: draw the whole grid three times in the theme's own accent,
/// good and bad, at small offsets. Comic-book, and it stays coherent under
/// `theme set` because the colours are the theme's rather than literal red
/// and blue.
///
/// Drop only. Full send was the explicit call, and this is the loudest thing
/// in the file.
export function splitFor(mood, beats) {
  if (mood !== 'drop') return null;
  const k = hit(beats, 1) * 2.0;
  if (k < 0.05) return null;
  // Two fringes, not three, and thinner. Three at high alpha over a trail
  // stack put more coloured dots on screen than uncoloured ones, which is
  // not a glitch on a face — it is just colour.
  return [
    { ink: 2, dx: -k, dy: 0, alpha: 0.4 },
    { ink: 4, dx: k, dy: 0, alpha: 0.4 },
  ];
}

// ── the river ───────────────────────────────────────────────────────────────
// The travel between views, per dot (docs/visage/redesign/README.md, "The
// travel"). Still a lens by the file's own definition — (dot position, time)
// -> screen position — just one whose time axis is a crossing rather than a
// bar. parkerd is never told the face came apart.
//
// The move is three phases with one shared middle, so the crossing looks the
// same regardless of which face is on:
//
//   fall    (0 → 0.22 of a dot's own run) — the dot moves from where it sits
//           to its slot on the path. Its offset from the face's centre is
//           *squeezed*, not released: 35% along the path, `comp` across it.
//           The face collapses into a narrow ribbon; it must never bloom
//           outward, which is what the earlier scatter version got wrong.
//   run     (0.22 → 0.8) — the ribbon runs a quadratic Bézier from entry
//           slot to exit slot. One control point shared by every dot, bowing
//           downward, so the stream sags like something poured.
//   reform  (0.8 → 1) — the dot lerps from its exit slot to its exact
//           destination, so the face is whole the frame the travel ends.
//
// A dot that has not launched yet stays exactly where it is — dropping
// unlaunched dots blanks the face for the first frames, the bug worth not
// reproducing.

export const RIVER = {
  ms: 780,
  stag: 0.5, // fraction of the travel spent launching
  comp: 0.12, // perpendicular squeeze onto the path
  arc: 0.2, // control-point offset, as a fraction of path length
  wob: 1.5, // braid amplitude, in dot widths
  trails: 2, // past samples per dot, drawn only while running
  shrink: 0.18, // dot radius reduction while in the stream
  speed: 0.12, // per-dot speed variance across the run
};

const FALL = 0.22;
const FORM = 0.8;
export const RUN = 1 - RIVER.stag; // a dot's own run, as travel fraction
export const TRAIL_STEP = 0.026; // travel-fraction lag per trail sample (~20ms)

/// The shared path between two geoms ({pitch, ox, oy} in canvas pixels).
/// Null when the crossing is too short to bother pouring.
export function riverPath(A, B, DOTS_W, DOTS_H) {
  const acx = A.ox + (DOTS_W / 2) * A.pitch;
  const acy = A.oy + (DOTS_H / 2) * A.pitch;
  const bcx = B.ox + (DOTS_W / 2) * B.pitch;
  const bcy = B.oy + (DOTS_H / 2) * B.pitch;
  const pathLen = Math.hypot(bcx - acx, bcy - acy);
  if (pathLen <= 4) return null;
  const ux = (bcx - acx) / pathLen;
  const uy = (bcy - acy) / pathLen;
  const vx = -uy;
  const vy = ux;
  // The run follows one shared curve rather than the chord: a control point
  // pushed off the midpoint, always bowing downward on screen so the stream
  // sags like something being poured.
  const bow = vy < 0 ? -1 : 1;
  return {
    acx,
    acy,
    bcx,
    bcy,
    ux,
    uy,
    vx,
    vy,
    pathLen,
    midPitch: (A.pitch + B.pitch) / 2,
    ctlOffx: vx * bow * RIVER.arc * pathLen,
    ctlOffy: vy * bow * RIVER.arc * pathLen,
  };
}

/// Launch-order span: min and max of the lit dots' offsets measured along
/// the direction of travel. Normalising over the lit dots rather than the
/// grid is the whole trick — an iris only occupies the middle ~40 columns,
/// and normalising over the grid compresses the pour into a third of the
/// travel.
export function riverSpan(dots, A, path, DOTS_W, DOTS_H) {
  let minL = Infinity;
  let maxL = -Infinity;
  for (let y = 0; y < DOTS_H; y++) {
    for (let x = 0; x < DOTS_W; x++) {
      if (!dots[y * DOTS_W + x]) continue;
      const off =
        (x + 0.5 - DOTS_W / 2) * A.pitch * path.ux + (y + 0.5 - DOTS_H / 2) * A.pitch * path.uy;
      if (off < minL) minL = off;
      if (off > maxL) maxL = off;
    }
  }
  return { minL, range: Math.max(1, maxL - minL), maxL };
}

/// Everything about one dot that does not depend on progress: its real start
/// and destination, its squeezed slots at each end of the run, its jitter,
/// and when it departs. The edge facing the destination leaves first.
export function riverDot(x, y, A, B, path, span) {
  const srcx = A.ox + (x + 0.5) * A.pitch;
  const srcy = A.oy + (y + 0.5) * A.pitch;
  const dstx = B.ox + (x + 0.5) * B.pitch;
  const dsty = B.oy + (y + 0.5) * B.pitch;
  const off = (srcx - path.acx) * path.ux + (srcy - path.acy) * path.uy;
  const s = Math.min(1, Math.max(0, (span.maxL - off) / span.range));
  const jit = dotRand(x, y, 0x9a17);
  const sov = (srcx - path.acx) * path.vx + (srcy - path.acy) * path.vy;
  const dou = (dstx - path.bcx) * path.ux + (dsty - path.bcy) * path.uy;
  const dov = (dstx - path.bcx) * path.vx + (dsty - path.bcy) * path.vy;
  return {
    srcx,
    srcy,
    dstx,
    dsty,
    s,
    jit,
    depart: (s * 0.86 + jit * 0.14) * RIVER.stag,
    entx: path.acx + path.ux * off * 0.35 + path.vx * sov * RIVER.comp,
    enty: path.acy + path.uy * off * 0.35 + path.vy * sov * RIVER.comp,
    exx: path.bcx + path.ux * dou * 0.35 + path.vx * dov * RIVER.comp,
    exy: path.bcy + path.uy * dou * 0.35 + path.vy * dov * RIVER.comp,
  };
}

/// One dot at progress `w` through its own run (already clamped to 0..1).
/// `thin` is how deep in the stream it is — 0 at rest, 1 mid-run — which
/// drives the radius shrink.
export function riverSample(d, path, w, secs, A, B, ease) {
  if (w < FALL) {
    const e = ease(w / FALL);
    return {
      px: d.srcx + (d.entx - d.srcx) * e,
      py: d.srcy + (d.enty - d.srcy) * e,
      pitch: A.pitch,
      thin: e,
    };
  }
  if (w < FORM) {
    let q = (w - FALL) / (FORM - FALL);
    q = Math.min(1, Math.max(0, q * (1 + (d.jit - 0.5) * RIVER.speed)));
    // Quadratic through the shared control point. Each dot's own bow varies
    // a little, which gives the ribbon body without letting it widen.
    const k = 0.82 + d.jit * 0.36;
    const cx2 = (d.entx + d.exx) / 2 + path.ctlOffx * k;
    const cy2 = (d.enty + d.exy) / 2 + path.ctlOffy * k;
    const m = 1 - q;
    const braid =
      Math.sin(d.s * 7 + secs * 8 + d.jit * TAU) * RIVER.wob * path.midPitch * Math.sin(Math.PI * q);
    return {
      px: m * m * d.entx + 2 * m * q * cx2 + q * q * d.exx + path.vx * braid,
      py: m * m * d.enty + 2 * m * q * cy2 + q * q * d.exy + path.vy * braid,
      pitch: A.pitch + (B.pitch - A.pitch) * q,
      thin: 1,
    };
  }
  const e = ease((w - FORM) / (1 - FORM));
  return {
    px: d.exx + (d.dstx - d.exx) * e,
    py: d.exy + (d.dsty - d.exy) * e,
    pitch: B.pitch,
    thin: 1 - e,
  };
}

/// A dot's run window: FALL..FORM is the only span trails are drawn in.
export function inRun(w) {
  return w > FALL && w < FORM;
}

/// A whole-face scale, applied by the stage rather than per dot: on a drop
/// the face lunges at the screen. Kept separate from the lens because it is
/// a property of the box, not of any dot in it.
/// A whole-face scale, applied by the stage rather than per dot: on a drop
/// the face lunges at the screen.
///
/// Modest, because it multiplies everything else. A 1.55 surge under a
/// scatter does not read as 55% bigger, it reads as the dots being thrown
/// half again as far — the two effects compound rather than layer, and the
/// face left the screen entirely.
export function surgeFor(mood, beats) {
  if (!mood) return 1;
  if (mood === 'drop') return 1 + arch(beats, 2) * 0.14 + hit(beats, 1) * 0.05;
  if (mood === 'build') return 1 + hit(beats, 1) * 0.05;
  return 1 + hit(beats, 1) * 0.03;
}
