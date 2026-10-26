// A pose: every number that describes parker's face at one instant.
//
// This is the whole animation system. Expressions are named poses, and a
// transition between any two is `lerp` — which is why motion comes out as
// real movement (an eye genuinely squashes, an iris genuinely slides) rather
// than a cross-fade. Interpolating the *rendered dots* would give dissolve
// noise; interpolating the numbers that generate them gives animation.
//
// One model covers all three looks, so they are not rival styles but points
// in a continuous space:
//
//   two cells apart, tall        → eyes
//   cells merged, round, ringed  → an aperture
//   one cell wide and flat, with amplitude → a waveform
//
// So eyes-becoming-an-iris-becoming-a-voice is a lerp, not three renderers.
// The same numbers drive the braille grid today and can drive the Kraken LCD
// or a monitor-sized surface later; a renderer is a backend, not a rewrite.

use std::f64::consts::{PI, TAU};

/// How many straight strokes a pose carries.
///
/// Was two, and the comment then said "two segments cover every glyph worth
/// having at this size" — which was true while the glyphs were a check and a
/// cross. A `Z` is three, and parker sleeps now (MOTION.md), so a single Z
/// consumed the whole budget and `zZZ` was not expressible at all. Six buys
/// two Zs at once, an exclamation with room for its dot, and four speed
/// lines, which is the comic-book vocabulary the idle beats are drawn from.
///
/// Cost is a wider array in every lerp; the strokes with `w == 0` are skipped
/// by the renderer, so an ordinary face pays nothing for the empty ones.
pub const MARKS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Half the distance between the two cells. 0 merges them into one.
    pub gap: f64,
    /// Cell half-width and half-height.
    pub w: f64,
    pub h: f64,
    /// Corner rounding, 0 (rectangle) to 1 (fully round).
    pub round: f64,
    /// Vertical offset of the whole face from centre.
    pub y: f64,
    /// Horizontal offset of the whole face from centre.
    pub x: f64,
    /// Extra vertical offset applied to the right cell only. Asymmetry is
    /// most of what separates a question from a stare.
    pub tilt: f64,
    /// Height multiplier for the right cell — the other half of asymmetry.
    pub squint: f64,

    /// Pupil radius. 0 means no pupil, which is the difference between a
    /// shape and something that is looking at you.
    pub pupil: f64,
    pub gaze_x: f64,
    pub gaze_y: f64,

    /// Surrounding ring. 0 radius means absent.
    pub ring: f64,
    pub ring_w: f64,
    /// How much of the ring is drawn, 0..1 — lets it sweep itself on.
    pub sweep: f64,
    /// Where a partial ring *starts*, in turns. 0 is straight up, which is
    /// where `sweep` always began before this existed.
    ///
    /// One number, and it is the difference between a ring that fills and a
    /// ring that *turns*: a radar sweep, a turntable, and a scratch (spin
    /// running backwards) are all this field moving. It interpolates like
    /// everything else, so a beat can spin the ring up and let it coast back.
    pub spin: f64,

    /// Vertical displacement applied along the cell, turning a flat cell into
    /// a waveform. 0 leaves it straight.
    pub amp: f64,
    pub freq: f64,

    /// Straight strokes. This is how parker draws things that are not faces —
    /// a check, a cross, a bar, a Z, an exclamation. Because they are just
    /// endpoints they interpolate like everything else: eyes can shrink away
    /// while a check draws itself, in one lerp.
    pub mark: [Stroke; MARKS],

    /// Colour. Not interpolated — a transition takes the target's inks
    /// immediately, because a colour crossfade at 1-bit is mud and because
    /// colour leading the shape is what makes an alarm read as an alarm.
    pub ink: Ink,
    /// One ink per mark stroke, because marks stopped being one glyph: three
    /// quavers in the theme's colours, a white scan line beside a red
    /// contact. A single ink was the actual limit the first multi-subject
    /// beats hit.
    pub mark_ink: [Ink; MARKS],
}

/// A straight segment in dot coordinates, relative to the centre of the face.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Stroke {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// 0 means absent, which is how a mark fades in and out.
    pub w: f64,
}

impl Stroke {
    pub const NONE: Stroke = Stroke {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
        w: 0.0,
    };

    /// Pad a handful of strokes out to a full mark array. Most glyphs use two
    /// or three, and writing four `Stroke::NONE`s after each one buries the
    /// glyph in punctuation.
    pub const fn pad(given: &[Stroke]) -> [Stroke; MARKS] {
        let mut out = [Stroke::NONE; MARKS];
        let mut i = 0;
        while i < given.len() && i < MARKS {
            out[i] = given[i];
            i += 1;
        }
        out
    }

    pub fn lerp(a: Stroke, b: Stroke, t: f64) -> Stroke {
        let m = |x: f64, y: f64| x + (y - x) * t;
        Stroke {
            x0: m(a.x0, b.x0),
            y0: m(a.y0, b.y0),
            x1: m(a.x1, b.x1),
            y1: m(a.y1, b.y1),
            w: m(a.w, b.w),
        }
    }
}

/// Semantic colour, resolved to actual values at render time from whatever
/// the theme handed in. Named by meaning rather than by hue so the face
/// follows `theme set` like everything else on this machine does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Ink {
    /// The terminal's own foreground. Most of the face, most of the time.
    #[default]
    Plain,
    /// The active theme's accent.
    Accent,
    Good,
    Bad,
}

impl Ink {
    /// Pad a handful of inks out to a full per-stroke array, `Plain` behind
    /// them — the mirror of `Stroke::pad`, and used the same way: a delta
    /// names the inks for the strokes it cares about and no others.
    pub const fn pad(given: &[Ink]) -> [Ink; MARKS] {
        let mut out = [Ink::Plain; MARKS];
        let mut i = 0;
        while i < given.len() && i < out.len() {
            out[i] = given[i];
            i += 1;
        }
        out
    }

