// What parker does when nobody is talking to him.
//
// Videogame idle flair, and the solved part of that problem is pacing rather
// than animation: a character who does something charming every four seconds
// is not charming, and one who does it on a timer is a machine. The rules
// this file exists to enforce (MOTION.md §1.1):
//
//   1. a settling period — nothing at all for the first threequarters of a
//      minute, because flair immediately after you stop typing reads as a
//      reaction to you, which is worse than stillness;
//   2. never metronomic — every interval is jittered;
//   3. never twice running — the pool rotates;
//   4. escalation — the longer the quiet, the deeper the beat;
//   5. instantly interruptible — the caller departs from wherever the face
//      actually is, beat included, and then drops the beat.
//
// ## What the clock measures
//
// Not the user's idle time. `sense` has no seat-idle signal and inventing one
// would be a dependency this does not need — the honest input is *time since
// anything happened to parker*, which both surfaces already know locally
// (an expression change on both; a keystroke and an engine event in visage
// as well). It means the same thing and it costs nothing.
//
// ## No sleep state
//
// There was a sleep state — droops, Zs, a wake-with-a-start — and it is gone
// on purpose (August 2026). parker is an AI that is literally on; a face
// that pretends otherwise is a worse wallpaper. Deep idle now reads as
// parker doing his own thing: the same short beats, and eventually the
// *vignette* phase — rarer, longer, deliberate pieces. Two phases rather
// than four tiers, because the four were doing pacing work that two do just
// as well, and the deepest one existed only to reach a state that no longer
// exists.
//
// `nap` (vignette/nap.rs) is not that state coming back. It is a bit he
// does and finishes on his own, like every other vignette — and any key
// ends it, like every other beat.
//
// ## Two pools
//
// The short beats are tables, below. The vignettes are minutes long and
// built rather than typed (vignette.rs); `catalog()` is both, in one list.
// In the vignette phase the long pieces lead and the short beats are
// sprinkled between them (`DEEP_SHARE`), so deep idle reads as parker
// getting on with something rather than as the alert pool on a slower clock.

use super::beat::{Beat, Keyframe, Player};
use super::pose::{hash01, Delta, Easing, Field, Ink, Motion, Op, Pose, Stroke};
use super::vignette;
use std::sync::LazyLock;

// ── tiers ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Just been spoken to. Parker holds still; anything else reads as a
    /// reaction to the user rather than as idling.
    Settled,
    /// Awake and unoccupied: the short beats, one or two a minute.
    Alert,
    /// Alone long enough to start his own projects. The pool for this phase
    /// is the long deliberate pieces (plus every short beat, which stays
    /// eligible — losing the small ones as he settles would make deep idle
    /// *less* alive, which is backwards).
    Vignette,
}

impl Tier {
    pub fn of(idle_secs: f64) -> Tier {
        match idle_secs {
            s if s < 45.0 => Tier::Settled,
            s if s < 1800.0 => Tier::Alert,
            _ => Tier::Vignette,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tier::Settled => "settled",
            Tier::Alert => "alert",
            Tier::Vignette => "vignette",
        }
    }

    /// How long to wait between beats at this phase, in seconds. Wider and
    /// slower as parker settles: a face that fidgets every twenty seconds
    /// deep into the quiet is not settled, it is nervous.
    pub fn interval(self) -> (f64, f64) {
        match self {
            Tier::Settled => (f64::INFINITY, f64::INFINITY),
            Tier::Alert => (25.0, 55.0),
            Tier::Vignette => (60.0, 150.0),
        }
    }
}

// ── glyphs ──────────────────────────────────────────────────────────────────

const fn stroke(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Stroke {
    Stroke { x0, y0, x1, y1, w }
}

// ── the catalog ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Entry {
    pub beat: Beat,
    /// The shallowest tier this may fire at. It stays in the pool at every
    /// tier below it, so parker still glances mid-vignette-phase — the deep
    /// pool grows, it never shrinks.
    pub tier: Tier,
}

// ── the new batch (July 2026 redesign, in progress) ─────────────────────────
//
// The first catalog was authored from the plan and shipped unseen. Watched on
// a real screen, none of it survived: the beats were too short, too subtle,
// and had no staging — a value moved and moved back, which is a fidget, not a
// behaviour. These are written under different rules:
//
//   1. every beat is three acts — setup, event, reaction. The reaction is
//      the character; a beat that ends when the event ends is a screensaver.
//   2. the acting happens in the holds. Hold longer than feels right.
//   3. one readable subject per beat, legible from across a room.
//   4. 4–8 seconds. Nothing under one second is worth interrupting rest for.
//
// Batch two's screen notes: lateral movement gives depth (keep leans and
// walk-backs), the pupil must not vanish when it focuses, and abstract
// machine subjects beat naturalistic ones — a fly made no sense on a face
// that lives in cyberspace, a sonar sweep is at home there.
//
// Batch three's: transformations should be total — a sonar face with the
// idle ring still around it is a costume, not a becoming. Rare events earn
// length. And marks stopped being one glyph, which is why `mark_ink` grew
// to one ink per stroke.
//
// Batch four's: **no eyebrows, ever, unless asked for** — small angled
// strokes floating near the face read as brows from across the room, and
// brows are a louder signal than whatever the beat actually meant. And an
// instrument must find things at the moment of contact: a scan line that
// skips ahead to its own discovery reads as scripted, which it is — the
// job is to hide that.

/// A quaver: stem and head. Two strokes, so three of them is the whole mark
/// budget, which is also exactly a chord. The head is deliberately heavy —
/// underweighted, three floating dashes in a row read as eyebrows.
const fn quaver(x: f64, dy: f64, sw: f64, hw: f64) -> [Stroke; 2] {
    [
        stroke(x, dy - 7.0, x, dy + 1.2, sw),
        stroke(x - 3.2, dy + 1.3, x - 0.5, dy + 2.2, hw),
    ]
}

/// A chord at height `base`, mid-bob `dy` — the middle quaver bobbing
/// against the outer two, mirrored. Three dancers, not one object.
const fn trio_at(base: f64, dy: f64, sw: f64, hw: f64) -> [Stroke; 6] {
    let a = quaver(-12.0, base + dy, sw, hw);
    let b = quaver(0.0, base - dy, sw, hw);
    let c = quaver(12.0, base + dy, sw, hw);
    [a[0], a[1], b[0], b[1], c[0], c[1]]
}

static N_HID: [Stroke; 6] = trio_at(0.0, 0.0, 0.0, 0.0);
static N_0: [Stroke; 6] = trio_at(0.0, 0.0, 1.8, 3.6);
static N_1: [Stroke; 6] = trio_at(-0.8, -2.2, 1.8, 3.6);
static N_2: [Stroke; 6] = trio_at(-1.6, 1.6, 1.8, 3.6);
static N_3: [Stroke; 6] = trio_at(-2.4, -2.2, 1.8, 3.6);
static N_4: [Stroke; 6] = trio_at(-3.2, 1.6, 1.8, 3.6);
static N_5: [Stroke; 6] = trio_at(-4.0, -2.2, 1.8, 3.6);
static N_6: [Stroke; 6] = trio_at(-4.4, 0.0, 1.8, 3.6);

