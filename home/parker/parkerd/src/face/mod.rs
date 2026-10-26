// parker's face.
//
// Not one face, and not always a face. An expression is a named pose
// (pose.rs); because poses are numbers, any expression flows into any other by
// interpolation. Some of them aren't faces at all — a check and a cross are
// strokes, and they morph in and out of eyes like everything else does, which
// is what keeps "parker expresses differently" from meaning "special cases".
//
// The face is parker's to wear. *Every* expression is chosen — through `reach
// face` — and no surface ever asserts one on its own (the 2026-08 redesign,
// DECISIONS.md): progress lives in the transcript, not on the face. `Trouble`
// used to be carved out as a machine assertion; it is not anymore. A face
// nobody picked is a face that changes while parker is saying nothing, and
// one flapping engine spent it seven times in fifteen seconds.
//
// Thirty-seven expressions: the original twelve, the redesign's twelve
// (docs/visage/redesign/README.md), and the faces round's thirteen (Oct 2026,
// the "visage — faces" canvas, DECISIONS.md) — three postures (waiting,
// weighing, vibing) and ten reactions, most of them gestures or jokes. The
// same round redrew eleven of the old ones under their old names: eight
// leaned on `squint`, which only shrinks the right eye, and read as lopsided
// rather than narrowed, and closed eyes drawn as cells are flat pills, which
// is deadpan's shape whatever ink they wear. A name and its meaning are what
// parker has learned; the look is free to change under them.
//
// A reaction may also have an *entrance* (entrance.rs): a few keyframes it
// plays as it lands, departing from wherever the face was, before it settles
// into its pose. An eye-roll is the roll, not the ceiling it ends on.
// Distinctness is still the bar for adding more.

pub mod beat;
pub mod braille;
pub mod catalog;
pub mod dance;
pub mod entrance;
pub mod idle;
pub mod log;
pub mod paint;
pub mod pose;
pub mod state;
pub mod vignette;

use braille::Grid;
use pose::{Ink, Motion, Pose, Stroke};
use std::f64::consts::TAU;

/// The canvas, in terminal cells. 48x10 cells is 96x40 dots, and a braille
/// dot is near square at a normal 1:2 cell aspect, so circles come out round.
pub const COLS: i32 = 48;
pub const ROWS: i32 = 10;

/// The window class the face runs under.
///
/// It must contain a dot: ghostty silently *ignores* `--class` unless the
/// value is a valid GTK application id, so `parker-face` produced a window
/// still claiming to be `com.mitchellh.ghostty` — and every window rule keyed
/// on it quietly did nothing. Verified live before fixing.
///
/// Kept here rather than only in nix because sense needs it too: parker's own
/// face must not count as a reason to leave the user alone.
/// home/desktop/hyprland.nix and home/parker/default.nix repeat the literal.
pub const FACE_CLASS: &str = "com.parker.face";

/// The longest a posture holds when nobody gave a duration: ten minutes.
/// Long enough that `away` or `listening` survives a quiet stretch, short
/// enough that a forgotten one does not sit on the face all evening. An
/// explicit hold of 0 still means "until something changes it" — the cap is
/// a net for forgetting, not a ban on choosing.
pub const POSTURE_CAP: f64 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expression {
    // ── the original twelve. The involuntary/voluntary split these two
    // groups used to name is gone — every one of them is parker's to wear —
    // but the *order* is load-bearing (catalog consumers key on it), so they
    // stay grouped as they were.
    Idle,
    Thinking,
    Speaking,
    Blocked,
    Away,
    Trouble,
    Yes,
    No,
    Curious,
    Surprised,
    Amused,
    Wince,
    // ── the redesign's twelve (docs/visage/redesign) — appended, never
    // reordered: catalog consumers key on order surviving. Work postures
    // first, then reactions, then the one that is half a gesture.
    Listening,
    Skimming,
    Digging,
    Smug,
    Sheepish,
    Deadpan,
    Fond,
    Melancholy,
    Conspiratorial,
    Delighted,
    Suspicious,
    Pointing,
    // ── the faces round's thirteen (Oct 2026) — appended for the same
    // reason. Postures first, then the reactions.
    Waiting,
    Weighing,
    Vibing,
    Disapproval,
    EyeRoll,
    DoubleTake,
    Shrug,
    TableFlip,
    Dead,
    Dizzy,
    Glitch,
    Creeper,
    Uwu,
}

use Expression::*;

// The four primitives every face is built from, plus the strokes.

/// A single aperture: cells merged, round, inside a ring.
fn iris(w: f64, ring: f64) -> Pose {
    Pose {
        gap: 0.0,
        w,
        h: w,
        round: 1.0,
        ring,
        ring_w: 2.0,
        ..Pose::ZERO
    }
}

/// A pair of eyes.
fn eyes(w: f64, h: f64, pupil: f64) -> Pose {
    Pose {
        gap: 18.0,
        w,
        h,
        round: 0.85,
        pupil,
        ..Pose::ZERO
    }
}

/// One flat cell, optionally undulating.
fn line(w: f64, amp: f64, freq: f64) -> Pose {
    Pose {
        gap: 0.0,
        w,
        h: 1.4,
        round: 0.0,
        amp,
        freq,
        ..Pose::ZERO
    }
}

/// A stroke at the weight the glyphs use.
fn stroke(x0: f64, y0: f64, x1: f64, y1: f64) -> Stroke {
    s(x0, y0, x1, y1, 3.0)
}

