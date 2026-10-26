// The third animation layer, owned in one place.
//
// `Face.svelte` renders and holds the transition between two expressions.
// This holds everything that happens *on top* of that — the idle clock and
// the dance — because both of them need to be interrupted by things
// Face.svelte cannot see: a keystroke in `App.svelte`, an engine event in
// `state.svelte.js`, the dance slot changing under `src-tauri`. The one it
// can see, a change of face, it reports itself.
//
// The stacking order, and it matters:
//
//   1. the transition        (Face.svelte)
//   2. a beat, or the dance  a deviation with its own timeline
//   3. a blink               only when nothing else owns the eye
//
// Dance replaces 2 outright rather than layering onto it: an idle beat
// quietly scaling a choreographed hit would put it off the beat.

import { Idle, tierOf } from './idle.js';
import { Dance } from './dance.js';
import { blink, blinkable, ease, lerp } from './face.js';

/// Only decorrelates this face from the terminal one running beside it.
const seed = () => Math.floor(Math.random() * 0xffffffff);

const idle = new Idle(seed());

/// Seconds, on the same clock `performance.now()` reports. Everything in the
/// beat and dance players takes seconds rather than timestamps.
const now = () => performance.now() / 1000;

export const motion = $state({
  /// What the dance camera should be doing, or null when parker is not
  /// dancing. The *only* thing that crosses from the choreographer to the
  /// camera — parkerd names the section, visage decides what it looks like.
  mood: null,
  /// Beats since this dance started, for the camera to lock onto.
  beats: 0,
  dancing: false,
  tier: 'settled',
});

/// When anything last happened to parker. Not the user's idle time: `sense`
/// cannot see that and this does not need it (idle.js).
let quietSince = now();
let dancer = null;

/// What `apply` last drew, before the blink, and whether a beat was in it.
/// A stir reads these to leave from where the face really was.
let drawn = null;
let drawingBeat = false;

/// A beat cut short by a stir, easing back into the resting face:
/// `{ from, at }`, or null. The same span and easing as a derived change of
/// face (Face.svelte's TRANSITION_MS, `ease.inOut`), because that is what it
/// is — the face going back to idle, just from a beat rather than another
/// expression.
let fade = null;
const FADE = 0.55;

/// Where the face is once an outgoing fade is applied to `pose`. Ends the
/// fade when it has run its span.
function faded(pose, t) {
  if (!fade) return pose;
  const k = (t - fade.at) / FADE;
  if (k >= 1) {
    fade = null;
    return pose;
  }
  return lerp(fade.from, pose, ease.inOut(k));
}

/// Restart the settling period and drop the beat — the part of a stir that
/// every path shares.
function settle() {
  quietSince = now();
  idle.interrupt(quietSince);
}

/// Something happened to parker: a keystroke, an engine event. Drops
/// whatever beat was playing and restarts the settling period — and if a
/// beat was on screen, eases out of it rather than cutting. A vignette is
/// minutes of the face somewhere else entirely (six bars, a bonsai), and a
/// keypress used to snap it back to idle in one frame.
///
/// A change of face does not come through here: it has a transition of its
/// own to leave from, and `depart` hands it the beat instead.
export function stir() {
  if (drawingBeat && drawn) {
    fade = { from: drawn, at: now() };
    drawingBeat = false;
  }
  settle();
}

/// The face is changing expression: where is it right now? `pose` is where
/// the transition has it; this adds the beat, if one is being drawn, and then
/// stirs. The order is the point — the beat is read before it is dropped, so
/// a pyramid or a sonar ring morphs into the next face instead of vanishing
/// a frame before it. Same order as the expression change in animate_face
/// (main.rs).
///
/// `restful` is what `apply` was last given: a beat that was not being drawn
/// is not part of where the face is. The dance is left out because it keeps
/// playing over the next expression, and departing from it would count it
/// twice.
///
/// A fade still running from an earlier stir is part of where it is too, so
/// it is folded in and ended here: the transition takes over from it.
export function depart(pose, restful) {
  const t = now();
  const here = faded(restful && !dancer ? idle.pose(pose, t) : pose, t);
  fade = null;
  drawingBeat = false;
  settle();
  return here;
}

/// The last value seen in the dance slot, so a poll that reports the same
/// thing twice is a no-op.
///
/// This has to key on the *slot string* rather than on the resolved routine.
/// `auto` resolves through a seed, so comparing resolved names would re-roll
/// four times a second and restart the dance on every poll that happened to
/// pick a different routine — a dance that never got past its first bar.
let wanted = null;

/// Start or stop dancing. `name` is a routine, `auto` to let parker pick, or
/// null to stop. Driven by the runtime slot, which `parker reach dance`
/// owns — this never decides on its own that parker should dance.
export function setDance(name, track = null) {
  if (name === wanted) return;
  wanted = name;

  if (!name) {
    dancer = null;
    motion.dancing = false;
    motion.mood = null;
    motion.beats = 0;
    // Do not walk out of a dance straight into a nap.
    quietSince = now();
    return;
  }

  const routine =
    name === 'auto' ? Dance.routineFor(track, seed()) : Dance.byName(name);
  idle.interrupt(now());
  dancer = new Dance(routine, 120, now(), seed());
  motion.dancing = true;
}

export function isDancing() {
  return dancer !== null;
}

/// The last beat cue seen, keyed on the raw slot line — the deadline differs
/// per write, so the same beat cued twice running still plays twice.
let beatSeen = null;

/// A cued beat (`parker face --play`, the beat slot). It is handed to the
/// idle clock to play at the next rest (`Idle.cue`): a cue sent while parker
/// is mid-turn, or wearing a face, used to be dropped without a word. The
/// dance owns the face while it is on, so a cue during one is still dropped
/// rather than queued behind it.
export function playBeat(raw) {
  if (raw === beatSeen) return;
  beatSeen = raw;
  if (!raw || dancer) return;
  const name = raw.split(/\s+/)[0];
  idle.cue(name, now());
}

/// Apply the third layer to whatever the transition produced.
///
/// `restful` is whether parker is idling — flair only ever plays over the
/// resting face. Glancing about while he is thinking, or while he is waiting
/// on an answer from you, would make the involuntary track a liar.
///
/// `blinks` is false while a face lands with an entrance (it steers the eye)
/// and on the faces that must not blink at all (Expression::blinks).
export function apply(pose, motionName, restful, blinks = true) {
  const t = now();

  if (dancer) {
    fade = null;
    drawingBeat = false;
    motion.mood = dancer.mood(t);
    motion.beats = dancer.clock.beats(t);
    return { pose: dancer.pose(pose, t), motion: dancer.motion };
  }

  let p = pose;
  let m = motionName;

  if (restful) {
    const quiet = t - quietSince;
    motion.tier = tierOf(quiet);
    idle.tick(t, quiet);
    p = idle.pose(p, t);
    m = idle.motion(m, t);
  }

  p = faded(p, t);
  drawn = p;
  drawingBeat = restful && idle.player.isPlaying;

  if (blinks && idle.blinkable(t) && blinkable(p)) {
    p = { ...p, h: p.h * blink(t) };
  }
  return { pose: p, motion: m };
}