/// Red, white, red — the house colours, one per quaver.
const NOTES_INKS: [Ink; 6] = [
    Ink::Bad,
    Ink::Bad,
    Ink::Plain,
    Ink::Plain,
    Ink::Bad,
    Ink::Bad,
];

/// Something off to the right has parker's full attention.
///
/// The glancer done with staging: a lazy wander establishes normal, then the
/// snap breaks it — flick, freeze, focus, lean in. The target moves once, is
/// gone, and the tail is the character beat: he eases off, and then checks
/// back one more time. Interest is the holds, not the moves.
const SPOTTED: &[Keyframe] = &[
    // Act one — an ordinary wander, so the snap has something to break.
    Keyframe::new(
        Delta::num(&[(Field::GazeX, Op::Set(-2.0))]),
        0.5,
        Easing::InOut,
    ),
    Keyframe::hold(Delta::num(&[(Field::GazeX, Op::Set(-2.0))]), 0.4),
    // Act two — caught. Flick, then the focus lands a hair later.
    Keyframe::new(
        Delta::num(&[(Field::GazeX, Op::Set(4.6)), (Field::GazeY, Op::Set(-0.8))]),
        0.07,
        Easing::Out,
    ),
    Keyframe::new(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(-0.8)),
            (Field::Pupil, Op::Mul(0.82)),
        ]),
        0.15,
        Easing::Out,
    ),
    Keyframe::hold(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(-0.8)),
            (Field::Pupil, Op::Mul(0.82)),
        ]),
        0.9,
    ),
    // Lean in: approach plus widen is what interest looks like.
    Keyframe::new(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(-0.8)),
            (Field::Pupil, Op::Mul(0.82)),
            (Field::X, Op::Set(2.5)),
            (Field::W, Op::Mul(1.12)),
            (Field::H, Op::Mul(1.12)),
        ]),
        0.5,
        Easing::InOut,
    ),
    Keyframe::hold(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(-0.8)),
            (Field::Pupil, Op::Mul(0.82)),
            (Field::X, Op::Set(2.5)),
            (Field::W, Op::Mul(1.12)),
            (Field::H, Op::Mul(1.12)),
        ]),
        1.1,
    ),
    // It moves — a second fixation, still leaning.
    Keyframe::new(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(1.8)),
            (Field::Pupil, Op::Mul(0.82)),
            (Field::X, Op::Set(2.5)),
            (Field::W, Op::Mul(1.12)),
            (Field::H, Op::Mul(1.12)),
        ]),
        0.08,
        Easing::Out,
    ),
    Keyframe::hold(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(1.8)),
            (Field::Pupil, Op::Mul(0.82)),
            (Field::X, Op::Set(2.5)),
            (Field::W, Op::Mul(1.12)),
            (Field::H, Op::Mul(1.12)),
        ]),
        0.7,
    ),
    // Act three — lost it. A blink that keeps the lean, an ease-off, and
    // then the tell: one last check-back.
    Keyframe::new(
        Delta::num(&[
            (Field::GazeX, Op::Set(4.6)),
            (Field::GazeY, Op::Set(1.8)),
            (Field::X, Op::Set(2.5)),
            (Field::H, Op::Mul(0.05)),
        ]),
        0.09,
        Easing::InOut,
    ),
    Keyframe::new(
        Delta::num(&[(Field::GazeX, Op::Set(3.0)), (Field::X, Op::Set(0.8))]),
        0.4,
        Easing::InOut,
    ),
    Keyframe::hold(
        Delta::num(&[(Field::GazeX, Op::Set(3.0)), (Field::X, Op::Set(0.8))]),
        0.3,
    ),
    Keyframe::new(
        Delta::num(&[(Field::GazeX, Op::Set(4.2)), (Field::X, Op::Set(0.8))]),
        0.1,
        Easing::Out,
    ),
    Keyframe::hold(
        Delta::num(&[(Field::GazeX, Op::Set(4.2)), (Field::X, Op::Set(0.8))]),
        0.55,
    ),
];

/// The flat line every wave frame restates (deltas are measured from rest,
/// so the shape must keep being said).
const fn wave(amp: f64, freq: f64) -> [(Field, Op); 7] {
    [
        (Field::W, Op::Set(38.0)),
        (Field::H, Op::Set(1.4)),
        (Field::Round, Op::Set(0.0)),
        (Field::Ring, Op::Set(0.0)),
        (Field::Pupil, Op::Set(0.0)),
        (Field::Amp, Op::Set(amp)),
        (Field::Freq, Op::Set(freq)),
    ]
}

const FLAT: [(Field, Op); 7] = wave(0.0, 3.0);
const SURGE: [(Field, Op); 7] = wave(7.0, 12.0);

// The ladder down: a geometric convergence, each step smaller and quicker
// than the last. The maths is the show — you can see the limit coming.
const L_1: [(Field, Op); 7] = wave(7.0, 9.5);
const L_2: [(Field, Op); 7] = wave(7.0, 7.5);
const L_3: [(Field, Op); 7] = wave(7.0, 6.0);
const L_4: [(Field, Op); 7] = wave(7.0, 4.9);
const L_5: [(Field, Op); 7] = wave(7.0, 4.1);
const L_6: [(Field, Op); 7] = wave(7.0, 3.5);
const L_7: [(Field, Op); 7] = wave(7.0, 3.15);
const L_8: [(Field, Op); 7] = wave(7.0, 2.95);
const LOCKED: [(Field, Op); 7] = wave(8.0, 3.0);
const BLOOM: [(Field, Op); 7] = wave(9.6, 3.0);

const fn on_air(num: &'static [(Field, Op)], secs: f64, ease: Easing) -> Keyframe {
    Keyframe::new(
        Delta {
            num,
            mark: None,
            ink: Some(super::pose::Ink::Accent),
            mark_ink: None,
        },
        secs,
        ease,
    )
}

/// Dialing in a frequency.
///
/// Third cut. The static of the second was messy without being cool; this
/// one is the opposite bet — nothing on screen but one wave converging.
/// parker flattens, the wave surges in fast and busy, and then the knob
/// steps it down a geometric ladder: every step smaller and quicker than
/// the last, one deliberate undershoot at the end, then the lock — colour
/// snaps, the wave blooms once, and the pure tone holds. Watching it is
/// watching a limit being taken.
const DIAL: &[Keyframe] = &[
    Keyframe::new(Delta::num(&FLAT), 0.45, Easing::InOut),
    Keyframe::new(Delta::num(&SURGE), 0.4, Easing::Out),
    Keyframe::new(Delta::num(&L_1), 0.38, Easing::InOut),
    Keyframe::new(Delta::num(&L_2), 0.33, Easing::InOut),
    Keyframe::new(Delta::num(&L_3), 0.28, Easing::InOut),
    Keyframe::new(Delta::num(&L_4), 0.24, Easing::InOut),
    Keyframe::new(Delta::num(&L_5), 0.2, Easing::InOut),
    Keyframe::new(Delta::num(&L_6), 0.16, Easing::InOut),
    Keyframe::new(Delta::num(&L_7), 0.13, Easing::InOut),
    Keyframe::new(Delta::num(&L_8), 0.11, Easing::InOut),
    // Lock. Colour snaps with it, and the wave blooms once.
    on_air(&LOCKED, 0.1, Easing::Linear),
    on_air(&BLOOM, 0.18, Easing::Out),
    on_air(&LOCKED, 0.25, Easing::InOut),
    on_air(&LOCKED, 1.4, Easing::Linear),
];