/// A stroke at any weight.
fn s(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Stroke {
    Stroke { x0, y0, x1, y1, w }
}

/// A dot is a stroke of no length.
fn dot(x: f64, y: f64, w: f64) -> Stroke {
    s(x, y, x, y, w)
}

/// A closed happy eye, ^, drawn: `half` wide each way, its peak `rise` above
/// the feet. Cells cannot draw an arc — closed, they are pills, and a pill is
/// deadpan — so the smiles are strokes.
fn caret(cx: f64, y: f64, half: f64, rise: f64, w: f64) -> [Stroke; 2] {
    [
        s(cx - half, y, cx, y - rise, w),
        s(cx, y - rise, cx + half, y, w),
    ]
}

/// Sunglasses with their lenses at height `y`: two fat lenses, the bridge,
/// the arms. (⌐■_■)
fn shades(y: f64) -> [Stroke; pose::MARKS] {
    Stroke::pad(&[
        s(-24.0, y, -12.0, y, 7.0),
        s(12.0, y, 24.0, y, 7.0),
        s(-12.0, y - 2.0, 12.0, y - 2.0, 1.6),
        s(-27.0, y - 2.2, -37.0, y - 3.5, 1.6),
        s(27.0, y - 2.2, 37.0, y - 3.5, 1.6),
    ])
}

/// ¯\_(ツ)_/¯'s arms — three strokes each, shoulder, forearm, hand — with
/// `lift` 0 for the shrug and more for arms on their way up.
fn shrug_arms(lift: f64) -> [Stroke; pose::MARKS] {
    let (hand, elbow) = (-6.0 + lift * 20.0, 3.0 + lift * 5.0);
    Stroke::pad(&[
        s(-12.0, 3.0, -23.0, elbow, 2.2),
        s(-23.0, elbow, -30.0, hand, 2.2),
        s(-30.0, hand, -37.0, hand, 2.2),
        s(12.0, 3.0, 23.0, elbow, 2.2),
        s(23.0, elbow, 30.0, hand, 2.2),
        s(30.0, hand, 37.0, hand, 2.2),
    ])
}

/// The cartoon knockout's star: a little plus.
fn plus(x: f64, y: f64) -> [Stroke; 2] {
    [
        s(x - 2.2, y, x + 2.2, y, 1.6),
        s(x, y - 2.2, x, y + 2.2, 1.6),
    ]
}

impl Expression {
    pub const ALL: [Expression; 37] = [
        Idle,
        Thinking,
        Speaking,
        Blocked,
        Away,
        Trouble,
        Yes,
        No,
        Curious,
        Surprised,
        Amused,
        Wince,
        Listening,
        Skimming,
        Digging,
        Smug,
        Sheepish,
        Deadpan,
        Fond,
        Melancholy,
        Conspiratorial,
        Delighted,
        Suspicious,
        Pointing,
        Waiting,
        Weighing,
        Vibing,
        Disapproval,
        EyeRoll,
        DoubleTake,
        Shrug,
        TableFlip,
        Dead,
        Dizzy,
        Glitch,
        Creeper,
        Uwu,
    ];

    /// Expressions parker may wear — all of them, which is why this is just
    /// `ALL` under a name that says why you are reading it. Two carve-outs
    /// died here: derived states like `Thinking` cannot be asked for (the
    /// 2026-08 redesign reversed it — the app infers nothing now), and
    /// `Trouble` is the machine's to assert (removed after it fired seven
    /// times in fifteen seconds on one bouncing engine). Nothing puts a face
    /// on parker but parker.
    pub const WEARABLE: [Expression; 37] = Self::ALL;

    /// The expressive faces — reactions, not postures. These land with a
    /// spring (back_out) where everything else eases, unless they have an
    /// entrance, which is how they land instead; the split is about how a
    /// face *arrives*, not who chose it, which is why `listening` and
    /// `thinking` are absent even though both are wearable now.
    pub const VOLUNTARY: [Expression; 24] = [
        Yes,
        No,
        Curious,
        Surprised,
        Amused,
        Wince,
        Smug,
        Sheepish,
        Deadpan,
        Fond,
        Melancholy,
        Conspiratorial,
        Delighted,
        Suspicious,
        Disapproval,
        EyeRoll,
        DoubleTake,
        Shrug,
        TableFlip,
        Dead,
        Dizzy,
        Glitch,
        Creeper,
        Uwu,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Idle => "idle",
            Thinking => "thinking",
            Speaking => "speaking",
            Blocked => "blocked",
            Away => "away",
            Trouble => "trouble",
            Yes => "yes",
            No => "no",
            Curious => "curious",
            Surprised => "surprised",
            Amused => "amused",
            Wince => "wince",
            Listening => "listening",
            Skimming => "skimming",
            Digging => "digging",
            Smug => "smug",
            Sheepish => "sheepish",
            Deadpan => "deadpan",
            Fond => "fond",
            Melancholy => "melancholy",
            Conspiratorial => "conspiratorial",
            Delighted => "delighted",
            Suspicious => "suspicious",
            Pointing => "pointing",
            Waiting => "waiting",
            Weighing => "weighing",
            Vibing => "vibing",
            Disapproval => "disapproval",
            EyeRoll => "eye-roll",
            DoubleTake => "double-take",
            Shrug => "shrug",
            TableFlip => "table-flip",
            Dead => "dead",
            Dizzy => "dizzy",
            Glitch => "glitch",
            Creeper => "creeper",
            Uwu => "uwu",
        }
    }

    /// What each face *means*, in one line. The names are English words and
    /// English words are approximate: nothing else here says how `amused`
    /// differs from `delighted`, or that `conspiratorial` is a shared secret
    /// rather than a shared joke, so a model picks between adjacent faces on
    /// vibes. This is the shading, and it is written once — `mcp.rs` hands it
    /// to the model with the enum rather than transcribing it, the same trade
    /// `catalog.rs` makes with visage.
    ///
    /// The shape: what it means, then — after a dash, and only where the
    /// meaning alone leaves the moment ambiguous — when it is for. "you" is
    /// Peter. A line, not a paragraph: the whole vocabulary rides in one tool
    /// description, every turn.
    pub fn gloss(self) -> &'static str {
        match self {
            Idle => {
                "nothing worn — the resting face, and where a posture goes when its work is done"
            }
            Thinking => "working it out, and the working is visible",
            Speaking => "mid-sentence; the face moving with the words",
            Blocked => {
                "stuck on something outside us both — a refusal, a missing key, a dead end; not anything you did"
            }
            Away => "here, but not present",
            Trouble => "something is wrong and you know it — alarm, not annoyance",
            Yes => "agreement, without needing to say it",
            No => "disagreement, or it cannot be done",
            Curious => "a question forming — the interesting kind of not-knowing",
            Surprised => "it landed differently than expected, and it shows at once",
            Amused => "the joke landed; lighter than delighted, warmer than smug",
            Wince => "that hurt, or it is about to — or the fix that keeps almost working",
            Listening => "attention on you, nothing else running",
            Skimming => {
                "reading fast — a file, a page, search results — looking for the place to stop"
            }
            Digging => "deep in it, and it will take a while",
            Smug => "right, and having known it — nailed it, and not making a thing of it",
            Sheepish => "mine, and I would rather it weren't — the awkward laugh at my own slip",
            Deadpan => "the joke is that nothing moved",
            Fond => "warmth, unhedged — given, or received when you are kind",
            Melancholy => "real weight, worn lightly and then set down",
            Conspiratorial => "a shared secret, not a shared joke — a wink, just between us",
            Delighted => "full brightness — rare on purpose, so spend it",
            Suspicious => "that does not add up yet",
            Pointing => "look at the thing I am showing you",
            Waiting => {
                "something I started is running and I am letting it — a build, a script, a download"
            }
            Weighing => "two good options on the scale — the moment before a recommendation",
            Vibing => "music on and I am into it — eyes shut, head nodding along",
            Disapproval => "I saw that — a verdict, where suspicious is still a doubt",
            EyeRoll => {
                "oh, come on — at you or at the situation, both fair: the third retry, the doc that lied"
            }
            DoubleTake => "wait — what? surprise that lands a beat late, on the second look",
            Shrug => "no idea, and at peace with it — why it works now, nobody knows",
            TableFlip => {
                "done with this, played for laughs — the fifth flaky failure; over in seconds"
            }
            Dead => "that killed me — laughing too hard to speak; or it crashed",
            Dizzy => "too much at once — the 400-line stack trace, the head spinning",
            Glitch => "that broke something in me — a cursed bug, input that does not compute",
            Creeper => "mischief incoming — about to do something gleefully chaotic",
            Uwu => "aww — cuteness overload; a joke, so spend it rarely",
        }
    }

    /// A posture is worn until parker changes it; a reaction lets go on its
    /// own. The split the `face` tool hands a model, because the two want
    /// opposite habits from the caller.
    pub fn is_posture(self) -> bool {
        matches!(
            self,
            Idle | Thinking
                | Speaking
                | Blocked
                | Away
                | Trouble
                | Listening
                | Skimming
                | Digging
                | Pointing
                | Waiting
                | Weighing
                | Vibing
        )
    }

    /// The postures that describe parker *doing* something inside a turn.
    /// Once the turn is over they are false by definition — he is not digging
    /// anymore — so the turn's end takes them off (`state::settle`). The other
    /// postures (idle, listening, blocked, away, trouble, waiting, vibing)
    /// describe a state that can honestly outlast a turn, and stay: a job left
    /// running is still running after he stops talking, and so is the music.
    ///
    /// Exists because parker forgets: a `pointing` left on after the work was
    /// done sat on the face, goofy, until Peter asked about it (Oct 2026).
    pub fn is_work(self) -> bool {
        matches!(
            self,
            Thinking | Speaking | Skimming | Digging | Pointing | Weighing
        )
    }

    pub fn parse(s: &str) -> Option<Expression> {
        let s = s.trim().to_lowercase();
        Expression::ALL.iter().copied().find(|e| e.name() == s)
    }

    /// How long a chosen expression holds when no duration was given. The real
    /// end of a beat is the next thing parker does; these are only a safety
    /// net against one sticking forever when nothing follows. The gestures and
    /// jokes are shorter on purpose: a table flipped for twenty seconds is not
    /// a joke anymore.
    pub fn default_hold(self) -> f64 {
        match self {
            Yes | No | Surprised | Wince | Disapproval => 12.0,
            Curious | Amused => 20.0,
            Smug | Sheepish | Deadpan | Fond | Melancholy | Conspiratorial | Delighted
            | Suspicious => 20.0,
            EyeRoll | DoubleTake | Shrug | Dizzy => 10.0,
            Dead | Creeper | Uwu => 8.0,
            TableFlip | Glitch => 6.0,
            // Postures are meant to be taken off by parker, and work postures
            // also come off at the turn's end. This is the net under both.
            _ => POSTURE_CAP,
        }
    }

    pub fn motion(self) -> Motion {
        match self {
            Idle | Curious => Motion::Drift,
            Thinking | Digging => Motion::Blades,
            Speaking => Motion::Wave,
            Surprised | Listening | Fond | Melancholy | Uwu => Motion::Breathe,
            Trouble => Motion::Pulse,
            Yes => Motion::Nod,
            No => Motion::Shake,
            Blocked => Motion::Bonk,
            Skimming => Motion::Read,
            Amused => Motion::Giggle,
            Wince => Motion::Tremble,
            Delighted => Motion::Bounce,
            Suspicious => Motion::Shifty,
            Waiting => Motion::Sand,
            Weighing => Motion::Tip,
            Vibing => Motion::Vibe,
            Dizzy => Motion::Orbit,
            Glitch => Motion::Glitch,
            Creeper => Motion::Fuse,
            Pointing => Motion::Jab,
            Away | Sheepish | Deadpan | Smug | Conspiratorial | Disapproval | EyeRoll
            | DoubleTake | Shrug | TableFlip | Dead => Motion::None,
        }
    }

    /// Whether the ordinary blink may close this face. Two must not: the
    /// hourglass's bulbs are hollowed eyes, and a blink would squash the
    /// glass; and a wink whose open eye blinks is two eyes shut.
    pub fn blinks(self) -> bool {
        !matches!(self, Waiting | Conspiratorial)
    }

    /// How this face lands, when it does more than spring into place
    /// (entrance.rs).
    pub fn entrance(self) -> Option<&'static beat::Beat> {
        entrance::of(self)
    }

    pub fn pose(self) -> Pose {
        match self {
            // The resting face. Barely doing anything, but looking around —
            // that is the whole brief, and the drift does most of it.
            Idle => Pose {
                pupil: 3.0,
                ..iris(9.0, 17.0)
            },
            Thinking => iris(4.0, 17.0),
            Speaking => line(46.0, 10.0, 3.0),

            // Stopped by something outside: a brick wall, and parker walking
            // into it twice a loop (Bonk holds the wall still). It used to be
            // two eyes staring — 85% the same dots as listening.
            Blocked => Pose {
                x: -5.0,
                gaze_x: 2.6,
                mark: [
                    s(36.0, -17.0, 36.0, 17.0, 3.4),
                    s(36.0, -9.0, 51.0, -9.0, 1.6),
                    s(36.0, 0.0, 51.0, 0.0, 1.6),
                    s(36.0, 9.0, 51.0, 9.0, 1.6),
                    s(43.0, -9.0, 43.0, 0.0, 1.6),
                    s(47.0, 0.0, 47.0, 9.0, 1.6),
                ],
                ..eyes(6.5, 8.0, 2.8)
            },
            Away => line(8.0, 0.0, 0.0),

            // Alarm is *contraction*, not dilation. The first version was big
            // wide red eyes, which read as bloodshot rather than alarmed — a
            // small tight aperture locked on one point reads as "I see it".
            // Only the ring and pupil take the colour, so the red is a
            // detail rather than a wash.
            Trouble => Pose {
                pupil: 1.8,
                ring_w: 3.0,
                ink: Ink::Bad,
                ..iris(5.0, 10.0)
            },

            // Not a face. A check, drawn.
            Yes => Pose {
                mark: Stroke::pad(&[stroke(-11.0, 1.0, -3.0, 9.0), stroke(-3.0, 9.0, 13.0, -9.0)]),
                mark_ink: [Ink::Good; pose::MARKS],
                ..Pose::ZERO
            },
            // Not a face. A cross.
            No => Pose {
                mark: Stroke::pad(&[
                    stroke(-10.0, -10.0, 10.0, 10.0),
                    stroke(10.0, -10.0, -10.0, 10.0),
                ]),
                mark_ink: [Ink::Bad; pose::MARKS],
                ..Pose::ZERO
            },

            // One brow up, one flat, and a tilt: the question. It was the
            // right eye squinted, which read as lopsided.
            Curious => Pose {
                tilt: -1.5,
                gaze_y: -1.2,
                gaze_x: 0.8,
                mark: Stroke::pad(&[
                    s(-25.0, -12.5, -11.0, -12.5, 2.0),
                    s(11.0, -15.0, 17.0, -18.5, 2.0),
                    s(17.0, -18.5, 25.0, -16.5, 2.0),
                ]),
                ..eyes(7.5, 9.0, 3.2)
            },
            // A single wide aperture with a tiny pupil — dilation reads as
            // surprise where it read as bloodshot on Trouble, because here
            // there is nothing else competing with it.
            Surprised => Pose {
                pupil: 2.0,
                ..iris(15.0, 19.0)
            },
            // ^ ^, drawn, chuckling in bursts (Giggle).
            Amused => Pose {
                mark: Stroke::pad(&[
                    caret(-18.0, 1.0, 7.0, 5.5, 2.6)[0],
                    caret(-18.0, 1.0, 7.0, 5.5, 2.6)[1],
                    caret(18.0, 1.0, 7.0, 5.5, 2.6)[0],
                    caret(18.0, 1.0, 7.0, 5.5, 2.6)[1],
                ]),
                ..Pose::ZERO
            },
            // >_< — squeezed shut, the little flat mouth, trembling. The
            // kaomoji for a wince, and for effort that is not working yet.
            Wince => Pose {
                mark: Stroke::pad(&[
                    s(-27.0, -6.0, -15.0, 0.0, 2.6),
                    s(-15.0, 0.0, -27.0, 6.0, 2.6),
                    s(27.0, -6.0, 15.0, 0.0, 2.6),
                    s(15.0, 0.0, 27.0, 6.0, 2.6),
                    s(-4.0, 9.0, 4.0, 9.0, 2.0),
                ]),
                ..Pose::ZERO
            },

            // ── the redesign's twelve. Numbers are the handoff's, verbatim
            // (docs/visage/redesign/parker-face.js), except where the faces
            // round redrew them; the grid bound to watch is
            // ring + ring_w/2 <= 20, and the largest here is 18.
            Listening => Pose {
                gaze_y: 0.6,
                squint: 0.92,
                ..eyes(7.5, 10.0, 3.2)
            },
            // Reading, for real: the gaze steps along a line, snaps back and
            // drops a line (Read). It was the thinking iris, smaller — at a
            // glance the three work postures were one face at three sizes.
            Skimming => eyes(6.5, 6.5, 2.6),
            Digging => Pose {
                ring_w: 2.8,
                ..iris(3.0, 16.0)
            },
            // (⌐■_■): the shades come down from above (the entrance) and
            // stay on.
            Smug => Pose {
                y: -1.0,
                mark: shades(-1.0),
                ..eyes(6.0, 3.5, 2.0)
            },
            // ^^; — the smile that is not quite working, and a drop beside
            // the head.
            Sheepish => Pose {
                y: -1.0,
                mark: Stroke::pad(&[s(31.0, -17.0, 31.0, -12.0, 1.6), dot(31.0, -10.0, 4.6)]),
                mark_ink: [Ink::Accent; pose::MARKS],
                ..eyes(7.5, 2.6, 0.0)
            },
            Deadpan => Pose {
                round: 0.0,
                ..eyes(9.0, 1.6, 0.0)
            },
            // Shut eyes and blush, ///, three a cheek, in the red ink — the
            // nearest thing to pink. Amused's old pills in blue were 78% the
            // same dots as amused.
            Fond => Pose {
                y: -3.0,
                round: 1.0,
                ink: Ink::Accent,
                mark: [
                    s(-25.0, 7.0, -23.0, 3.0, 1.6),
                    s(-20.0, 7.0, -18.0, 3.0, 1.6),
                    s(-15.0, 7.0, -13.0, 3.0, 1.6),
                    s(13.0, 7.0, 15.0, 3.0, 1.6),
                    s(18.0, 7.0, 20.0, 3.0, 1.6),
                    s(23.0, 7.0, 25.0, 3.0, 1.6),
                ],
                mark_ink: [Ink::Bad; pose::MARKS],
                ..eyes(7.0, 2.6, 0.0)
            },
            // Both eyes heavy and lowered, brows lifted in the middle, the
            // face sunk a little.
            Melancholy => Pose {
                gaze_y: 2.4,
                y: 2.5,
                mark: Stroke::pad(&[
                    s(-26.0, -8.0, -12.0, -11.0, 1.8),
                    s(12.0, -11.0, 26.0, -8.0, 1.8),
                ]),
                ..eyes(7.5, 5.0, 2.6)
            },
            // A wink, at you: one open eye looking straight out, the other
            // shut as a ^ drawn in strokes. The open eye is the single cell,
            // moved to where the left eye sits; the strokes stand where the
            // right one was. It snaps shut in the entrance.
            Conspiratorial => Pose {
                gap: 0.0,
                x: -18.0,
                gaze_x: 0.4,
                mark: Stroke::pad(&[s(29.0, 0.5, 36.0, -4.0, 2.6), s(36.0, -4.0, 43.0, 0.5, 2.6)]),
                ..eyes(7.5, 8.5, 3.2)
            },
            // Beaming: big ^ ^ in the accent ink, hopping, one sparkle. It
            // was shut eyes inside a ring, which read as a porthole.
            Delighted => Pose {
                ink: Ink::Accent,
                mark: [
                    caret(-18.0, 2.0, 9.0, 8.0, 3.0)[0],
                    caret(-18.0, 2.0, 9.0, 8.0, 3.0)[1],
                    caret(18.0, 2.0, 9.0, 8.0, 3.0)[0],
                    caret(18.0, 2.0, 9.0, 8.0, 3.0)[1],
                    s(33.0, -13.0, 39.0, -13.0, 1.6),
                    s(36.0, -16.0, 36.0, -10.0, 1.6),
                ],
                mark_ink: [Ink::Accent; pose::MARKS],
                ..Pose::ZERO
            },
            // ¬_¬ — both eyes narrowed to slits, side-eye, and a check the
            // other way (Shifty). It was conspiratorial's mirror image.
            Suspicious => Pose {
                round: 0.35,
                gaze_x: -4.4,
                ..eyes(8.0, 3.2, 2.0)
            },
            // Both eyes looking hard right, moved left to make room, and an
            // arrow with a head well clear of them that pokes twice a loop
            // (Jab). It was one iris with a headless shaft through its ring:
            // a round body, an eye and a beak, which read as a penguin.
            Pointing => Pose {
                x: -12.0,
                gaze_x: 2.6,
                mark: Stroke::pad(&[
                    stroke(31.0, 0.0, 54.0, 0.0),
                    stroke(47.0, -7.0, 54.0, 0.0),
                    stroke(47.0, 7.0, 54.0, 0.0),
                ]),
                mark_ink: [Ink::Accent; pose::MARKS],
                ..eyes(6.5, 8.0, 2.6)
            },

            // ── the faces round's thirteen.
            // An hourglass. The bulbs are the two cells, hollowed by a pupil
            // as wide as the eye and stacked by `tilt`; the bars and the sand
            // are strokes, and Sand moves the sand. Not a progress bar, which
            // would claim to know how far along the job is.
            Waiting => Pose {
                gap: 0.6,
                w: 7.0,
                h: 6.6,
                round: 1.0,
                pupil: 5.6,
                y: -6.0,
                tilt: 12.0,
                mark: Stroke::pad(&[
                    s(-6.0, -7.6, 6.0, -7.6, 2.4),
                    s(-6.0, 19.6, 6.0, 19.6, 2.4),
                    s(-0.6, 0.64, 0.6, 0.64, 8.3),
                ]),
                mark_ink: Ink::pad(&[
                    Ink::Plain,
                    Ink::Plain,
                    Ink::Accent,
                    Ink::Accent,
                    Ink::Accent,
                ]),
                ..Pose::ZERO
            },
            // A balance: beam, post, base, two pans, the pivot. Tip rocks the
            // beam and carries the pans.
            Weighing => Pose {
                mark: [
                    s(-20.0, -9.0, 20.0, -9.0, 2.0),
                    s(0.0, -9.0, 0.0, 12.0, 2.0),
                    s(-9.0, 12.5, 9.0, 12.5, 2.6),
                    s(-26.0, 0.0, -14.0, 0.0, 2.4),
                    s(14.0, 0.0, 26.0, 0.0, 2.4),
                    dot(0.0, -10.5, 4.2),
                ],
                ..Pose::ZERO
            },
            // Eyes shut, head going, one note in the air.
            Vibing => Pose {
                round: 0.7,
                y: -1.0,
                mark: Stroke::pad(&[
                    dot(34.0, -5.0, 4.4),
                    s(35.8, -5.5, 35.8, -14.0, 1.4),
                    s(35.8, -14.0, 39.5, -11.0, 1.4),
                ]),
                mark_ink: [Ink::Accent; pose::MARKS],
                ..eyes(7.0, 1.5, 0.0)
            },
            // ಠ_ಠ — round eyes, dead-level brows, a flat little mouth, and
            // no motion at all.
            Disapproval => Pose {
                round: 1.0,
                mark: Stroke::pad(&[
                    s(-25.0, -10.5, -11.0, -10.5, 2.0),
                    s(11.0, -10.5, 25.0, -10.5, 2.0),
                    s(-4.0, 9.0, 4.0, 9.0, 2.0),
                ]),
                ..eyes(6.5, 6.5, 2.4)
            },
            // Where the roll ends: a look at the ceiling, lids down a little.
            EyeRoll => Pose {
                gaze_y: -3.0,
                gaze_x: 1.0,
                y: 0.5,
                ..eyes(7.5, 6.0, 3.0)
            },
            // Where the take ends: wide, pupils pinned, leaning in.
            DoubleTake => Pose {
                gaze_x: 3.4,
                y: -1.5,
                ..eyes(8.5, 11.0, 1.8)
            },
            // ¯\_(ツ)_/¯ — the ring is the head, the eyes close to dashes,
            // and six strokes are exactly the two arms.
            Shrug => Pose {
                ring: 11.0,
                ring_w: 1.6,
                gap: 4.5,
                w: 2.2,
                h: 1.1,
                round: 1.0,
                y: -1.0,
                mark: shrug_arms(0.0),
                ..Pose::ZERO
            },
            // (╯°□°)╯︵ ┻━┻ — where the flip ends: arms up, eyes wide, the
            // table on its back. The face sits left; strokes are written
            // from its centre, so the table's x carries the shift.
            TableFlip => Pose {
                x: -14.0,
                gap: 9.0,
                w: 4.2,
                h: 5.0,
                round: 1.0,
                pupil: 1.3,
                mark: Stroke::pad(&[
                    s(-11.0, 2.0, -19.0, -10.0, 2.2),
                    s(11.0, 2.0, 19.0, -10.0, 2.2),
                    s(55.0, 14.0, 33.0, 14.0, 2.6),
                    s(52.0, 14.0, 52.0, 6.0, 2.0),
                    s(36.0, 14.0, 36.0, 6.0, 2.0),
                ]),
                ..Pose::ZERO
            },
            // x_x, as in "I'm dead".
            Dead => Pose {
                mark: Stroke::pad(&[
                    s(-24.0, -6.0, -12.0, 6.0, 2.4),
                    s(-12.0, -6.0, -24.0, 6.0, 2.4),
                    s(12.0, -6.0, 24.0, 6.0, 2.4),
                    s(24.0, -6.0, 12.0, 6.0, 2.4),
                    s(-5.0, 11.0, 5.0, 11.0, 2.0),
                ]),
                ..Pose::ZERO
            },
            // Pupils rolling round, three stars on the ellipse Orbit drives.
            Dizzy => Pose {
                gap: 16.0,
                round: 1.0,
                mark: [
                    plus(24.0, -14.0)[0],
                    plus(24.0, -14.0)[1],
                    plus(-12.0, -10.9)[0],
                    plus(-12.0, -10.9)[1],
                    plus(-12.0, -17.1)[0],
                    plus(-12.0, -17.1)[1],
                ],
                mark_ink: [Ink::Accent; pose::MARKS],
                ..eyes(5.5, 5.5, 2.2)
            },
            // The resting face, and Glitch tears it.
            Glitch => Pose {
                pupil: 3.0,
                ..iris(9.0, 17.0)
            },
            // A creeper on its 8×8 pixel grid at 4.5 dots a pixel: square
            // eyes (round 0), the mouth and its legs, all green. Fuse flashes
            // it, faster and faster; it never actually goes off.
            Creeper => Pose {
                gap: 9.0,
                w: 4.5,
                h: 4.5,
                round: 0.0,
                y: -9.0,
                ink: Ink::Good,
                mark: [
                    s(-2.2, 6.75, 2.2, 6.75, 4.4),
                    s(-7.4, 10.5, 7.4, 10.5, 3.0),
                    s(-7.4, 13.5, 7.4, 13.5, 3.0),
                    s(-7.4, 16.5, 7.4, 16.5, 3.0),
                    s(-6.75, 19.2, -6.75, 21.3, 4.2),
                    s(6.75, 19.2, 6.75, 21.3, 4.2),
                ],
                mark_ink: [Ink::Good; pose::MARKS],
                ..Pose::ZERO
            },
            Uwu => Pose {
                gap: 17.0,
                y: -3.0,
                round: 1.0,
                mark: [
                    s(-8.0, 4.0, -4.0, 8.0, 1.6),
                    s(-4.0, 8.0, 0.0, 5.0, 1.6),
                    s(0.0, 5.0, 4.0, 8.0, 1.6),
                    s(4.0, 8.0, 8.0, 4.0, 1.6),
                    s(-28.0, 3.0, -23.0, 3.0, 1.8),
                    s(23.0, 3.0, 28.0, 3.0, 1.8),
                ],
                mark_ink: Ink::pad(&[
                    Ink::Plain,
                    Ink::Plain,
                    Ink::Plain,
                    Ink::Plain,
                    Ink::Bad,
                    Ink::Bad,
                ]),
                ..eyes(5.5, 1.6, 0.0)
            },
        }
    }
}

