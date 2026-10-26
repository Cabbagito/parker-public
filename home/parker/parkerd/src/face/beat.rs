// A beat: something parker does once, and then stops doing.
//
// The face had two layers and neither could express this. A *pose* is a place
// you go and stay; a *motion* is periodic and eternal. Everything parker does
// that has a beginning, a middle and an end — a glance, a yawn, a headbang —
// is a third thing, and this is it (MOTION.md §0).
//
// A beat is a list of keyframes. A keyframe is a partial pose, a duration and
// an easing. The player walks them with the same `Pose::lerp` that carries one
// expression into another, so a beat is not a separate animation system
// bolted alongside the real one — it is the real one, driven from data.
//
// Three properties, all of which fall out of that and all of which matter:
//
//   composable   keyframes are deltas against whatever parker is resting in,
//                so a glance is a glance whether he is idling or curious.
//   returnable   the player appends the trip home. A beat cannot strand the
//                face somewhere, which is the entire class of bug that
//                hand-written one-shot animations are made of.
//   portable     it is data. It survives being written out as JSON and read
//                back by visage, which is what keeps one catalog rather than
//                two (MOTION.md §3).
//
// What a beat is *not* is an expression. Expressions are the two-track model —
// what parker is doing, and what parker chose to show. A beat is neither: it
// is idling visibly, and if a glance could be mistaken for `Curious` the
// involuntary track would be lying. Beats have no name anyone types, cannot be
// worn, and never appear in `Expression`.

use super::pose::{Delta, Easing, Motion, Pose};

/// One waypoint: where to be, how long to take getting there, and how.
///
/// **A delta is measured from the resting pose, never from the keyframe
/// before it.** So a keyframe states the whole deviation it wants, and
/// reading one tells you where the face is without reading the four above it.
/// The alternative — each delta stacking on the last — reads shorter and
/// costs you the ability to understand any single line in isolation, makes
/// `Mul` compound in ways nobody predicts, and turns a reordered catalog
/// entry into a silent change rather than an obvious one.
///
/// The price is restatement: a beat that closes parker's eyes and *then*
/// moves something must keep saying the eyes are closed. That is the right
/// trade at this size, and forgetting it is loud rather than subtle — the
/// eyes spring open mid-beat.
#[derive(Clone, Copy, Debug)]
pub struct Keyframe {
    pub to: Delta,
    pub secs: f64,
    pub ease: Easing,
    /// The motion while this keyframe plays, over the beat's own. A beat a
    /// few seconds long wants one motion; a vignette minutes long wants one
    /// per section — breathing while parker naps, still while he sweeps
    /// (vignette.rs). `None` defers to the beat.
    pub motion: Option<Motion>,
    /// Let the ordinary blink through while this keyframe plays. Off for
    /// every short beat (they steer the eye and a blink would fight them over
    /// `h`); on for the long stretches of a vignette, where two minutes of
    /// an eye that never blinks reads as a frozen screen.
    pub blink: bool,
}

impl Keyframe {
    pub const fn new(to: Delta, secs: f64, ease: Easing) -> Keyframe {
        Keyframe {
            to,
            secs,
            ease,
            motion: None,
            blink: false,
        }
    }