/// What is left of parker while the notes have the stage: a core the size of
/// a dot, everything else lent out.
const GONE: &[(Field, Op)] = &[
    (Field::W, Op::Set(0.8)),
    (Field::H, Op::Set(0.8)),
    (Field::Ring, Op::Set(0.0)),
    (Field::Pupil, Op::Set(0.0)),
];

const fn noted(marks: &'static [Stroke], secs: f64, ease: Easing) -> Keyframe {
    Keyframe::new(
        Delta {
            num: GONE,
            mark: Some(marks),
            ink: None,
            mark_ink: Some(&NOTES_INKS),
        },
        secs,
        ease,
    )
}

/// parker becomes the music.
///
/// The orb shrinks to a core and three quavers take its place — red, white,
/// red — bobbing in opposition and rising as they go, the way notes leave a
/// speaker in every cartoon that ever drew one. The trip home is the trick
/// played backwards for free: the return lerp carries the risen chord back
/// down into the reforming face.
const NOTES: &[Keyframe] = &[
    // Teleport the hidden chord into place, so nothing streaks from centre.
    Keyframe::new(
        Delta {
            num: &[],
            mark: Some(&N_HID),
            ink: None,
            mark_ink: None,
        },
        0.01,
        Easing::Linear,
    ),
    noted(&N_0, 0.45, Easing::InOut),
    noted(&N_0, 0.3, Easing::Linear),
    noted(&N_1, 0.3, Easing::InOut),
    noted(&N_2, 0.3, Easing::InOut),
    noted(&N_3, 0.3, Easing::InOut),
    noted(&N_4, 0.3, Easing::InOut),
    noted(&N_5, 0.3, Easing::InOut),
    noted(&N_6, 0.3, Easing::InOut),
    noted(&N_6, 0.3, Easing::Linear),
];

// ── sonar ───────────────────────────────────────────────────────────────────
//
// A total transformation: parker becomes a sonar screen — a core dot and one
// thin white line turning clockwise. Nothing of the idle face survives it.
// The detection happens at the moment of contact, never before: a scan line
// that skips ahead to its own discovery reads as scripted.

/// The whole face reduced to the instrument's centre.
const CORE: &[(Field, Op)] = &[
    (Field::W, Op::Set(2.2)),
    (Field::H, Op::Set(2.2)),
    (Field::Ring, Op::Set(0.0)),
    (Field::Pupil, Op::Set(0.0)),
];

/// The scan line, from the centre out.
const fn ray(x: f64, y: f64) -> Stroke {
    stroke(0.0, 0.0, x, y, 1.8)
}

/// The contact, at bearing 120°, inside the sweep radius.
const fn ping(w: f64) -> Stroke {
    stroke(8.7, 5.0, 9.4, 5.5, w)
}

/// A spent stroke: nothing, nowhere.
const DEAD: Stroke = stroke(0.0, 0.0, 0.0, 0.0, 0.0);

/// The ripple: a diamond expanding from the contact — the closest thing to
/// an off-centre circle six straight strokes will admit.
const fn ripple(r: f64, w: f64) -> [Stroke; 4] {
    [
        stroke(9.0, 5.2 - r, 9.0 + r, 5.2, w),
        stroke(9.0 + r, 5.2, 9.0, 5.2 + r, w),
        stroke(9.0, 5.2 + r, 9.0 - r, 5.2, w),
        stroke(9.0 - r, 5.2, 9.0, 5.2 - r, w),
    ]
}

/// Slot layout for the whole beat: 0 the scan line (white), 1 the contact
/// (red), 2–5 ripple or beam (red). This is what per-stroke mark ink is for.
const RADAR_INKS: [Ink; 6] = [Ink::Plain, Ink::Bad, Ink::Bad, Ink::Bad, Ink::Bad, Ink::Bad];

// One revolution in eight stops, clockwise from twelve. The contact rides
// along invisibly so its pop happens in place — at contact, not before.
static R_000: [Stroke; 2] = [ray(0.0, -14.0), ping(0.0)];
static R_045: [Stroke; 2] = [ray(9.9, -9.9), ping(0.0)];
static R_090: [Stroke; 2] = [ray(14.0, 0.0), ping(0.0)];
static R_135: [Stroke; 2] = [ray(9.9, 9.9), ping(0.0)];
static R_180: [Stroke; 2] = [ray(0.0, 14.0), ping(0.0)];
static R_225: [Stroke; 2] = [ray(-9.9, 9.9), ping(0.0)];
static R_270: [Stroke; 2] = [ray(-14.0, 0.0), ping(0.0)];
static R_315: [Stroke; 2] = [ray(-9.9, -9.9), ping(0.0)];

/// The moment of contact: the line arrives on the bearing and the dot is
/// simply there, in the same frame.
static R_120_HIT: [Stroke; 2] = [ray(12.1, 7.0), ping(2.4)];

const fn rippling(r: f64, w: f64) -> [Stroke; 6] {
    let d = ripple(r, w);
    [ray(12.1, 7.0), ping(2.4), d[0], d[1], d[2], d[3]]
}

static RIP_A: [Stroke; 6] = rippling(2.5, 1.6);
static RIP_B: [Stroke; 6] = rippling(5.5, 0.0);

/// The line drawn back into the centre before the weapon comes out.
static RETRACTED: [Stroke; 2] = [DEAD, ping(2.4)];

/// The weapon parker becomes: a small tight face, eye on the bearing.
const TINY: &[(Field, Op)] = &[
    (Field::W, Op::Set(4.5)),
    (Field::H, Op::Set(4.5)),
    (Field::Ring, Op::Set(0.0)),
    (Field::Pupil, Op::Set(1.6)),
    (Field::GazeX, Op::Set(2.6)),
    (Field::GazeY, Op::Set(1.5)),
];

/// Aimed: the same weapon, aperture narrowed to a firing slit.
const TINY_AIM: &[(Field, Op)] = &[
    (Field::W, Op::Set(4.5)),
    (Field::H, Op::Set(2.0)),
    (Field::Ring, Op::Set(0.0)),
    (Field::Pupil, Op::Set(1.2)),
    (Field::GazeX, Op::Set(2.6)),
    (Field::GazeY, Op::Set(1.5)),
];

