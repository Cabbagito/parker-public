// parker's face, ported from parkerd/src/face/{pose,mod}.rs.
//
// The Rust file says it best: "a renderer is a backend, not a rewrite." This
// is that promised second backend — the same pose numbers, the same lerp, the
// same thirty-seven expressions, rasterized to a dot grid and drawn as discs
// on a canvas instead of braille in a terminal. Geometry constants are in dot
// units on the original 96x40 grid, so proportions survive verbatim.
//
// Keep this file in step with the Rust: an expression changed there is
// changed here, same numbers, same names. Divergence is a bug, not a style —
// and `parity.mjs` catches it, every pose and the new motions checked against
// the answers parkerd writes into catalog.json.

export const DOTS_W = 96;
export const DOTS_H = 40;

const TAU = Math.PI * 2;

/// Strokes per pose. Widened from two when parker learned to sleep: a `Z` is
/// three strokes and `zZZ` wants two of them at once (pose.rs::MARKS).
export const MARKS = 6;

// ── pose ────────────────────────────────────────────────────────────────────

const noStroke = () => ({ x0: 0, y0: 0, x1: 0, y1: 0, w: 0 });

export const ZERO = {
  gap: 0, w: 0, h: 0, round: 1, y: 0, x: 0, tilt: 0, squint: 1,
  pupil: 0, gazeX: 0, gazeY: 0,
  ring: 0, ringW: 0, sweep: 1, spin: 0,
  amp: 0, freq: 0,
  mark: Array.from({ length: MARKS }, noStroke),
  ink: 'plain',
  // One ink per mark stroke (pose.rs grew the same array): three quavers in
  // the house colours, a white scan line beside a red contact.
  markInk: Array.from({ length: MARKS }, () => 'plain'),
};

/// Pad a few inks out to a full per-stroke array (`Ink::pad`).
export function padInks(given) {
  const out = Array.from({ length: MARKS }, () => 'plain');
  for (let i = 0; i < Math.min(given.length, MARKS); i++) out[i] = given[i];
  return out;
}

// Field names, and deliberately the same strings the exported catalog uses —
// `Field::name()` in pose.rs emits these spellings for exactly this lookup.
const NUMERIC = [
  'gap', 'w', 'h', 'round', 'y', 'x', 'tilt', 'squint',
  'pupil', 'gazeX', 'gazeY', 'ring', 'ringW', 'sweep', 'spin', 'amp', 'freq',
];

/// Pad a few strokes out to a full mark array (`Stroke::pad`).
export function padMarks(given) {
  const out = Array.from({ length: MARKS }, noStroke);
  for (let i = 0; i < Math.min(given.length, MARKS); i++) out[i] = given[i];
  return out;
}

// t outside 0..1 is allowed on purpose: overshoot is the spring.
export function lerp(a, b, t) {
  const out = { ...ZERO };
  for (const k of NUMERIC) out[k] = a[k] + (b[k] - a[k]) * t;
  out.mark = a.mark.map((m, i) => {
    const n = b.mark[i] ?? m;
    return {
      x0: m.x0 + (n.x0 - m.x0) * t,
      y0: m.y0 + (n.y0 - m.y0) * t,
      x1: m.x1 + (n.x1 - m.x1) * t,
      y1: m.y1 + (n.y1 - m.y1) * t,
      w: m.w + (n.w - m.w) * t,
    };
  });
  // Colour is not interpolated: a transition takes the target's inks at once.
  out.ink = b.ink;
  out.markInk = b.markInk;
  return out;
}

export const ease = {
  inOut(t) {
    t = Math.min(1, Math.max(0, t));
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  },
  backOut(t) {
    t = Math.min(1, Math.max(0, t));
    const c1 = 1.70158, c3 = c1 + 1;
    return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
  },
  // Leaves fast, arrives gently — the shape of a flick.
  out(t) {
    t = Math.min(1, Math.max(0, t));
    return 1 - Math.pow(1 - t, 3);
  },
  linear(t) {
    return Math.min(1, Math.max(0, t));
  },
};

/// Easing by name, so a keyframe out of the exported catalog can say which.
export const EASINGS = {
  linear: ease.linear,
  inOut: ease.inOut,
  out: ease.out,
  backOut: ease.backOut,
};

// ── involuntary movement ────────────────────────────────────────────────────

