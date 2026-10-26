// parker dances — the pose half.
//
// Ported from parkerd/src/face/dance.rs: the metronome, the drives, and the
// choreographer. Routines and moves are data out of `catalog.json`.
//
// The camera half is *not* here and must not migrate here. parkerd names what
// a section is — intro, build, drop, outro — and visage decides what a drop
// looks like on this screen (lens.js). That is the pose/placement rule, and
// dance is the first thing that genuinely needs both halves at once.
//
// Quantization is the whole trick: every parameter is one the idle face
// already uses, and what makes it read as dancing rather than drifting is
// that the values step.

import catalog from './catalog.json' with { type: 'json' };
import { hash01 } from './face.js';

export const ROUTINES = catalog.dance.routines;
const SURPRISES = catalog.dance.surprises;

/// A fixed tempo, and the whole of v0.
///
/// Not a placeholder: dancing to a steady 120 is most of the effect and needs
/// no audio at all. Media gating and real onset detection are new
/// implementations of this same little interface, not a rewrite.
export class Metronome {
  constructor(bpm, started) {
    this.bpm = Math.min(220, Math.max(40, bpm));
    this.started = started;
  }

  beats(now) {
    return Math.max(0, ((now - this.started) * this.bpm) / 60);
  }

  bars(now) {
    return this.beats(now) / 4;
  }

  /// A synthesized envelope: kick on the odd beats, snare on the even ones,
  /// eighth-note flutter up top. A drum machine, not an analysis — it exists
  /// so every move can be written against `energy` today and stay correct
  /// when the source is real audio.
  energy(band, now) {
    const b = this.beats(now);
    const into = b - Math.floor(b);
    const idx = ((Math.floor(b) % 4) + 4) % 4;
    const decay = (1 - into) ** 2;
    if (band === 'low') return idx % 2 === 0 ? decay : decay * 0.25;
    if (band === 'mid') return idx % 2 === 1 ? decay : decay * 0.2;
    const e = b * 2 - Math.floor(b * 2);
    return (1 - e) ** 1.5 * 0.8;
  }
}

/// A drive's value inside one subdivision. Unipolar drives run 0..1, bipolar
/// ones -1..1; `amp` scales whichever it gets.
function driveAt(move, beats, clock, now) {
  const div = move.div > 0 ? move.div : 1;
  const u = beats / div;
  const into = u - Math.floor(u);
  const step = Math.floor(u);
  const even = ((step % 2) + 2) % 2 === 0;
  switch (move.drive) {
    // Cubic decay: an attack you can see land, not a fade.
    case 'hit': return (1 - into) ** 3;
    case 'pulse': return Math.sin(into * Math.PI);
    case 'sine': return Math.sin(into * Math.PI * 2);
    case 'square': return even ? 1 : 0;
    case 'alt': return even ? 1 : -1;
    case 'ramp': return into;
    case 'energy': return clock.energy(move.band, now);
    default: return 0;
  }
}

/// Keep a routine from drawing something impossible. Moves compose freely and
/// a `mul` on top of a `set` can land anywhere; a negative width draws
/// nothing, and a face that vanishes mid-drop looks exactly like a crash.
function sane(p) {
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  p.w = clamp(p.w, 0.2, 46);
  p.h = clamp(p.h, 0.2, 19);
  p.gap = clamp(p.gap, 0, 24);
  p.round = clamp(p.round, 0, 1);
  p.pupil = Math.max(0, p.pupil);
  p.ring = clamp(p.ring, 0, 24);
  p.ringW = Math.max(0, p.ringW);
  p.sweep = clamp(p.sweep, 0, 1);
  p.squint = clamp(p.squint, 0, 2);
  p.amp = clamp(p.amp, 0, 14);
  p.freq = clamp(p.freq, 0, 8);
  p.x = clamp(p.x, -30, 30);
  p.y = clamp(p.y, -30, 30);
  return p;
}

export class Dance {
  constructor(routine, bpm, started, seed = 0) {
    this.routine = routine;
    this.clock = new Metronome(bpm, started);
    this.seed = seed;
  }

  /// Pick a routine, seeded by the track when there is one — so the same song
  /// dances the same way twice, which reads as parker knowing it.
  static routineFor(track, seed = 0) {
    let key = seed;
    if (track && track.trim()) {
      key = 0;
      for (const ch of track) key = (key * 31 + ch.charCodeAt(0)) >>> 0;
    }
    return ROUTINES[Math.abs(key) % ROUTINES.length];
  }

  static byName(name) {
    return ROUTINES.find((r) => r.name === name) ?? ROUTINES[0];
  }

  get name() {
    return this.routine.name;
  }

  barInRoutine(now) {
    return this.clock.bars(now) % this.routine.bars;
  }

  /// Which bar of the whole performance — the surprise slot must not repeat
  /// every time the routine loops.
  absoluteBar(now) {
    return Math.floor(this.clock.bars(now));
  }

  /// One bar in sixteen, ignore the routine and do something stupid. What
  /// makes choreography feel authored rather than generated is that it is
  /// occasionally wrong.
  surprise(now) {
    const bar = this.absoluteBar(now);
    // Never in the opening bars: the routine should establish itself before
    // it starts lying.
    if (bar < 2) return null;
    const r = hash01(bar, this.seed ^ 0x5111);
    if (r >= 0.0625) return null;
    return SURPRISES[Math.floor(r * 1000) % SURPRISES.length];
  }

  section(now) {
    const bar = this.barInRoutine(now);
    return (
      this.routine.sections.find((s) => bar >= s.from && bar < s.to) ??
      this.routine.sections[0]
    );
  }

  /// What this instant is, musically. The camera reads it — and nothing else
  /// crosses from parkerd about what a dance should look like.
  mood(now) {
    // A surprise is its own thing; the camera should not be mid-drop for it.
    if (this.surprise(now)) return 'intro';
    return this.section(now).mood;
  }

  /// Full send: the ink snaps to a new slot every bar during a drop.
  /// Snapping is not a compromise — ink is deliberately never interpolated
  /// ("a colour crossfade at 1-bit is mud"), and a strobe is exactly a colour
  /// that does not interpolate.
  ink(now) {
    if (this.section(now).mood !== 'drop') return 'plain';
    const b = ((this.absoluteBar(now) % 4) + 4) % 4;
    return ['accent', 'good', 'bad', 'plain'][b];
  }

  /// Dance drives every field it cares about explicitly; a resting motion
  /// underneath would add a saccade to a face that is hitting marks.
  get motion() {
    return 'none';
  }

  pose(base, now) {
    const beats = this.clock.beats(now);
    const moves = this.surprise(now) ?? this.section(now).moves;
    const p = { ...base, mark: base.mark };
    for (const m of moves) {
      const v = driveAt(m, beats, this.clock, now);
      const b = base[m.field] ?? 0;
      p[m.field] =
        m.blend === 'add' ? b + v * m.amp
        : m.blend === 'mul' ? b * (1 + v * (m.amp - 1))
        : v * m.amp;
    }
    p.ink = this.ink(now);
    return sane(p);
  }
}