const fn beam_stroke(w: f64) -> Stroke {
    stroke(2.8, 1.9, 8.2, 4.7, w)
}

static L_ON: [Stroke; 3] = [DEAD, ping(2.4), beam_stroke(2.4)];
static L_OFF: [Stroke; 3] = [DEAD, ping(2.4), beam_stroke(0.0)];
static L_ZAP: [Stroke; 3] = [DEAD, ping(0.0), beam_stroke(2.4)];
static L_DONE: [Stroke; 3] = [DEAD, ping(0.0), beam_stroke(0.0)];

const fn radar(
    num: &'static [(Field, Op)],
    marks: &'static [Stroke],
    secs: f64,
    ease: Easing,
) -> Keyframe {
    Keyframe::new(
        Delta {
            num,
            mark: Some(marks),
            ink: None,
            mark_ink: Some(&RADAR_INKS),
        },
        secs,
        ease,
    )
}

/// parker becomes a sonar screen, and the sonar finds something.
///
/// The face contracts to a centre dot; one thin white line turns clockwise,
/// twice. A third of the way into the next pass the line arrives on the
/// bearing and the red contact is there in the same frame — detection at
/// the moment of contact. The line stops dead, one ripple rings out, then
/// the line draws back into the centre; only then does the core grow into a
/// small tight eye, narrow to a slit, and laser the contact off the screen.
/// A beat to confirm the kill, and the trip home regrows the whole idle
/// face from the little one.
const SONAR: &[Keyframe] = &[
    // Become the instrument.
    radar(CORE, &R_000, 0.5, Easing::InOut),
    // Two revolutions, steady.
    radar(CORE, &R_045, 0.3, Easing::Linear),
    radar(CORE, &R_090, 0.3, Easing::Linear),
    radar(CORE, &R_135, 0.3, Easing::Linear),
    radar(CORE, &R_180, 0.3, Easing::Linear),
    radar(CORE, &R_225, 0.3, Easing::Linear),
    radar(CORE, &R_270, 0.3, Easing::Linear),
    radar(CORE, &R_315, 0.3, Easing::Linear),
    radar(CORE, &R_000, 0.3, Easing::Linear),
    radar(CORE, &R_045, 0.3, Easing::Linear),
    radar(CORE, &R_090, 0.3, Easing::Linear),
    radar(CORE, &R_135, 0.3, Easing::Linear),
    radar(CORE, &R_180, 0.3, Easing::Linear),
    radar(CORE, &R_225, 0.3, Easing::Linear),
    radar(CORE, &R_270, 0.3, Easing::Linear),
    radar(CORE, &R_315, 0.3, Easing::Linear),
    radar(CORE, &R_000, 0.3, Easing::Linear),
    // Third pass — steady, still nothing...
    radar(CORE, &R_045, 0.3, Easing::Linear),
    radar(CORE, &R_090, 0.3, Easing::Linear),
    // — contact. The line reaches the bearing and stops dead, the dot
    // already under it.
    radar(CORE, &R_120_HIT, 0.2, Easing::Linear),
    radar(CORE, &R_120_HIT, 0.4, Easing::Linear),
    // One ripple rings out.
    radar(CORE, &RIP_A, 0.2, Easing::Out),
    radar(CORE, &RIP_B, 0.3, Easing::Out),
    radar(CORE, &R_120_HIT, 0.15, Easing::Linear),
    // The line draws back into the centre.
    radar(CORE, &RETRACTED, 0.45, Easing::InOut),
    radar(CORE, &RETRACTED, 0.2, Easing::Linear),
    // The instrument becomes a weapon.
    radar(TINY, &RETRACTED, 0.45, Easing::InOut),
    radar(TINY_AIM, &RETRACTED, 0.18, Easing::InOut),
    radar(TINY_AIM, &RETRACTED, 0.25, Easing::Linear),
    // Fire. Three pulses; the third one lands.
    radar(TINY_AIM, &L_ON, 0.12, Easing::Linear),
    radar(TINY_AIM, &L_OFF, 0.08, Easing::Linear),
    radar(TINY_AIM, &L_ON, 0.12, Easing::Linear),
    radar(TINY_AIM, &L_OFF, 0.08, Easing::Linear),
    radar(TINY_AIM, &L_ZAP, 0.3, Easing::Linear),
    // Confirm the kill: the slit relaxes, the eye stays on the bearing.
    radar(TINY, &L_DONE, 0.2, Easing::InOut),
    radar(TINY, &L_DONE, 0.8, Easing::Linear),
];

// ── pyramid ─────────────────────────────────────────────────────────────────
//
// Ambience: parker becomes a wireframe tetrahedron and turns in 3D, then is
// the orb again. No eye, no centre, nothing red — just the shape. A regular
// tetrahedron has exactly six edges, which is exactly the mark budget; the
// tables below are its vertices rotated about the vertical axis in 30°
// steps, pitched 20° toward the camera, projected orthographically
// (generated — the maths happened offline, marks lerping between stops is
// what makes it read as turning).

/// Nothing of the face at all. The marks are the whole body.
const VOID: &[(Field, Op)] = &[
    (Field::W, Op::Set(0.0)),
    (Field::H, Op::Set(0.0)),
    (Field::Ring, Op::Set(0.0)),
    (Field::Pupil, Op::Set(0.0)),
];

const fn faceted(marks: &'static [Stroke], secs: f64, ease: Easing) -> Keyframe {
    Keyframe::new(
        Delta {
            num: VOID,
            mark: Some(marks),
            ink: None,
            mark_ink: None,
        },
        secs,
        ease,
    )
}

const PYRAMID: &[Keyframe] = &[
    // Teleport the hidden wireframe into place, so nothing streaks from
    // centre, then trade the orb for it.
    Keyframe::new(
        Delta {
            num: &[],
            mark: Some(&PYR_HIDDEN),
            ink: None,
            mark_ink: None,
        },
        0.01,
        Easing::Linear,
    ),
    faceted(&PYR_00, 0.5, Easing::InOut),
    // One full turn.
    faceted(&PYR_01, 0.32, Easing::Linear),
    faceted(&PYR_02, 0.32, Easing::Linear),
    faceted(&PYR_03, 0.32, Easing::Linear),
    faceted(&PYR_04, 0.32, Easing::Linear),
    faceted(&PYR_05, 0.32, Easing::Linear),
    faceted(&PYR_06, 0.32, Easing::Linear),
    faceted(&PYR_07, 0.32, Easing::Linear),
    faceted(&PYR_08, 0.32, Easing::Linear),
    faceted(&PYR_09, 0.32, Easing::Linear),
    faceted(&PYR_10, 0.32, Easing::Linear),
    faceted(&PYR_11, 0.32, Easing::Linear),
    faceted(&PYR_00, 0.32, Easing::Linear),
    // A breath at rest, and the return collapses it back into the orb.
    faceted(&PYR_00, 0.25, Easing::Linear),
];