    pub fn name(self) -> &'static str {
        match self {
            Ink::Plain => "plain",
            Ink::Accent => "accent",
            Ink::Good => "good",
            Ink::Bad => "bad",
        }
    }

    pub fn slot(self) -> u8 {
        match self {
            Ink::Plain => 0,
            Ink::Accent => 1,
            Ink::Good => 2,
            Ink::Bad => 3,
        }
    }
}

impl Pose {
    pub const ZERO: Pose = Pose {
        gap: 0.0,
        w: 0.0,
        h: 0.0,
        round: 1.0,
        y: 0.0,
        x: 0.0,
        tilt: 0.0,
        squint: 1.0,
        pupil: 0.0,
        gaze_x: 0.0,
        gaze_y: 0.0,
        ring: 0.0,
        ring_w: 0.0,
        sweep: 1.0,
        spin: 0.0,
        amp: 0.0,
        freq: 0.0,
        mark: [Stroke::NONE; MARKS],
        ink: Ink::Plain,
        mark_ink: [Ink::Plain; MARKS],
    };

    /// Straight-line blend. `t` outside 0..1 is allowed on purpose: an
    /// overshoot past 1 is what gives a transition its spring, and clamping
    /// here would quietly remove that.
    pub fn lerp(a: Pose, b: Pose, t: f64) -> Pose {
        let m = |x: f64, y: f64| x + (y - x) * t;
        let mut mark = [Stroke::NONE; MARKS];
        for (i, slot) in mark.iter_mut().enumerate() {
            *slot = Stroke::lerp(a.mark[i], b.mark[i], t);
        }
        Pose {
            gap: m(a.gap, b.gap),
            w: m(a.w, b.w),
            h: m(a.h, b.h),
            round: m(a.round, b.round),
            y: m(a.y, b.y),
            x: m(a.x, b.x),
            tilt: m(a.tilt, b.tilt),
            squint: m(a.squint, b.squint),
            pupil: m(a.pupil, b.pupil),
            gaze_x: m(a.gaze_x, b.gaze_x),
            gaze_y: m(a.gaze_y, b.gaze_y),
            ring: m(a.ring, b.ring),
            ring_w: m(a.ring_w, b.ring_w),
            sweep: m(a.sweep, b.sweep),
            spin: m(a.spin, b.spin),
            amp: m(a.amp, b.amp),
            freq: m(a.freq, b.freq),
            mark,
            ink: b.ink,
            mark_ink: b.mark_ink,
        }
    }

    pub fn get(&self, f: Field) -> f64 {
        match f {
            Field::Gap => self.gap,
            Field::W => self.w,
            Field::H => self.h,
            Field::Round => self.round,
            Field::Y => self.y,
            Field::X => self.x,
            Field::Tilt => self.tilt,
            Field::Squint => self.squint,
            Field::Pupil => self.pupil,
            Field::GazeX => self.gaze_x,
            Field::GazeY => self.gaze_y,
            Field::Ring => self.ring,
            Field::RingW => self.ring_w,
            Field::Sweep => self.sweep,
            Field::Spin => self.spin,
            Field::Amp => self.amp,
            Field::Freq => self.freq,
        }
    }

    pub fn set(&mut self, f: Field, v: f64) {
        match f {
            Field::Gap => self.gap = v,
            Field::W => self.w = v,
            Field::H => self.h = v,
            Field::Round => self.round = v,
            Field::Y => self.y = v,
            Field::X => self.x = v,
            Field::Tilt => self.tilt = v,
            Field::Squint => self.squint = v,
            Field::Pupil => self.pupil = v,
            Field::GazeX => self.gaze_x = v,
            Field::GazeY => self.gaze_y = v,
            Field::Ring => self.ring = v,
            Field::RingW => self.ring_w = v,
            Field::Sweep => self.sweep = v,
            Field::Spin => self.spin = v,
            Field::Amp => self.amp = v,
            Field::Freq => self.freq = v,
        }
    }
}

/// A pose's numeric fields, addressable by name.
///
/// The pose struct is the ergonomic way to *write* a face; this is the way to
/// talk about one field of it generically, which is what the beat and dance
/// layers need — a keyframe says "gaze_x, set, 4.5" and a dance move says
/// "y, pulse, 3.5, every beat". Without it both layers would have to be
/// written as code per field instead of data, and data is what survives the
/// crossing to visage (MOTION.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Gap,
    W,
    H,
    Round,
    Y,
    X,
    Tilt,
    Squint,
    Pupil,
    GazeX,
    GazeY,
    Ring,
    RingW,
    Sweep,
    Spin,
    Amp,
    Freq,
}

impl Field {
    pub const ALL: [Field; 17] = [
        Field::Gap,
        Field::W,
        Field::H,
        Field::Round,
        Field::Y,
        Field::X,
        Field::Tilt,
        Field::Squint,
        Field::Pupil,
        Field::GazeX,
        Field::GazeY,
        Field::Ring,
        Field::RingW,
        Field::Sweep,
        Field::Spin,
        Field::Amp,
        Field::Freq,
    ];

    /// The name this field carries across to visage. Deliberately the
    /// JavaScript spelling, because that is the side that reads the exported
    /// catalog and a rename there is a silent miss rather than a build error.
    pub fn name(self) -> &'static str {
        match self {
            Field::Gap => "gap",
            Field::W => "w",
            Field::H => "h",
            Field::Round => "round",
            Field::Y => "y",
            Field::X => "x",
            Field::Tilt => "tilt",
            Field::Squint => "squint",
            Field::Pupil => "pupil",
            Field::GazeX => "gazeX",
            Field::GazeY => "gazeY",
            Field::Ring => "ring",
            Field::RingW => "ringW",
            Field::Sweep => "sweep",
            Field::Spin => "spin",
            Field::Amp => "amp",
            Field::Freq => "freq",
        }
    }
}