    /// Stay where the previous keyframe left off. A hold is the difference
    /// between a glance and a twitch, and it is most of what makes a beat
    /// legible — the eye needs time on the target to register that it looked.
    pub const fn hold(to: Delta, secs: f64) -> Keyframe {
        Keyframe::new(to, secs, Easing::Linear)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Beat {
    pub name: &'static str,
    pub frames: &'static [Keyframe],

    /// How long the trip back to rest takes. The player adds it; no catalog
    /// entry has to remember to end where it started.
    pub ret_secs: f64,

    /// Where to jump back to when the frames run out, instead of returning.
    /// `Some(i)` makes the beat endless: frames before `i` are the way in,
    /// frames from `i` on are the loop. Sleep is the only thing that wants
    /// this — you fall asleep once and then you are asleep.
    pub loop_from: Option<usize>,

    /// Replace the underlying motion for the beat's duration.
    ///
    /// Usually `Some(Motion::None)`, and the reason is not stylistic: the
    /// resting motion is `Drift`, which *adds* a saccade to `gaze_x` after
    /// the beat has set it. A glance that sets gaze to 4.5 while the saccade
    /// adds another 3.4 lands off the eye. Any beat that steers the gaze must
    /// take the wheel.
    pub motion: Option<Motion>,
}

impl Beat {
    /// Does this beat ever end on its own?
    pub fn terminal(&self) -> bool {
        self.loop_from.is_some()
    }

    fn frames_secs(&self) -> f64 {
        self.frames.iter().map(|f| f.secs).sum()
    }

    /// How long the whole thing takes, return trip included. Endless beats
    /// report their way-in only; asking how long sleep lasts is the wrong
    /// question.
    pub fn secs(&self) -> f64 {
        self.frames_secs() + if self.terminal() { 0.0 } else { self.ret_secs }
    }

    /// Which keyframe is playing `elapsed` seconds in, and when it began.
    /// `None` once the frames have run out (the trip home, or rest).
    ///
    /// A scan of durations, not of poses: a vignette runs to a thousand
    /// keyframes, and applying every delta up to the current one on every
    /// frame is work the answer never needed.
    pub fn frame_at(&self, elapsed: f64) -> Option<(usize, f64)> {
        let elapsed = self.wrap(elapsed.max(0.0));
        let mut acc = 0.0;
        for (i, kf) in self.frames.iter().enumerate() {
            if kf.secs > 0.0 && elapsed < acc + kf.secs {
                return Some((i, acc));
            }
            acc += kf.secs;
        }
        None
    }

    /// The pose `elapsed` seconds in, departing from and returning to `base`.
    ///
    /// Deltas are measured from rest, never stacked (see `Keyframe`), so the
    /// pose between two keyframes needs those two and nothing before them.
    pub fn at(&self, base: Pose, elapsed: f64) -> Pose {
        self.departing(base, base, elapsed)
    }

    /// `at`, leaving from `origin` instead of from `base`: the first keyframe
    /// starts wherever the face was. That is an entrance (entrance.rs) — it
    /// departs from the face being taken off and lands in the one being put
    /// on — and for a beat over its own base the two are the same thing.
    pub fn departing(&self, origin: Pose, base: Pose, elapsed: f64) -> Pose {
        let from = |i: usize| match i {
            0 => origin,
            _ => self.frames[i - 1].to.apply(base),
        };
        if let Some((i, began)) = self.frame_at(elapsed) {
            let kf = &self.frames[i];
            let t = (self.wrap(elapsed.max(0.0)) - began) / kf.secs;
            return Pose::lerp(from(i), kf.to.apply(base), kf.ease.at(t));
        }

        let last = from(self.frames.len());
        if self.terminal() {
            // `wrap` guarantees we never get here on a looping beat, but a
            // catalog entry with a loop point past its own frames would.
            return last;
        }
        let acc = self.frames_secs();
        let elapsed = elapsed.max(0.0);
        if self.ret_secs > 0.0 && elapsed < acc + self.ret_secs {
            let t = (elapsed - acc) / self.ret_secs;
            return Pose::lerp(last, base, Easing::InOut.at(t));
        }
        base
    }

    /// The motion `elapsed` seconds in: the playing keyframe's, else the
    /// beat's, else whatever the face was already doing.
    pub fn motion_at(&self, base: Motion, elapsed: f64) -> Motion {
        self.frame_at(elapsed)
            .and_then(|(i, _)| self.frames[i].motion)
            .or(self.motion)
            .unwrap_or(base)
    }

    /// Whether the ordinary blink may be drawn `elapsed` seconds in. Never on
    /// the trip home: that is still the beat's move, not rest.
    pub fn blinks_at(&self, elapsed: f64) -> bool {
        self.frame_at(elapsed)
            .is_some_and(|(i, _)| self.frames[i].blink)
    }

    /// Fold time past the end of an endless beat back into its loop.
    fn wrap(&self, elapsed: f64) -> f64 {
        let Some(from) = self.loop_from else {
            return elapsed;
        };
        let head: f64 = self.frames.iter().take(from).map(|f| f.secs).sum();
        let tail: f64 = self.frames_secs() - head;
        if tail <= 0.0 || elapsed < head + tail {
            return elapsed;
        }
        head + (elapsed - head) % tail
    }
}

/// Plays at most one beat at a time, over whatever the face is already doing.
///
/// Time arrives as seconds rather than as an `Instant` for two reasons: it is
/// testable without sleeping, and it is the same shape the visage port reads
/// off `performance.now()`. Nothing here needs a clock of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Player {
    beat: Option<&'static Beat>,
    started: f64,
}

impl Player {
    pub fn new() -> Player {
        Player::default()
    }

    pub fn play(&mut self, beat: &'static Beat, now: f64) {
        self.beat = Some(beat);
        self.started = now;
    }

    /// Drop the beat immediately.
    ///
    /// Blunt on purpose. A beat is interrupted because something real
    /// happened, and the caller is about to transition away from the pose
    /// anyway — `animate_face` departs from wherever the face actually is, so
    /// the cut is invisible and easing out of a glance parker is no longer
    /// having would only delay the thing that interrupted it.
    pub fn stop(&mut self) {
        self.beat = None;
    }

    pub fn is_playing(&self) -> bool {
        self.beat.is_some()
    }