static PYR_00: [Stroke; 6] = [
    stroke(7.5, -4.5, 7.5, 4.5, 1.6),
    stroke(7.5, -4.5, -7.5, -9.6, 1.6),
    stroke(7.5, -4.5, -7.5, 9.6, 1.6),
    stroke(7.5, 4.5, -7.5, -9.6, 1.6),
    stroke(7.5, 4.5, -7.5, 9.6, 1.6),
    stroke(-7.5, -9.6, -7.5, 9.6, 1.6),
];
static PYR_01: [Stroke; 6] = [
    stroke(10.3, -6.1, 2.7, 3.5, 1.6),
    stroke(10.3, -6.1, -10.3, -8.0, 1.6),
    stroke(10.3, -6.1, -2.7, 10.6, 1.6),
    stroke(2.7, 3.5, -10.3, -8.0, 1.6),
    stroke(2.7, 3.5, -2.7, 10.6, 1.6),
    stroke(-10.3, -8.0, -2.7, 10.6, 1.6),
];
static PYR_02: [Stroke; 6] = [
    stroke(10.3, -8.0, -2.7, 3.5, 1.6),
    stroke(10.3, -8.0, -10.3, -6.1, 1.6),
    stroke(10.3, -8.0, 2.7, 10.6, 1.6),
    stroke(-2.7, 3.5, -10.3, -6.1, 1.6),
    stroke(-2.7, 3.5, 2.7, 10.6, 1.6),
    stroke(-10.3, -6.1, 2.7, 10.6, 1.6),
];
static PYR_03: [Stroke; 6] = [
    stroke(7.5, -9.6, -7.5, 4.5, 1.6),
    stroke(7.5, -9.6, -7.5, -4.5, 1.6),
    stroke(7.5, -9.6, 7.5, 9.6, 1.6),
    stroke(-7.5, 4.5, -7.5, -4.5, 1.6),
    stroke(-7.5, 4.5, 7.5, 9.6, 1.6),
    stroke(-7.5, -4.5, 7.5, 9.6, 1.6),
];
static PYR_04: [Stroke; 6] = [
    stroke(2.7, -10.6, -10.3, 6.1, 1.6),
    stroke(2.7, -10.6, -2.7, -3.5, 1.6),
    stroke(2.7, -10.6, 10.3, 8.0, 1.6),
    stroke(-10.3, 6.1, -2.7, -3.5, 1.6),
    stroke(-10.3, 6.1, 10.3, 8.0, 1.6),
    stroke(-2.7, -3.5, 10.3, 8.0, 1.6),
];
static PYR_05: [Stroke; 6] = [
    stroke(-2.7, -10.6, -10.3, 8.0, 1.6),
    stroke(-2.7, -10.6, 2.7, -3.5, 1.6),
    stroke(-2.7, -10.6, 10.3, 6.1, 1.6),
    stroke(-10.3, 8.0, 2.7, -3.5, 1.6),
    stroke(-10.3, 8.0, 10.3, 6.1, 1.6),
    stroke(2.7, -3.5, 10.3, 6.1, 1.6),
];
static PYR_06: [Stroke; 6] = [
    stroke(-7.5, -9.6, -7.5, 9.6, 1.6),
    stroke(-7.5, -9.6, 7.5, -4.5, 1.6),
    stroke(-7.5, -9.6, 7.5, 4.5, 1.6),
    stroke(-7.5, 9.6, 7.5, -4.5, 1.6),
    stroke(-7.5, 9.6, 7.5, 4.5, 1.6),
    stroke(7.5, -4.5, 7.5, 4.5, 1.6),
];
static PYR_07: [Stroke; 6] = [
    stroke(-10.3, -8.0, -2.7, 10.6, 1.6),
    stroke(-10.3, -8.0, 10.3, -6.1, 1.6),
    stroke(-10.3, -8.0, 2.7, 3.5, 1.6),
    stroke(-2.7, 10.6, 10.3, -6.1, 1.6),
    stroke(-2.7, 10.6, 2.7, 3.5, 1.6),
    stroke(10.3, -6.1, 2.7, 3.5, 1.6),
];
static PYR_08: [Stroke; 6] = [
    stroke(-10.3, -6.1, 2.7, 10.6, 1.6),
    stroke(-10.3, -6.1, 10.3, -8.0, 1.6),
    stroke(-10.3, -6.1, -2.7, 3.5, 1.6),
    stroke(2.7, 10.6, 10.3, -8.0, 1.6),
    stroke(2.7, 10.6, -2.7, 3.5, 1.6),
    stroke(10.3, -8.0, -2.7, 3.5, 1.6),
];
static PYR_09: [Stroke; 6] = [
    stroke(-7.5, -4.5, 7.5, 9.6, 1.6),
    stroke(-7.5, -4.5, 7.5, -9.6, 1.6),
    stroke(-7.5, -4.5, -7.5, 4.5, 1.6),
    stroke(7.5, 9.6, 7.5, -9.6, 1.6),
    stroke(7.5, 9.6, -7.5, 4.5, 1.6),
    stroke(7.5, -9.6, -7.5, 4.5, 1.6),
];
static PYR_10: [Stroke; 6] = [
    stroke(-2.7, -3.5, 10.3, 8.0, 1.6),
    stroke(-2.7, -3.5, 2.7, -10.6, 1.6),
    stroke(-2.7, -3.5, -10.3, 6.1, 1.6),
    stroke(10.3, 8.0, 2.7, -10.6, 1.6),
    stroke(10.3, 8.0, -10.3, 6.1, 1.6),
    stroke(2.7, -10.6, -10.3, 6.1, 1.6),
];
static PYR_11: [Stroke; 6] = [
    stroke(2.7, -3.5, 10.3, 6.1, 1.6),
    stroke(2.7, -3.5, -2.7, -10.6, 1.6),
    stroke(2.7, -3.5, -10.3, 8.0, 1.6),
    stroke(10.3, 6.1, -2.7, -10.6, 1.6),
    stroke(10.3, 6.1, -10.3, 8.0, 1.6),
    stroke(-2.7, -10.6, -10.3, 8.0, 1.6),
];
static PYR_HIDDEN: [Stroke; 6] = [
    stroke(7.5, -4.5, 7.5, 4.5, 0.0),
    stroke(7.5, -4.5, -7.5, -9.6, 0.0),
    stroke(7.5, -4.5, -7.5, 9.6, 0.0),
    stroke(7.5, 4.5, -7.5, -9.6, 0.0),
    stroke(7.5, 4.5, -7.5, 9.6, 0.0),
    stroke(-7.5, -9.6, -7.5, 9.6, 0.0),
];

const fn steered(name: &'static str, frames: &'static [Keyframe], ret: f64) -> Beat {
    Beat {
        name,
        frames,
        ret_secs: ret,
        loop_from: None,
        // Anything that steers the gaze must take the wheel from the resting
        // saccade, or the two add together and the look lands off the eye.
        motion: Some(Motion::None),
    }
}

