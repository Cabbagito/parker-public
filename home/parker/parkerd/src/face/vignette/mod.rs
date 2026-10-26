// Vignettes: the long idle pieces, built rather than written out.
//
// A beat in idle.rs is a table: fifteen keyframes, typed once, const. A
// vignette is minutes long — a nap is thirty breaths, reading is two dozen
// pages of saccades, pong is five rallies of a ball that has to land where
// the paddle is — and a table of a thousand keyframes typed by hand would be
// a table nobody can read or change. So a vignette is *code that writes the
// table*: sections (yawn, asleep × 30, waking), each a run of keyframes,
// flattened once, on first use, into one ordinary `Beat`.
//
// The player does not know the difference. What a vignette adds is two
// things per keyframe, set here per section (beat.rs): the motion to play
// under it (a nap breathes, a sweep holds still), and whether the ordinary
// blink comes through (two minutes of reading without a blink reads as a
// frozen screen).
//
// Everything is drawn from the parts the face already has — two cells, a
// ring, a pupil, a waveform, six strokes, four inks. Nothing here needed a
// new pose field, and a vignette that does is a sign it belongs somewhere
// else.
//
// Numbers are rounded as they are written (positions to a hundredth of a
// dot, durations to the millisecond), so the export carries `0.3` and not
// `0.30000000000000004`, and both faces play exactly the numbers in it.
//
// Designed on the "visage — vignettes" canvas, where each one played live on
// visage's renderer before it was written here.

use super::beat::{Beat, Keyframe};
use super::pose::{Delta, Easing, Field, Ink, Motion, Op, Stroke};

pub mod bonsai;
pub mod chores;
pub mod nap;
pub mod pong;
pub mod power;
pub mod reading;
pub mod shine;
pub mod sort;
pub mod warp;