    /// Has the current beat run out? Endless ones never have.
    pub fn finished(&self, now: f64) -> bool {
        match self.beat {
            None => true,
            Some(b) => !b.terminal() && now - self.started >= b.secs(),
        }
    }

    /// Retire a finished beat, so `is_playing` means what it says.
    pub fn tick(&mut self, now: f64) {
        if self.finished(now) {
            self.beat = None;
        }
    }

    pub fn pose(&self, base: Pose, now: f64) -> Pose {
        match self.beat {
            None => base,
            Some(b) => b.at(base, now - self.started),
        }
    }

    pub fn motion(&self, base: Motion, now: f64) -> Motion {
        match self.beat {
            None => base,
            Some(b) => b.motion_at(base, now - self.started),
        }
    }

    /// Whether the playing beat lets the ordinary blink through right now.
    /// False with nothing playing: whether rest blinks is not the player's
    /// question (idle.rs asks it).
    pub fn blinks(&self, now: f64) -> bool {
        self.beat.is_some_and(|b| b.blinks_at(now - self.started))
    }
}

/// Inspection, for tests and for the visage port to mirror.
///
/// Split out and allowed dead rather than deleted: these are the shape the
/// JavaScript player has to expose, and a terminal face that happens not to
/// call them is not evidence they are unwanted. `--all-targets` does not
/// count test-only use for a binary crate, so the alternative is deleting
/// the parity surface to satisfy a lint.
#[allow(dead_code)]
impl Player {
    pub fn playing(&self) -> Option<&'static Beat> {
        self.beat
    }