const fn plain(name: &'static str, frames: &'static [Keyframe], ret: f64) -> Beat {
    Beat {
        name,
        frames,
        ret_secs: ret,
        loop_from: None,
        motion: None,
    }
}

/// The short beats, written out as tables. `catalog()` is these plus the
/// vignettes.
static BEATS: &[Entry] = &[
    Entry {
        beat: steered("spotted", SPOTTED, 0.5),
        tier: Tier::Alert,
    },
    Entry {
        beat: Beat {
            name: "dial",
            frames: DIAL,
            ret_secs: 0.6,
            loop_from: None,
            motion: Some(Motion::Wave),
        },
        tier: Tier::Alert,
    },
    Entry {
        beat: plain("notes", NOTES, 0.55),
        tier: Tier::Alert,
    },
    Entry {
        beat: steered("sonar", SONAR, 0.5),
        tier: Tier::Alert,
    },
    Entry {
        beat: plain("pyramid", PYRAMID, 0.6),
        tier: Tier::Alert,
    },
];

/// The short beats alone — what `parker face --catalog` writes out; the
/// vignettes go out on their own (catalog.rs).
pub fn tables() -> &'static [Entry] {
    BEATS
}

/// Every beat parker can play: the short tables above, then the vignettes.
///
/// The vignettes are built on first use (vignette.rs) and kept for the life
/// of the process. The short set pieces among them — a shine, a power cycle,
/// a warp, a sort — are short enough for the alert pool; the minutes-long
/// ones wait for the vignette phase.
pub fn catalog() -> &'static [Entry] {
    static ALL: LazyLock<Vec<Entry>> = LazyLock::new(|| {
        let alert = [
            vignette::shine::beat(),
            vignette::power::beat(),
            vignette::warp::beat(),
            vignette::sort::beat(),
        ];
        let deep = [
            vignette::nap::beat(),
            vignette::chores::beat(),
            vignette::bonsai::beat(),
            vignette::reading::beat(),
            vignette::pong::beat(),
        ];
        let mut all = BEATS.to_vec();
        all.extend(alert.into_iter().map(|beat| Entry {
            beat,
            tier: Tier::Alert,
        }));
        all.extend(deep.into_iter().map(|beat| Entry {
            beat,
            tier: Tier::Vignette,
        }));
        all
    });
    &ALL
}

/// In the vignette phase, how often the next beat is one of the long pieces
/// rather than a short one. The short beats stay in the deep pool (it grows,
/// it never shrinks) but as a sprinkle: with five vignettes against nine
/// short beats, an even draw would make deep idle mostly the alert pool on a
/// slower clock.
pub const DEEP_SHARE: f64 = 0.7;

/// A cued beat (`parker face --play`) waits for this much quiet before it
/// plays. Rest alone is not enough: the end of a turn lets the face go and
/// then brings a second or two of engine chatter, each event a stir, and a
/// cue played on the first restful frame was cut down by the next one.
pub const CUE_SETTLE: f64 = 4.0;

/// How long a cue waits for that rest before it is dropped: a piece asked
/// for ten minutes ago is not the one wanted now. The same ten minutes as a
/// forgotten posture's safety net, so a cue sent mid-turn outlives the turn.
pub const CUE_WAIT: f64 = 600.0;

// ── the clock ───────────────────────────────────────────────────────────────

/// Chooses beats and plays them. One per surface; parkerd and visage each
/// keep their own, because the clock they measure is their own.
#[derive(Debug)]
pub struct Idle {
    player: Player,
    next_at: f64,
    last: &'static str,
    seed: u64,
    step: u64,
    /// A beat asked for by name, and when: played at the next rest.
    cued: Option<(String, f64)>,
}

impl Idle {
    /// `seed` only decorrelates two surfaces running side by side; nothing
    /// depends on it being unpredictable. Tests pass 0.
    pub fn new(seed: u64) -> Idle {
        Idle {
            player: Player::new(),
            next_at: 0.0,
            last: "",
            seed,
            step: 0,
            cued: None,
        }
    }

    fn roll(&mut self) -> f64 {
        self.step += 1;
        hash01(self.step, self.seed ^ 0x1D1E)
    }

    /// Advance. `idle_secs` is time since anything happened to parker.
    pub fn tick(&mut self, now: f64, idle_secs: f64) {
        self.player.tick(now);
        if let Some((name, at)) = self.cued.take() {
            if now - at > CUE_WAIT {
                // Asked for too long ago: dropped.
            } else if idle_secs < CUE_SETTLE {
                self.cued = Some((name, at));
            } else if self.play_named(&name, now) {
                // A request, so it plays over whatever the clock had rolled.
                return;
            }
        }
        if self.player.is_playing() {
            return;
        }

        let tier = Tier::of(idle_secs);
        if tier == Tier::Settled {
            // Keep the clock pushed out ahead of the settling period so that
            // the first beat after a conversation is not instant.
            self.next_at = now + Self::jitter(tier, self.seed, self.step);
            return;
        }
        if now < self.next_at {
            return;
        }

        let Some(beat) = self.choose(tier) else {
            self.next_at = now + 5.0;
            return;
        };
        self.last = beat.name;
        self.player.play(beat, now);
        self.next_at = now + beat.secs() + Self::jitter(tier, self.seed, self.step);
    }

    fn jitter(tier: Tier, seed: u64, step: u64) -> f64 {
        let (lo, hi) = tier.interval();
        if !lo.is_finite() {
            return 45.0;
        }
        lo + hash01(step, seed ^ 0x9A17) * (hi - lo)
    }