/// Deterministic hash, step index to 0..1. splitmix64's finalizer, in BigInt
/// because the mixing is defined on 64-bit wraparound and doubles cannot hold
/// it. The result is taken from the top 53 bits — exactly what a double holds
/// exactly — so this produces bit-identical values to the Rust (pose.rs).
const M64 = (1n << 64n) - 1n;
export function hash01(n, salt) {
  let z = (BigInt(n) * 0x9e3779b97f4a7c15n) & M64;
  z ^= (BigInt(salt) * 0xbf58476d1ce4e5b9n) & M64;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & M64;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & M64;
  z ^= z >> 31n;
  return Number(z >> 11n) / 9007199254740992;
}

/// Where the eye is looking at `t` seconds, in dots from centre.
///
/// Flick, fixate, and a slow wander inside the fixation. Real eyes do not
/// glide, and the sine this replaced was the one motion an eye never makes.
/// See pose.rs::saccade for the reasoning; the numbers are its numbers.
export function saccade(t) {
  const DWELL = 0.62, FLICK = 0.045, REACH_X = 3.4, REACH_Y = 1.5;

  const anchor = (slot) => {
    let s = slot;
    for (let i = 0; i < 2; i++) {
      if (s > 0 && hash01(s, 0x5eed) < 0.34) s -= 1;
      else break;
    }
    return s;
  };

  const target = (slot) => {
    const s = anchor(slot);
    // Eye contact, often enough to notice and not often enough to stare.
    if (hash01(s, 0xc0ffee) < 0.22) return [0, 0];
    return [
      (hash01(s, 0xa11ce) * 2 - 1) * REACH_X,
      (hash01(s, 0xb0b) * 2 - 1) * REACH_Y,
    ];
  };

  const slot = Math.floor(t / DWELL);
  const into = t - slot * DWELL;
  const [fx, fy] = target(slot);
  const [px, py] = target(slot - 1);
  const k = into < FLICK ? ease.out(into / FLICK) : 1;

  const dx = Math.sin(t * 1.7) * 0.22 + Math.sin(t * 0.53) * 0.12;
  const dy = Math.cos(t * 1.13) * 0.14;
  return [px + (fx - px) * k + dx, py + (fy - py) * k + dy];
}

/// How much to squash an eye at `t`, 1 open and ~0 shut (pose.rs::blink).
///
/// This used to be `Math.random()` here and nothing at all in parkerd, so the
/// two faces blinked differently and one of them not at all. Deterministic on
/// the clock means both agree without a channel between them.
export function blink(t) {
  const SLOT = 1.05, SHUT = 0.14, AGAIN = 0.23;
  const slot = Math.floor(t / SLOT);
  let squash = 1;
  // This slot and the one before it: a blink starting late has to finish in
  // the next one.
  for (const s of [slot - 1, slot]) {
    if (hash01(s, 0xb11) >= 0.34) continue;
    const begins = s * SLOT + hash01(s, 0xb12) * Math.max(0, SLOT - SHUT - AGAIN);
    const starts = [begins];
    if (hash01(s, 0xb13) < 0.18) starts.push(begins + AGAIN);
    for (const start of starts) {
      const into = (t - start) / SHUT;
      if (into >= 0 && into < 1) {
        squash = Math.min(squash, Math.max(0.06, Math.abs(1 - into * 2)));
      }
    }
  }
  return squash;
}

/// Whether this pose has anything a blink could close.
export function blinkable(p) {
  return p.pupil > 0.3 || (p.gap > 0.6 && p.h > 2);
}

// ── motion ──────────────────────────────────────────────────────────────────

// Stroke arithmetic for the motions that move strokes (pose.rs, same names).
function turned(m, px, py, th) {
  const s = Math.sin(th), c = Math.cos(th);
  const at = (x, y) => [px + (x - px) * c - (y - py) * s, py + (x - px) * s + (y - py) * c];
  const [x0, y0] = at(m.x0, m.y0);
  const [x1, y1] = at(m.x1, m.y1);
  return { ...m, x0, y0, x1, y1 };
}
const moved = (m, dx, dy) => ({ ...m, x0: m.x0 + dx, y0: m.y0 + dy, x1: m.x1 + dx, y1: m.y1 + dy });
const scaled = (m, px, py, k) => ({
  ...m,
  x0: px + (m.x0 - px) * k,
  y0: py + (m.y0 - py) * k,
  x1: px + (m.x1 - px) * k,
  y1: py + (m.y1 - py) * k,
});
/// Every drawn stroke through `f`, with its index; absent ones untouched.
const eachMark = (marks, f) => marks.map((m, i) => (m.w > 0.05 ? f(i, m) : m));