/// Keep a built slice for the life of the process — the lifetime a `static`
/// table would have had. Vignettes are built once, behind `idle::catalog`'s
/// `LazyLock`, so this runs a bounded number of times and never again.
fn keep<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// Positions and sizes to a hundredth of a dot — finer than any grid this
/// face is drawn on, and a sixth off the size of the exported file.
fn q(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Durations to the millisecond: a 35 ms strum stroke has to survive.
fn qt(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// The numbers a keyframe states. Later entries replace earlier ones for the
/// same field, so `with` can lay a change over a shared base.
pub type Look = Vec<(Field, Op)>;

/// Every field set to a value.
pub fn set(pairs: &[(Field, f64)]) -> Look {
    pairs.iter().map(|&(f, v)| (f, Op::Set(v))).collect()
}

/// `base` with `more` laid over it, field by field.
pub fn with(base: &[(Field, Op)], more: &[(Field, Op)]) -> Look {
    let mut out: Look = base
        .iter()
        .filter(|(f, _)| !more.iter().any(|(g, _)| g == f))
        .copied()
        .collect();
    out.extend_from_slice(more);
    out
}

/// What one keyframe asks for, before it is made static.
#[derive(Clone, Debug, Default)]
pub struct To {
    pub num: Look,
    pub mark: Option<Vec<Stroke>>,
    pub ink: Option<Ink>,
    pub mark_ink: Option<Vec<Ink>>,
}

impl To {
    pub fn num(num: Look) -> To {
        To {
            num,
            ..To::default()
        }
    }
    pub fn mark(mut self, mark: Vec<Stroke>) -> To {
        self.mark = Some(mark);
        self
    }
    pub fn inks(mut self, inks: Vec<Ink>) -> To {
        self.mark_ink = Some(inks);
        self
    }
    pub fn ink(mut self, ink: Ink) -> To {
        self.ink = Some(ink);
        self
    }

    fn delta(self) -> Delta {
        let num = self
            .num
            .into_iter()
            .map(|(f, op)| {
                let op = match op {
                    Op::Set(v) => Op::Set(q(v)),
                    Op::Mul(v) => Op::Mul(q(v)),
                    Op::Add(v) => Op::Add(q(v)),
                };
                (f, op)
            })
            .collect();
        Delta {
            num: keep(num),
            mark: self.mark.map(|m| {
                keep(
                    m.into_iter()
                        .map(|s| Stroke {
                            x0: q(s.x0),
                            y0: q(s.y0),
                            x1: q(s.x1),
                            y1: q(s.y1),
                            w: q(s.w.max(0.0)),
                        })
                        .collect(),
                )
            }),
            ink: self.ink,
            mark_ink: self.mark_ink.map(keep),
        }
    }
}

pub struct Frame {
    to: To,
    secs: f64,
    ease: Easing,
}

pub fn kf(to: To, secs: f64, ease: Easing) -> Frame {
    Frame { to, secs, ease }
}

/// Stay put, linearly — the hold of beat.rs.
pub fn hold(to: To, secs: f64) -> Frame {
    kf(to, secs, Easing::Linear)
}

/// A run of keyframes that share a motion and a blink rule.
pub struct Section {
    pub frames: Vec<Frame>,
    pub motion: Motion,
    pub blink: bool,
}

/// Still, no blink: the default for anything that steers the eye.
pub fn still(frames: Vec<Frame>) -> Section {
    Section {
        frames,
        motion: Motion::None,
        blink: false,
    }
}

/// Still, but the ordinary blink comes through.
pub fn blinking(frames: Vec<Frame>) -> Section {
    Section {
        frames,
        motion: Motion::None,
        blink: true,
    }
}

/// Flatten sections into one beat. The beat's own motion is `None`, so the
/// resting saccade never adds itself to a gaze a section is steering; every
/// keyframe names its section's motion explicitly anyway.
pub fn build(name: &'static str, ret_secs: f64, sections: Vec<Section>) -> Beat {
    let frames: Vec<Keyframe> = sections
        .into_iter()
        .flat_map(|s| {
            let (motion, blink) = (s.motion, s.blink);
            s.frames.into_iter().map(move |f| Keyframe {
                to: f.to.delta(),
                secs: qt(f.secs),
                ease: f.ease,
                motion: Some(motion),
                blink,
            })
        })
        .collect();
    Beat {
        name,
        frames: keep(frames),
        ret_secs,
        loop_from: None,
        motion: Some(Motion::None),
    }
}

// ── drawing helpers ─────────────────────────────────────────────────────────

pub fn s(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Stroke {
    Stroke { x0, y0, x1, y1, w }
}

/// A stroke that draws nothing, parked at the origin.
pub fn gone() -> Stroke {
    s(0.0, 0.0, 0.0, 0.0, 0.0)
}

/// `n` strokes that draw nothing.
pub fn none(n: usize) -> Vec<Stroke> {
    vec![gone(); n]
}

/// A dot: a stroke of no length.
pub fn dot(x: f64, y: f64, w: f64) -> Stroke {
    s(x, y, x + 0.01, y, w)
}

/// A Z, three strokes, centred on (x, y) with half-size `h`.
pub fn zee(x: f64, y: f64, h: f64, w: f64) -> [Stroke; 3] {
    [
        s(x - h, y - h, x + h, y - h, w),
        s(x + h, y - h, x - h, y + h, w),
        s(x - h, y + h, x + h, y + h, w),
    ]
}

/// A regular polygon's edges: `n` strokes round (cx, cy), radius `r`.
pub fn poly(n: usize, r: f64, w: f64, cx: f64, cy: f64, rot: f64) -> Vec<Stroke> {
    let tau = std::f64::consts::TAU;
    (0..n)
        .map(|i| {
            let a0 = rot + i as f64 * tau / n as f64;
            let a1 = rot + (i + 1) as f64 * tau / n as f64;
            s(
                cx + a0.cos() * r,
                cy + a0.sin() * r,
                cx + a1.cos() * r,
                cy + a1.sin() * r,
                w,
            )
        })
        .collect()
}

pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    x.max(lo).min(hi)
}

/// Ease in and out, for builders that sample their own curves.
pub fn smooth(t: f64) -> f64 {
    Easing::InOut.at(t)
}