    /// Pick from everything eligible at this tier, never the same one twice
    /// running. The no-repeat rule is worth more than it looks: two glances
    /// in a row read as a bug, not as a character.
    ///
    /// In the vignette phase one roll first decides which pool — the long
    /// pieces (`DEEP_SHARE` of the time) or the short ones — and a second
    /// picks within it.
    fn choose(&mut self, tier: Tier) -> Option<&'static Beat> {
        let deep = tier == Tier::Vignette && self.roll() < DEEP_SHARE;
        let fits = |e: &&'static Entry| {
            if tier == Tier::Vignette {
                (e.tier == Tier::Vignette) == deep
            } else {
                e.tier <= tier
            }
        };
        let pool: Vec<&'static Entry> = catalog()
            .iter()
            .filter(fits)
            .filter(|e| e.beat.name != self.last)
            .collect();
        let pool = if pool.is_empty() {
            catalog().iter().filter(fits).collect()
        } else {
            pool
        };
        if pool.is_empty() {
            return None;
        }
        let pick = (self.roll() * pool.len() as f64) as usize;
        Some(&pool[pick.min(pool.len() - 1)].beat)
    }

    /// Something happened. Drop whatever is playing and restart the settling
    /// period. Nothing to wake from anymore: sleep went with the drowsy tier.
    pub fn interrupt(&mut self, now: f64) {
        self.player.stop();
        self.next_at = now + 45.0;
    }

    /// Ask for a beat by name, to play at the next rest — the beat slot's
    /// landing point (`parker face --play`, face/state.rs). Only `tick` plays
    /// it, and a surface ticks only over the resting face, so a cue sent while
    /// parker is mid-turn or wearing a face waits for him to settle
    /// (`CUE_SETTLE`) instead of being dropped. An interrupt does not clear
    /// it: a stir postpones a cue, it does not cancel one. A newer cue
    /// replaces it; an unknown name is dropped when it comes due.
    pub fn cue(&mut self, name: &str, now: f64) {
        self.cued = Some((name.to_string(), now));
    }

    /// Play a beat by name, right now — where a cue lands once it is due. A
    /// request, not a choice: it does not touch the idle clock, so the pool's
    /// own next beat still arrives when it was always going to.
    pub fn play_named(&mut self, name: &str, now: f64) -> bool {
        let Some(beat) = catalog()
            .iter()
            .find(|e| e.beat.name == name)
            .map(|e| &e.beat)
        else {
            return false;
        };
        self.last = beat.name;
        self.player.play(beat, now);
        true
    }

    pub fn pose(&self, base: Pose, now: f64) -> Pose {
        self.player.pose(base, now)
    }

    pub fn motion(&self, base: Motion, now: f64) -> Motion {
        self.player.motion(base, now)
    }

    /// Whether an ordinary blink should be drawn on top. Always at rest;
    /// never while a beat is steering the eye, or the beat and the blink
    /// fight over `h`; and in the stretches of a vignette that let it through.
    pub fn blinkable(&self, now: f64) -> bool {
        !self.player.is_playing() || self.player.blinks(now)
    }
}