/// What a delta does to the field it names.
///
/// Three, because beats genuinely want all three: a glance *sets* gaze to a
/// point, a stretch *multiplies* size by 1.14 whatever size it started at,
/// and a nod *adds* to whatever y the face is already holding. Collapsing
/// them to one would push arithmetic into every catalog entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Set(f64),
    Mul(f64),
    /// No beat in the current catalog happens to add, but the JS player and
    /// the catalog format both carry the op — it is vocabulary, not usage.
    #[allow(dead_code)]
    Add(f64),
}

impl Op {
    pub fn apply(self, base: f64) -> f64 {
        match self {
            Op::Set(v) => v,
            Op::Mul(v) => base * v,
            Op::Add(v) => base + v,
        }
    }

    pub fn parts(self) -> (&'static str, f64) {
        match self {
            Op::Set(v) => ("set", v),
            Op::Mul(v) => ("mul", v),
            Op::Add(v) => ("add", v),
        }
    }
}

/// A partial pose: what a keyframe changes, relative to the pose parker is
/// resting in. Everything unnamed is left exactly as it was, which is what
/// makes a beat composable with whatever face is underneath it — a glance is
/// a glance whether parker is idling or curious.
#[derive(Clone, Copy, Debug, Default)]
pub struct Delta {
    pub num: &'static [(Field, Op)],
    pub mark: Option<&'static [Stroke]>,
    pub ink: Option<Ink>,
    pub mark_ink: Option<&'static [Ink]>,
}

impl Delta {
    /// Change nothing. Every beat's first and last keyframe, implicitly.
    #[allow(dead_code)]
    pub const NONE: Delta = Delta {
        num: &[],
        mark: None,
        ink: None,
        mark_ink: None,
    };

    /// The common case: a keyframe that only moves numbers.
    pub const fn num(num: &'static [(Field, Op)]) -> Delta {
        Delta {
            num,
            mark: None,
            ink: None,
            mark_ink: None,
        }
    }

    pub fn apply(&self, base: Pose) -> Pose {
        let mut p = base;
        for (f, op) in self.num {
            p.set(*f, op.apply(base.get(*f)));
        }
        if let Some(m) = self.mark {
            p.mark = Stroke::pad(m);
        }
        if let Some(i) = self.ink {
            p.ink = i;
        }
        if let Some(i) = self.mark_ink {
            p.mark_ink = Ink::pad(i);
        }
        p
    }
}

/// Easing. Transitions read mechanical without it — the difference between a
/// value changing and something *moving*.
pub mod ease {
    /// Smooth at both ends. The default for a change of state.
    pub fn in_out(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    /// Overshoots and settles. For arrivals that should feel alive rather
    /// than positioned — the pop of a surprise, a beat landing.
    pub fn back_out(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.0;
        1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
    }

    /// Leaves fast, arrives gently. The shape of a flick — a saccade and a
    /// glance both start at full speed, which `in_out` specifically does not.
    pub fn out(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// No easing at all. For a step that is *meant* to read as mechanical —
    /// a quantized sidestep in a dance is not supposed to look organic.
    pub fn linear(t: f64) -> f64 {
        t.clamp(0.0, 1.0)
    }
}

/// Which easing a keyframe uses, as data rather than as a function pointer —
/// the catalog has to survive being written out as JSON and read back in
/// JavaScript (MOTION.md §3), and a `fn(f64) -> f64` does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Easing {
    Linear,
    #[default]
    InOut,
    Out,
    /// Same standing as `Op::Add`: no current beat overshoots, but the
    /// catalog format and the JS player carry it — vocabulary, not usage.
    #[allow(dead_code)]
    BackOut,
}

impl Easing {
    pub fn at(self, t: f64) -> f64 {
        match self {
            Easing::Linear => ease::linear(t),
            Easing::InOut => ease::in_out(t),
            Easing::Out => ease::out(t),
            Easing::BackOut => ease::back_out(t),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Easing::Linear => "linear",
            Easing::InOut => "inOut",
            Easing::Out => "out",
            Easing::BackOut => "backOut",
        }
    }
}

/// Idle movement layered on top of a pose. Kept separate from the pose itself
/// because it is a *loop*, not a destination — you interpolate to a pose, but
/// you never interpolate to a phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Perfectly still. Reads as crashed; use sparingly and deliberately.
    None,
    /// A slow size pulse. Proof of life, and almost invisible.
    Breathe,
    /// Blades sweeping the ring — handled by the renderer, which needs the
    /// phase directly.
    Blades,
    /// Travelling displacement, for a waveform.
    Wave,
    /// Vertical bob.
    Nod,
    /// Horizontal shake.
    Shake,
    /// Fast tight contraction. Urgency — reads as locked-on, not sleepy.
    Pulse,
    /// The resting gaze: flick, fixate, and a slow wander inside the
    /// fixation. See `saccade` — this used to be a plain sine and the change
    /// is the single biggest difference between a face that is animated and a
    /// face that is looking at something.
    Drift,