/// A drawn frame: the braille rows, plus which ink each cell belongs to.
/// Colour is applied separately (paint.rs) so the rows stay plain text for
/// tests, exports, and any surface that does its own colouring.
pub struct Frame {
    pub rows: Vec<String>,
    pub inks: Vec<u8>,
}

/// Draw a pose.
///
/// `phase` is 0..1 through the idle loop; `t` is absolute seconds, for the
/// motions that must not repeat on it (pose.rs::saccade).
pub fn draw(p: Pose, motion: Motion, phase: f64, t: f64) -> Frame {
    let mut g = Grid::new(COLS, ROWS);
    // Motion first: it can move `y`, and reading the centre before applying it
    // silently discards every vertical motion (Nod drew a perfectly still nod).
    let p = motion.apply(p, phase, t);
    let (cx, cy) = (g.w as f64 / 2.0 + p.x, g.h as f64 / 2.0 + p.y);

    g.use_ink(p.ink.slot());

    if p.ring > 1.0 && p.ring_w > 0.1 {
        ring(&mut g, cx, cy, p.ring, p.ring_w, p.sweep, p.spin);
    }
    if motion == Motion::Blades {
        for k in 0..3 {
            let a = phase * TAU + k as f64 * TAU / 3.0;
            arc(&mut g, cx, cy, (p.ring - 5.0).max(4.0), a, a + 0.7, 2.0);
        }
    }

    // One cell or two. As `gap` closes the two overlap and read as one, which
    // is what makes eyes-into-an-iris a single lerp.
    let centres: Vec<(f64, f64, f64)> = if p.gap < 0.6 {
        vec![(cx, cy, 1.0)]
    } else {
        vec![(cx - p.gap, cy, 1.0), (cx + p.gap, cy + p.tilt, p.squint)]
    };

    for (ex, ey, hscale) in centres {
        let h = (p.h * hscale).max(0.5);
        if p.w > 0.05 && h > 0.05 {
            cell(&mut g, ex, ey, p.w, h, p.round, p.amp, p.freq, phase);
            if p.pupil > 0.3 && p.amp == 0.0 {
                // A hole, not a dot: the pupil is where the cell is not.
                g.clear_ellipse(
                    ex + p.gaze_x,
                    ey + p.gaze_y,
                    p.pupil,
                    (p.pupil * hscale).max(0.6),
                );
            }
        }
    }

    // Marks last and each in its own ink, so a check reads green over
    // whatever the face was doing underneath, and a white scan line can
    // share the stage with a red contact.
    for (i, m) in p.mark.iter().enumerate() {
        if m.w > 0.05 {
            g.use_ink(p.mark_ink[i].slot());
            g.stroke(cx + m.x0, cy + m.y0, cx + m.x1, cy + m.y1, m.w);
        }
    }

    Frame {
        inks: g.cell_inks(),
        rows: g.to_lines(),
    }
}