/// `phase` is 0..1 through the idle loop and drives everything periodic; `t`
/// is absolute seconds, for the motions that must not repeat on it.
export function applyMotion(motion, pose, phase, t) {
  const p = { ...pose, mark: pose.mark };
  const a = phase * TAU;
  switch (motion) {
    // ── the faces round (pose.rs, same order, same numbers). Most were
    // written for one face, and the ones that move strokes know that face's
    // strokes by index.
    case 'tremble':
      p.x += Math.sin(t * 53) * 0.5 + Math.sin(t * 31) * 0.3;
      p.y += Math.sin(t * 41) * 0.35;
      break;
    case 'bounce':
      p.y -= Math.abs(Math.sin(a * 3)) * 2;
      break;
    case 'giggle': {
      const gp = (phase * 3) % 1;
      if (gp < 0.4) p.y -= Math.abs(Math.sin((gp / 0.4) * Math.PI * 3)) * 1.3;
      break;
    }
    case 'vibe':
      p.y += (0.5 - 0.5 * Math.cos(a * 9)) * 1.8;
      p.x += Math.sin(a) * 2;
      break;
    case 'orbit': {
      const [ox, oy, rx, ry] = [0, -14, 24, 3.6];
      p.gazeX += Math.cos(a * 4) * 2.2;
      p.gazeY += Math.sin(a * 4) * 1.6;
      p.mark = eachMark(p.mark, (i, m) => {
        const cx = (m.x0 + m.x1) / 2, cy = (m.y0 + m.y1) / 2;
        const ang = Math.atan2((cy - oy) / ry, (cx - ox) / rx) + a * 2;
        return moved(m, ox + Math.cos(ang) * rx - cx, oy + Math.sin(ang) * ry - cy);
      });
      break;
    }
    case 'tip': {
      const th = Math.sin(a * 2) * 0.2;
      p.mark = eachMark(p.mark, (i, m) => {
        if (i === 0) return turned(m, 0, -9, th);
        if (i !== 3 && i !== 4) return m;
        // Rust's signum: zero counts as positive.
        const side = (m.x0 + m.x1) / 2 - 0 < 0 ? -1 : 1;
        return moved(m, (Math.cos(th) - 1) * 20 * side, Math.sin(th) * 20 * side);
      });
      break;
    }
    case 'shifty': {
      const k =
        phase < 0.5 ? 0
        : phase < 0.58 ? (phase - 0.5) / 0.08
        : phase < 0.76 ? 1
        : phase < 0.84 ? 1 - (phase - 0.76) / 0.08
        : 0;
      p.gazeX += ease.inOut(k) * 8.4;
      break;
    }
    case 'read': {
      const LINES = 6;
      const lp = phase * LINES, line = Math.floor(lp), u = lp - line;
      let along;
      if (u < 0.84) {
        const s5 = (u / 0.84) * 5, f5 = Math.floor(s5);
        along = (f5 + ease.out(Math.min(1, (s5 - f5) / 0.25))) / 5;
      } else along = 1 - ease.inOut((u - 0.84) / 0.16);
      p.gazeX += -3 + 6 * along;
      p.gazeY += -1.8 + (3.6 * line) / (LINES - 1);
      break;
    }
    case 'bonk': {
      const bp = (phase * 2) % 1;
      const push =
        bp < 0.12 ? ease.inOut(bp / 0.12)
        : bp < 0.2 ? 1 - 1.4 * ease.out((bp - 0.12) / 0.08)
        : bp < 0.4 ? -0.4 + 0.4 * ease.inOut((bp - 0.2) / 0.2)
        : 0;
      const dx = push * 3.5;
      p.x += dx;
      if (push > 0.75) {
        p.w *= 0.84;
        p.h *= 1.08;
      }
      p.mark = eachMark(p.mark, (i, m) => moved(m, -dx, 0));
      break;
    }
    case 'fuse': {
      const flash = (t * (1.5 + 7 * phase)) % 1 < 0.35;
      const s = 1 + 0.07 * phase;
      p.w *= s;
      p.h *= s;
      p.mark = eachMark(p.mark, (i, m) => scaled(m, 0, 0, s));
      if (flash) {
        p.ink = 'plain';
        p.markInk = Array.from({ length: MARKS }, () => 'plain');
      }
      break;
    }
    case 'glitch': {
      const slot = Math.floor(Math.max(0, t) * 9);
      if (hash01(slot, 0x61) < 0.4) {
        const j = hash01(slot, 0x62) * 2 - 1;
        const yy = (hash01(slot, 0x65) * 2 - 1) * 14;
        p.x += j * 7;
        p.w *= 0.7 + hash01(slot, 0x63) * 0.8;
        p.gazeX += j * 2;
        p.ink = hash01(slot, 0x64) < 0.5 ? 'accent' : 'bad';
        p.mark = p.mark.slice();
        p.mark[5] = { x0: -44 - p.x, y0: yy, x1: 44 - p.x, y1: yy, w: 1 };
        p.markInk = p.markInk.slice();
        p.markInk[5] = p.ink;
      }
      break;
    }
    case 'sand': {
      const R = 5.2;
      const drained = Math.min(1, phase / 0.84);
      const bottom = p.tilt, neck = p.tilt / 2;
      const full = 1.6 * R;
      const topT = (1 - drained) * full, pileT = drained * full;
      const lump = (cy, thick) => {
        const yc = cy + R - 0.4 - thick / 2, dy = yc - cy;
        const half = Math.sqrt(Math.max(0, R * R - dy * dy)) - thick / 2 - 0.4;
        return { x0: -Math.max(0, half), y0: yc, x1: Math.max(0, half), y1: yc, w: thick > 0.8 ? thick : 0 };
      };
      const mark = p.mark.slice();
      mark[2] = lump(0, topT);
      mark[3] = lump(bottom, pileT);
      mark[4] = { x0: 0, y0: neck, x1: 0, y1: bottom + R - 0.4 - pileT, w: drained > 0.02 && drained < 1 ? 1 : 0 };
      p.mark = mark;
      if (phase > 0.84) {
        const c = Math.cos(Math.PI * ease.inOut((phase - 0.84) / 0.16)), ac = Math.abs(c);
        const sink = neck * (1 - ac);
        p.y += sink;
        p.tilt = bottom * ac;
        p.h *= ac;
        p.mark = eachMark(p.mark, (i, m) => ({
          ...m,
          y0: neck + (m.y0 - neck) * c - sink,
          y1: neck + (m.y1 - neck) * c - sink,
        }));
      }
      break;
    }
    case 'jab': {
      const poke = (start) => {
        const k = (phase - start) / 0.08;
        if (!(k >= 0 && k <= 1)) return 0;
        return k < 0.5 ? ease.out(k * 2) : 1 - ease.inOut(k * 2 - 1);
      };
      const dx = (poke(0.03) + poke(0.14)) * 3.5;
      p.mark = eachMark(p.mark, (i, m) => moved(m, dx, 0));
      break;
    }

    case 'breathe': {
      const s = 1 + Math.sin(a) * 0.06;
      p.w *= s;
      p.h *= s;
      if (p.pupil > 0) p.pupil *= 1 + Math.sin(a) * 0.12;
      break;
    }
    case 'wave':
      p.amp *= 1 + Math.sin(a * 0.5) * 0.15;
      break;
    case 'pulse': {
      const s = 1 - Math.max(0, Math.sin(a * 5)) * 0.22;
      p.w *= s;
      p.h *= s;
      p.ring *= 1 + Math.sin(a * 5) * 0.05;
      break;
    }
    case 'nod':
      p.y += Math.sin(a * 7) * 3.5;
      break;
    case 'shake':
      p.x += Math.sin(a * 9) * 3.5;
      break;
    case 'drift': {
      const [gx, gy] = saccade(t);
      p.gazeX += gx;
      p.gazeY += gy;
      break;
    }
    // 'none' and 'blades': blades are drawn from phase by the renderer.
  }
  return p;
}