    // ── the faces round (Oct 2026, the "visage — faces" canvas). Most of
    // these were written for one face each, and the ones that move strokes
    // know that face's strokes by index — a motion is still a pure function
    // of the loop, it just knows what it is moving. Appended, never
    // reordered: the names cross to visage.
    /// A shiver too fine to read as movement, only as nerves (wince's >_<).
    Tremble,
    /// Hops: |sin| never dips below rest, so it is a bounce, not a sway
    /// (delighted).
    Bounce,
    /// Chuckles in bursts — three quick little hops every third of a loop,
    /// then still (amused).
    Giggle,
    /// Head nodding on the beat, nine a loop, and a slow lean (vibing).
    Vibe,
    /// The gaze goes round in circles and the strokes ride an ellipse over
    /// the head: the cartoon knockout's stars (dizzy).
    Orbit,
    /// A balance tipping: stroke 0 is the beam and rocks on its pivot,
    /// strokes 3 and 4 are the pans and ride its ends, level (weighing).
    Tip,
    /// Side-eye that will not settle: held to one side, a quick check the
    /// other way, and back (suspicious).
    Shifty,
    /// Reading fast: the gaze steps along a line in fixations, snaps back
    /// and drops a line, six lines a loop (skimming).
    Read,
    /// Walks into the wall twice a loop — push, squash, recoil. The strokes
    /// are the wall and are held where they are (blocked).
    Bonk,
    /// A creeper's fuse: flashing to plain ink faster and faster, swelling
    /// (creeper).
    Fuse,
    /// Signal trouble: in a few random slots a second the face jumps
    /// sideways, changes width and ink, and a scanline tears across stroke
    /// 5 (glitch).
    Glitch,
    /// An hourglass: the cells are the bulbs, strokes 2, 3 and 4 the sand —
    /// the top sinks onto the neck, a stream falls, the pile heaps up — and
    /// then the glass folds flat through the neck and comes out mirrored,
    /// which is the full glass again (waiting).
    Sand,
    /// Two quick pokes toward where the strokes point, then still: "there —
    /// there" (pointing's arrow). Moves every stroke, not the eyes.
    Jab,
}

/// A stroke's endpoints turned about a point.
fn turned(m: Stroke, px: f64, py: f64, th: f64) -> Stroke {
    let (s, c) = th.sin_cos();
    let at = |x: f64, y: f64| {
        (
            px + (x - px) * c - (y - py) * s,
            py + (x - px) * s + (y - py) * c,
        )
    };
    let (x0, y0) = at(m.x0, m.y0);
    let (x1, y1) = at(m.x1, m.y1);
    Stroke {
        x0,
        y0,
        x1,
        y1,
        ..m
    }
}

fn moved(m: Stroke, dx: f64, dy: f64) -> Stroke {
    Stroke {
        x0: m.x0 + dx,
        y0: m.y0 + dy,
        x1: m.x1 + dx,
        y1: m.y1 + dy,
        ..m
    }
}

fn scaled(m: Stroke, px: f64, py: f64, k: f64) -> Stroke {
    Stroke {
        x0: px + (m.x0 - px) * k,
        y0: py + (m.y0 - py) * k,
        x1: px + (m.x1 - px) * k,
        y1: py + (m.y1 - py) * k,
        ..m
    }
}

/// Every drawn stroke through `f`, with its index; absent ones untouched.
fn each_mark(p: &mut Pose, f: impl Fn(usize, Stroke) -> Stroke) {
    for (i, m) in p.mark.iter_mut().enumerate() {
        if m.w > 0.05 {
            *m = f(i, *m);
        }
    }
}