/// A filled cell: a rounded rectangle, optionally displaced along its length
/// into a wave. One routine covers an eye, an iris, a flat line and a
/// waveform, which is what lets them interpolate into each other.
#[allow(clippy::too_many_arguments)]
fn cell(g: &mut Grid, cx: f64, cy: f64, w: f64, h: f64, round: f64, amp: f64, freq: f64, ph: f64) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = round.clamp(0.0, 1.0) * w.min(h);
    let flat = (w - r).max(0.0);

    let steps = (w.ceil() as i32).max(1);
    for dx in -steps..=steps {
        let x = dx as f64;
        if x.abs() > w {
            continue;
        }
        let half = if x.abs() <= flat {
            h
        } else {
            let d = x.abs() - flat;
            (r * r - d * d).max(0.0).sqrt() + (h - r).max(0.0)
        };
        let disp = if amp > 0.0 {
            amp * ((x / w.max(1.0)) * freq * TAU + ph * TAU).sin()
        } else {
            0.0
        };
        let top = cy + disp;
        let hi = half.ceil() as i32;
        for dy in -hi..=hi {
            if (dy as f64).abs() <= half {
                g.set((cx + x).round() as i32, (top + dy as f64).round() as i32);
            }
        }
    }
}

/// A ring, whole or partial. `spin` turns where a partial one starts, in
/// turns from straight up; a whole ring ignores it, having no visible seam to
/// turn.
fn ring(g: &mut Grid, cx: f64, cy: f64, r: f64, w: f64, sweep: f64, spin: f64) {
    let sweep = sweep.clamp(0.0, 1.0);
    if sweep >= 1.0 {
        g.ring(
            cx.round() as i32,
            cy.round() as i32,
            r.round() as i32,
            r.round() as i32,
            w.round().max(1.0) as i32,
        );
    } else {
        let from = -TAU / 4.0 + spin * TAU;
        arc(g, cx, cy, r, from, from + sweep * TAU, w);
    }
}