/// Inspection, for tests and for the visage port to mirror (see the same
/// note on `beat::Player`).
#[allow(dead_code)]
impl Idle {
    pub fn playing(&self) -> Option<&'static str> {
        self.player.playing().map(|b| b.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle_pose() -> Pose {
        crate::face::Expression::Idle.pose()
    }

    #[test]
    fn tiers_escalate_at_the_documented_boundaries() {
        assert_eq!(Tier::of(0.0), Tier::Settled);
        assert_eq!(Tier::of(44.0), Tier::Settled);
        assert_eq!(Tier::of(45.0), Tier::Alert);
        assert_eq!(Tier::of(1799.0), Tier::Alert);
        assert_eq!(Tier::of(1800.0), Tier::Vignette);
        assert!(Tier::Settled < Tier::Alert && Tier::Alert < Tier::Vignette);
    }

    #[test]
    fn nothing_at_all_happens_while_parker_is_settled() {
        // Rule 1, and the one most likely to be quietly broken: flair right
        // after you stop typing reads as a reaction to you.
        let mut idle = Idle::new(0);
        let mut t = 0.0;
        while t < 44.0 {
            idle.tick(t, t);
            assert!(idle.playing().is_none(), "a beat fired at {t}s");
            t += 0.5;
        }
    }

    #[test]
    fn beats_eventually_fire_once_parker_is_alert() {
        let mut idle = Idle::new(7);
        let mut fired = 0;
        let mut t = 45.0;
        while t < 400.0 {
            idle.tick(t, t);
            if idle.playing().is_some() {
                fired += 1;
            }
            t += 0.25;
        }
        assert!(fired > 0, "nothing ever happened in six minutes of idling");
    }

    #[test]
    fn the_same_beat_never_fires_twice_running() {
        let mut idle = Idle::new(3);
        let mut seen: Vec<&'static str> = Vec::new();
        let mut t = 45.0;
        while t < 3000.0 {
            let before = idle.playing();
            idle.tick(t, t);
            let after = idle.playing();
            if before.is_none() {
                if let Some(name) = after {
                    seen.push(name);
                }
            }
            t += 0.2;
        }
        assert!(seen.len() > 8, "only {} beats in the sample", seen.len());
        for pair in seen.windows(2) {
            assert_ne!(pair[0], pair[1], "{} fired twice running", pair[0]);
        }
    }

    #[test]
    fn intervals_are_jittered_rather_than_metronomic() {
        let mut idle = Idle::new(11);
        let mut starts: Vec<f64> = Vec::new();
        let mut t = 45.0;
        while t < 4000.0 {
            let before = idle.playing().is_some();
            idle.tick(t, t.min(1100.0));
            if !before && idle.playing().is_some() {
                starts.push(t);
            }
            t += 0.2;
        }
        let gaps: Vec<f64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.len() > 5);
        let spread = gaps.iter().cloned().fold(0.0_f64, f64::max)
            - gaps.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread > 5.0, "gaps were near-identical: {gaps:?}");
    }

    #[test]
    fn short_beats_survive_into_the_vignette_phase() {
        // The deep pool grows, it never shrinks: losing the small beats as
        // parker settles would make deep idle *less* alive.
        let mut idle = Idle::new(5);
        assert!(idle.choose(Tier::Vignette).is_some());
        let pool: Vec<&str> = catalog()
            .iter()
            .filter(|e| e.tier <= Tier::Vignette)
            .map(|e| e.beat.name)
            .collect();
        assert!(pool.contains(&"spotted"));
    }

    #[test]
    fn an_interrupt_stops_whatever_was_playing() {
        let mut idle = Idle::new(9);
        let mut t = 45.0;
        while t < 400.0 && idle.playing().is_none() {
            idle.tick(t, t);
            t += 0.2;
        }
        assert!(idle.playing().is_some());
        idle.interrupt(t);
        assert!(idle.playing().is_none());
    }

    #[test]
    fn a_cue_waits_for_rest_and_then_plays() {
        let mut idle = Idle::new(9);
        idle.cue("sort", 10.0);
        // Mid-turn the surface does not tick at all. Then the turn ends and
        // the face lets go, but the engine is still chattering.
        idle.tick(30.0, 0.5);
        assert!(idle.playing().is_none(), "played before parker settled");
        idle.interrupt(30.5);
        idle.tick(31.0, 0.5);
        assert!(idle.playing().is_none(), "an interrupt is not a rest");
        idle.tick(35.0, CUE_SETTLE);
        assert_eq!(idle.playing(), Some("sort"));
        // Once played, it is spent.
        idle.interrupt(36.0);
        idle.tick(50.0, 14.0);
        assert_ne!(idle.playing(), Some("sort"));
    }

    #[test]
    fn a_stale_cue_is_dropped() {
        let mut idle = Idle::new(9);
        idle.cue("sort", 0.0);
        idle.tick(CUE_WAIT + 1.0, CUE_SETTLE);
        assert_ne!(idle.playing(), Some("sort"));
    }

    #[test]
    fn no_beat_holds_the_face_forever() {
        // Sleep was the one terminal beat, and sleep is gone. If a loop
        // shows up in the catalog again it had better be on purpose.
        for e in catalog() {
            assert!(
                !e.beat.terminal(),
                "{} would hold the face forever",
                e.beat.name
            );
        }
    }

    #[test]
    fn every_beat_in_the_catalog_returns_the_face_to_rest() {
        // The property that makes the catalog safe to extend: no entry can
        // strand parker mid-beat.
        for e in catalog() {
            let base = idle_pose();
            let end = e.beat.at(base, e.beat.secs() + 1.0);
            for f in Field::ALL {
                assert!(
                    (end.get(f) - base.get(f)).abs() < 1e-9,
                    "{} left {} at {} instead of {}",
                    e.beat.name,
                    f.name(),
                    end.get(f),
                    base.get(f)
                );
            }
            for (i, m) in end.mark.iter().enumerate() {
                assert!(m.w < 1e-9, "{} left mark {i} drawn", e.beat.name);
            }
        }
    }

    #[test]
    fn every_beat_keeps_the_face_on_the_canvas() {
        // Dot units on a 96x40 grid, drawn from the centre: anything past 48
        // across or 20 down is off-screen. Measured where it is drawn — a
        // pot that sits low (bonsai) is fine, a pot that falls off is not.
        let on = |x: f64, y: f64| x.abs() <= 48.0 && y.abs() <= 20.0;
        for e in catalog().iter().map(|e| &e.beat) {
            let mut t = 0.0;
            while t < e.secs() + 3.0 {
                let p = e.at(idle_pose(), t);
                let at = format!("{} at {t:.2}s", e.name);
                assert!(p.w >= 0.0 && p.w < 47.0, "{at}: w {}", p.w);
                assert!(p.h >= 0.0 && p.h < 20.0, "{at}: h {}", p.h);
                if p.w > 0.05 {
                    assert!(p.x.abs() + p.gap + p.w <= 48.5, "{at}: cell off the side");
                    assert!(
                        p.y.abs() + p.tilt.abs() + p.h <= 20.5,
                        "{at}: cell off the edge"
                    );
                }
                if p.ring > 1.0 {
                    assert!(
                        p.y.abs() + p.ring + p.ring_w / 2.0 <= 21.0,
                        "{at}: ring off"
                    );
                }
                assert!(p.gaze_x.abs() < 8.0, "{at}: gaze {}", p.gaze_x);
                // A stroke may run off the edge (a streak leaving the frame,
                // a ball out of play) but never be drawn entirely off it.
                for m in p.mark.iter().filter(|m| m.w > 0.05) {
                    assert!(
                        on(p.x + m.x0, p.y + m.y0) || on(p.x + m.x1, p.y + m.y1),
                        "{at}: a stroke drawn wholly off the canvas"
                    );
                }
                t += 0.02;
            }
        }
    }

    #[test]
    fn a_beat_that_steers_the_gaze_takes_the_wheel() {
        // Otherwise the resting saccade adds to a gaze the beat is setting.
        // Per keyframe, because a vignette's motion changes by section.
        for e in catalog() {
            for (i, k) in e.beat.frames.iter().enumerate() {
                let steers =
                    k.to.num
                        .iter()
                        .any(|(f, _)| matches!(f, Field::GazeX | Field::GazeY));
                let motion = k.motion.or(e.beat.motion);
                if steers {
                    assert!(
                        motion.is_some_and(|m| m != Motion::Drift),
                        "{} keyframe {i} moves the gaze but leaves the saccade running",
                        e.beat.name
                    );
                }
            }
        }
    }

    #[test]
    fn blinking_stops_while_a_beat_owns_the_eye() {
        let mut idle = Idle::new(9);
        assert!(idle.blinkable(0.0));
        assert!(idle.play_named("spotted", 0.0));
        assert!(!idle.blinkable(1.0), "a blink would fight the beat over h");
    }

    #[test]
    fn a_vignette_lets_the_blink_through_where_it_says() {
        // Reading is minutes of an open eye: its lines blink. Shine's
        // windup does not — the eye is acting.
        let reading = catalog().iter().find(|e| e.beat.name == "reading").unwrap();
        let blinks = reading.beat.frames.iter().filter(|k| k.blink).count();
        assert!(
            blinks * 2 > reading.beat.frames.len(),
            "reading barely blinks"
        );

        let mut idle = Idle::new(3);
        assert!(idle.play_named("reading", 0.0));
        assert!(idle.blinkable(20.0), "two minutes of reading with no blink");
        assert!(idle.play_named("shine", 0.0));
        let windup = catalog()
            .iter()
            .find(|e| e.beat.name == "shine")
            .unwrap()
            .beat
            .frames
            .iter()
            .position(|k| k.to.ink == Some(Ink::Bad))
            .unwrap();
        let began: f64 = catalog()
            .iter()
            .find(|e| e.beat.name == "shine")
            .unwrap()
            .beat
            .frames[..windup]
            .iter()
            .map(|k| k.secs)
            .sum();
        assert!(!idle.blinkable(began + 0.1), "a blink mid-windup");
    }

    #[test]
    fn a_vignette_breathes_in_its_sleep() {
        let nap = catalog().iter().find(|e| e.beat.name == "nap").unwrap();
        assert_eq!(nap.beat.motion_at(Motion::Drift, 60.0), Motion::Breathe);
        assert_eq!(
            nap.beat.motion_at(Motion::Drift, 1.0),
            Motion::None,
            "the yawn steers"
        );
    }

    #[test]
    fn the_vignette_phase_leads_with_vignettes() {
        let mut idle = Idle::new(11);
        let (mut deep, n) = (0, 400);
        for _ in 0..n {
            let b = idle.choose(Tier::Vignette).unwrap();
            idle.last = b.name;
            if catalog()
                .iter()
                .any(|e| e.beat.name == b.name && e.tier == Tier::Vignette)
            {
                deep += 1;
            }
        }
        let share = deep as f64 / n as f64;
        assert!(
            (0.6..0.8).contains(&share),
            "vignettes were {share} of picks"
        );
    }

    #[test]
    fn the_alert_phase_never_plays_a_vignette() {
        let mut idle = Idle::new(13);
        for _ in 0..300 {
            let b = idle.choose(Tier::Alert).unwrap();
            idle.last = b.name;
            let e = catalog().iter().find(|e| e.beat.name == b.name).unwrap();
            assert_eq!(e.tier, Tier::Alert, "{} is a vignette", b.name);
        }
    }

    #[test]
    fn short_beats_are_short_and_vignettes_end_within_five_minutes() {
        for e in catalog() {
            let limit = if e.tier == Tier::Vignette {
                300.0
            } else {
                30.0
            };
            assert!(
                e.beat.secs() <= limit,
                "{} runs {:.0}s, past {limit}s",
                e.beat.name,
                e.beat.secs()
            );
        }
    }

    #[test]
    fn every_catalog_name_is_unique() {
        let names: std::collections::HashSet<&str> =
            catalog().iter().map(|e| e.beat.name).collect();
        assert_eq!(names.len(), catalog().len(), "two beats share a name");
    }
}