/// A deterministic, stateless hash from a step index to 0..1.
///
/// Stateless because the same sequence has to come out of two
/// implementations in two languages with no shared RNG: parkerd and visage
/// both compute the gaze from the clock alone, so they agree without ever
/// talking. splitmix64's finalizer, which is small enough to port by eye and
/// well-distributed enough that consecutive steps look unrelated.
pub fn hash01(n: u64, salt: u64) -> f64 {
    let mut z = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // The top 53 bits, which is exactly what a JS double can hold without
    // rounding — so the port produces bit-identical values.
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Where the eye is looking at time `t`, in dots from centre.
///
/// Real eyes do not glide. They *flick* — 30-50ms, ballistic, too fast to
/// follow — and then hold dead still for a third of a second or so before
/// flicking somewhere else. The old drift was a sine on gaze, and a sine is
/// exactly the motion an eye never makes; it read as a screensaver because
/// it was one.
///
/// Three things layered, in the order that matters:
///
///   1. a fixation grid: one target per `DWELL`, chosen by hash;
///   2. a flick between targets over `FLICK`, eased `out` so it leaves at
///      full speed and lands softly, which is what a ballistic movement
///      decelerating into position actually looks like;
///   3. ocular drift *within* the fixation — a fraction of a dot, far too
///      small to read as movement and the whole reason the held pose does
///      not look frozen.
///
/// Two refinements that carry most of the character:
///
///   - some slots hold their predecessor's target, so fixations come in
///     lengths of one, two or three dwells instead of a metronome;
///   - roughly one in five targets is dead centre, which reads as parker
///     looking back at *you*. Eye contact is the thing you notice.
///
/// Keyed on absolute seconds rather than on the loop phase, deliberately.
/// Phase wraps every few seconds and a saccade sequence that repeats on that
/// period is more obviously mechanical than the sine it replaced — the
/// discreteness that makes it read as alive is the same discreteness that
/// makes a repeat unmissable.
pub fn saccade(t: f64) -> (f64, f64) {
    /// One fixation slot. Human fixations run 200-600ms; this is the floor
    /// and slots merge upward from it.
    const DWELL: f64 = 0.62;
    const FLICK: f64 = 0.045;
    const REACH_X: f64 = 3.4;
    const REACH_Y: f64 = 1.5;

    // Which slot a target actually belongs to: hold the previous one often
    // enough that fixations vary in length without needing a running clock.
    let anchor = |slot: i64| -> i64 {
        let mut s = slot;
        // At most two merges, so a fixation never exceeds three dwells and
        // parker never appears to have locked up.
        for _ in 0..2 {
            if s > 0 && hash01(s as u64, 0x5EED) < 0.34 {
                s -= 1;
            } else {
                break;
            }
        }
        s
    };

    let target = |slot: i64| -> (f64, f64) {
        let s = anchor(slot);
        // Eye contact. Not rare enough to be a surprise, not common enough
        // to be a stare.
        if hash01(s as u64, 0xC0FFEE) < 0.22 {
            return (0.0, 0.0);
        }
        (
            (hash01(s as u64, 0xA11CE) * 2.0 - 1.0) * REACH_X,
            (hash01(s as u64, 0xB0B) * 2.0 - 1.0) * REACH_Y,
        )
    };

    let slot = (t / DWELL).floor() as i64;
    let into = t - slot as f64 * DWELL;
    let (fx, fy) = target(slot);
    let (px, py) = target(slot - 1);

    // Mid-flick, or held. `target` returning the same point for merged slots
    // makes the flick a no-op there for free.
    let k = if into < FLICK {
        ease::out(into / FLICK)
    } else {
        1.0
    };

    // Ocular drift. Two incommensurable periods so it never visibly cycles.
    let dx = (t * 1.7).sin() * 0.22 + (t * 0.53).sin() * 0.12;
    let dy = (t * 1.13).cos() * 0.14;

    (px + (fx - px) * k + dx, py + (fy - py) * k + dy)
}

/// How much to squash an eye at time `t`, 1.0 wide open and ~0 shut.
///
/// A blink is involuntary, continuous, and belongs to *having eyes* rather
/// than to any one expression — so it is neither a pose nor a beat, and it
/// lives here beside the saccade for the same reason: both surfaces compute
/// it from the clock alone and therefore agree without a channel.
///
/// It was previously visage's alone, rolled from `Math.random()` in
/// `Face.svelte`, and the terminal face simply never blinked. One
/// implementation, and a deterministic one, fixes both halves of that.
///
/// The caller decides whether the current pose has anything blinkable in it;
/// a waveform and a check do not.
pub fn blink(t: f64) -> f64 {
    /// Candidate slots. Roughly a third of them blink, which lands the
    /// average interval near three seconds — inside the human 2-6s range.
    const SLOT: f64 = 1.05;
    const SHUT: f64 = 0.14;
    /// A double blink lands the second one this long after the first.
    const AGAIN: f64 = 0.23;

    let slot = (t / SLOT).floor() as i64;
    // Look at this slot and the one before it: a blink that starts late in
    // its slot has to finish in the next one.
    let mut squash: f64 = 1.0;
    for s in [slot - 1, slot] {
        if hash01(s as u64, 0xB11) >= 0.34 {
            continue;
        }
        let begins = s as f64 * SLOT + hash01(s as u64, 0xB12) * (SLOT - SHUT - AGAIN).max(0.0);
        let double = hash01(s as u64, 0xB13) < 0.18;
        for start in [Some(begins), double.then_some(begins + AGAIN)]
            .into_iter()
            .flatten()
        {
            let into = (t - start) / SHUT;
            if (0.0..1.0).contains(&into) {
                // Down and back up: |1 - 2x| is a V, floored so the lid never
                // quite reaches zero and the eye keeps a seam.
                squash = squash.min((1.0 - into * 2.0).abs().max(0.06));
            }
        }
    }
    squash
}

impl Motion {
    /// The name this motion carries across to visage, matching the string
    /// `face.js` switches on.
    pub fn name(self) -> &'static str {
        match self {
            Motion::None => "none",
            Motion::Breathe => "breathe",
            Motion::Blades => "blades",
            Motion::Wave => "wave",
            Motion::Nod => "nod",
            Motion::Shake => "shake",
            Motion::Pulse => "pulse",
            Motion::Drift => "drift",
            Motion::Tremble => "tremble",
            Motion::Bounce => "bounce",
            Motion::Giggle => "giggle",
            Motion::Vibe => "vibe",
            Motion::Orbit => "orbit",
            Motion::Tip => "tip",
            Motion::Shifty => "shifty",
            Motion::Read => "read",
            Motion::Bonk => "bonk",
            Motion::Fuse => "fuse",
            Motion::Glitch => "glitch",
            Motion::Sand => "sand",
            Motion::Jab => "jab",
        }
    }

    /// Apply this motion's modulation.
    ///
    /// `phase` is 0..1 through the idle loop and drives everything periodic.
    /// `t` is absolute seconds and drives everything that must *not* repeat
    /// on the loop — which is currently the saccade and nothing else, but is
    /// also what any beat-locked motion would need.
    pub fn apply(self, mut p: Pose, phase: f64, t: f64) -> Pose {
        let a = phase * TAU;
        match self {
            Motion::None | Motion::Blades => {}
            Motion::Breathe => {
                let s = 1.0 + a.sin() * 0.06;
                p.w *= s;
                p.h *= s;
                if p.pupil > 0.0 {
                    p.pupil *= 1.0 + a.sin() * 0.12;
                }
            }
            Motion::Wave => {
                // The travelling term lives in the renderer via phase; here we
                // just breathe the amplitude so it never looks looped.
                p.amp *= 1.0 + (a * 0.5).sin() * 0.15;
            }
            Motion::Pulse => {
                let s = 1.0 - ((a * 5.0).sin().max(0.0)) * 0.22;
                p.w *= s;
                p.h *= s;
                p.ring *= 1.0 + (a * 5.0).sin() * 0.05;
            }
            Motion::Nod => p.y += (a * 7.0).sin() * 3.5,
            // A shake moves the head, not the pupils: a mark-only pose (a cross)
            // has no pupil to move, and gaze-only shaking left it perfectly still.
            Motion::Shake => p.x += (a * 9.0).sin() * 3.5,
            Motion::Drift => {
                let (gx, gy) = saccade(t);
                p.gaze_x += gx;
                p.gaze_y += gy;
            }
            Motion::Tremble => {
                p.x += (t * 53.0).sin() * 0.5 + (t * 31.0).sin() * 0.3;
                p.y += (t * 41.0).sin() * 0.35;
            }
            Motion::Bounce => p.y -= (a * 3.0).sin().abs() * 2.0,
            Motion::Giggle => {
                let gp = (phase * 3.0) % 1.0;
                if gp < 0.4 {
                    p.y -= (gp / 0.4 * PI * 3.0).sin().abs() * 1.3;
                }
            }
            Motion::Vibe => {
                p.y += (0.5 - 0.5 * (a * 9.0).cos()) * 1.8;
                p.x += a.sin() * 2.0;
            }
            Motion::Orbit => {
                // The ellipse the stars ride: centre, then the two radii.
                const O: (f64, f64, f64, f64) = (0.0, -14.0, 24.0, 3.6);
                p.gaze_x += (a * 4.0).cos() * 2.2;
                p.gaze_y += (a * 4.0).sin() * 1.6;
                each_mark(&mut p, |_, m| {
                    let (cx, cy) = ((m.x0 + m.x1) / 2.0, (m.y0 + m.y1) / 2.0);
                    let ang = ((cy - O.1) / O.3).atan2((cx - O.0) / O.2) + a * 2.0;
                    moved(m, O.0 + ang.cos() * O.2 - cx, O.1 + ang.sin() * O.3 - cy)
                });
            }
            Motion::Tip => {
                const PIVOT: (f64, f64) = (0.0, -9.0);
                const HALF: f64 = 20.0;
                let th = (a * 2.0).sin() * 0.2;
                each_mark(&mut p, |i, m| match i {
                    0 => turned(m, PIVOT.0, PIVOT.1, th),
                    3 | 4 => {
                        let side = ((m.x0 + m.x1) / 2.0 - PIVOT.0).signum();
                        moved(m, (th.cos() - 1.0) * HALF * side, th.sin() * HALF * side)
                    }
                    _ => m,
                });
            }
            Motion::Shifty => {
                let k = if phase < 0.5 {
                    0.0
                } else if phase < 0.58 {
                    (phase - 0.5) / 0.08
                } else if phase < 0.76 {
                    1.0
                } else if phase < 0.84 {
                    1.0 - (phase - 0.76) / 0.08
                } else {
                    0.0
                };
                p.gaze_x += ease::in_out(k) * 8.4;
            }
            Motion::Read => {
                const LINES: f64 = 6.0;
                let lp = phase * LINES;
                let line = lp.floor();
                let u = lp - line;
                let along = if u < 0.84 {
                    // Five fixations a line, each a flick and a hold.
                    let s5 = u / 0.84 * 5.0;
                    let f5 = s5.floor();
                    (f5 + ease::out(((s5 - f5) / 0.25).min(1.0))) / 5.0
                } else {
                    1.0 - ease::in_out((u - 0.84) / 0.16)
                };
                p.gaze_x += -3.0 + 6.0 * along;
                p.gaze_y += -1.8 + 3.6 * line / (LINES - 1.0);
            }
            Motion::Bonk => {
                let bp = (phase * 2.0) % 1.0;
                let push = if bp < 0.12 {
                    ease::in_out(bp / 0.12)
                } else if bp < 0.2 {
                    1.0 - 1.4 * ease::out((bp - 0.12) / 0.08)
                } else if bp < 0.4 {
                    -0.4 + 0.4 * ease::in_out((bp - 0.2) / 0.2)
                } else {
                    0.0
                };
                let dx = push * 3.5;
                p.x += dx;
                if push > 0.75 {
                    p.w *= 0.84;
                    p.h *= 1.08;
                }
                each_mark(&mut p, |_, m| moved(m, -dx, 0.0));
            }
            Motion::Fuse => {
                let flash = (t * (1.5 + 7.0 * phase)) % 1.0 < 0.35;
                let s = 1.0 + 0.07 * phase;
                p.w *= s;
                p.h *= s;
                each_mark(&mut p, |_, m| scaled(m, 0.0, 0.0, s));
                if flash {
                    p.ink = Ink::Plain;
                    p.mark_ink = [Ink::Plain; MARKS];
                }
            }
            Motion::Glitch => {
                let slot = (t.max(0.0) * 9.0).floor() as u64;
                if hash01(slot, 0x61) < 0.4 {
                    let j = hash01(slot, 0x62) * 2.0 - 1.0;
                    let yy = (hash01(slot, 0x65) * 2.0 - 1.0) * 14.0;
                    p.x += j * 7.0;
                    p.w *= 0.7 + hash01(slot, 0x63) * 0.8;
                    p.gaze_x += j * 2.0;
                    p.ink = if hash01(slot, 0x64) < 0.5 {
                        Ink::Accent
                    } else {
                        Ink::Bad
                    };
                    // Held across the whole width wherever the face jumped.
                    p.mark[5] = Stroke {
                        x0: -44.0 - p.x,
                        y0: yy,
                        x1: 44.0 - p.x,
                        y1: yy,
                        w: 1.0,
                    };
                    p.mark_ink[5] = p.ink;
                }
            }
            Motion::Sand => {
                // The bulbs' inner radius; the bottom bulb sits `tilt` below.
                const R: f64 = 5.2;
                let drained = (phase / 0.84).min(1.0);
                let (bottom, neck) = (p.tilt, p.tilt / 2.0);
                let full = 1.6 * R;
                let (top_t, pile_t) = ((1.0 - drained) * full, drained * full);
                // Sand resting on the floor of the bulb centred at `cy`: one
                // fat stroke, as wide as the bulb is at that height.
                let lump = |cy: f64, thick: f64| -> Stroke {
                    let yc = cy + R - 0.4 - thick / 2.0;
                    let dy = yc - cy;
                    let half = (R * R - dy * dy).max(0.0).sqrt() - thick / 2.0 - 0.4;
                    Stroke {
                        x0: -half.max(0.0),
                        y0: yc,
                        x1: half.max(0.0),
                        y1: yc,
                        w: if thick > 0.8 { thick } else { 0.0 },
                    }
                };
                p.mark[2] = lump(0.0, top_t);
                p.mark[3] = lump(bottom, pile_t);
                p.mark[4] = Stroke {
                    x0: 0.0,
                    y0: neck,
                    x1: 0.0,
                    y1: bottom + R - 0.4 - pile_t,
                    w: if drained > 0.02 && drained < 1.0 {
                        1.0
                    } else {
                        0.0
                    },
                };
                if phase > 0.84 {
                    let c = (PI * ease::in_out((phase - 0.84) / 0.16)).cos();
                    let ac = c.abs();
                    let sink = neck * (1.0 - ac);
                    p.y += sink;
                    p.tilt = bottom * ac;
                    p.h *= ac;
                    each_mark(&mut p, |_, m| Stroke {
                        y0: neck + (m.y0 - neck) * c - sink,
                        y1: neck + (m.y1 - neck) * c - sink,
                        ..m
                    });
                }
            }
            Motion::Jab => {
                // Each poke goes out fast and comes back slower; the second
                // follows close behind the first, and the rest of the loop
                // holds still, so it reads as a gesture rather than a pulse.
                const REACH: f64 = 3.5;
                let poke = |start: f64| {
                    let k = (phase - start) / 0.08;
                    if !(0.0..=1.0).contains(&k) {
                        0.0
                    } else if k < 0.5 {
                        ease::out(k * 2.0)
                    } else {
                        1.0 - ease::in_out(k * 2.0 - 1.0)
                    }
                };
                let dx = (poke(0.03) + poke(0.14)) * REACH;
                each_mark(&mut p, |_, m| moved(m, dx, 0.0));
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> Pose {
        Pose {
            gap: 18.0,
            w: 7.0,
            h: 11.0,
            ..Pose::ZERO
        }
    }
    fn b() -> Pose {
        Pose {
            gap: 0.0,
            w: 6.0,
            h: 6.0,
            ring: 17.0,
            ..Pose::ZERO
        }
    }

    #[test]
    fn lerp_hits_both_ends_exactly() {
        assert_eq!(Pose::lerp(a(), b(), 0.0), a());
        assert_eq!(Pose::lerp(a(), b(), 1.0), b());
    }

    #[test]
    fn lerp_moves_every_field_together() {
        let m = Pose::lerp(a(), b(), 0.5);
        assert_eq!(m.gap, 9.0);
        assert_eq!(m.w, 6.5);
        assert_eq!(m.h, 8.5);
        assert_eq!(m.ring, 8.5, "the ring grows in as the cells converge");
    }

    #[test]
    fn lerp_permits_overshoot() {
        // Deliberate: clamping here would silently kill every spring.
        let over = Pose::lerp(a(), b(), 1.1);
        assert!(over.gap < 0.0);
    }

    #[test]
    fn easings_are_anchored_at_both_ends() {
        for f in [ease::in_out as fn(f64) -> f64, ease::back_out] {
            assert!((f(0.0) - 0.0).abs() < 1e-9);
            assert!((f(1.0) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn in_out_is_symmetric_and_monotonic() {
        assert!((ease::in_out(0.5) - 0.5).abs() < 1e-9);
        let mut prev = -1.0;
        for i in 0..=50 {
            let v = ease::in_out(i as f64 / 50.0);
            assert!(v >= prev, "eased value went backwards");
            prev = v;
        }
    }

    #[test]
    fn back_out_actually_overshoots() {
        // If it never exceeds 1.0 it is not a spring, it is just an ease.
        let peak = (0..100)
            .map(|i| ease::back_out(i as f64 / 100.0))
            .fold(0.0_f64, f64::max);
        assert!(
            peak > 1.0,
            "back_out peaked at {peak}, expected an overshoot"
        );
    }

    #[test]
    fn easings_clamp_outside_their_domain() {
        assert_eq!(ease::in_out(-1.0), 0.0);
        assert_eq!(ease::in_out(2.0), 1.0);
        assert_eq!(ease::back_out(2.0), 1.0);
    }

    #[test]
    fn breathe_changes_size_without_moving_the_face() {
        let p = Pose {
            w: 10.0,
            h: 10.0,
            ..Pose::ZERO
        };
        let big = Motion::Breathe.apply(p, 0.25, 0.0);
        let small = Motion::Breathe.apply(p, 0.75, 0.0);
        assert!(big.w > p.w && small.w < p.w);
        assert_eq!(big.y, p.y, "breathing must not drift the face");
    }

    #[test]
    fn still_motions_leave_the_pose_untouched() {
        let p = a();
        assert_eq!(Motion::None.apply(p, 0.3, 1.0), p);
        // Blades are drawn from phase in the renderer, not by modulating.
        assert_eq!(Motion::Blades.apply(p, 0.3, 1.0), p);
    }

    #[test]
    fn nod_and_shake_move_on_different_axes() {
        let p = Pose {
            w: 8.0,
            h: 8.0,
            ..Pose::ZERO
        };
        // Both move the whole face, not the pupils — a mark-only pose (a
        // cross) has no pupil, and gaze-only shaking left it perfectly still.
        assert_ne!(Motion::Nod.apply(p, 0.25, 0.0).y, p.y);
        assert_eq!(Motion::Nod.apply(p, 0.25, 0.0).x, p.x);
        assert_ne!(Motion::Shake.apply(p, 0.25, 0.0).x, p.x);
        assert_eq!(Motion::Shake.apply(p, 0.25, 0.0).y, p.y);
    }

    // ── saccades ────────────────────────────────────────────────────────

    #[test]
    fn the_gaze_holds_still_far_more_often_than_it_moves() {
        // The whole point. A sine is in motion 100% of the time; an eye is
        // in motion for the ~45ms of the flick and still for the rest.
        let step = 0.01;
        let mut moving = 0;
        let mut total = 0;
        let mut prev = saccade(0.0);
        let mut t = step;
        while t < 60.0 {
            let now = saccade(t);
            // Ocular drift is always nonzero, so "still" means "moving far
            // slower than a flick", not "identical".
            let speed = ((now.0 - prev.0).powi(2) + (now.1 - prev.1).powi(2)).sqrt() / step;
            if speed > 8.0 {
                moving += 1;
            }
            total += 1;
            prev = now;
            t += step;
        }
        let fraction = moving as f64 / total as f64;
        assert!(
            fraction < 0.2,
            "the eye is in flight {:.0}% of the time; it should mostly be holding",
            fraction * 100.0
        );
        assert!(fraction > 0.0, "it never flicked at all");
    }

    #[test]
    fn the_gaze_stays_on_the_face() {
        let mut t = 0.0;
        while t < 300.0 {
            let (x, y) = saccade(t);
            assert!(x.abs() < 4.5, "gaze_x {x} at t={t} is off the eye");
            assert!(y.abs() < 2.5, "gaze_y {y} at t={t} is off the eye");
            t += 0.017;
        }
    }

    #[test]
    fn the_gaze_never_repeats_on_the_idle_loop() {
        // The reason `apply` takes absolute seconds as well as phase. If the
        // saccade were keyed on phase this would be exactly equal, and the
        // repeat would be far more obvious than the sine it replaced.
        for loop_secs in [4.0_f64, 6.0] {
            let a = saccade(1.3);
            let b = saccade(1.3 + loop_secs);
            assert!(
                (a.0 - b.0).abs() > 0.01 || (a.1 - b.1).abs() > 0.01,
                "the gaze repeated after {loop_secs}s"
            );
        }
    }

    #[test]
    fn parker_looks_you_in_the_eye_sometimes_but_does_not_stare() {
        let mut centred = 0;
        let mut total = 0;
        let mut t = 0.31; // mid-fixation, clear of the flicks
        while t < 200.0 {
            let (x, y) = saccade(t);
            if x.abs() < 0.5 && y.abs() < 0.5 {
                centred += 1;
            }
            total += 1;
            t += 0.62;
        }
        let fraction = centred as f64 / total as f64;
        assert!(
            (0.08..0.5).contains(&fraction),
            "eye contact {:.0}% of the time",
            fraction * 100.0
        );
    }

    #[test]
    fn the_hash_is_spread_and_stays_inside_the_unit_interval() {
        let mut buckets = [0usize; 10];
        for n in 0..4000u64 {
            let v = hash01(n, 0x5EED);
            assert!((0.0..1.0).contains(&v));
            buckets[(v * 10.0) as usize] += 1;
        }
        // Every tenth gets a share. A bad mix piles up and the gaze visibly
        // favours one corner.
        for (i, count) in buckets.iter().enumerate() {
            assert!(*count > 250, "bucket {i} only got {count} of 4000");
        }
    }

    // ── the data model ──────────────────────────────────────────────────

    #[test]
    fn a_delta_touches_only_what_it_names() {
        let base = Pose {
            gaze_x: 1.0,
            w: 10.0,
            h: 10.0,
            ..a()
        };
        let d = Delta::num(&[(Field::GazeX, Op::Set(4.5)), (Field::W, Op::Mul(1.5))]);
        let out = d.apply(base);
        assert_eq!(out.gaze_x, 4.5);
        assert_eq!(out.w, 15.0);
        assert_eq!(out.h, base.h, "h was not named and must not have moved");
        assert_eq!(out.gap, base.gap);
    }

    #[test]
    fn an_empty_delta_is_the_identity() {
        assert_eq!(Delta::NONE.apply(a()), a());
    }

    #[test]
    fn ops_compose_against_the_base_not_against_each_other() {
        // Both read `w` from the *base*, so the order they are listed in
        // cannot change the result. A beat author must never have to think
        // about it.
        let base = Pose { w: 10.0, ..a() };
        let one = Delta::num(&[(Field::W, Op::Mul(2.0)), (Field::W, Op::Add(1.0))]).apply(base);
        assert_eq!(one.w, 11.0, "the second op saw the base, not 20");
    }

    #[test]
    fn every_field_round_trips_through_get_and_set() {
        for (i, f) in Field::ALL.iter().enumerate() {
            let mut p = Pose::ZERO;
            let v = 3.0 + i as f64;
            p.set(*f, v);
            assert_eq!(p.get(*f), v, "{} did not round-trip", f.name());
        }
    }

    #[test]
    fn field_names_are_unique_and_are_the_javascript_spellings() {
        let names: std::collections::HashSet<&str> = Field::ALL.iter().map(|f| f.name()).collect();
        assert_eq!(names.len(), Field::ALL.len(), "two fields share a name");
        // snake_case here would silently miss on the visage side.
        assert!(names.contains("gazeX"));
        assert!(names.contains("ringW"));
    }

    #[test]
    fn padding_strokes_fills_the_rest_with_nothing() {
        let s = Stroke {
            x0: 1.0,
            y0: 2.0,
            x1: 3.0,
            y1: 4.0,
            w: 3.0,
        };
        let padded = Stroke::pad(&[s, s]);
        assert_eq!(padded[0], s);
        assert_eq!(padded[1], s);
        for empty in padded.iter().skip(2) {
            assert_eq!(*empty, Stroke::NONE);
        }
    }

    #[test]
    fn every_easing_is_anchored_and_named() {
        for e in [Easing::Linear, Easing::InOut, Easing::Out, Easing::BackOut] {
            assert!((e.at(0.0) - 0.0).abs() < 1e-9, "{}", e.name());
            assert!((e.at(1.0) - 1.0).abs() < 1e-9, "{}", e.name());
        }
        // `out` leaves fast: that is the only reason it exists.
        assert!(ease::out(0.15) > ease::in_out(0.15) * 2.0);
    }

    #[test]
    fn six_marks_lerp_all_the_way_across() {
        let mut to = Pose::ZERO;
        to.mark = Stroke::pad(
            &[Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 5.0,
                y1: 5.0,
                w: 3.0,
            }; MARKS],
        );
        let mid = Pose::lerp(Pose::ZERO, to, 0.5);
        for (i, m) in mid.mark.iter().enumerate() {
            assert!(m.w > 0.0, "stroke {i} was dropped by the lerp");
        }
    }

    #[test]
    fn spin_interpolates_like_every_other_number() {
        let from = Pose { spin: 0.0, ..a() };
        let to = Pose { spin: 2.0, ..a() };
        assert_eq!(Pose::lerp(from, to, 0.5).spin, 1.0);
    }
}