// ── expressions ─────────────────────────────────────────────────────────────
// Same numbers as face/mod.rs, same builders. `parity.mjs` checks every pose
// here against parkerd's own (catalog.json carries them), strokes and inks
// included, because thirty-seven faces is past what "line for line" survives
// unchecked.

const iris = (w, ring) => ({ ...ZERO, gap: 0, w, h: w, round: 1, ring, ringW: 2 });
const eyes = (w, h, pupil) => ({ ...ZERO, gap: 18, w, h, round: 0.85, pupil });
const line = (w, amp, freq) => ({ ...ZERO, gap: 0, w, h: 1.4, round: 0, amp, freq });
const s = (x0, y0, x1, y1, w) => ({ x0, y0, x1, y1, w });
const stroke = (x0, y0, x1, y1) => s(x0, y0, x1, y1, 3);
const dot = (x, y, w) => s(x, y, x, y, w);
const every = (ink) => Array.from({ length: MARKS }, () => ink);
/// A closed happy eye, ^, drawn (mod.rs::caret).
const caret = (cx, y, half, rise, w) => [s(cx - half, y, cx, y - rise, w), s(cx, y - rise, cx + half, y, w)];
/// (⌐■_■), lenses at height `y` (mod.rs::shades).
const shades = (y) =>
  padMarks([
    s(-24, y, -12, y, 7),
    s(12, y, 24, y, 7),
    s(-12, y - 2, 12, y - 2, 1.6),
    s(-27, y - 2.2, -37, y - 3.5, 1.6),
    s(27, y - 2.2, 37, y - 3.5, 1.6),
  ]);