    /// How far into the current beat. visage keys a camera flourish off it.
    pub fn elapsed(&self, now: f64) -> f64 {
        now - self.started
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::pose::{Field, Op};

    fn base() -> Pose {
        Pose {
            gap: 0.0,
            w: 9.0,
            h: 9.0,
            round: 1.0,
            pupil: 3.0,
            ring: 17.0,
            ring_w: 2.0,
            ..Pose::ZERO
        }
    }

    static LOOK: Beat = Beat {
        name: "look",
        frames: &[
            Keyframe::new(
                Delta::num(&[(Field::GazeX, Op::Set(4.5))]),
                0.1,
                Easing::Out,
            ),
            Keyframe::hold(Delta::num(&[(Field::GazeX, Op::Set(4.5))]), 0.5),
        ],
        ret_secs: 0.2,
        loop_from: None,
        motion: Some(Motion::None),
    };

    // Every keyframe restates `h`, because a delta is measured from the
    // resting pose and not from the keyframe before it. Drop it from the
    // last two and parker's eyes spring back open while he snores.
    static SNORE: Beat = Beat {
        name: "snore",
        frames: &[
            Keyframe::new(Delta::num(&[(Field::H, Op::Set(1.6))]), 0.4, Easing::InOut),
            Keyframe::new(
                Delta::num(&[(Field::H, Op::Set(1.6)), (Field::Y, Op::Set(1.0))]),
                0.5,
                Easing::InOut,
            ),
            Keyframe::new(
                Delta::num(&[(Field::H, Op::Set(1.6)), (Field::Y, Op::Set(-1.0))]),
                0.5,
                Easing::InOut,
            ),
        ],
        ret_secs: 0.0,
        loop_from: Some(1),
        motion: Some(Motion::Breathe),
    };

    #[test]
    fn a_beat_starts_and_ends_exactly_at_rest() {
        // The property the whole layer rests on: a beat cannot strand the
        // face somewhere. If this breaks, every beat is a potential deadlock.
        assert_eq!(LOOK.at(base(), 0.0), base());
        assert_eq!(LOOK.at(base(), LOOK.secs()), base());
        assert_eq!(LOOK.at(base(), LOOK.secs() + 100.0), base());
    }

    #[test]
    fn a_beat_actually_reaches_its_target() {
        // End of the flick, start of the hold.
        let held = LOOK.at(base(), 0.3);
        assert!(
            (held.gaze_x - 4.5).abs() < 1e-9,
            "gaze_x was {}",
            held.gaze_x
        );
    }

    #[test]
    fn a_beat_leaves_everything_it_did_not_name_alone() {
        let mid = LOOK.at(base(), 0.3);
        assert_eq!(mid.w, base().w);
        assert_eq!(mid.ring, base().ring);
        assert_eq!(mid.pupil, base().pupil);
    }

    #[test]
    fn the_return_trip_is_added_without_the_catalog_asking() {
        // Halfway home: past the target, not yet at rest.
        let home = LOOK.at(base(), 0.6 + 0.1);
        assert!(home.gaze_x < 4.5 && home.gaze_x > 0.0);
    }

    #[test]
    fn a_beat_composes_onto_whatever_is_underneath_it() {
        // Same beat, a different resting face. Only the named field moves,
        // and it moves to the same place.
        let curious = Pose {
            gap: 18.0,
            tilt: -3.0,
            squint: 0.65,
            ..base()
        };
        let mid = LOOK.at(curious, 0.3);
        assert!((mid.gaze_x - 4.5).abs() < 1e-9);
        assert_eq!(mid.tilt, -3.0, "the beat overwrote the face under it");
        assert_eq!(mid.gap, 18.0);
    }

    #[test]
    fn an_endless_beat_never_returns_and_never_finishes() {
        assert!(SNORE.terminal());
        let mut p = Player::new();
        p.play(&SNORE, 0.0);
        assert!(!p.finished(1000.0));
        // Still asleep, still with its lids down, an hour later.
        let late = SNORE.at(base(), 3600.0);
        assert!((late.h - 1.6).abs() < 1e-9, "h drifted to {}", late.h);
    }

    #[test]
    fn an_endless_beat_loops_only_the_tail() {
        // The way in (frame 0) plays once; frames 1..  repeat. If `wrap`
        // folded the whole thing, parker would keep re-closing his eyes.
        let tail = 0.5 + 0.5;
        let a = SNORE.at(base(), 0.4 + 0.25);
        let b = SNORE.at(base(), 0.4 + 0.25 + tail);
        assert!((a.y - b.y).abs() < 1e-9, "the loop did not line up");
        assert!((a.h - 1.6).abs() < 1e-9, "it fell asleep a second time");
    }

    #[test]
    fn the_player_retires_a_finished_beat() {
        let mut p = Player::new();
        assert!(!p.is_playing());
        p.play(&LOOK, 10.0);
        assert!(p.is_playing());
        p.tick(10.0 + LOOK.secs() / 2.0);
        assert!(p.is_playing(), "retired halfway through");
        p.tick(10.0 + LOOK.secs() + 0.01);
        assert!(!p.is_playing());
    }

    #[test]
    fn stopping_is_immediate_and_leaves_the_base_pose() {
        let mut p = Player::new();
        p.play(&LOOK, 0.0);
        p.stop();
        assert_eq!(p.pose(base(), 0.3), base());
        assert_eq!(p.motion(Motion::Drift, 0.3), Motion::Drift);
    }

    #[test]
    fn a_beat_takes_the_wheel_from_the_resting_motion() {
        // Without this the saccade adds to a gaze the beat is steering, and
        // a deliberate glance lands somewhere the beat never asked for.
        let mut p = Player::new();
        p.play(&LOOK, 0.0);
        assert_eq!(p.motion(Motion::Drift, 0.1), Motion::None);
    }

    #[test]
    fn a_delta_is_measured_from_rest_not_from_the_keyframe_before_it() {
        // The rule that makes a catalog entry readable one line at a time.
        // Frame 1 says nothing about `w`, so `w` is back at rest there — it
        // does not inherit frame 0's doubling.
        static TWO: Beat = Beat {
            name: "two",
            frames: &[
                Keyframe::new(Delta::num(&[(Field::W, Op::Mul(2.0))]), 0.2, Easing::Linear),
                Keyframe::new(Delta::num(&[(Field::Y, Op::Set(3.0))]), 0.2, Easing::Linear),
            ],
            ret_secs: 0.1,
            loop_from: None,
            motion: None,
        };
        let at_first = TWO.at(base(), 0.2);
        assert!((at_first.w - base().w * 2.0).abs() < 1e-9);
        let at_second = TWO.at(base(), 0.4);
        assert!(
            (at_second.w - base().w).abs() < 1e-9,
            "w was {} — deltas stacked instead of measuring from rest",
            at_second.w
        );
        assert!((at_second.y - 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_zero_length_keyframe_is_a_cut_not_a_divide_by_zero() {
        static CUT: Beat = Beat {
            name: "cut",
            frames: &[
                Keyframe::new(Delta::num(&[(Field::W, Op::Mul(2.0))]), 0.0, Easing::Linear),
                Keyframe::hold(Delta::num(&[(Field::W, Op::Mul(2.0))]), 0.2),
            ],
            ret_secs: 0.1,
            loop_from: None,
            motion: None,
        };
        let p = CUT.at(base(), 0.05);
        assert!(p.w.is_finite());
        assert!((p.w - base().w * 2.0).abs() < 1e-9);
    }

    #[test]
    fn every_beat_stays_finite_all_the_way_through() {
        for b in [&LOOK, &SNORE] {
            let mut t = 0.0;
            while t < b.secs() + 5.0 {
                let p = b.at(base(), t);
                for f in crate::face::pose::Field::ALL {
                    assert!(
                        p.get(f).is_finite(),
                        "{} went non-finite in {}",
                        f.name(),
                        b.name
                    );
                }
                t += 0.01;
            }
        }
    }
}
