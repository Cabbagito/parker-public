// The beat player, ported from parkerd/src/face/beat.rs.
//
// A beat is something parker does once and then stops doing — a glance, a
// yawn, a headbang. Neither of the other two layers can express that: a pose
// is a place you go and stay, a motion is periodic and eternal.
//
// What is ported here is the *player*, about a hundred lines of walking
// keyframes. The keyframes themselves are not ported: they arrive as data in
// `catalog.json`, written out by `parker face --catalog --json`. That split is
// deliberate and it is what makes the whole idea survive (MOTION.md §3) — a
// hundred hand-copied numbers drift silently, whereas a player that is wrong
// is wrong on the first frame in a way you cannot miss.
//
// A delta is measured from the *resting pose*, never from the keyframe before
// it. Same rule as the Rust, same reason: a keyframe you can read on its own.

import { lerp, padMarks, padInks, EASINGS, MARKS } from './face.js';

/// Apply one exported delta to a base pose.
///
/// `num` entries arrive as `[field, op, value]` triples, with the field names
/// spelled the way this file reads them — `Field::name()` in pose.rs emits
/// the JavaScript spellings for exactly this lookup.
export function applyDelta(delta, base) {
  const p = { ...base, mark: base.mark };
  for (const [field, op, value] of delta.num ?? []) {
    const b = base[field] ?? 0;
    p[field] = op === 'set' ? value : op === 'mul' ? b * value : b + value;
  }
  if (delta.mark) {
    p.mark = padMarks(
      delta.mark.map(([x0, y0, x1, y1, w]) => ({ x0, y0, x1, y1, w })),
    );
  }
  if (delta.ink) p.ink = delta.ink;
  // The catalog exports the padded six, but pad anyway: the rule lives in
  // one place on each side, not in the wire format.
  if (delta.markInk) p.markInk = padInks(delta.markInk);
  return p;
}

function framesSecs(beat) {
  return beat.frames.reduce((a, f) => a + f.secs, 0);
}

/// Does this beat ever end on its own? Sleep is the only one that does not.
export function terminal(beat) {
  return beat.loopFrom !== null && beat.loopFrom !== undefined;
}

/// How long the whole thing takes, return trip included.
export function beatSecs(beat) {
  return framesSecs(beat) + (terminal(beat) ? 0 : beat.retSecs);
}

/// Fold time past the end of an endless beat back into its loop.
function wrap(beat, elapsed) {
  if (!terminal(beat)) return elapsed;
  const from = beat.loopFrom;
  const head = beat.frames.slice(0, from).reduce((a, f) => a + f.secs, 0);
  const tail = framesSecs(beat) - head;
  if (tail <= 0 || elapsed < head + tail) return elapsed;
  return head + ((elapsed - head) % tail);
}

/// Which keyframe is playing `elapsed` seconds in, and when it began; null
/// once the frames have run out (`Beat::frame_at`). A scan of durations, not
/// of poses: a vignette runs to a thousand keyframes.
export function frameAt(beat, elapsed) {
  elapsed = wrap(beat, Math.max(0, elapsed));
  let acc = 0;
  for (let i = 0; i < beat.frames.length; i++) {
    const kf = beat.frames[i];
    if (kf.secs > 0 && elapsed < acc + kf.secs) return { i, began: acc };
    acc += kf.secs;
  }
  return null;
}

/// The pose `elapsed` seconds into `beat`, departing from and returning to
/// `base`. The return trip is added by the player, so no catalog entry has to
/// remember to end where it started — a beat cannot strand the face.
///
/// Deltas are measured from rest, never stacked, so the pose between two
/// keyframes needs those two and nothing before them.
///
/// `origin` is where the first keyframe leaves from — `base` for a beat over
/// its own resting face, the face being taken off for an entrance, which
/// lands in `base` (`Beat::departing`, entrance.rs).
export function beatAt(beat, base, elapsed, origin = base) {
  const from = (i) => (i === 0 ? origin : applyDelta(beat.frames[i - 1].to, base));
  const at = frameAt(beat, elapsed);
  if (at) {
    const kf = beat.frames[at.i];
    const t = (wrap(beat, Math.max(0, elapsed)) - at.began) / kf.secs;
    return lerp(from(at.i), applyDelta(kf.to, base), (EASINGS[kf.ease] ?? EASINGS.inOut)(t));
  }

  const last = from(beat.frames.length);
  if (terminal(beat)) return last;
  const acc = framesSecs(beat);
  elapsed = Math.max(0, elapsed);
  if (beat.retSecs > 0 && elapsed < acc + beat.retSecs) {
    return lerp(last, base, EASINGS.inOut((elapsed - acc) / beat.retSecs));
  }
  return base;
}

/// The motion `elapsed` seconds in: the playing keyframe's (a vignette's
/// section), else the beat's, else whatever the face was already doing.
export function motionAt(beat, base, elapsed) {
  const at = frameAt(beat, elapsed);
  return (at && beat.frames[at.i].motion) ?? beat.motion ?? base;
}

/// Whether the ordinary blink may be drawn `elapsed` seconds in — only in the
/// stretches of a vignette that let it through, never on the trip home.
export function blinksAt(beat, elapsed) {
  const at = frameAt(beat, elapsed);
  return Boolean(at && beat.frames[at.i].blink);
}

/// Plays at most one beat at a time, over whatever the face is already doing.
/// Time arrives in seconds (from `performance.now() / 1000`), same as the
/// Rust takes seconds rather than an Instant.
export class Player {
  constructor() {
    this.beat = null;
    this.started = 0;
  }

  play(beat, now) {
    this.beat = beat;
    this.started = now;
  }

  /// Blunt on purpose: a beat is interrupted because something real happened,
  /// and Stage/Face depart from wherever the face actually is, so the cut is
  /// invisible.
  stop() {
    this.beat = null;
  }

  get playing() {
    return this.beat;
  }

  get isPlaying() {
    return this.beat !== null;
  }

  finished(now) {
    if (!this.beat) return true;
    return !terminal(this.beat) && now - this.started >= beatSecs(this.beat);
  }

  tick(now) {
    if (this.finished(now)) this.beat = null;
  }

  elapsed(now) {
    return now - this.started;
  }

  pose(base, now) {
    return this.beat ? beatAt(this.beat, base, now - this.started) : base;
  }

  motion(base, now) {
    return this.beat ? motionAt(this.beat, base, now - this.started) : base;
  }

  /// Whether the playing beat lets the ordinary blink through right now.
  blinks(now) {
    return this.beat ? blinksAt(this.beat, now - this.started) : false;
  }
}

export { MARKS };