/// ¯\_(ツ)_/¯'s arms (mod.rs::shrug_arms).
const shrugArms = (lift) => {
  const hand = -6 + lift * 20, elbow = 3 + lift * 5;
  return padMarks([
    s(-12, 3, -23, elbow, 2.2),
    s(-23, elbow, -30, hand, 2.2),
    s(-30, hand, -37, hand, 2.2),
    s(12, 3, 23, elbow, 2.2),
    s(23, elbow, 30, hand, 2.2),
    s(30, hand, 37, hand, 2.2),
  ]);
};
const plus = (x, y) => [s(x - 2.2, y, x + 2.2, y, 1.6), s(x, y - 2.2, x, y + 2.2, 1.6)];

/// `blinks: false` where the ordinary blink must not close the face
/// (Expression::blinks): the hourglass's bulbs, and a wink's open eye.
export const EXPRESSIONS = {
  idle: { pose: { ...iris(9, 17), pupil: 3 }, motion: 'drift' },
  thinking: { pose: iris(4, 17), motion: 'blades' },
  speaking: { pose: line(46, 10, 3), motion: 'wave' },
  // A brick wall, and parker walking into it.
  blocked: {
    pose: {
      ...eyes(6.5, 8, 2.8),
      x: -5,
      gazeX: 2.6,
      mark: [
        s(36, -17, 36, 17, 3.4),
        s(36, -9, 51, -9, 1.6),
        s(36, 0, 51, 0, 1.6),
        s(36, 9, 51, 9, 1.6),
        s(43, -9, 43, 0, 1.6),
        s(47, 0, 47, 9, 1.6),
      ],
    },
    motion: 'bonk',
  },
  away: { pose: line(8, 0, 0), motion: 'none' },
  trouble: {
    pose: { ...iris(5, 10), pupil: 1.8, ringW: 3, ink: 'bad' },
    motion: 'pulse',
  },
  yes: {
    pose: {
      ...ZERO,
      mark: padMarks([stroke(-11, 1, -3, 9), stroke(-3, 9, 13, -9)]),
      markInk: every('good'),
    },
    motion: 'nod',
  },
  no: {
    pose: {
      ...ZERO,
      mark: padMarks([stroke(-10, -10, 10, 10), stroke(10, -10, -10, 10)]),
      markInk: every('bad'),
    },
    motion: 'shake',
  },
  curious: {
    pose: {
      ...eyes(7.5, 9, 3.2),
      tilt: -1.5,
      gazeY: -1.2,
      gazeX: 0.8,
      mark: padMarks([s(-25, -12.5, -11, -12.5, 2), s(11, -15, 17, -18.5, 2), s(17, -18.5, 25, -16.5, 2)]),
    },
    motion: 'drift',
  },
  surprised: { pose: { ...iris(15, 19), pupil: 2 }, motion: 'breathe' },
  amused: {
    pose: { ...ZERO, mark: padMarks([...caret(-18, 1, 7, 5.5, 2.6), ...caret(18, 1, 7, 5.5, 2.6)]) },
    motion: 'giggle',
  },
  wince: {
    pose: {
      ...ZERO,
      mark: padMarks([
        s(-27, -6, -15, 0, 2.6),
        s(-15, 0, -27, 6, 2.6),
        s(27, -6, 15, 0, 2.6),
        s(15, 0, 27, 6, 2.6),
        s(-4, 9, 4, 9, 2),
      ]),
    },
    motion: 'tremble',
  },

  // The redesign's twelve (docs/visage/redesign), with the faces round's
  // redraws. The grid bound to watch is ring + ringW/2 <= 20; the largest
  // here is 18.
  listening: { pose: { ...eyes(7.5, 10, 3.2), gazeY: 0.6, squint: 0.92 }, motion: 'breathe' },
  skimming: { pose: eyes(6.5, 6.5, 2.6), motion: 'read' },
  digging: { pose: { ...iris(3, 16), ringW: 2.8 }, motion: 'blades' },
  smug: { pose: { ...eyes(6, 3.5, 2), y: -1, mark: shades(-1) }, motion: 'none' },
  sheepish: {
    pose: {
      ...eyes(7.5, 2.6, 0),
      y: -1,
      mark: padMarks([s(31, -17, 31, -12, 1.6), dot(31, -10, 4.6)]),
      markInk: every('accent'),
    },
    motion: 'none',
  },
  deadpan: { pose: { ...eyes(9, 1.6, 0), round: 0 }, motion: 'none' },
  fond: {
    pose: {
      ...eyes(7, 2.6, 0),
      y: -3,
      round: 1,
      ink: 'accent',
      mark: [
        s(-25, 7, -23, 3, 1.6),
        s(-20, 7, -18, 3, 1.6),
        s(-15, 7, -13, 3, 1.6),
        s(13, 7, 15, 3, 1.6),
        s(18, 7, 20, 3, 1.6),
        s(23, 7, 25, 3, 1.6),
      ],
      markInk: every('bad'),
    },
    motion: 'breathe',
  },
  melancholy: {
    pose: {
      ...eyes(7.5, 5, 2.6),
      gazeY: 2.4,
      y: 2.5,
      mark: padMarks([s(-26, -8, -12, -11, 1.8), s(12, -11, 26, -8, 1.8)]),
    },
    motion: 'breathe',
  },
  conspiratorial: {
    pose: {
      ...eyes(7.5, 8.5, 3.2),
      gap: 0,
      x: -18,
      gazeX: 0.4,
      mark: padMarks([s(29, 0.5, 36, -4, 2.6), s(36, -4, 43, 0.5, 2.6)]),
    },
    motion: 'none',
    blinks: false,
  },
  delighted: {
    pose: {
      ...ZERO,
      ink: 'accent',
      mark: [...caret(-18, 2, 9, 8, 3), ...caret(18, 2, 9, 8, 3), s(33, -13, 39, -13, 1.6), s(36, -16, 36, -10, 1.6)],
      markInk: every('accent'),
    },
    motion: 'bounce',
  },
  suspicious: { pose: { ...eyes(8, 3.2, 2), round: 0.35, gazeX: -4.4 }, motion: 'shifty' },
  pointing: {
    pose: {
      ...eyes(6.5, 8, 2.6),
      x: -12,
      gazeX: 2.6,
      mark: padMarks([stroke(31, 0, 54, 0), stroke(47, -7, 54, 0), stroke(47, 7, 54, 0)]),
      markInk: every('accent'),
    },
    motion: 'jab',
  },

  // The faces round's thirteen.
  waiting: {
    pose: {
      ...ZERO,
      gap: 0.6,
      w: 7,
      h: 6.6,
      round: 1,
      pupil: 5.6,
      y: -6,
      tilt: 12,
      mark: padMarks([s(-6, -7.6, 6, -7.6, 2.4), s(-6, 19.6, 6, 19.6, 2.4), s(-0.6, 0.64, 0.6, 0.64, 8.3)]),
      markInk: padInks(['plain', 'plain', 'accent', 'accent', 'accent']),
    },
    motion: 'sand',
    blinks: false,
  },
  weighing: {
    pose: {
      ...ZERO,
      mark: [
        s(-20, -9, 20, -9, 2),
        s(0, -9, 0, 12, 2),
        s(-9, 12.5, 9, 12.5, 2.6),
        s(-26, 0, -14, 0, 2.4),
        s(14, 0, 26, 0, 2.4),
        dot(0, -10.5, 4.2),
      ],
    },
    motion: 'tip',
  },
  vibing: {
    pose: {
      ...eyes(7, 1.5, 0),
      round: 0.7,
      y: -1,
      mark: padMarks([dot(34, -5, 4.4), s(35.8, -5.5, 35.8, -14, 1.4), s(35.8, -14, 39.5, -11, 1.4)]),
      markInk: every('accent'),
    },
    motion: 'vibe',
  },
  disapproval: {
    pose: {
      ...eyes(6.5, 6.5, 2.4),
      round: 1,
      mark: padMarks([s(-25, -10.5, -11, -10.5, 2), s(11, -10.5, 25, -10.5, 2), s(-4, 9, 4, 9, 2)]),
    },
    motion: 'none',
  },
  'eye-roll': { pose: { ...eyes(7.5, 6, 3), gazeY: -3, gazeX: 1, y: 0.5 }, motion: 'none' },
  'double-take': { pose: { ...eyes(8.5, 11, 1.8), gazeX: 3.4, y: -1.5 }, motion: 'none' },
  shrug: {
    pose: { ...ZERO, ring: 11, ringW: 1.6, gap: 4.5, w: 2.2, h: 1.1, round: 1, y: -1, mark: shrugArms(0) },
    motion: 'none',
  },
  'table-flip': {
    pose: {
      ...ZERO,
      x: -14,
      gap: 9,
      w: 4.2,
      h: 5,
      round: 1,
      pupil: 1.3,
      mark: padMarks([
        s(-11, 2, -19, -10, 2.2),
        s(11, 2, 19, -10, 2.2),
        s(55, 14, 33, 14, 2.6),
        s(52, 14, 52, 6, 2),
        s(36, 14, 36, 6, 2),
      ]),
    },
    motion: 'none',
  },
  dead: {
    pose: {
      ...ZERO,
      mark: padMarks([
        s(-24, -6, -12, 6, 2.4),
        s(-12, -6, -24, 6, 2.4),
        s(12, -6, 24, 6, 2.4),
        s(24, -6, 12, 6, 2.4),
        s(-5, 11, 5, 11, 2),
      ]),
    },
    motion: 'none',
  },
  dizzy: {
    pose: {
      ...eyes(5.5, 5.5, 2.2),
      gap: 16,
      round: 1,
      mark: [...plus(24, -14), ...plus(-12, -10.9), ...plus(-12, -17.1)],
      markInk: every('accent'),
    },
    motion: 'orbit',
  },
  glitch: { pose: { ...iris(9, 17), pupil: 3 }, motion: 'glitch' },
  creeper: {
    pose: {
      ...ZERO,
      gap: 9,
      w: 4.5,
      h: 4.5,
      round: 0,
      y: -9,
      ink: 'good',
      mark: [
        s(-2.2, 6.75, 2.2, 6.75, 4.4),
        s(-7.4, 10.5, 7.4, 10.5, 3),
        s(-7.4, 13.5, 7.4, 13.5, 3),
        s(-7.4, 16.5, 7.4, 16.5, 3),
        s(-6.75, 19.2, -6.75, 21.3, 4.2),
        s(6.75, 19.2, 6.75, 21.3, 4.2),
      ],
      markInk: every('good'),
    },
    motion: 'fuse',
  },
  uwu: {
    pose: {
      ...eyes(5.5, 1.6, 0),
      gap: 17,
      y: -3,
      round: 1,
      mark: [
        s(-8, 4, -4, 8, 1.6),
        s(-4, 8, 0, 5, 1.6),
        s(0, 5, 4, 8, 1.6),
        s(4, 8, 8, 4, 1.6),
        s(-28, 3, -23, 3, 1.8),
        s(23, 3, 28, 3, 1.8),
      ],
      markInk: padInks(['plain', 'plain', 'plain', 'plain', 'bad', 'bad']),
    },
    motion: 'breathe',
  },
};