fn arc(g: &mut Grid, cx: f64, cy: f64, r: f64, from: f64, to: f64, weight: f64) {
    let steps = (((to - from).abs() * r).max(8.0)) as i32;
    let rad = (weight / 2.0).round().max(1.0) as i32;
    for i in 0..=steps {
        let a = from + (to - from) * i as f64 / steps as f64;
        g.ellipse(
            (cx + r * a.cos()).round() as i32,
            (cy + r * a.sin()).round() as i32,
            rad,
            rad,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dots(f: &Frame) -> usize {
        f.rows
            .iter()
            .flat_map(|l| l.chars())
            .map(|c| (c as u32 - 0x2800).count_ones() as usize)
            .sum()
    }

    fn frame(e: Expression, phase: f64) -> Frame {
        draw(e.pose(), e.motion(), phase, phase * 4.0)
    }

    #[test]
    fn every_expression_draws_and_fits() {
        for e in Expression::ALL {
            let f = frame(e, 0.5);
            assert_eq!(f.rows.len(), ROWS as usize, "{}", e.name());
            for line in &f.rows {
                assert_eq!(line.chars().count(), COLS as usize, "{}", e.name());
            }
            assert_eq!(f.inks.len(), (COLS * ROWS) as usize, "{}", e.name());
            let floor = if e == Away { 12 } else { 60 };
            assert!(dots(&f) > floor, "{} is nearly blank", e.name());
        }
    }

    #[test]
    fn names_round_trip_and_nonsense_is_rejected() {
        for e in Expression::ALL {
            assert_eq!(Expression::parse(e.name()), Some(e));
        }
        assert_eq!(Expression::parse("  CURIOUS "), Some(Curious));
        assert_eq!(Expression::parse("squirrel"), None);
        assert_eq!(Expression::parse("working"), None, "merged into thinking");
    }

    #[test]
    fn every_face_is_parkers_to_claim() {
        // The regression this guards is a surface growing its own opinion
        // again. `Trouble` is the one that keeps trying: it was the last
        // carve-out, and an engine bouncing behind a 2s reconnect spent it
        // seven times in fifteen seconds while parker said nothing.
        for e in Expression::ALL {
            assert!(
                Expression::WEARABLE.contains(&e),
                "{} is not parker's to wear",
                e.name()
            );
        }
    }

    #[test]
    fn reactions_let_go_and_postures_do_not() {
        for e in Expression::VOLUNTARY {
            assert!(e.default_hold() > 0.0, "{} would stick forever", e.name());
        }
        // Postures hold for the cap, never forever by default.
        for e in [Idle, Listening, Skimming, Digging, Pointing, Away] {
            assert_eq!(e.default_hold(), POSTURE_CAP, "{} should hold", e.name());
            assert!(e.is_posture(), "{} should be a posture", e.name());
        }
    }

    /// Every face is exactly one of posture or reaction, and the reactions are
    /// the ones that spring in. If these ever disagree, the `face` tool is
    /// describing a face as one kind and the machinery is treating it as the
    /// other.
    #[test]
    fn postures_and_reactions_partition_the_faces() {
        for e in Expression::ALL {
            assert_ne!(
                e.is_posture(),
                Expression::VOLUNTARY.contains(&e),
                "{} is both or neither",
                e.name()
            );
            assert!(e.default_hold() > 0.0, "{} would stick forever", e.name());
        }
    }

    #[test]
    fn work_postures_are_postures_and_states_are_not_work() {
        for e in Expression::ALL.into_iter().filter(|e| e.is_work()) {
            assert!(e.is_posture(), "{} is work but not a posture", e.name());
        }
        for e in [Idle, Listening, Blocked, Away, Trouble] {
            assert!(!e.is_work(), "{} should outlast a turn", e.name());
        }
    }

    /// A face nobody defined is a face picked on vibes. New expression, new
    /// line of gloss — the vocabulary parker is handed is generated from this,
    /// so an empty one ships an empty definition.
    #[test]
    fn every_expression_says_what_it_means() {
        for e in Expression::ALL {
            let g = e.gloss();
            assert!(!g.is_empty(), "{} has no gloss", e.name());
            assert!(
                !g.ends_with('.'),
                "{}: gloss is a phrase, not a sentence",
                e.name()
            );
        }
    }

    #[test]
    fn a_transition_between_any_two_expressions_stays_on_canvas() {
        for a in Expression::ALL {
            for b in Expression::ALL {
                for step in [0.25, 0.5, 0.75] {
                    let f = draw(Pose::lerp(a.pose(), b.pose(), step), Motion::None, 0.5, 0.0);
                    assert_eq!(f.rows.len(), ROWS as usize, "{}->{}", a.name(), b.name());
                    for line in &f.rows {
                        assert_eq!(line.chars().count(), COLS as usize);
                    }
                }
            }
        }
    }

    #[test]
    fn a_glyph_morphs_out_of_a_face_rather_than_cutting_to_it() {
        // Halfway from eyes to a check: the eyes must still be shrinking and
        // the strokes must already be drawing. If either is absent, the
        // "everything is a lerp" promise is broken for marks.
        let mid = Pose::lerp(Listening.pose(), Yes.pose(), 0.5);
        assert!(mid.w > 0.0 && mid.w < Listening.pose().w, "eyes shrinking");
        assert!(mid.mark[0].w > 0.0, "the check is already being drawn");
        assert!(
            mid.mark[0].w < Yes.pose().mark[0].w,
            "and not yet full weight"
        );
    }

    #[test]
    fn marks_carry_their_own_ink() {
        let yes = frame(Yes, 0.5);
        let no = frame(No, 0.5);
        assert!(
            yes.inks.contains(&Ink::Good.slot()),
            "the check is good-inked"
        );
        assert!(no.inks.contains(&Ink::Bad.slot()), "the cross is bad-inked");
        assert!(
            !frame(Idle, 0.5).inks.iter().any(|i| *i != 0),
            "an ordinary face is all one ink"
        );
    }

    #[test]
    fn trouble_is_focused_rather_than_bloodshot() {
        // The first version was big wide red eyes, which read as stoned.
        // Alarm is contraction: it must be smaller than the resting face.
        assert!(Trouble.pose().w < Idle.pose().w, "alarm contracts");
        assert!(Trouble.pose().ring < Idle.pose().ring, "and tightens");
        assert!(
            Trouble.pose().pupil < Idle.pose().pupil,
            "pupil contracts too"
        );
        assert!(dots(&frame(Trouble, 0.5)) < dots(&frame(Surprised, 0.5)));
    }

    #[test]
    fn away_is_the_quietest_expression_there_is() {
        let away = dots(&frame(Away, 0.5));
        for e in Expression::ALL {
            if e != Away {
                assert!(
                    away < dots(&frame(e, 0.5)),
                    "{} is quieter than away",
                    e.name()
                );
            }
        }
    }

    #[test]
    fn animated_expressions_actually_move() {
        for e in Expression::ALL {
            if e.motion() == Motion::None {
                continue;
            }
            let seen: std::collections::HashSet<_> =
                (0..37).map(|i| frame(e, i as f64 / 37.0).rows).collect();
            assert!(seen.len() > 1, "{} never moves", e.name());
        }
    }

    #[test]
    fn a_nod_is_fast_enough_to_read_as_a_nod() {
        // One cycle per four-second loop was far too slow to gauge.
        let ys: Vec<i32> = (0..40)
            .map(|i| {
                Motion::Nod
                    .apply(Yes.pose(), i as f64 / 40.0, 0.0)
                    .y
                    .round() as i32
            })
            .collect();
        let reversals = ys
            .windows(3)
            .filter(|w| (w[1] - w[0]).signum() != (w[2] - w[1]).signum())
            .count();
        assert!(
            reversals >= 8,
            "only {reversals} direction changes per loop"
        );
    }

    /// The bar for a face is that it reads as itself (the header). Measured
    /// the way the faces round found the problem: the dots two held faces
    /// share, each grown by one dot so a one-dot shift is not a difference.
    /// Before the round, blocked and listening shared 85%, suspicious and
    /// conspiratorial 83% — mirror images.
    #[test]
    fn no_two_faces_look_alike() {
        let lit = |e: Expression| -> std::collections::HashSet<(i32, i32)> {
            let f = draw(e.pose(), Motion::None, 0.0, 0.0);
            let mut out = std::collections::HashSet::new();
            for (row, line) in f.rows.iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    let bits = ch as u32 - 0x2800;
                    for (i, (dx, dy)) in [
                        (0, 0),
                        (0, 1),
                        (0, 2),
                        (1, 0),
                        (1, 1),
                        (1, 2),
                        (0, 3),
                        (1, 3),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if bits & (1 << i) != 0 {
                            let (x, y) = (col as i32 * 2 + dx, row as i32 * 4 + dy);
                            for (gx, gy) in [
                                (0, 0),
                                (1, 0),
                                (-1, 0),
                                (0, 1),
                                (0, -1),
                                (1, 1),
                                (-1, -1),
                                (1, -1),
                                (-1, 1),
                            ] {
                                out.insert((x + gx, y + gy));
                            }
                        }
                    }
                }
            }
            out
        };
        // Same pose by design: glitch is the resting face, torn by its motion.
        let twins = [(Idle, Glitch)];
        let shots: Vec<_> = Expression::ALL.iter().map(|e| (*e, lit(*e))).collect();
        let mut worst = (0.0, Idle, Idle);
        for (i, (a, sa)) in shots.iter().enumerate() {
            for (b, sb) in &shots[i + 1..] {
                if twins.contains(&(*a, *b)) {
                    continue;
                }
                let shared = sa.intersection(sb).count() as f64;
                let jaccard = shared / (sa.len() as f64 + sb.len() as f64 - shared);
                if jaccard > worst.0 {
                    worst = (jaccard, *a, *b);
                }
            }
        }
        assert!(
            worst.0 < 0.8,
            "{} and {} share {:.0}% of their dots",
            worst.1.name(),
            worst.2.name(),
            worst.0 * 100.0
        );
    }

    /// The motions that move strokes keep them on the canvas: a wall that
    /// walks off the edge, or a star thrown off its orbit, is a face missing
    /// a part.
    #[test]
    fn a_face_in_motion_stays_on_the_canvas() {
        for e in Expression::ALL {
            for i in 0..48 {
                let phase = i as f64 / 48.0;
                let p = e.motion().apply(e.pose(), phase, phase * 4.0 + 7.0);
                for m in p.mark.iter().filter(|m| m.w > 0.05) {
                    for (x, y) in [(m.x0, m.y0), (m.x1, m.y1)] {
                        let (x, y) = (48.0 + p.x + x, 20.0 + p.y + y);
                        let slack = m.w / 2.0 + 1.0;
                        assert!(
                            (-slack..=96.0 + slack).contains(&x)
                                && (-slack..=40.0 + slack).contains(&y),
                            "{} at phase {phase:.2} draws a stroke at ({x:.1}, {y:.1})",
                            e.name()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_hourglass_and_the_wink_do_not_blink() {
        // The bulbs are hollowed eyes and a blink squashes the glass; a wink
        // whose open eye blinks is two eyes shut.
        assert!(!Waiting.blinks() && !Conspiratorial.blinks());
        assert!(Idle.blinks() && Listening.blinks());
    }
}
