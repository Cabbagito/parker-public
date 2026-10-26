// What parker does when nobody is talking to him.
//
// Ported from parkerd/src/face/idle.rs — the clock and the selection rules.
// The beats themselves are data out of `catalog.json` and are not transcribed
// here (MOTION.md §3).
//
// The pacing rules, because they are the whole feature and they are easy to
// break by accident:
//
//   1. nothing at all for the first threequarters of a minute — flair right
//      after you stop typing reads as a reaction to you, which is worse than
//      stillness;
//   2. every interval jittered, never metronomic;
//   3. never the same beat twice running;
//   4. escalation — two phases: short beats while the quiet is short,
//      vignettes once it is long;
//   5. instantly interruptible.
//
// ## What the clock measures
//
// Time since anything happened to *parker*, which in visage means the last
// keystroke, the last engine event or the last change of face — not the
// user's idle time, which `sense` cannot see and this does not need.
//
// ## No sleep state
//
// There was a sleep state and it is gone on purpose (August 2026): parker is
// an AI that is literally on, and a face that pretends otherwise is a worse
// wallpaper. Deep idle is the vignette phase now — parker doing his own
// thing, longer and rarer — with the short beats still sprinkled between.
// `nap` is one of those things, finished on his own, not the state back.
//
// ## The vignettes
//
// The long pieces come out of `vignettes.json` (`parker face --vignettes
// --json`), apart from the catalog because they are most of a megabyte. They
// load after first paint and join the pool when they land: nothing can pick
// one sooner than 45 s in anyway.

import catalog from './catalog.json' with { type: 'json' };
import { Player, beatSecs } from './beat.js';
import { hash01 } from './face.js';

export const TIERS = ['settled', 'alert', 'vignette'];

const BOUNDS = catalog.idle.tiers;
const INTERVAL = Object.fromEntries(
  BOUNDS.map((b) => [b.tier.name, b.tier.interval]),
);

export function tierOf(idleSecs) {
  let name = 'settled';
  for (const b of BOUNDS) if (idleSecs >= b.at) name = b.tier.name;
  return name;
}

const depth = (tier) => TIERS.indexOf(tier);

/// In the vignette phase, how often the long pieces lead (idle.rs).
const DEEP_SHARE = catalog.idle.deepShare;

/// How much quiet a cued beat waits for, and how long it waits at most
/// (idle.rs `CUE_SETTLE`, `CUE_WAIT`).
const CUE_SETTLE = catalog.idle.cueSettle;
const CUE_WAIT = catalog.idle.cueWait;

/// The short beats now, and the vignettes behind them once they land — in
/// the same order as idle.rs's `catalog()`, tables first.
let CATALOG = catalog.idle.beats;
import('./vignettes.json', { with: { type: 'json' } })
  .then((m) => {
    CATALOG = catalog.idle.beats.concat(m.default.beats);
  })
  .catch(() => {
    // Without them parker still idles, on the short beats alone.
  });

export class Idle {
  /// `seed` only decorrelates this face from the terminal one; nothing
  /// depends on it being unpredictable.
  constructor(seed = 0) {
    this.player = new Player();
    this.nextAt = 0;
    this.last = '';
    this.seed = seed;
    this.step = 0;
    /// A beat asked for by name, and when: `{ name, at }`, played at the
    /// next rest.
    this.cued = null;
  }

  get playing() {
    return this.player.playing?.name ?? null;
  }

  roll() {
    this.step += 1;
    return hash01(this.step, this.seed ^ 0x1d1e);
  }

  jitter(tier) {
    const range = INTERVAL[tier];
    if (!range) return 45;
    const [lo, hi] = range;
    return lo + hash01(this.step, this.seed ^ 0x9a17) * (hi - lo);
  }

  /// Pick from everything eligible at this tier, never the same one twice
  /// running. Two glances in a row read as a bug, not as a character. In the
  /// vignette phase one roll first decides the pool — the long pieces
  /// `DEEP_SHARE` of the time, else the short ones — and a second picks.
  choose(tier) {
    const deep = tier === 'vignette' && this.roll() < DEEP_SHARE;
    const fits = (b) =>
      tier === 'vignette' ? (b.tier === 'vignette') === deep : depth(b.tier) <= depth(tier);
    let pool = CATALOG.filter((b) => fits(b) && b.name !== this.last);
    if (!pool.length) pool = CATALOG.filter(fits);
    if (!pool.length) return null;
    return pool[Math.min(pool.length - 1, Math.floor(this.roll() * pool.length))];
  }

  /// Advance. `idleSecs` is time since anything happened to parker.
  tick(now, idleSecs) {
    this.player.tick(now);
    if (this.cued) {
      const { name, at } = this.cued;
      if (now - at > CUE_WAIT) {
        this.cued = null; // asked for too long ago
      } else if (idleSecs >= CUE_SETTLE) {
        this.cued = null;
        // A request, so it plays over whatever the clock had rolled.
        if (this.playNamed(name, now)) return;
      }
    }
    if (this.player.isPlaying) return;

    const tier = tierOf(idleSecs);
    if (tier === 'settled') {
      // Keep the clock pushed out ahead of the settling period, so the first
      // beat after a conversation is not instant.
      this.nextAt = now + this.jitter('alert');
      return;
    }
    if (now < this.nextAt) return;

    const beat = this.choose(tier);
    if (!beat) {
      this.nextAt = now + 5;
      return;
    }
    this.last = beat.name;
    this.player.play(beat, now);
    this.nextAt = now + beatSecs(beat) + this.jitter(tier);
  }

  /// Something happened. Drop whatever is playing and restart the settling
  /// period. Nothing to wake from anymore: sleep went with the drowsy tier.
  interrupt(now) {
    this.player.stop();
    this.nextAt = now + 45;
  }

  /// Ask for a beat by name, to play at the next rest — the beat slot's
  /// landing point (`parker face --play`). Only `tick` plays it, and the face
  /// ticks only while restful, so a cue sent mid-turn or under a worn face
  /// waits for parker to settle instead of being dropped. An interrupt does
  /// not clear it; a newer cue replaces it. Port of `Idle::cue` in idle.rs.
  cue(name, now) {
    this.cued = { name, at: now };
  }

  /// Play a beat by name, right now — where a cue lands once it is due. A
  /// request, not a choice: it does not touch the idle clock, so the pool's
  /// own next beat still arrives when it was always going to. Port of
  /// `Idle::play_named` in idle.rs.
  playNamed(name, now) {
    const beat = CATALOG.find((b) => b.name === name);
    if (!beat) return false;
    this.last = beat.name;
    this.player.play(beat, now);
    return true;
  }

  pose(base, now) {
    return this.player.pose(base, now);
  }

  motion(base, now) {
    return this.player.motion(base, now);
  }

  /// Whether an ordinary blink may be drawn on top: always at rest, never
  /// while a beat is steering the eye (the two would fight over `h`), and in
  /// the stretches of a vignette that let it through.
  blinkable(now) {
    return !this.player.isPlaying || this.player.blinks(now);
  }
}