// The expressive faces — reactions, not postures. These land with a spring
// (backOut) where everything else eases, unless catalog.json has an entrance
// for them; mirrors Expression::VOLUNTARY.
export const VOLUNTARY = [
  'yes', 'no', 'curious', 'surprised', 'amused', 'wince',
  'smug', 'sheepish', 'deadpan', 'fond', 'melancholy', 'conspiratorial',
  'delighted', 'suspicious',
  'disapproval', 'eye-roll', 'double-take', 'shrug', 'table-flip', 'dead',
  'dizzy', 'glitch', 'creeper', 'uwu',
];

// ── rasterizer ──────────────────────────────────────────────────────────────
// A dot grid instead of a braille grid; each set dot remembers its ink slot.

export class Grid {
  constructor(w = DOTS_W, h = DOTS_H) {
    this.w = w;
    this.h = h;
    this.dots = new Uint8Array(w * h); // 0 = off, else ink index + 1
    this.inkIdx = 1;
  }
  useInk(name) {
    this.inkIdx = { plain: 1, accent: 2, good: 3, bad: 4 }[name] ?? 1;
  }
  set(x, y) {
    x = Math.round(x);
    y = Math.round(y);
    if (x >= 0 && x < this.w && y >= 0 && y < this.h) this.dots[y * this.w + x] = this.inkIdx;
  }
  clear(x, y) {
    x = Math.round(x);
    y = Math.round(y);
    if (x >= 0 && x < this.w && y >= 0 && y < this.h) this.dots[y * this.w + x] = 0;
  }
  span(x, y0, y1) {
    for (let y = Math.round(y0); y <= Math.round(y1); y++) this.set(x, y);
  }
  clearEllipse(cx, cy, rx, ry) {
    for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y++) {
      for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++) {
        const nx = (x - cx) / rx, ny = (y - cy) / Math.max(ry, 0.1);
        if (nx * nx + ny * ny <= 1) this.clear(x, y);
      }
    }
  }
  stroke(x0, y0, x1, y1, w) {
    const dx = x1 - x0, dy = y1 - y0;
    const len = Math.hypot(dx, dy);
    const steps = Math.max(1, Math.ceil(len * 2));
    const r = w / 2;
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const px = x0 + dx * t, py = y0 + dy * t;
      for (let oy = -Math.ceil(r); oy <= Math.ceil(r); oy++)
        for (let ox = -Math.ceil(r); ox <= Math.ceil(r); ox++)
          if (ox * ox + oy * oy <= r * r) this.set(px + ox, py + oy);
    }
  }
  /// A ring, whole or partial. `spin` turns where a partial one starts, in
  /// turns from straight up; a whole ring ignores it, having no seam to turn.
  ringAt(cx, cy, radius, width, sweep, spin) {
    const s = Math.min(1, Math.max(0, sweep));
    const from = -TAU / 4 + spin * TAU;
    this.ringBand(cx, cy, radius, width, from, from + s * TAU);
  }
  ringBand(cx, cy, radius, width, a0, a1) {
    const steps = Math.max(24, Math.ceil(radius * TAU * 2));
    for (let i = 0; i <= steps; i++) {
      const a = a0 + ((a1 - a0) * i) / steps;
      for (let r = radius - width / 2; r <= radius + width / 2; r += 0.5) {
        this.set(cx + Math.cos(a) * r, cy + Math.sin(a) * r);
      }
    }
  }
}

// One routine covers an eye, an iris, a flat line and a waveform.
function cell(g, cx, cy, w, h, round, amp, freq, phase) {
  if (w <= 0 || h <= 0) return;
  const r = Math.min(1, Math.max(0, round)) * Math.min(w, h);
  const flat = Math.max(0, w - r);
  const steps = Math.max(1, Math.ceil(w));
  for (let dx = -steps; dx <= steps; dx++) {
    const x = dx;
    if (Math.abs(x) > w) continue;
    let half;
    if (Math.abs(x) <= flat) half = h;
    else {
      const over = (Math.abs(x) - flat) / Math.max(r, 0.001);
      half = h * Math.sqrt(Math.max(0, 1 - over * over));
    }
    // The travelling term of a wave lives here, via phase.
    const dy = amp > 0 ? amp * Math.sin(TAU * (freq * (x / (2 * w)) - phase)) : 0;
    g.span(cx + x, cy + dy - half, cy + dy + half);
  }
}

/// Draw a pose into a fresh grid. `phase` is 0..1 through the idle loop; `t`
/// is absolute seconds (for the saccade, which must not repeat on the loop).
export function draw(pose, motion, phase, t, grid) {
  const g = grid ?? new Grid();
  g.dots.fill(0);
  const p = applyMotion(motion, pose, phase, t);
  const cx = g.w / 2 + p.x;
  const cy = g.h / 2 + p.y;

  g.useInk(p.ink);

  if (p.ring > 1 && p.ringW > 0.1) {
    g.ringAt(cx, cy, p.ring, p.ringW, p.sweep, p.spin);
  }
  if (motion === 'blades') {
    for (let k = 0; k < 3; k++) {
      const a = phase * TAU + (k * TAU) / 3;
      g.ringBand(cx, cy, Math.max(4, p.ring - 5), 2, a, a + 0.7);
    }
  }

  const centres =
    p.gap < 0.6
      ? [[cx, cy, 1]]
      : [
          [cx - p.gap, cy, 1],
          [cx + p.gap, cy + p.tilt, p.squint],
        ];

  for (const [ex, ey, hscale] of centres) {
    const h = Math.max(0.5, p.h * hscale);
    if (p.w > 0.05 && h > 0.05) {
      cell(g, ex, ey, p.w, h, p.round, p.amp, p.freq, phase);
      if (p.pupil > 0.3 && p.amp === 0) {
        g.clearEllipse(ex + p.gazeX, ey + p.gazeY, p.pupil, Math.max(0.6, p.pupil * hscale));
      }
    }
  }

  // Marks last and each in its own ink, so a white scan line can share the
  // stage with a red contact.
  p.mark.forEach((m, i) => {
    if (m.w > 0.05) {
      g.useInk(p.markInk[i] ?? 'plain');
      g.stroke(cx + m.x0, cy + m.y0, cx + m.x1, cy + m.y1, m.w);
    }
  });
  return g;
}
